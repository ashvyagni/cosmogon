//! Physical f64 test-particle flight, reusing finite authored vehicle inventories.
//! This module's core is being connected to the authoritative world scheduler and UI.
use crate::{
    astro::{StarSystem, G},
    supply_vehicle::{Cargo, Command, Interval, Plan, State},
    BodyRef, Universe, Vec3d,
};
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

pub const MODEL: &str = "cosmogon-finite-world-vehicle/1";
pub const CONTACT_MODEL: &str = "cosmogon-finite-world-vehicle/2";
pub const SHEAR_MODEL: &str = "cosmogon-finite-world-vehicle/3";
pub const FRAME: &str = "system-local-inertial-simulation-axes";
/// Bounded handling rule: cargo material may leave the craft at up to this
/// speed relative to the craft centre in the inertial frame. Co-moving
/// removal (zero relative velocity) is impulse-free; any bounded non-zero
/// relative velocity is an accounted, dissipative ejection, never a free
/// impulse.
pub const MAX_CARGO_EJECTION_SPEED_M_S: f64 = 1000.;
pub(crate) const SUBSTEP_S: f64 = 0.25;
pub(crate) const TASK: usize = 5;
pub(crate) const PERIOD: f64 = 1.;
pub(crate) const MAX_VEHICLES: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub model: String,
    pub frame: String,
    pub parent: BodyRef,
    pub plan: Plan,
    /// Relative to the body's actual translation, in non-rotating system axes.
    pub initial_relative_position_m: Vec3d,
    /// Added to the body's actual inertial translational velocity; not body-fixed velocity.
    pub initial_relative_velocity_m_s: Vec3d,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<crate::vehicle_contact_dynamics::Hardware>,
    /// Opt-in strictly versioned cargo mass ledgers; absent legacy state stays exact.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cargo_ledgers: bool,
    /// Authored timed species removals applied through the shared cargo ledger
    /// at the first represented propulsion-clock boundary at or after each time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cargo_removals: Vec<AuthoredCargoRemoval>,
    /// Opt-in finite surface handling equipment; absent legacy definitions stay exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handling: Option<crate::vehicle_handling::Equipment>,
    /// Opt-in finite launch structural limits and trajectory evidence; absent
    /// legacy definitions stay exact. Launch-limited craft require contact gear.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch: Option<crate::vehicle_launch::Limits>,
    /// Opt-in stagnation-point entry-heating limits and thermal evidence;
    /// absent legacy definitions stay exact. Entry-limited craft require
    /// contact gear and an atmosphere-bearing starting body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<crate::vehicle_entry::Entry>,
    /// Opt-in touchdown-speed limits and first-touchdown evidence; absent
    /// legacy definitions stay exact. Landing-limited craft require contact gear.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing: Option<crate::vehicle_landing::Limits>,
}
/// One authored, timed finite species-removal request in the craft-relative frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredCargoRemoval {
    pub at_elapsed_s: f64,
    pub requested: Cargo,
    /// Added to the craft's current inertial velocity; bounded by MAX_CARGO_EJECTION_SPEED_M_S.
    pub velocity_relative_m_s: [f64; 3],
}
impl Definition {
    pub fn with_contact(mut self, world:&Universe, hardware:crate::vehicle_contact_dynamics::Hardware)->Result<Self,String> {
        self.model=hardware.flight_model().into();self.contact=Some(hardware);
        Flight::new(world.next_vehicle_id,self.clone(),world)?;
        Ok(self)
    }
    pub fn circular_example(u: &Universe, parent: BodyRef, plan: Plan) -> Result<Self, String> {
        let body = u
            .systems
            .get(parent.system as usize)
            .and_then(|s| s.bodies.get(parent.body as usize))
            .ok_or("No such starting body")?;
        let r = body.radius + 200_000.;
        let d = Self {
            model: MODEL.into(),
            frame: FRAME.into(),
            parent,
            plan,
            initial_relative_position_m: Vec3d::new(r, 0., 0.),
            initial_relative_velocity_m_s: Vec3d::new(0., (G * body.mass / r).sqrt(), 0.),
            contact: None,
            cargo_ledgers: false,
            cargo_removals: vec![],
            handling: None,
            launch: None,
            entry: None,
            landing: None,
        };
        d.validate(u)?;
        Ok(d)
    }
    /// Opt in to strictly versioned cargo mass ledgers for this flight.
    pub fn with_cargo_ledgers(mut self) -> Result<Self, String> {
        self.cargo_ledgers = true;
        self.validate_structure()?;
        Ok(self)
    }
    /// Opt in to the versioned surface-handling equipment limits used by
    /// preflight and co-located transfers.
    pub fn with_handling(mut self) -> Result<Self, String> {
        let equipment = crate::vehicle_handling::Equipment::example();
        equipment.validate()?;
        self.handling = Some(equipment);
        self.validate_structure()?;
        Ok(self)
    }
    /// Read a versioned world-flight definition file, validating its
    /// world-independent structure before returning it.
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        let definition: Self = serde_json::from_str(&crate::habitat_recording::read(
            path,
            1024 * 1024,
        )?)
        .map_err(|e| e.to_string())?;
        definition.validate_structure()?;
        Ok(definition)
    }
    /// Write this definition to a new protected file.
    pub fn write(&self, path: &std::path::Path) -> Result<(), String> {
        use std::io::Write;
        self.validate_structure()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("Existing definitions are protected: {e}"))?;
        file.write_all(&bytes).map_err(|e| e.to_string())
    }
    pub fn validate_structure(&self) -> Result<(), String> {
        if self.frame != FRAME || !matches!((self.model.as_str(), &self.contact), (MODEL, None) | (CONTACT_MODEL | SHEAR_MODEL, Some(_))) {
            return Err("Unsupported finite flight model or reference frame".into());
        }
        if let Some(h) = &self.contact { h.validate()?; if self.model != h.flight_model() {return Err("World-flight and contact hardware model versions disagree".into());} }
        if !self.cargo_removals.is_empty() && !self.cargo_ledgers {
            return Err("Authored cargo removals require cargo ledgers".into());
        }
        for removal in &self.cargo_removals {
            let relative = Vec3d::new(
                removal.velocity_relative_m_s[0],
                removal.velocity_relative_m_s[1],
                removal.velocity_relative_m_s[2],
            );
            if !removal.at_elapsed_s.is_finite()
                || !(0.0..=1e7).contains(&removal.at_elapsed_s)
                || !crate::vehicle_cargo_mass::finite_cargo(&removal.requested)
                || !finite(relative)
                || relative.length() > MAX_CARGO_EJECTION_SPEED_M_S
            {
                return Err(
                    "Authored cargo removals require a finite 0–1e7 s clock, supported cargo and a bounded relative ejection velocity".into(),
                );
            }
        }
        if let Some(h) = &self.handling {
            h.validate()?;
            if !self.cargo_ledgers {
                return Err("Handling equipment requires cargo ledgers".into());
            }
            if !self.contact.as_ref().is_some_and(|c| c.operation.is_some()) {
                return Err(
                    "Handling equipment requires authored contact surface-operation hardware".into(),
                );
            }
        }
        if let Some(l) = &self.launch {
            l.validate()?;
            if self.contact.is_none() {
                return Err("Launch limits require authored contact landing gear".into());
            }
        }
        if let Some(e) = &self.entry {
            e.validate()?;
            if self.contact.is_none() {
                return Err("Entry limits require authored contact landing gear".into());
            }
        }
        if let Some(l) = &self.landing {
            l.validate()?;
            if self.contact.is_none() {
                return Err("Landing limits require authored contact landing gear".into());
            }
        }
        self.plan.validate()?;
        if !finite(self.initial_relative_position_m)
            || self.initial_relative_position_m.length() > 1e15
            || !finite(self.initial_relative_velocity_m_s)
            || self.initial_relative_velocity_m_s.length() > 1e6
        {
            return Err("Initial flight vectors exceed their supported finite bounds".into());
        }
        Ok(())
    }
    pub fn validate(&self, u: &Universe) -> Result<(), String> {
        self.validate_structure()?;
        let body = u
            .systems
            .get(self.parent.system as usize)
            .and_then(|s| s.bodies.get(self.parent.body as usize))
            .ok_or("No such starting body")?;
        if !body.exists()
            || !body.mass.is_finite()
            || body.mass <= 0.
            || !body.radius.is_finite()
            || body.radius <= 0.
        {
            return Err("Starting body lacks supported finite physical mass/radius".into());
        }
        if self.entry.is_some() && !body.atmosphere.is_present() {
            return Err("Entry heating requires a starting body with a modeled atmosphere".into());
        }
        if self.plan.definition.inert_mass_kg() + self.plan.definition.initial_propellant_kg
            > body.mass * 1e-6
        {
            return Err(
                "Vehicle exceeds the supported test-particle initial parent mass ratio (1e-6)"
                    .into(),
            );
        }
        if !finite(self.initial_relative_position_m)
            || self.initial_relative_position_m.length() > 1e15
            || self.contact.is_none() && self.initial_relative_position_m.length() <= body.radius
        {
            return Err("Choose a finite initial position outside the starting body's spherical boundary, within 1e15 m".into());
        }
        if !finite(self.initial_relative_velocity_m_s)
            || self.initial_relative_velocity_m_s.length() > 1e6
        {
            return Err("Initial relative velocity must be finite and at most 1e6 m/s".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body_index", rename_all = "snake_case")]
pub enum Source {
    Star,
    Companion,
    Body(u32),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GravitySample {
    pub source: Source,
    pub position_m: Vec3d,
    pub mu_m3_s2: f64,
    pub radius_m: f64,
}
fn source_samples(sources: &[Attractor]) -> Vec<GravitySample> {
    sources
        .iter()
        .map(|s| GravitySample {
            source: s.source.clone(),
            position_m: s.position_m,
            mu_m3_s2: s.mu,
            radius_m: s.radius_m,
        })
        .collect()
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Phase {
    Flying,
    Impact {
        source: Source,
        source_position_m: Vec3d,
        radius_m: f64,
        position_residual_m: f64,
        tolerance_m: f64,
        source_sample_duration_s: f64,
    },
    Unsupported {
        reason: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flight {
    pub id: u64,
    pub definition: Definition,
    pub launched_at_s: f64,
    pub last_time_s: f64,
    pub position_m: Vec3d,
    pub velocity_m_s: Vec3d,
    pub propulsion: State,
    pub last_interval: Option<Interval>,
    pub phase: Phase,
    /// Actual source parameters at the retained craft clock; terminal values are historical.
    pub last_sources: Vec<GravitySample>,
    pub(crate) edit_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<crate::vehicle_contact_dynamics::RigidState>,
    /// Opt-in solver-integrated surface-operation window evidence; created on first sample.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_ledger: Option<crate::vehicle_operation::Ledger>,
    /// Opt-in strictly versioned cargo mass ledger; absent legacy flights stay byte-exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<crate::vehicle_cargo_mass::Ledger>,
    /// Per-removal world evidence: momentum closure, bounded relative ejection and
    /// the uniform-gyration co-rotating angular share. One entry per ledger event.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cargo_events: Vec<CargoEvent>,
    /// Applied atomic habitat transfers; each consumes one co-moving cargo event
    /// and a matching habitat receipt. Absent legacy flights stay byte-exact.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transfers: Vec<crate::vehicle_transfer::Transfer>,
    /// Opt-in launch trajectory evidence; created with launch-limited
    /// definitions. Absent legacy flights stay byte-exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_ledger: Option<crate::vehicle_launch::Ledger>,
    /// Opt-in entry-heating evidence; created with entry-limited
    /// definitions. Absent legacy flights stay byte-exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_ledger: Option<crate::vehicle_entry::Ledger>,
    /// Opt-in first-touchdown evidence; created with landing-limited
    /// definitions. Absent legacy flights stay byte-exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing_ledger: Option<crate::vehicle_landing::Ledger>,
}
/// World-side evidence for one cargo removal from an authoritative flight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoEvent {
    pub removal: crate::vehicle_cargo_mass::Removal,
    pub velocity_relative_m_s: Vec3d,
    pub craft_velocity_before_m_s: Vec3d,
    pub craft_velocity_after_m_s: Vec3d,
    pub craft_angular_velocity_before_body_rad_s: Vec3d,
    pub craft_angular_velocity_after_body_rad_s: Vec3d,
    pub carried_angular_momentum_magnitude_kg_m2_s: f64,
    pub carried_rotational_energy_j: f64,
    pub vehicle_mass_before_kg: f64,
    pub vehicle_mass_after_kg: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_index: Option<usize>,
}
impl Flight {
    pub fn new(id: u64, d: Definition, u: &Universe) -> Result<Self, String> {
        d.validate(u)?;
        let s = &u.systems[d.parent.system as usize];
        let centre = s.body_state(d.parent.body as usize, u.time);
        let position = centre.pos + d.initial_relative_position_m;
        let velocity = centre.vel + d.initial_relative_velocity_m_s;
        let sources = gravity_sources(s, u.time)?;
        if !finite(position)
            || !finite(velocity)
            || velocity.length() > 1e6
            || sources
                .iter()
                .any(|b| !(d.contact.is_some() && b.source == Source::Body(d.parent.body)) && (position - b.position_m).length() <= b.radius_m)
        {
            return Err(
                "Initial craft state is invalid or inside a modeled spherical contact boundary"
                    .into(),
            );
        }
        supported_motion(position, velocity)?;
        let launch_ledger = match &d.launch {
            Some(limits) => {
                limits.validate()?;
                let clearance = crate::vehicle_contact::Surface::sample(
                    u,
                    d.parent,
                    u.time,
                    position,
                    1.,
                )?
                .radial_clearance_m;
                if clearance > limits.maximum_start_altitude_m {
                    return Err(format!(
                        "Launch definitions require a surface start at or below {} m above the local terrain (current {clearance} m)",
                        limits.maximum_start_altitude_m
                    ));
                }
                Some(crate::vehicle_launch::Ledger::new(
                    d.plan.definition.initial_propellant_kg,
                    clearance,
                ))
            }
            None => None,
        };
        let propulsion = State::new(&d.plan.definition)?;
        let cargo = if d.cargo_ledgers {
            Some(crate::vehicle_cargo_mass::Ledger::new(&d.plan.definition)?)
        } else {
            None
        };
        let contact = d.contact.as_ref().map(|h| crate::vehicle_contact_dynamics::RigidState::new(u, d.parent, position, velocity, h)).transpose()?;
        let entry_ledger = d
            .entry
            .map(|_| crate::vehicle_entry::Ledger::new());
        let landing_ledger = match &d.landing {
            Some(limits) => {
                limits.validate()?;
                let clearance = crate::vehicle_contact::Surface::sample(
                    u,
                    d.parent,
                    u.time,
                    position,
                    1.,
                )?
                .radial_clearance_m;
                Some(crate::vehicle_landing::Ledger::new(
                    clearance > limits.liftoff_altitude_m,
                    clearance,
                ))
            }
            None => None,
        };
        Ok(Self {
            id,
            definition: d,
            launched_at_s: u.time,
            last_time_s: u.time,
            position_m: position,
            velocity_m_s: velocity,
            propulsion,
            last_interval: None,
            phase: Phase::Flying,
            last_sources: source_samples(&sources),
            edit_cursor: u.edits.len() as u64,
            contact,
            operation_ledger: None,
            cargo,
            cargo_events: vec![],
            transfers: vec![],
            launch_ledger,
            entry_ledger,
            landing_ledger,
        })
    }
    /// Actual current vehicle mass: dry + held cargo (ledger epoch) + remaining propellant.
    pub fn mass_kg(&self) -> f64 {
        match &self.cargo {
            Some(ledger) => ledger.mass_kg(&self.definition.plan.definition, &self.propulsion),
            None => self.propulsion.mass_kg(&self.definition.plan.definition),
        }
    }
    pub(crate) fn pending_authored_removal(&self) -> Option<usize> {
        let elapsed = self.propulsion.elapsed_s;
        self.definition
            .cargo_removals
            .iter()
            .enumerate()
            .find(|(index, removal)| {
                removal.at_elapsed_s <= elapsed
                    && !self.cargo_events.iter().any(|e| e.authored_index == Some(*index))
            })
            .map(|(index, _)| index)
    }
    pub(crate) fn next_authored_removal_time(&self) -> Option<f64> {
        let elapsed = self.propulsion.elapsed_s;
        self.definition
            .cargo_removals
            .iter()
            .enumerate()
            .filter(|(index, removal)| {
                removal.at_elapsed_s > elapsed
                    && !self.cargo_events.iter().any(|e| e.authored_index == Some(*index))
            })
            .fold(None, |acc: Option<f64>, (_, removal)| match acc {
                Some(best) if best <= removal.at_elapsed_s => acc,
                _ => Some(removal.at_elapsed_s),
            })
    }
    pub(crate) fn apply_authored_removal(&mut self, index: usize) -> Result<CargoEvent, String> {
        let removal = self
            .definition
            .cargo_removals
            .get(index)
            .cloned()
            .ok_or("Missing authored cargo removal")?;
        let relative = Vec3d::new(
            removal.velocity_relative_m_s[0],
            removal.velocity_relative_m_s[1],
            removal.velocity_relative_m_s[2],
        );
        self.remove_cargo_inner(removal.requested, relative, Some(index))
    }
    /// Interactive bounded cargo removal through the retained ledger. Co-moving
    /// material (zero relative velocity) leaves craft velocity and, under the
    /// uniform-gyration layout rule, angular velocity unchanged; the removed
    /// material carries its linear momentum and co-rotating angular share.
    /// Eccentric (offset) cargo layouts are not representable.
    pub fn remove_cargo(
        &mut self,
        requested: Cargo,
        velocity_relative_m_s: Vec3d,
    ) -> Result<CargoEvent, String> {
        self.remove_cargo_inner(requested, velocity_relative_m_s, None)
    }
    fn remove_cargo_inner(
        &mut self,
        requested: Cargo,
        relative: Vec3d,
        authored_index: Option<usize>,
    ) -> Result<CargoEvent, String> {
        if self.phase != Phase::Flying {
            return Err("Cargo removal requires an active flying craft".into());
        }
        if self.cargo.is_none() {
            return Err("Cargo removal requires authored cargo ledgers".into());
        }
        if !finite(relative) || relative.length() > MAX_CARGO_EJECTION_SPEED_M_S {
            return Err(format!(
                "Cargo removal velocity relative to the craft must be finite and at most {MAX_CARGO_EJECTION_SPEED_M_S} m/s"
            ));
        }
        let definition = &self.definition.plan.definition;
        let mass_before = self.mass_kg();
        let velocity_before = self.velocity_m_s;
        let omega_before = self
            .contact
            .as_ref()
            .map(|c| c.angular_velocity_body_rad_s)
            .unwrap_or(Vec3d::ZERO);
        let material_velocity = velocity_before + relative;
        let ledger = self.cargo.as_mut().expect("checked above");
        let removal = ledger.remove(definition, &self.propulsion, requested, material_velocity)?;
        let mass_after = removal.final_mass_kg;
        let removed_mass = removal.actual_removed.mass_kg();
        // Linear momentum closes exactly: m_before·v_before = m_after·v_after + carried.
        let velocity_after =
            (velocity_before * mass_before - material_velocity * removed_mass) / mass_after;
        // Uniform-gyration layout rule: removed material shares the craft's
        // per-mass inertia, so its co-rotating angular momentum is exactly the
        // removed mass fraction and the craft's angular velocity is unchanged.
        let (omega_after, carried_angular_magnitude, carried_rotational_energy) =
            if let Some(hardware) = self.definition.contact.as_ref() {
                let g = hardware.gyration_radii_m;
                let delta_i = Vec3d::new(
                    g.x * g.x * removed_mass,
                    g.y * g.y * removed_mass,
                    g.z * g.z * removed_mass,
                );
                let body = Vec3d::new(
                    omega_before.x * delta_i.x,
                    omega_before.y * delta_i.y,
                    omega_before.z * delta_i.z,
                );
                (omega_before, body.length(), 0.5 * omega_before.dot(body))
            } else {
                (Vec3d::ZERO, 0., 0.)
            };
        self.velocity_m_s = velocity_after;
        let event = CargoEvent {
            removal,
            velocity_relative_m_s: relative,
            craft_velocity_before_m_s: velocity_before,
            craft_velocity_after_m_s: velocity_after,
            craft_angular_velocity_before_body_rad_s: omega_before,
            craft_angular_velocity_after_body_rad_s: omega_after,
            carried_angular_momentum_magnitude_kg_m2_s: carried_angular_magnitude,
            carried_rotational_energy_j: carried_rotational_energy,
            vehicle_mass_before_kg: mass_before,
            vehicle_mass_after_kg: mass_after,
            authored_index,
        };
        self.cargo_events.push(event.clone());
        Ok(event)
    }
    /// Validate retained state without reinterpreting original authored conditions
    /// against a body whose mass, radius or existence may have changed since launch.
    pub(crate) fn validate_saved(&self, u: &Universe) -> Result<(), String> {
        self.definition.validate_structure()?;
        if self.contact.is_some() && matches!(self.phase,Phase::Impact {..}) {
            return Err("Terrain contact cannot reinterpret a version-1 spherical terminal impact".into());
        }
        match (&self.definition.contact, &self.contact) {
            (None, None) => {},
            (Some(h), Some(state)) => state.validate(h, self)?,
            _ => return Err("Flight contact hardware/state model mismatch".into()),
        }
        match (
            &self.operation_ledger,
            self.definition.contact.as_ref().and_then(|h| h.operation.as_ref()),
        ) {
            (None, _) => {}
            (Some(ledger), Some(authored)) => {
                ledger
                    .validate()
                    .map_err(|e| format!("Invalid retained surface-operation ledger: {e}"))?;
                if ledger.model != authored.model {
                    return Err(
                        "Retained operation ledger and authored hardware models disagree".into()
                    );
                }
            }
            (Some(_), None) => {
                return Err("Operation ledger requires authored contact operation hardware".into())
            }
        }
        let system = u
            .systems
            .get(self.definition.parent.system as usize)
            .ok_or("Missing vehicle system")?;
        if self.definition.parent.body as usize >= system.bodies.len() {
            return Err("Missing original vehicle parent reference".into());
        }
        supported_motion(self.position_m, self.velocity_m_s)?;
        if self.edit_cursor > u.edits.len() as u64
            || self.last_sources.is_empty()
            || self.last_sources.len() > system.bodies.len() + 2
            || self.last_sources[0].source != Source::Star
        {
            return Err("Invalid retained gravity source inventory or edit cursor".into());
        }
        let mut source_ids = std::collections::BTreeSet::new();
        for source in &self.last_sources {
            if !source_ids.insert(source.source.clone())
                || !finite(source.position_m)
                || !source.mu_m3_s2.is_finite()
                || source.mu_m3_s2 < 0.
                || !source.radius_m.is_finite()
                || source.radius_m < 0.
                || matches!(source.source,Source::Body(i) if i as usize >= system.bodies.len())
            {
                return Err("Invalid retained gravity sample".into());
            }
        }
        let d = &self.definition.plan.definition;
        let p = &self.propulsion;
        let mut preview = p.clone();
        preview.apply(
            d,
            &Command {
                duration_s: 0.,
                throttle: 0.,
                direction: [1., 0., 0.],
            },
        )?;
        let mass_tolerance = 1e-8 * d.initial_propellant_kg.max(1.);
        if p.exhausted_propellant_kg > d.initial_propellant_kg + mass_tolerance
            || (p.propellant_kg + p.exhausted_propellant_kg - d.initial_propellant_kg).abs()
                > mass_tolerance
        {
            return Err("Retained flight propellant inventory does not close".into());
        }
        let impulse = p.exhausted_propellant_kg * d.exhaust_velocity_m_s();
        if Vec3d::new(
            p.velocity_change_m_s[0],
            p.velocity_change_m_s[1],
            p.velocity_change_m_s[2],
        )
        .length()
            > p.cumulative_ideal_delta_v_m_s
                + 1e-8 * p.cumulative_ideal_delta_v_m_s.max(1.)
            || p.powered_duration_s + p.fuel_limited_duration_s
                > p.elapsed_s + 1e-8 * p.elapsed_s.max(1.)
        {
            return Err(
                "Retained propulsion counters do not match finite mass or elapsed support".into(),
            );
        }
        match (&self.cargo, self.definition.cargo_ledgers) {
            (None, false) => {
                let capability =
                    d.exhaust_velocity_m_s() * (p.exhausted_propellant_kg / p.mass_kg(d)).dln1p();
                if (p.cumulative_impulse_n_s - impulse).abs() > 1e-8 * impulse.max(1.)
                    || (p.cumulative_ideal_delta_v_m_s - capability).abs() > 1e-8 * capability.max(1.)
                {
                    return Err(
                        "Retained propulsion counters do not match finite mass or elapsed support".into(),
                    );
                }
            }
            (Some(ledger), true) => {
                if ledger.model != crate::vehicle_cargo_mass::MODEL {
                    return Err("Unsupported retained cargo ledger model".into());
                }
                ledger
                    .validate(d, p)
                    .map_err(|e| format!("Invalid retained cargo ledger: {e}"))?;
                if self.cargo_events.len() != ledger.events.len() {
                    return Err("Cargo event evidence does not match the retained ledger".into());
                }
                let mut authored = std::collections::BTreeSet::new();
                for event in &self.cargo_events {
                    let material_velocity =
                        event.craft_velocity_before_m_s + event.velocity_relative_m_s;
                    if !finite(event.velocity_relative_m_s)
                        || event.velocity_relative_m_s.length() > MAX_CARGO_EJECTION_SPEED_M_S
                        || !finite(event.craft_velocity_before_m_s)
                        || !finite(event.craft_velocity_after_m_s)
                        || (event.removal.velocity_m_s - material_velocity).length()
                            > 1e-8 * material_velocity.length().max(1.)
                        || (event.vehicle_mass_before_kg - event.vehicle_mass_after_kg
                            - event.removal.actual_removed.mass_kg())
                            .abs()
                            > mass_tolerance
                    {
                        return Err("Invalid cargo removal evidence".into());
                    }
                    let momentum = event.craft_velocity_after_m_s * event.vehicle_mass_after_kg
                        + material_velocity * event.removal.actual_removed.mass_kg()
                        - event.craft_velocity_before_m_s * event.vehicle_mass_before_kg;
                    if momentum.length() > 1e-8 * event.vehicle_mass_before_kg.max(1.) {
                        return Err("Cargo removal momentum does not close".into());
                    }
                    if (event.craft_angular_velocity_after_body_rad_s
                        - event.craft_angular_velocity_before_body_rad_s)
                        .length()
                        > 0.
                    {
                        return Err("Cargo removal changed craft angular velocity".into());
                    }
                    if let Some(hardware) = self.definition.contact.as_ref() {
                        let g = hardware.gyration_radii_m;
                        let removed = event.removal.actual_removed.mass_kg();
                        let delta_i = Vec3d::new(
                            g.x * g.x * removed,
                            g.y * g.y * removed,
                            g.z * g.z * removed,
                        );
                        let w = event.craft_angular_velocity_before_body_rad_s;
                        let body =
                            Vec3d::new(w.x * delta_i.x, w.y * delta_i.y, w.z * delta_i.z);
                        if (event.carried_angular_momentum_magnitude_kg_m2_s - body.length()).abs()
                            > 1e-8 * body.length().max(1.)
                            || (event.carried_rotational_energy_j - 0.5 * w.dot(body)).abs()
                                > 1e-8 * (0.5 * w.dot(body)).abs().max(1.)
                        {
                            return Err("Cargo removal angular evidence does not match".into());
                        }
                    } else if event.carried_angular_momentum_magnitude_kg_m2_s != 0.
                        || event.carried_rotational_energy_j != 0.
                    {
                        return Err("Cargo removal angular evidence without contact hardware".into());
                    }
                    if let Some(index) = event.authored_index {
                        let removal = self
                            .definition
                            .cargo_removals
                            .get(index)
                            .ok_or("Cargo event references a missing authored removal")?;
                        if !authored.insert(index)
                            || removal.requested != event.removal.requested
                            || event.removal.elapsed_s + 1e-9 < removal.at_elapsed_s
                        {
                            return Err(
                                "Authored cargo removal evidence does not match the flight definition".into(),
                            );
                        }
                    }
                }
                for (index, removal) in self.definition.cargo_removals.iter().enumerate() {
                    if removal.at_elapsed_s <= p.elapsed_s && !authored.contains(&index) {
                        return Err(
                            "An elapsed authored cargo removal is missing from retained evidence".into(),
                        );
                    }
                }
                let mass = self.mass_kg();
                if !mass.is_finite() || mass <= 0. {
                    return Err("Invalid retained cargo-aware vehicle mass".into());
                }
            }
            _ => {
                return Err("Cargo ledgers and retained cargo evidence disagree".into());
            }
        }
        crate::vehicle_transfer::validate_transfers(self, u)
            .map_err(|e| format!("Invalid retained transfer records: {e}"))?;
        match (&self.definition.launch, &self.launch_ledger) {
            (Some(limits), Some(ledger)) => {
                limits
                    .validate()
                    .map_err(|e| format!("Invalid retained launch limits: {e}"))?;
                ledger
                    .validate()
                    .map_err(|e| format!("Invalid retained launch ledger: {e}"))?;
            }
            (Some(_), None) => {
                return Err("Launch-limited flights require a retained launch ledger".into())
            }
            (None, Some(_)) => {
                return Err("Retained launch ledgers require authored launch limits".into())
            }
            (None, None) => {}
        }
        match (&self.definition.entry, &self.entry_ledger) {
            (Some(limits), Some(ledger)) => {
                limits
                    .validate()
                    .map_err(|e| format!("Invalid retained entry limits: {e}"))?;
                ledger
                    .validate()
                    .map_err(|e| format!("Invalid retained entry ledger: {e}"))?;
            }
            (Some(_), None) => {
                return Err("Entry-limited flights require a retained entry ledger".into())
            }
            (None, Some(_)) => {
                return Err("Retained entry ledgers require authored entry limits".into())
            }
            (None, None) => {}
        }
        match (&self.definition.landing, &self.landing_ledger) {
            (Some(limits), Some(ledger)) => {
                limits
                    .validate()
                    .map_err(|e| format!("Invalid retained landing limits: {e}"))?;
                ledger
                    .validate()
                    .map_err(|e| format!("Invalid retained landing ledger: {e}"))?;
            }
            (Some(_), None) => {
                return Err("Landing-limited flights require a retained landing ledger".into())
            }
            (None, Some(_)) => {
                return Err("Retained landing ledgers require authored landing limits".into())
            }
            (None, None) => {}
        }
        let clock_tolerance =
            (64. * f64::EPSILON * self.last_time_s.abs().max(self.launched_at_s.abs())).max(1e-6);
        if !u.time.is_finite()
            || !self.launched_at_s.is_finite()
            || !self.last_time_s.is_finite()
            || self.launched_at_s > self.last_time_s
            || self.last_time_s > u.time + clock_tolerance
            || (self.launched_at_s + p.elapsed_s - self.last_time_s).abs() > clock_tolerance
            || self.phase == Phase::Flying && u.time - self.last_time_s > PERIOD + clock_tolerance
        {
            return Err("Invalid retained flight/world clocks".into());
        }
        match &self.phase {
            Phase::Flying => {}
            Phase::Unsupported { reason } => {
                if reason.trim().is_empty() || reason.len() > 4096 {
                    return Err("Invalid unsupported-flight reason".into());
                }
            }
            Phase::Impact {
                source,
                source_position_m,
                radius_m,
                position_residual_m,
                tolerance_m,
                source_sample_duration_s,
            } => {
                let valid_source = match source {
                    Source::Star => true,
                    Source::Companion => true,
                    Source::Body(i) => (*i as usize) < system.bodies.len(),
                };
                if !valid_source
                    || !finite(*source_position_m)
                    || !radius_m.is_finite()
                    || *radius_m <= 0.
                    || !position_residual_m.is_finite()
                    || !tolerance_m.is_finite()
                    || *tolerance_m < 0.001
                    || *tolerance_m > radius_m * 0.01
                    || position_residual_m.abs() > 2. * tolerance_m
                    || ((self.position_m - *source_position_m).length()
                        - radius_m
                        - position_residual_m)
                        .abs()
                        > *tolerance_m
                    || !source_sample_duration_s.is_finite()
                    || !(0.0..=SUBSTEP_S).contains(source_sample_duration_s)
                {
                    return Err("Invalid retained spherical contact evidence".into());
                }
            }
        }
        if let Some(i) = &self.last_interval {
            if [
                i.duration_s,
                i.initial_mass_kg,
                i.final_mass_kg,
                i.commanded_thrust_n,
                i.requested_propellant_kg,
                i.actual_propellant_kg,
                i.impulse_n_s,
                i.ideal_delta_v_m_s,
                i.powered_duration_s,
                i.fuel_limited_duration_s,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
                || !i.mass_residual_kg.is_finite()
                || i.duration_s > SUBSTEP_S + 1e-8
                || (i.final_mass_kg
                    - match &self.cargo {
                        None => p.mass_kg(d),
                        // A removal at the very end boundary of the last interval was
                        // applied after that interval integrated; add it back when
                        // checking the interval's recorded final mass.
                        Some(_) => {
                            self.mass_kg()
                                + self
                                    .cargo_events
                                    .iter()
                                    .filter(|e| p.elapsed_s - e.removal.elapsed_s <= 1e-6)
                                    .map(|e| e.removal.actual_removed.mass_kg())
                                    .sum::<f64>()
                        }
                    })
                    .abs()
                    > mass_tolerance
                || (i.initial_mass_kg - i.final_mass_kg - i.actual_propellant_kg).abs()
                    > mass_tolerance
                || i.powered_duration_s + i.fuel_limited_duration_s > i.duration_s + 1e-8
                || [
                    i.initial_powered_acceleration_m_s2,
                    i.final_powered_acceleration_m_s2,
                ]
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || *v < 0.)
            {
                return Err("Invalid retained last flight subinterval".into());
            }
        } else if p.elapsed_s != 0. {
            return Err("Missing retained flight subinterval".into());
        }
        Ok(())
    }
    /// One scheduled macro interval. External callers cannot skip unresolved flight.
    pub(crate) fn advance_system(&mut self, system: &StarSystem, to: f64) -> Result<(), String> {
        if self.contact.is_some() { return Err("Terrain flight requires the complete modeled world".into()); }
        if self.phase != Phase::Flying {
            return Ok(());
        }
        let dt = to - self.last_time_s;
        if !to.is_finite() || !dt.is_finite() || !(0.0..=1.000001).contains(&dt) {
            return Err(
                "Finite flight requires a supported scheduled interval of at most one second"
                    .into(),
            );
        }
        let retained_sources = gravity_sources(system, self.last_time_s)?;
        if retained_sources.len() != self.last_sources.len()
            || retained_sources
                .iter()
                .zip(&self.last_sources)
                .any(|(a, b)| {
                    a.source != b.source || a.mu != b.mu_m3_s2 || a.radius_m != b.radius_m
                })
        {
            return Err("Gravity source membership, mass or radius changed since the last resolved flight state; this interval cannot reconstruct that discontinuity".into());
        }
        let mut next = self.clone();
        let mut remaining = dt;
        while remaining > 0. {
            while let Some(index) = next.pending_authored_removal() {
                next.apply_authored_removal(index)?;
            }
            let held_definition;
            let d: &crate::supply_vehicle::Definition = match &next.cargo {
                Some(ledger) => {
                    held_definition =
                        ledger.current_definition(&next.definition.plan.definition)?;
                    &held_definition
                }
                None => &next.definition.plan.definition,
            };
            let elapsed = next.propulsion.elapsed_s;
            let (command, left) = active_command(&next.definition.plan, elapsed);
            let rate =
                d.rated_thrust_n * command.throttle / d.exhaust_velocity_m_s();
            let cutoff = if rate > 0. && next.propulsion.propellant_kg > 0. {
                exhaustion_time(next.propulsion.propellant_kg, rate)
            } else {
                f64::INFINITY
            };
            let mut h = remaining.min(SUBSTEP_S).min(left).min(cutoff);
            if let Some(t) = next.next_authored_removal_time() {
                let gap = t - elapsed;
                if gap > 0. {
                    h = h.min(gap);
                }
            }
            if h <= 0. || elapsed + h == elapsed {
                return Err("Flight interval is below represented elapsed-time precision".into());
            }
            let time = next.launched_at_s + elapsed;
            let initial = gravity_sources(system, time)?;
            let final_sources = gravity_sources(system, time + h)?;
            let sub = Command {
                duration_s: h,
                ..command
            };
            let tolerance = (64.
                * f64::EPSILON
                * next
                    .position_m
                    .x
                    .abs()
                    .max(next.position_m.y.abs())
                    .max(next.position_m.z.abs()))
            .max(0.001);
            if let Some(source) = initial.iter().find(|source| {
                (next.position_m - source.position_m).length() <= source.radius_m + tolerance
            }) {
                next.last_sources = source_samples(&initial);
                next.phase = Phase::Impact {
                    source: source.source.clone(),
                    source_position_m: source.position_m,
                    radius_m: source.radius_m,
                    position_residual_m: (next.position_m - source.position_m).length()
                        - source.radius_m,
                    tolerance_m: tolerance,
                    source_sample_duration_s: 0.,
                };
                *self = next;
                return Ok(());
            }
            let acceleration = gravity(next.position_m, &initial)?;
            let path = |t: f64| -> Result<Vec3d, String> {
                Ok(drift(
                    d,
                    &next.propulsion,
                    next.position_m,
                    next.velocity_m_s,
                    &Command {
                        duration_s: t,
                        ..sub.clone()
                    },
                    acceleration,
                )?
                .0)
            };
            let mut contact: Option<(f64, usize, f64)> = None;
            for (index, (a, b)) in initial.iter().zip(&final_sources).enumerate() {
                let radius = a.radius_m.max(b.radius_m);
                if radius <= 0. {
                    continue;
                }
                if tolerance > radius * 0.01 {
                    return Err(
                        "Source contact boundary is below supported coordinate precision".into(),
                    );
                }
                if let Some(t) = first_contact(
                    &path,
                    a.position_m,
                    b.position_m,
                    radius,
                    h,
                    acceleration.length() + d.rated_thrust_n * sub.throttle / d.inert_mass_kg(),
                    tolerance,
                )? {
                    if contact.as_ref().is_none_or(|(old, _, _)| t < *old) {
                        contact = Some((t, index, radius));
                    }
                }
            }
            if let Some((t, index, radius)) = contact {
                let partial = Command {
                    duration_s: t,
                    ..sub
                };
                let (position, v, propulsion, interval) = drift(
                    d,
                    &next.propulsion,
                    next.position_m,
                    next.velocity_m_s,
                    &partial,
                    acceleration,
                )?;
                let fraction = t / h;
                let sources = initial
                    .iter()
                    .zip(&final_sources)
                    .map(|(a, b)| Attractor {
                        source: a.source.clone(),
                        position_m: a.position_m.lerp(b.position_m, fraction),
                        mu: a.mu,
                        radius_m: a.radius_m.max(b.radius_m),
                    })
                    .collect::<Vec<_>>();
                let velocity =
                    v + gravity_contact(position, &sources, index, tolerance)? * (t / 2.);
                let body = &sources[index];
                let residual = (position - body.position_m).length() - radius;
                if residual > tolerance * 2. || residual < -tolerance * 2. {
                    return Err("Resolved contact exceeds its supported spatial tolerance".into());
                }
                supported_motion(position, velocity)?;
                next.position_m = position;
                next.velocity_m_s = velocity;
                next.propulsion = propulsion;
                next.last_interval = Some(interval);
                next.last_time_s = next.launched_at_s + next.propulsion.elapsed_s;
                next.last_sources = source_samples(&sources);
                next.phase = Phase::Impact {
                    source: body.source.clone(),
                    source_position_m: body.position_m,
                    radius_m: radius,
                    position_residual_m: residual,
                    tolerance_m: tolerance,
                    source_sample_duration_s: h,
                };
                *self = next;
                return Ok(());
            }
            let (position, velocity, propulsion, interval) = integrate(
                d,
                &next.propulsion,
                next.position_m,
                next.velocity_m_s,
                &sub,
                &initial,
                &final_sources,
            )?;
            supported_motion(position, velocity)?;
            next.position_m = position;
            next.velocity_m_s = velocity;
            next.propulsion = propulsion;
            next.last_interval = Some(interval);
            remaining = (remaining - h).max(0.);
        }
        while let Some(index) = next.pending_authored_removal() {
            next.apply_authored_removal(index)?;
        }
        next.last_time_s = to;
        next.last_sources = source_samples(&gravity_sources(system, to)?);
        *self = next;
        Ok(())
    }
}
fn supported_motion(position: Vec3d, velocity: Vec3d) -> Result<(), String> {
    if !finite(position) || !finite(velocity) || position.length() > 1e15 || velocity.length() > 1e6
    {
        return Err(
            "Finite flight left the supported 1e15 m / 1e6 m/s system-local Newtonian regime"
                .into(),
        );
    }
    Ok(())
}

impl Universe {
    pub(crate) fn validate_vehicle_state(&self) -> Result<(), String> {
        if self.vehicles.len() > MAX_VEHICLES {
            return Err("More than 64 finite vehicles".into());
        }
        let active = self.vehicles.iter().any(|v| v.phase == Phase::Flying);
        if !self.vehicles.is_empty() {
            let task = self
                .scheduler
                .tasks
                .get(TASK)
                .ok_or("Missing vehicle scheduler")?;
            if task.name != "vehicles"
                || !task.origin.is_finite()
                || task.steps == u64::MAX
                || !task.next_due().is_finite()
                || active
                    && (task.period != PERIOD
                        || task.next_due() <= self.time
                        || task.next_due() - self.time > PERIOD + 0.001)
                || !active && task.period != crate::scheduler::NEVER
            {
                return Err("Invalid finite vehicle scheduler".into());
            }
            if self.habitats.is_empty() {
                let habitat = self
                    .scheduler
                    .tasks
                    .get(crate::habitat::TASK)
                    .ok_or("Missing habitat task reservation")?;
                if habitat.name != "habitats"
                    || habitat.period != crate::scheduler::NEVER
                    || !habitat.origin.is_finite()
                    || habitat.steps != 0
                {
                    return Err("Invalid reserved habitat task before vehicle scheduler".into());
                }
            }
        } else if self.scheduler.tasks.get(TASK).is_some() {
            return Err("Vehicle scheduler without retained vehicles".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for flight in &self.vehicles {
            if flight.id >= self.next_vehicle_id || !ids.insert(flight.id) {
                return Err("Duplicate or out-of-range finite vehicle identifier".into());
            }
            flight.validate_saved(self)?;
        }
        Ok(())
    }
    pub(crate) fn ensure_vehicle_task(&mut self) {
        if self.vehicles.is_empty() {
            if self.scheduler.tasks.len() == TASK + 1
                && self.scheduler.tasks[TASK].name == "vehicles"
            {
                self.scheduler.tasks.remove(TASK);
                self.ensure_habitat_task();
            }
            return;
        }
        if self.scheduler.tasks.len() == crate::habitat::TASK {
            self.scheduler
                .add("habitats", self.time, crate::scheduler::NEVER);
        }
        let active = self.vehicles.iter().any(|v| v.phase == Phase::Flying);
        if self.scheduler.tasks.len() == TASK {
            self.scheduler.add(
                "vehicles",
                self.time,
                if active {
                    PERIOD
                } else {
                    crate::scheduler::NEVER
                },
            );
        } else if let Some(task) = self.scheduler.tasks.get_mut(TASK) {
            if active && task.period == crate::scheduler::NEVER
                || !active && task.period != crate::scheduler::NEVER
            {
                *task = crate::scheduler::Task {
                    name: "vehicles".into(),
                    origin: self.time,
                    period: if active {
                        PERIOD
                    } else {
                        crate::scheduler::NEVER
                    },
                    steps: 0,
                };
            }
        }
    }
    pub(crate) fn step_vehicles(&mut self, time: f64) {
        for index in 0..self.vehicles.len() {
            let mut flight = self.vehicles[index].clone();
            if flight.phase != Phase::Flying {
                continue;
            }
            let discontinuity = self
                .edits
                .iter()
                .skip(flight.edit_cursor as usize)
                .any(|record| {
                    let e = &record.edit;
                    if e.system() != flight.definition.parent.system {
                        return false;
                    }
                    matches!(
                        e,
                        crate::sandbox::Edit::AddBody { .. }
                            | crate::sandbox::Edit::RemoveBody { .. }
                            | crate::sandbox::Edit::SetState { .. }
                            | crate::sandbox::Edit::Impulse { .. }
                            | crate::sandbox::Edit::SetOrbit { .. }
                            | crate::sandbox::Edit::SetStar { .. }
                            | crate::sandbox::Edit::SetPhysics { .. }
                            | crate::sandbox::Edit::SetProperty {
                                property: crate::sandbox::BodyProperty::Mass(_)
                                    | crate::sandbox::BodyProperty::Radius(_),
                                ..
                            }
                    )
                    || flight.contact.is_some() && matches!(e,
                        crate::sandbox::Edit::SetMarsTerrain { .. }
                        | crate::sandbox::Edit::SetProperty { property: crate::sandbox::BodyProperty::RotationPeriod(_) | crate::sandbox::BodyProperty::AxialTilt(_), .. })
                });
            let result = if discontinuity {
                Err("A gravity/frame edit occurred after the last resolved craft state; event-time source discontinuity integration is not supported".into())
            } else if flight.contact.is_some() {
                crate::vehicle_contact_dynamics::advance(&mut flight, self, time)
            } else {
                self.systems
                    .get(flight.definition.parent.system as usize)
                    .ok_or_else(|| "Vehicle's gravitational system is unavailable".to_string())
                    .and_then(|system| flight.advance_system(system, time))
            };
            flight.edit_cursor = self.edits.len() as u64;
            if let Err(reason) = result {
                // The core previews each macro interval; failed integration retains the
                // last supported state and exposes the reason instead of inventing motion.
                flight.phase = Phase::Unsupported { reason };
            }
            if flight.phase != Phase::Flying {
                self.history.push(crate::history::Event {
                    time:if matches!(flight.phase,Phase::Unsupported {..}) {time} else {flight.last_time_s}, category:crate::history::Category::Space,
                    importance:4, title:format!("Finite vehicle {} stopped", flight.definition.plan.definition.name),
                    detail: match &flight.phase {
                        Phase::Impact {source,position_residual_m,..} => format!("First modeled spherical contact with {source:?}; raw boundary residual {position_residual_m} m. This is not a safe landing or cargo delivery."),
                        Phase::Unsupported {reason} => format!("Integration stopped at the last supported state: {reason}"),
                        Phase::Flying => unreachable!(),
                    }, system:Some(flight.definition.parent.system), body:Some(flight.definition.parent.body), civ:None, cause: None,
                });
            }
            self.vehicles[index] = flight;
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Attractor {
    pub(crate) source: Source,
    pub(crate) position_m: Vec3d,
    pub(crate) mu: f64,
    pub(crate) radius_m: f64,
}
pub(crate) fn gravity_sources(system: &StarSystem, time: f64) -> Result<Vec<Attractor>, String> {
    let mut out = vec![Attractor {
        source: Source::Star,
        position_m: system.star_local_position(time),
        mu: system.star.mu(),
        radius_m: system.star.current_radius(time),
    }];
    if let (Some(companion), Some(position)) =
        (&system.companion, system.companion_local_position(time))
    {
        out.push(Attractor {
            source: Source::Companion,
            position_m: position,
            mu: companion.star.mu(),
            radius_m: companion.star.current_radius(time),
        });
    }
    for (i, b) in system.bodies.iter().enumerate() {
        if b.exists() {
            out.push(Attractor {
                source: Source::Body(i as u32),
                position_m: system.body_state(i, time).pos,
                mu: G * b.mass,
                radius_m: b.radius,
            });
        }
    }
    if out.iter().any(|b| {
        !finite(b.position_m)
            || !b.mu.is_finite()
            || b.mu < 0.
            || !b.radius_m.is_finite()
            || b.radius_m < 0.
    }) {
        return Err("Unsupported physical gravitational source state".into());
    }
    Ok(out)
}
fn gravity(position: Vec3d, sources: &[Attractor]) -> Result<Vec3d, String> {
    let mut acceleration = Vec3d::ZERO;
    for b in sources {
        let delta = b.position_m - position;
        let r = delta.length();
        if r <= b.radius_m || r == 0. {
            return Err("Craft lies inside a modeled contact boundary".into());
        }
        acceleration += delta * (b.mu / (r * r * r));
    }
    if !finite(acceleration) {
        return Err("Unsupported finite gravitational acceleration".into());
    }
    Ok(acceleration)
}
pub(crate) fn active_command(plan: &Plan, elapsed: f64) -> (Command, f64) {
    let mut end = 0.;
    for c in &plan.commands {
        end += c.duration_s;
        if elapsed < end {
            return (c.clone(), end - elapsed);
        }
    }
    (
        Command {
            duration_s: 1.,
            throttle: 0.,
            direction: [1., 0., 0.],
        },
        f64::INFINITY,
    )
}
/// Exact integral of ideal velocity increase during a constant represented mass-flow burn.
fn powered_displacement(
    d: &crate::supply_vehicle::Definition,
    initial: &State,
    interval: &Interval,
) -> f64 {
    let t = interval.powered_duration_s;
    if t == 0. || interval.actual_propellant_kg == 0. {
        return 0.;
    }
    let z = interval.actual_propellant_kg / initial.mass_kg(d);
    let factor = if z < 1e-3 {
        let mut power = z;
        let mut sum = 0.;
        for n in 1..=8 {
            sum += power / (n * (n + 1)) as f64;
            power *= z;
        }
        sum
    } else {
        1. + (1. - z) / z * (-z).dln1p()
    };
    d.exhaust_velocity_m_s() * t * factor + interval.ideal_delta_v_m_s * (interval.duration_s - t)
}
fn integrate(
    d: &crate::supply_vehicle::Definition,
    state: &State,
    position: Vec3d,
    velocity: Vec3d,
    command: &Command,
    initial: &[Attractor],
    final_sources: &[Attractor],
) -> Result<(Vec3d, Vec3d, State, Interval), String> {
    let (p, v, propulsion, interval) = drift(
        d,
        state,
        position,
        velocity,
        command,
        gravity(position, initial)?,
    )?;
    let velocity = v + gravity(p, final_sources)? * (command.duration_s / 2.);
    if !finite(p) || !finite(velocity) {
        return Err("Unsupported finite propagated flight state".into());
    }
    Ok((p, velocity, propulsion, interval))
}
pub(crate) fn drift(
    d: &crate::supply_vehicle::Definition,
    state: &State,
    position: Vec3d,
    velocity: Vec3d,
    command: &Command,
    acceleration: Vec3d,
) -> Result<(Vec3d, Vec3d, State, Interval), String> {
    let mut propulsion = state.clone();
    let interval = propulsion.apply(d, command)?;
    let v = velocity + acceleration * (command.duration_s / 2.);
    let direction = Vec3d::new(
        command.direction[0],
        command.direction[1],
        command.direction[2],
    );
    let p =
        position + v * command.duration_s + direction * powered_displacement(d, state, &interval);
    let velocity = v + direction * interval.ideal_delta_v_m_s;
    if !finite(p) || !finite(velocity) {
        return Err("Unsupported finite powered drift".into());
    }
    Ok((p, velocity, propulsion, interval))
}
fn gravity_contact(
    position: Vec3d,
    sources: &[Attractor],
    contact: usize,
    tolerance: f64,
) -> Result<Vec3d, String> {
    let mut acceleration = Vec3d::ZERO;
    for (index, b) in sources.iter().enumerate() {
        let delta = b.position_m - position;
        let r = delta.length();
        if r == 0. || (r <= b.radius_m && (index != contact || r < b.radius_m - 2. * tolerance)) {
            return Err("Contact force sample is outside the supported boundary".into());
        }
        acceleration += delta * (b.mu / (r * r * r));
    }
    if !finite(acceleration) {
        return Err("Unsupported finite contact acceleration".into());
    }
    Ok(acceleration)
}
pub(crate) fn exhaustion_time(fuel: f64, rate: f64) -> f64 {
    let mut t = fuel / rate;
    // Round the computed cutoff upward only until represented consumption reaches
    // the held inventory. Avoid leaving an unadvanceable positive one-ulp residue.
    for _ in 0..3 {
        if rate * t >= fuel {
            break;
        }
        t = f64::from_bits(t.to_bits() + 1);
    }
    t
}
/// Earliest supported contact of a curved craft path with a linearly sampled body.
/// Curve/chord deviation is bounded by max_acceleration*duration²/8. Numerical
/// contact tolerance is explicit; exceeding the recursion/evaluation budget refuses.
fn first_contact(
    path: &impl Fn(f64) -> Result<Vec3d, String>,
    centre_start: Vec3d,
    centre_end: Vec3d,
    radius: f64,
    duration: f64,
    max_acceleration: f64,
    tolerance: f64,
) -> Result<Option<f64>, String> {
    if duration <= 0.
        || !duration.is_finite()
        || !radius.is_finite()
        || radius <= 0.
        || !max_acceleration.is_finite()
        || max_acceleration < 0.
        || !tolerance.is_finite()
        || tolerance <= 0.
    {
        return Err("Unsupported contact search domain".into());
    }
    struct Search<'a, F> {
        path: &'a F,
        a: Vec3d,
        b: Vec3d,
        r: f64,
        dt: f64,
        acc: f64,
        tol: f64,
        left: usize,
    }
    impl<F: Fn(f64) -> Result<Vec3d, String>> Search<'_, F> {
        fn relative(&self, t: f64) -> Result<Vec3d, String> {
            Ok((self.path)(t)? - self.a.lerp(self.b, t / self.dt))
        }
        fn visit(
            &mut self,
            lo: f64,
            hi: f64,
            a: Vec3d,
            b: Vec3d,
            depth: u32,
        ) -> Result<Option<f64>, String> {
            let delta = b - a;
            let length = delta.length_squared();
            let f = if length > 0. {
                (-a.dot(delta) / length).clamp(0., 1.)
            } else {
                0.
            };
            let deviation = self.acc * (hi - lo) * (hi - lo) / 8.;
            if (a + delta * f).length() > self.r + self.tol + deviation {
                return Ok(None);
            }
            if a.length() <= self.r + self.tol {
                return Ok(Some(lo));
            }
            if deviation <= self.tol / 4. {
                // The inflated radius includes the certified curve deviation. Keep the
                // actual curved point and residual; this is tolerance-based contact.
                let r = self.r + self.tol + deviation;
                let aa = delta.length_squared();
                let bb = a.dot(delta);
                let cc = a.length_squared() - r * r;
                if aa == 0. {
                    return Ok(None);
                }
                let disc = bb * bb - aa * cc;
                if disc < 0. {
                    return Ok(None);
                }
                let fraction = ((-bb - disc.sqrt()) / aa).clamp(0., 1.);
                return Ok(Some(lo + (hi - lo) * fraction));
            }
            if depth >= 48 || self.left == 0 {
                return Err("Contact search exceeded supported resolution budget; flight must remain unchanged".into());
            }
            self.left -= 1;
            let mid = (lo + hi) / 2.;
            if mid == lo || mid == hi {
                return Err("Contact search is below time representation precision".into());
            }
            let point = self.relative(mid)?;
            if let Some(t) = self.visit(lo, mid, a, point, depth + 1)? {
                return Ok(Some(t));
            }
            self.visit(mid, hi, point, b, depth + 1)
        }
    }
    let mut search = Search {
        path,
        a: centre_start,
        b: centre_end,
        r: radius,
        dt: duration,
        acc: max_acceleration,
        tol: tolerance,
        left: 256,
    };
    let a = search.relative(0.)?;
    let b = search.relative(duration)?;
    search.visit(0., duration, a, b, 0)
}

fn finite(v: Vec3d) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn launched_world() -> Universe {
        let mut u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let d = Definition::circular_example(&u, BodyRef { system: 0, body: 3 }, Plan::example())
            .unwrap();
        u.apply_edit(crate::sandbox::Edit::CreateVehicle {
            definition: Box::new(d),
        })
        .unwrap();
        u
    }
    #[test]
    fn authoritative_world_flight_is_exact_across_time_chunks_and_saved_continuation() {
        let mut one = launched_world();
        let mut chunks = one.clone();
        let mut resumed = one.clone();
        let start = one.time;
        one.advance_to(start + 360., None, None);
        for n in 1..=720 {
            chunks.advance_to(start + n as f64 * 0.5, None, None);
        }
        resumed.advance_to(start + 120.25, None, None);
        resumed =
            crate::save::from_json(&crate::save::to_json(&resumed, "mid-flight").unwrap()).unwrap();
        resumed.advance_to(start + 360., None, None);
        let bytes = |u: &Universe| serde_json::to_string(u).unwrap();
        assert_eq!(bytes(&one), bytes(&chunks));
        assert_eq!(bytes(&one), bytes(&resumed));
        assert_eq!(one.vehicles[0].propulsion.propellant_kg, 0.);
        assert_eq!(one.vehicles[0].phase, Phase::Flying);
        assert_eq!(one.vehicles[0].propulsion.elapsed_s, 360.);
        assert!(one.habitats.is_empty());
        one.validate_vehicle_state().unwrap();
    }
    #[test]
    fn habitat_vehicle_task_slots_survive_creation_removal_and_reload() {
        let mut u = launched_world();
        assert_eq!(
            u.scheduler.tasks[crate::habitat::TASK].period,
            crate::scheduler::NEVER
        );
        let d = crate::habitat::Definition::at(BodyRef { system: 0, body: 3 }, 0., 0.);
        u.apply_edit(crate::sandbox::Edit::CreateHabitat {
            definition: Box::new(d.clone()),
        })
        .unwrap();
        assert_eq!(
            u.scheduler.tasks[crate::habitat::TASK].period,
            crate::habitat::PERIOD
        );
        u.advance_by(2.25);
        let vehicle_clock = u.vehicles[0].last_time_s;
        u.apply_edit(crate::sandbox::Edit::RemoveHabitat { system: 0, id: 0 })
            .unwrap();
        assert_eq!(u.scheduler.tasks[TASK].name, "vehicles");
        assert_eq!(
            u.scheduler.tasks[crate::habitat::TASK].period,
            crate::scheduler::NEVER
        );
        u = crate::save::from_json(&crate::save::to_json(&u, "reserved slot").unwrap()).unwrap();
        u.advance_by(1.);
        assert!(u.vehicles[0].last_time_s > vehicle_clock);
        u.apply_edit(crate::sandbox::Edit::CreateHabitat {
            definition: Box::new(d),
        })
        .unwrap();
        assert_eq!(
            u.scheduler.tasks[crate::habitat::TASK].period,
            crate::habitat::PERIOD
        );
        u.apply_edit(crate::sandbox::Edit::RemoveVehicle { system: 0, id: 0 })
            .unwrap();
        assert_eq!(u.scheduler.tasks.len(), TASK);
        assert_eq!(
            u.scheduler.tasks[crate::habitat::TASK].period,
            crate::habitat::PERIOD
        );
        u.apply_edit(crate::sandbox::Edit::RemoveHabitat { system: 0, id: 1 })
            .unwrap();
        assert_eq!(u.scheduler.tasks.len(), crate::habitat::TASK);
        u = crate::save::from_json(&crate::save::to_json(&u, "removed fleet").unwrap()).unwrap();
        assert!(u.vehicles.is_empty());
        assert_eq!(u.next_vehicle_id, 1);
        assert!(crate::save::data_versions(&u).contains_key("cosmogon-finite-world-vehicle"));
    }
    #[test]
    fn live_impact_parks_scheduler_and_retains_exact_historical_event() {
        let mut u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let parent = BodyRef { system: 0, body: 3 };
        let mut d = Definition::circular_example(&u, parent, Plan::example()).unwrap();
        d.initial_relative_position_m = Vec3d::new(u.body(parent).radius + 1000., 0., 0.);
        d.initial_relative_velocity_m_s = Vec3d::new(-10000., 0., 0.);
        u.apply_edit(crate::sandbox::Edit::CreateVehicle {
            definition: Box::new(d),
        })
        .unwrap();
        u.advance_by(1.);
        assert!(matches!(u.vehicles[0].phase, Phase::Impact { .. }));
        assert_eq!(u.scheduler.tasks[TASK].period, crate::scheduler::NEVER);
        let retained = u.vehicles[0].clone();
        let event_count = u.history.events.len();
        u = crate::save::from_json(&crate::save::to_json(&u, "impact").unwrap()).unwrap();
        u.advance_by(3.);
        assert_eq!(u.vehicles[0], retained);
        assert_eq!(u.history.events.len(), event_count);
        let d = Definition::circular_example(&u, parent, Plan::example()).unwrap();
        u.apply_edit(crate::sandbox::Edit::CreateVehicle {
            definition: Box::new(d),
        })
        .unwrap();
        assert_eq!(u.scheduler.tasks[TASK].period, PERIOD);
        u.advance_by(1.);
        assert_eq!(u.vehicles[0], retained);
        assert!(u.vehicles[1].propulsion.elapsed_s > 0.);
    }
    #[test]
    fn corrupt_vehicle_save_pins_counters_clocks_contacts_and_task_slots_are_refused() {
        let mut u = launched_world();
        u.advance_by(4.25);
        let original: serde_json::Value =
            serde_json::from_str(&crate::save::to_json(&u, "flight").unwrap()).unwrap();
        for (pointer, value) in [
            (
                "/header/data_versions/cosmogon-finite-world-vehicle",
                serde_json::json!("wrong"),
            ),
            (
                "/header/data_versions/cosmogon-supply-vehicle-reference",
                serde_json::json!("wrong"),
            ),
            ("/universe/vehicles/0/id", serde_json::json!(5)),
            (
                "/universe/vehicles/0/propulsion/propellant_kg",
                serde_json::json!(999999),
            ),
            (
                "/universe/vehicles/0/propulsion/exhausted_propellant_kg",
                serde_json::json!(999999),
            ),
            (
                "/universe/vehicles/0/propulsion/cumulative_impulse_n_s",
                serde_json::json!(0),
            ),
            (
                "/universe/vehicles/0/launched_at_s",
                serde_json::json!(u.time + 100.),
            ),
            (
                "/universe/vehicles/0/last_time_s",
                serde_json::json!(u.time + 100.),
            ),
            (
                "/universe/vehicles/0/definition/frame",
                serde_json::json!("body-fixed"),
            ),
            (
                "/universe/vehicles/0/definition/parent/body",
                serde_json::json!(999999),
            ),
            (
                "/universe/vehicles/0/last_interval/final_mass_kg",
                serde_json::json!(1),
            ),
            (
                "/universe/scheduler/tasks/5/steps",
                serde_json::json!(u64::MAX),
            ),
            (
                "/universe/scheduler/tasks/5/name",
                serde_json::json!("habitats"),
            ),
            ("/universe/scheduler/tasks/4/period", serde_json::json!(1)),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = value;
            assert!(
                crate::save::from_json(&changed.to_string()).is_err(),
                "Accepted corrupt field {pointer}"
            );
        }
        let mut changed = original.clone();
        changed["universe"]["vehicles"][0]["phase"] = serde_json::json!({"kind":"impact","source":{"kind":"body","body_index":3},"source_position_m":{"x":0,"y":0,"z":0},"radius_m":1,"position_residual_m":0,"tolerance_m":0.001,"source_sample_duration_s":0.25});
        assert!(crate::save::from_json(&changed.to_string()).is_err());
        // Original authoring assumptions are historical; later ordinary edits do not
        // rewrite the launch conditions or make the retained world unloadable.
        u.systems[0].bodies[3].radius *= 100.;
        assert!(
            crate::save::from_json(&crate::save::to_json(&u, "changed parent").unwrap()).is_ok()
        );
    }
    #[test]
    fn invalid_launch_and_counter_overflow_are_atomic_and_legacy_state_is_absent() {
        let mut u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let bytes = serde_json::to_string(&u).unwrap();
        assert!(!bytes.contains("next_vehicle_id"));
        assert!(!bytes.contains("\"vehicles\""));
        let mut d =
            Definition::circular_example(&u, BodyRef { system: 0, body: 3 }, Plan::example())
                .unwrap();
        d.initial_relative_position_m = Vec3d::ZERO;
        assert!(u
            .apply_edit(crate::sandbox::Edit::CreateVehicle {
                definition: Box::new(d.clone())
            })
            .is_err());
        assert_eq!(bytes, serde_json::to_string(&u).unwrap());
        d = Definition::circular_example(&u, BodyRef { system: 0, body: 3 }, Plan::example())
            .unwrap();
        u.next_vehicle_id = u64::MAX;
        let before = serde_json::to_string(&u).unwrap();
        assert!(u
            .apply_edit(crate::sandbox::Edit::CreateVehicle {
                definition: Box::new(d)
            })
            .is_err());
        assert_eq!(before, serde_json::to_string(&u).unwrap());
    }
    #[test]
    fn changed_sources_and_unsupported_speed_freeze_the_last_actual_world_state() {
        let mut u = launched_world();
        u.advance_by(1.25);
        let retained = u.vehicles[0].clone();
        u.apply_edit(crate::sandbox::Edit::SetProperty {
            body: BodyRef { system: 0, body: 3 },
            property: crate::sandbox::BodyProperty::Mass(u.systems[0].bodies[3].mass * 0.5),
        })
        .unwrap();
        u.advance_by(1.);
        let stopped = &u.vehicles[0];
        assert!(matches!(stopped.phase, Phase::Unsupported { .. }));
        assert_eq!(stopped.position_m, retained.position_m);
        assert_eq!(stopped.velocity_m_s, retained.velocity_m_s);
        assert_eq!(stopped.propulsion, retained.propulsion);
        assert_eq!(u.scheduler.tasks[TASK].period, crate::scheduler::NEVER);
        assert!(
            crate::save::from_json(&crate::save::to_json(&u, "source discontinuity").unwrap())
                .is_ok()
        );
        let mut u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let mut plan = Plan::example();
        plan.definition.dry_mass_kg = 1.;
        plan.definition.cargo = crate::supply_vehicle::Cargo {
            oxygen_reserve_kg: 0.,
            potable_water_kg: 0.,
            food_dry_kg: 0.,
            food_water_kg: 0.,
        };
        plan.definition.initial_propellant_kg = 10.;
        plan.definition.effective_isp_s = 1e5;
        plan.definition.rated_thrust_n = 1e9;
        let d = Definition::circular_example(&u, BodyRef { system: 0, body: 3 }, plan).unwrap();
        u.apply_edit(crate::sandbox::Edit::CreateVehicle {
            definition: Box::new(d),
        })
        .unwrap();
        let original = u.vehicles[0].clone();
        u.advance_by(1.);
        assert!(matches!(u.vehicles[0].phase, Phase::Unsupported { .. }));
        assert_eq!(u.vehicles[0].propulsion, original.propulsion);
        assert_eq!(u.vehicles[0].last_time_s, original.last_time_s);
        assert!(
            crate::save::from_json(&crate::save::to_json(&u, "unsupported speed").unwrap()).is_ok()
        );
    }
    #[test]
    fn terminal_flight_parking_is_exact_across_fractional_caller_chunks() {
        let mut one = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let parent = BodyRef { system: 0, body: 3 };
        let mut d = Definition::circular_example(&one, parent, Plan::example()).unwrap();
        d.initial_relative_position_m = Vec3d::new(one.body(parent).radius + 1000., 0., 0.);
        d.initial_relative_velocity_m_s = Vec3d::new(-10000., 0., 0.);
        one.apply_edit(crate::sandbox::Edit::CreateVehicle {
            definition: Box::new(d),
        })
        .unwrap();
        let mut chunks = one.clone();
        let start = one.time;
        one.advance_to(start + 4., None, None);
        for n in 1..=40 {
            chunks.advance_to(start + n as f64 * 0.1, None, None);
        }
        assert_eq!(
            serde_json::to_vec(&one).unwrap(),
            serde_json::to_vec(&chunks).unwrap()
        );
        assert!(matches!(one.vehicles[0].phase, Phase::Impact { .. }));
        assert_eq!(one.scheduler.tasks[TASK].origin, start + 1.);
        let restored =
            crate::save::from_json(&crate::save::to_json(&chunks, "chunked impact").unwrap())
                .unwrap();
        assert_eq!(
            serde_json::to_vec(&one).unwrap(),
            serde_json::to_vec(&restored).unwrap()
        );
    }
    #[test]
    fn physical_contact_retains_only_pre_contact_propellant_and_freezes_the_event() {
        let u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let parent = BodyRef { system: 0, body: 3 };
        let mut d = Definition::circular_example(&u, parent, Plan::example()).unwrap();
        d.initial_relative_position_m = Vec3d::new(u.body(parent).radius + 1000., 0., 0.);
        d.initial_relative_velocity_m_s = Vec3d::new(-10000., 0., 0.);
        d.plan.commands[0].direction = [-1., 0., 0.];
        let mut flight = Flight::new(0, d, &u).unwrap();
        let cargo = flight.definition.plan.definition.cargo.clone();
        flight.advance_system(&u.systems[0], u.time + 1.).unwrap();
        match &flight.phase {
            Phase::Impact {
                source,
                position_residual_m,
                tolerance_m,
                ..
            } => {
                assert_eq!(*source, Source::Body(3));
                assert!(position_residual_m.abs() <= 2. * tolerance_m);
            }
            _ => panic!("Physical crossing was not retained: {:?}", flight.phase),
        }
        assert!(flight.propulsion.elapsed_s > 0. && flight.propulsion.elapsed_s < 0.11);
        let rate = flight.definition.plan.definition.rated_thrust_n
            / flight.definition.plan.definition.exhaust_velocity_m_s();
        assert!(
            (flight.propulsion.exhausted_propellant_kg - rate * flight.propulsion.elapsed_s).abs()
                < 1e-9
        );
        assert_eq!(flight.definition.plan.definition.cargo, cargo);
        let retained = flight.clone();
        flight.advance_system(&u.systems[0], u.time + 100.).unwrap();
        assert_eq!(flight, retained);
        assert!(flight.last_time_s < u.time + 1.);
    }
    #[test]
    fn live_sampled_gravity_and_fuel_cutoff_do_not_leave_a_precision_residue() {
        let mut u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let parent = BodyRef { system: 0, body: 3 };
        let plan = Plan::example();
        let d = Definition::circular_example(&u, parent, plan.clone()).unwrap();
        let mut flight = Flight::new(0, d, &u).unwrap();
        let start = u.time;
        for step in 1..=360 {
            u.advance_to(start + step as f64, None, None);
            flight.advance_system(&u.systems[0], u.time).unwrap();
        }
        assert_eq!(flight.phase, Phase::Flying);
        assert_eq!(flight.propulsion.propellant_kg, 0.);
        assert!(
            (flight.propulsion.cumulative_ideal_delta_v_m_s
                - plan.definition.initial_ideal_delta_v_m_s())
            .abs()
                < 1e-8
        );
        assert_eq!(flight.propulsion.elapsed_s, 360.);
        assert!(flight.propulsion.fuel_limited_duration_s > 0.);
        assert_eq!(
            flight.definition.plan.definition.cargo,
            plan.definition.cargo
        );
        assert!(u.habitats.is_empty());
    }
    #[test]
    fn swept_curved_contacts_catch_pass_through_and_refuse_false_chord_contacts() {
        let line = |t: f64| Ok(Vec3d::new(-10. + 20. * t, 0., 0.));
        let t = first_contact(&line, Vec3d::ZERO, Vec3d::ZERO, 1., 1., 0., 1e-4)
            .unwrap()
            .unwrap();
        assert!((t - 0.45).abs() < 1e-5);
        let moving = first_contact(
            &line,
            Vec3d::new(1., 0., 0.),
            Vec3d::new(2., 0., 0.),
            1.,
            1.,
            0.,
            1e-4,
        )
        .unwrap()
        .unwrap();
        assert!((moving - 10. / 19.).abs() < 1e-5);
        let miss = |t: f64| Ok(Vec3d::new(-10. + 20. * t, 0.9 + 4. * t * (1. - t), 0.));
        assert!(
            first_contact(&miss, Vec3d::ZERO, Vec3d::ZERO, 1., 1., 8., 1e-4)
                .unwrap()
                .is_none()
        );
        let graze = |t: f64| Ok(Vec3d::new(-10. + 20. * t, 2. - 4. * t * (1. - t), 0.));
        let t = first_contact(&graze, Vec3d::ZERO, Vec3d::ZERO, 1., 1., 8., 1e-4)
            .unwrap()
            .unwrap();
        assert!((t - 0.5).abs() < 0.001);
        assert!((graze(t).unwrap().length() - 1.).abs() < 1.3e-4);
        assert!(first_contact(&line, Vec3d::ZERO, Vec3d::ZERO, 1., 1., f64::NAN, 1e-4).is_err());
    }
    #[test]
    fn companion_gravity_does_not_mix_galactic_and_local_frames() {
        let mut u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::Sol,
            ..Default::default()
        });
        let mut system = u.systems.remove(0);
        system.companion = Some(crate::astro::Companion {
            star: system.star.clone(),
            orbit: crate::astro::Orbit::circular(1e12, 0.),
        });
        let t = u.time;
        let sources = gravity_sources(&system, t).unwrap();
        let sample = Vec3d::new(5e11, 2e11, 0.);
        let force = gravity(sample, &sources).unwrap();
        system.position = Vec3d::new(1e24, -2e24, 3e24);
        let shifted = gravity_sources(&system, t).unwrap();
        assert_eq!(force, gravity(sample, &shifted).unwrap());
        assert_eq!(sources[1].position_m, shifted[1].position_m);
    }
    #[test]
    fn changing_mass_displacement_matches_analytic_vacuum_and_small_ratio() {
        let d = crate::supply_vehicle::Definition::example();
        let state = State::new(&d).unwrap();
        let c = Command {
            duration_s: 30.,
            throttle: 1.,
            direction: [1., 0., 0.],
        };
        let (p, v, s, i) = integrate(
            &d,
            &state,
            Vec3d::ZERO,
            Vec3d::new(10., 0., 0.),
            &c,
            &[],
            &[],
        )
        .unwrap();
        let q = d.rated_thrust_n / d.exhaust_velocity_m_s();
        let mf = s.mass_kg(&d);
        let analytic =
            d.exhaust_velocity_m_s() * (c.duration_s - mf / q * (state.mass_kg(&d) / mf).dln());
        assert!((p.x - 10. * c.duration_s - analytic).abs() < 1e-8);
        assert_eq!(v.x, 10. + i.ideal_delta_v_m_s);
        assert_eq!(s.exhausted_propellant_kg, i.actual_propellant_kg);
        let mut heavy = d;
        heavy.dry_mass_kg = 1e12;
        heavy.initial_propellant_kg = 1.;
        let state = State::new(&heavy).unwrap();
        let command = Command {
            duration_s: 0.001,
            ..c
        };
        let (p, v, _, i) =
            integrate(&heavy, &state, Vec3d::ZERO, Vec3d::ZERO, &command, &[], &[]).unwrap();
        assert!(p.x > 0. && v.x > 0.);
        assert!((p.x / (i.ideal_delta_v_m_s * command.duration_s / 2.) - 1.).abs() < 1e-12);
    }
    #[test]
    fn fixed_gravity_steps_converge_and_preserve_angular_momentum() {
        let sources = [Attractor {
            source: Source::Star,
            position_m: Vec3d::ZERO,
            mu: 1e10,
            radius_m: 1000.,
        }];
        let mut d = crate::supply_vehicle::Definition::example();
        d.rated_thrust_n = 0.;
        let r = 100_000.;
        let omega = (sources[0].mu / (r * r * r)).sqrt();
        let expected = Vec3d::new(r * (omega * 100.).dcos(), r * (omega * 100.).dsin(), 0.);
        let run = |h: f64| {
            let mut p = Vec3d::new(r, 0., 0.);
            let mut v = Vec3d::new(0., (sources[0].mu / r).sqrt(), 0.);
            let mut s = State::new(&d).unwrap();
            for _ in 0..(100. / h) as usize {
                let (pp, vv, ss, _) = integrate(
                    &d,
                    &s,
                    p,
                    v,
                    &Command {
                        duration_s: h,
                        throttle: 0.,
                        direction: [1., 0., 0.],
                    },
                    &sources,
                    &sources,
                )
                .unwrap();
                p = pp;
                v = vv;
                s = ss;
            }
            assert!((p.cross(v).z / (r * (sources[0].mu / r).sqrt()) - 1.).abs() < 1e-12);
            (p - expected).length()
        };
        let coarse = run(1.);
        let fine = run(0.25);
        assert!(fine < coarse / 10.);
    }
    #[test]
    fn actual_world_translation_velocity_and_invalid_steps_are_explicit() {
        let u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let parent = BodyRef { system: 0, body: 3 };
        let d = Definition::circular_example(&u, parent, Plan::example()).unwrap();
        let centre = u.systems[0].body_state(3, u.time);
        let mut flight = Flight::new(0, d.clone(), &u).unwrap();
        assert_eq!(
            flight.position_m,
            centre.pos + d.initial_relative_position_m
        );
        assert_eq!(
            flight.velocity_m_s,
            centre.vel + d.initial_relative_velocity_m_s
        );
        let before = flight.clone();
        assert!(flight.advance_system(&u.systems[0], u.time + 2.).is_err());
        assert_eq!(flight, before);
        flight.advance_system(&u.systems[0], u.time + 1.).unwrap();
        assert!(flight.propulsion.propellant_kg < before.propulsion.propellant_kg);
        assert_eq!(
            flight.definition.plan.definition.cargo,
            before.definition.plan.definition.cargo
        );
        assert_ne!(flight.position_m, before.position_m);
    }
    fn cargo( water: f64) -> Cargo {
        Cargo {
            oxygen_reserve_kg: 0.,
            potable_water_kg: water,
            food_dry_kg: 0.,
            food_water_kg: 0.,
        }
    }
    fn launched_with_ledgers(water: f64) -> Universe {
        let mut u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let mut d =
            Definition::circular_example(&u, BodyRef { system: 0, body: 3 }, Plan::example())
                .unwrap()
                .with_cargo_ledgers()
                .unwrap();
        d.plan.definition.payload_capacity_kg = 20_000.;
        d.plan.definition.cargo.potable_water_kg = water;
        u.apply_edit(crate::sandbox::Edit::CreateVehicle {
            definition: Box::new(d),
        })
        .unwrap();
        u
    }
    #[test]
    fn co_moving_cargo_removal_is_impulse_free_and_closes_momentum() {
        let mut u = launched_with_ledgers(5000.);
        let velocity_before = u.vehicles[0].velocity_m_s;
        let mass_before = u.vehicles[0].mass_kg();
        let event = u.vehicles[0].remove_cargo(cargo(1000.), Vec3d::ZERO).unwrap();
        assert_eq!(event.removal.actual_removed, cargo(1000.));
        let mass_after = u.vehicles[0].mass_kg();
        assert!((mass_before - mass_after - 1000.).abs() < 1e-9);
        assert!(
            (u.vehicles[0].velocity_m_s - velocity_before).length()
                < 1e-9 * velocity_before.length().max(1.)
        );
        assert_eq!(u.vehicles[0].cargo.as_ref().unwrap().held.potable_water_kg, 4000.);
        assert_eq!(u.vehicles[0].cargo_events.len(), 1);
        u.validate_vehicle_state().unwrap();
    }
    #[test]
    fn bounded_ejection_closes_linear_momentum_exactly() {
        let mut u = launched_with_ledgers(5000.);
        let relative = Vec3d::new(120., -40., 90.);
        let velocity_before = u.vehicles[0].velocity_m_s;
        let mass_before = u.vehicles[0].mass_kg();
        let event = u.vehicles[0].remove_cargo(cargo(800.), relative).unwrap();
        let carried = event.removal.velocity_m_s * event.removal.actual_removed.mass_kg();
        let closed = u.vehicles[0].velocity_m_s * u.vehicles[0].mass_kg() + carried;
        assert!((closed - velocity_before * mass_before).length() < 1e-6);
        assert!(
            (u.vehicles[0].velocity_m_s - velocity_before).length() > 1e-6,
            "non-zero relative ejection must change craft velocity"
        );
        u.validate_vehicle_state().unwrap();
    }
    #[test]
    fn cargo_removal_refuses_missing_ledgers_overdraw_unbounded_velocity_and_non_flight() {
        let mut plain = launched_world();
        assert!(plain.vehicles[0]
            .remove_cargo(cargo(1.), Vec3d::ZERO)
            .unwrap_err()
            .contains("ledgers"));
        let mut u = launched_with_ledgers(1000.);
        assert!(u.vehicles[0]
            .remove_cargo(cargo(5000.), Vec3d::ZERO)
            .is_err());
        assert!(u.vehicles[0]
            .remove_cargo(cargo(10.), Vec3d::new(MAX_CARGO_EJECTION_SPEED_M_S + 1., 0., 0.))
            .is_err());
        u.vehicles[0].phase = Phase::Unsupported {
            reason: "test".into(),
        };
        assert!(u.vehicles[0]
            .remove_cargo(cargo(10.), Vec3d::ZERO)
            .unwrap_err()
            .contains("flying"));
    }
    #[test]
    fn tampered_cargo_evidence_fails_saved_validation() {
        let mut u = launched_with_ledgers(2000.);
        u.vehicles[0].remove_cargo(cargo(300.), Vec3d::new(50., 0., 0.)).unwrap();
        u.validate_vehicle_state().unwrap();
        u.vehicles[0].cargo_events[0].craft_velocity_after_m_s += Vec3d::new(1., 0., 0.);
        assert!(u.validate_vehicle_state().is_err());
    }
    #[test]
    fn authored_cargo_removals_apply_at_represented_boundaries() {
        let mut u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let mut d =
            Definition::circular_example(&u, BodyRef { system: 0, body: 3 }, Plan::example())
                .unwrap()
                .with_cargo_ledgers()
                .unwrap();
        d.plan.definition.payload_capacity_kg = 20_000.;
        d.plan.definition.cargo.potable_water_kg = 3000.;
        d.cargo_removals = vec![AuthoredCargoRemoval {
            at_elapsed_s: 0.6,
            requested: cargo(400.),
            velocity_relative_m_s: [0., 0., 0.],
        }];
        u.apply_edit(crate::sandbox::Edit::CreateVehicle {
            definition: Box::new(d),
        })
        .unwrap();
        let start = u.time;
        u.advance_to(start + 2., None, None);
        let flight = &u.vehicles[0];
        assert_eq!(flight.cargo_events.len(), 1);
        assert!(flight.propulsion.elapsed_s >= 0.6);
        assert_eq!(flight.cargo.as_ref().unwrap().held.potable_water_kg, 2600.);
        assert_eq!(flight.cargo_events[0].authored_index, Some(0));
        u.validate_vehicle_state().unwrap();
    }
    #[test]
    fn ledger_flights_expose_cargo_keys_and_legacy_flights_stay_byte_exact() {
        let plain = launched_world();
        let value: serde_json::Value = serde_json::to_value(&plain.vehicles[0]).unwrap();
        let object = value.as_object().unwrap();
        assert!(!object.contains_key("cargo"));
        assert!(!object.contains_key("cargo_events"));
        let ledger = launched_with_ledgers(1000.);
        let value: serde_json::Value = serde_json::to_value(&ledger.vehicles[0]).unwrap();
        let object = value.as_object().unwrap();
        assert!(object.contains_key("cargo"));
        assert!(!object.contains_key("cargo_events"));
        let mut u = launched_with_ledgers(1000.);
        u.vehicles[0].remove_cargo(cargo(10.), Vec3d::ZERO).unwrap();
        let value: serde_json::Value = serde_json::to_value(&u.vehicles[0]).unwrap();
        assert!(value.as_object().unwrap().contains_key("cargo_events"));
        let roundtrip: crate::Universe =
            crate::save::from_json(&crate::save::to_json(&ledger, "cargo").unwrap()).unwrap();
        roundtrip.validate_vehicle_state().unwrap();
    }
    #[test]
    fn definition_structure_rejects_removals_without_ledgers_and_unsupported_inputs() {
        let u = Universe::new(crate::UniverseSettings {
            scenario: crate::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let parent = BodyRef { system: 0, body: 3 };
        let mut d = Definition::circular_example(&u, parent, Plan::example()).unwrap();
        d.cargo_removals = vec![AuthoredCargoRemoval {
            at_elapsed_s: 1.,
            requested: cargo(10.),
            velocity_relative_m_s: [0., 0., 0.],
        }];
        assert!(d.validate_structure().is_err());
        let mut d = d.with_cargo_ledgers().unwrap();
        d.cargo_removals[0].at_elapsed_s = f64::NAN;
        assert!(d.validate_structure().is_err());
        d.cargo_removals[0].at_elapsed_s = 1.;
        d.cargo_removals[0].velocity_relative_m_s = [2000., 0., 0.];
        assert!(d.validate_structure().is_err());
        d.cargo_removals[0].requested = cargo(f64::NAN);
        assert!(d.validate_structure().is_err());
    }
}
