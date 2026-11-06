//! The life and death of stars, and what it does to their planets.
//!
//! A scheduled task watches every star:
//! * **Red giant**: the swelling star engulfs any world whose orbit dips inside it.
//! * **Gentle end** (< 8 M☉): the envelope drifts away as a planetary nebula; the slow
//!   mass loss widens surviving orbits adiabatically (a ∝ 1/M, orbits stay circular) and a
//!   white dwarf remains.
//! * **Core collapse** (≥ 8 M☉): a supernova. Its ~10⁴⁴ J of ejecta vaporise worlds that
//!   intercept more than their gravitational binding energy and sterilise and strip the
//!   rest; the sudden mass loss leaves orbits unbound if more than half the mass goes; a
//!   neutron star or black hole remains. Radiation reaches other star systems at the speed
//!   of light; within ~30 light-years it drives extinctions (ozone loss: Gehrels et al. 2003).
//! Every transition happens on the task's fixed time grid, so outcomes don't depend on
//! frame rate or speed.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use serde::{Deserialize, Serialize};

use crate::astro::star::{StarKind, StellarPhase};
use crate::astro::{Orbit, Removal, RemovalCause, StarSystem, G, LIGHT_YEAR, SPEED_OF_LIGHT};
use crate::history::{Category, Event};
use crate::impact::ImpactWinter;
use crate::life::Stage;
use crate::time::{group_digits, SECONDS_PER_YEAR};
use crate::{BodyRef, Universe};

/// Kinetic energy of core-collapse supernova ejecta (J).
pub const SUPERNOVA_ENERGY: f64 = 1.0e44;
/// Radiation fluence (J/m²) at which a living world's biosphere starts to suffer, and at
/// which it is wiped out (ozone destruction and surface radiation; order-of-magnitude).
pub const FLUENCE_HARM: f64 = 1.0e7;
pub const FLUENCE_STERILE: f64 = 1.0e10;
/// Fraction of the explosion energy emitted as ionising radiation (X/γ, cosmic rays).
const RADIATION_FRACTION: f64 = 0.01;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NebulaKind {
    /// Gas shed by a dying low-mass star, lit by the hot white dwarf.
    Planetary,
    /// Debris of a supernova sweeping up interstellar gas.
    SupernovaRemnant,
}

/// An expanding shell of gas around a star system (rendered; no gravity).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Nebula {
    pub kind: NebulaKind,
    pub born: f64,
    /// Expansion speed (m/s).
    pub speed: f64,
    /// Ejected mass (M☉).
    pub mass: f64,
    pub seed: u64,
}

impl Nebula {
    /// Radius (m) at `t`: free expansion, then (for remnants) the Sedov–Taylor slowdown
    /// r ∝ t^0.4 after a few centuries.
    pub fn radius(&self, t: f64) -> f64 {
        let age = (t - self.born).max(0.0);
        match self.kind {
            NebulaKind::Planetary => self.speed * age,
            NebulaKind::SupernovaRemnant => {
                let t0 = 300.0 * SECONDS_PER_YEAR;
                if age < t0 {
                    self.speed * age
                } else {
                    self.speed * t0 * (age / t0).dpowf(0.4)
                }
            }
        }
    }
    /// How visible it still is (1 young → 0 dispersed).
    pub fn brightness(&self, t: f64) -> f64 {
        let age = (t - self.born).max(0.0) / SECONDS_PER_YEAR;
        let life = match self.kind {
            NebulaKind::Planetary => 30_000.0,
            NebulaKind::SupernovaRemnant => 100_000.0,
        };
        (1.0 - age / life).clamp(0.0, 1.0)
    }
    pub fn label(&self) -> &'static str {
        match self.kind {
            NebulaKind::Planetary => "Planetary nebula",
            NebulaKind::SupernovaRemnant => "Supernova remnant",
        }
    }
}

/// A supernova's radiation front travelling outwards at light speed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Blast {
    pub system: u32,
    pub time: f64,
    pub energy: f64,
    /// Systems already reached.
    pub reached: Vec<u32>,
}

/// Index of the stellar task in the scheduler.
pub(crate) const TASK_STARS: usize = 3;
const STAR_PERIOD: f64 = SECONDS_PER_YEAR;
/// While a star is a giant, check engulfment this often (years).
const GIANT_CHECK_YEARS: u64 = 1_000;

impl Universe {
    /// Make sure the stellar task exists (older saves and new universes).
    pub fn ensure_star_task(&mut self) {
        while self.scheduler.tasks.len() < TASK_STARS {
            self.scheduler.add("placeholder", self.time, crate::scheduler::NEVER);
        }
        if self.scheduler.tasks.len() == TASK_STARS {
            self.scheduler.add("stars", self.time, STAR_PERIOD);
        }
        // Reschedule from now (an edit may have brought a star's end closer).
        let next = self.next_stellar_index();
        self.scheduler.tasks[TASK_STARS].steps = next;
    }

    /// The stellar task's next useful step: giant onsets, ends of life, blast arrivals.
    fn next_stellar_index(&self) -> u64 {
        let k_now = self.scheduler.index_at_or_after(TASK_STARS, self.time);
        let mut next = f64::INFINITY;
        for sys in &self.systems {
            let s = &sys.star;
            if s.kind != StarKind::Normal {
                continue;
            }
            match s.phase(self.time) {
                StellarPhase::Giant => return k_now,
                _ => next = next.min(s.giant_onset()).min(s.end_of_life()),
            }
        }
        for b in &self.blasts {
            let origin = self.systems[b.system as usize].position;
            for sys in self.systems.iter().filter(|x| !b.reached.contains(&x.id)) {
                next = next.min(b.time + (sys.position - origin).length() / SPEED_OF_LIGHT);
            }
        }
        next = next.min(self.next_nursery_time());
        let now = self.scheduler.index_at_or_after(TASK_STARS, self.time);
        if !next.is_finite() {
            return u64::MAX / 4;
        }
        self.scheduler.index_at_or_after(TASK_STARS, next.max(self.time)).max(now)
    }

    /// Run at the stellar task's step `k` (time `t`). Returns the next step index.
    pub(crate) fn step_stars(&mut self, t: f64, k: u64) -> u64 {
        self.step_nurseries(t);
        for s in 0..self.systems.len() {
            let star = &self.systems[s].star;
            if star.kind != StarKind::Normal {
                continue;
            }
            match star.phase(t) {
                StellarPhase::Giant => self.engulf(s, t),
                p if p.is_remnant() => self.end_of_life(s, t),
                _ => {}
            }
        }
        self.propagate_blasts(t);
        // While a star is a giant, check every GIANT_CHECK_YEARS — but never step past the
        // moment it ends.
        let giants: Vec<f64> = self.systems.iter().filter(|x| x.star.kind == StarKind::Normal && x.star.phase(t) == StellarPhase::Giant).map(|x| x.star.end_of_life()).collect();
        if giants.is_empty() {
            self.next_stellar_index().max(k + 1)
        } else {
            let end = giants.into_iter().fold(f64::INFINITY, f64::min);
            let at_end = self.scheduler.index_at_or_after(TASK_STARS, end);
            (k + GIANT_CHECK_YEARS).min(at_end).max(k + 1)
        }
    }

    /// Distance (m) from the star at which `body` comes closest now.
    fn closest_approach(sys: &StarSystem, body: usize, t: f64) -> f64 {
        if sys.dynamics.is_some() {
            (sys.body_local_position(body, t) - sys.star_local_position(t)).length()
        } else {
            let top = sys.top_level(body);
            sys.bodies[top].orbit.a * (1.0 - sys.bodies[top].orbit.e)
        }
    }

    fn engulf(&mut self, s: usize, t: f64) {
        let r_star = self.systems[s].star.current_radius(t);
        let star_name = self.systems[s].star.name.clone();
        let victims: Vec<usize> = (0..self.systems[s].bodies.len())
            .filter(|&j| self.systems[s].bodies[j].exists() && self.systems[s].bodies[j].parent.is_none())
            .filter(|&j| Self::closest_approach(&self.systems[s], j, t) < r_star + self.systems[s].bodies[j].radius)
            .collect();
        for j in victims {
            let name = self.systems[s].bodies[j].name.clone();
            self.remove_with_moons(s, j, t, RemovalCause::MergedInto(None), &format!("was engulfed by the red giant {star_name}"));
            self.history.push(Event { time: t, category: Category::Astronomy, importance: 5, title: format!("{name} engulfed by {star_name}"), detail: format!("The swelling red giant, now {:.2} AU across in radius, swallows the planet.", r_star / crate::astro::AU), system: Some(s as u32), body: Some(j as u32), civ: None, cause: None });
        }
    }

    /// Remove a body and its moons (they share its fate).
    fn remove_with_moons(&mut self, s: usize, j: usize, t: f64, cause: RemovalCause, how: &str) {
        let mut doomed = vec![j];
        let mut k = 0;
        while k < doomed.len() {
            let p = doomed[k];
            doomed.extend(self.systems[s].moons_of(p).collect::<Vec<_>>());
            k += 1;
        }
        for &b in &doomed {
            let sys = &mut self.systems[s];
            if sys.bodies[b].removed.is_some() {
                continue;
            }
            sys.bodies[b].removed = Some(Removal { time: t, cause });
            if let Some(d) = sys.dynamics.as_mut() {
                if let Some(slot) = d.bodies.get_mut(b) {
                    *slot = None;
                }
            }
            self.on_body_destroyed(s, b, t, how);
        }
    }

    fn end_of_life(&mut self, s: usize, t: f64) {
        let sys = &mut self.systems[s];
        if sys.dynamics.is_some() {
            sys.sync_to(t);
        }
        // States of every top-level body before the change (moons ride along).
        let tops: Vec<usize> = (0..sys.bodies.len()).filter(|&j| sys.bodies[j].exists() && sys.bodies[j].parent.is_none()).collect();
        let before: Vec<(usize, crate::astro::dynamics::State)> = tops.iter().map(|&j| (j, sys.body_state(j, t))).collect();
        let star_state = crate::astro::dynamics::State { pos: sys.star_local_position(t), vel: sys.dynamics.as_ref().map(|d| d.star.vel).unwrap_or_default() };
        let m0 = sys.star.mass;
        let name = sys.star.name.clone();
        let fate = sys.star.become_remnant(t);
        let m1 = fate.mass;
        let seed = crate::rng::mix(s as u64, t.to_bits());

        if !fate.supernova {
            // Slow wind: orbits expand adiabatically (a ∝ 1/M), keeping their shape.
            let k = m0 / m1;
            for (j, st) in &before {
                let rel_p = (st.pos - star_state.pos) * k;
                let rel_v = (st.vel - star_state.vel) / k;
                Self::set_top_state(sys, *j, star_state.pos + rel_p, star_state.vel + rel_v, t);
            }
            sys.nebulae.push(Nebula { kind: NebulaKind::Planetary, born: t, speed: 20_000.0, mass: m0 - m1, seed });
            self.history.push(Event {
                time: t,
                category: Category::Astronomy,
                importance: 5,
                title: format!("{name} becomes a white dwarf"),
                detail: format!("It sheds {:.2} M☉ as a glowing planetary nebula; a {:.2} M☉ white dwarf the size of Earth remains. Surviving orbits widen {k:.2}×.", m0 - m1, m1),
                system: Some(s as u32),
                body: None,
                civ: None, cause: None,
            });
            self.refresh_system(s, t);
            return;
        }

        // Core collapse.
        let remnant = if fate.kind == StarKind::BlackHole { "black hole" } else { "neutron star" };
        self.history.push(Event {
            time: t,
            category: Category::Astronomy,
            importance: 5,
            title: format!("Supernova! {name} explodes"),
            detail: format!("The core of the {m0:.1} M☉ star collapses into a {m1:.1} M☉ {remnant}; ~10⁴⁴ J of debris races out at 10 000 km/s."),
            system: Some(s as u32),
            body: None,
            civ: None, cause: None,
        });
        let sys = &mut self.systems[s];
        sys.nebulae.push(Nebula { kind: NebulaKind::SupernovaRemnant, born: t, speed: 1.0e7, mass: m0 - m1, seed });
        // Worlds: vaporised if they intercept more than their binding energy.
        let mut vaporised = Vec::new();
        let mut scorched = Vec::new();
        for j in 0..sys.bodies.len() {
            let b = &sys.bodies[j];
            if !b.exists() {
                continue;
            }
            let d = (sys.body_local_position(j, t) - star_state.pos).length().max(b.radius);
            let hit = SUPERNOVA_ENERGY * b.radius * b.radius / (4.0 * d * d);
            let binding = 0.6 * G * b.mass * b.mass / b.radius;
            if hit > binding {
                vaporised.push(j);
            } else {
                scorched.push((j, hit / binding));
            }
        }
        for j in vaporised {
            let n = self.systems[s].bodies[j].name.clone();
            self.remove_with_moons(s, j, t, RemovalCause::Vaporised, "was vaporised by a supernova");
            self.history.push(Event { time: t, category: Category::Disaster, importance: 5, title: format!("{n} vaporised"), detail: "The supernova blast delivers more energy than holds the world together.".into(), system: Some(s as u32), body: Some(j as u32), civ: None, cause: None });
        }
        for (j, frac) in scorched {
            if !self.systems[s].bodies[j].exists() {
                continue;
            }
            let b = &mut self.systems[s].bodies[j];
            // Atmospheres and oceans are blown off; surfaces melt where the blast is strong.
            b.atmosphere.pressure_bar *= (1.0 - frac.sqrt().min(1.0)).max(0.0) * 0.01;
            b.hydro.water_inventory *= 0.01;
            b.hydro.ocean_fraction = 0.0;
            self.sterilise_world(s, j, t, "the supernova's blast and radiation");
            self.refresh_body_environment(s, j, t);
        }
        // Sudden mass loss: bound or not now depends on the new, lighter remnant.
        let sys = &mut self.systems[s];
        let mut escaped = Vec::new();
        if sys.dynamics.is_none() {
            for (j, st) in &before {
                if !sys.bodies[*j].exists() {
                    continue;
                }
                let mu = sys.star.mu() + sys.bodies[*j].mu();
                match Orbit::from_state(st.pos - star_state.pos, st.vel - star_state.vel, mu, t) {
                    Some(o) if o.e < 1.0 => sys.bodies[*j].orbit = o,
                    _ => escaped.push(*j),
                }
            }
        }
        for j in escaped {
            let n = self.systems[s].bodies[j].name.clone();
            self.remove_with_moons(s, j, t, RemovalCause::Ejected, "was flung out of the system");
            self.history.push(Event { time: t, category: Category::Astronomy, importance: 4, title: format!("{n} escapes"), detail: "With most of the star's mass gone, its orbit is no longer bound: it drifts into interstellar space.".into(), system: Some(s as u32), body: Some(j as u32), civ: None, cause: None });
        }
        self.blasts.push(Blast { system: s as u32, time: t, energy: SUPERNOVA_ENERGY, reached: vec![s as u32] });
        self.refresh_system(s, t);
    }

    fn set_top_state(sys: &mut StarSystem, j: usize, pos: crate::Vec3d, vel: crate::Vec3d, t: f64) {
        match sys.dynamics.as_mut() {
            Some(d) => {
                if let Some(Some(st)) = d.bodies.get_mut(j) {
                    *st = crate::astro::dynamics::State { pos, vel };
                }
            }
            None => {
                let mu = sys.star.mu() + sys.bodies[j].mu();
                let star = crate::Vec3d::ZERO;
                if let Some(o) = Orbit::from_state(pos - star, vel, mu, t) {
                    sys.bodies[j].orbit = o;
                }
            }
        }
    }

    /// Climate and habitability of every world after the star changed.
    fn refresh_system(&mut self, s: usize, t: f64) {
        for j in 0..self.systems[s].bodies.len() {
            if self.systems[s].bodies[j].exists() {
                self.refresh_body_environment(s, j, t);
            }
        }
    }

    fn sterilise_world(&mut self, s: usize, j: usize, t: f64, why: &str) {
        let name = self.systems[s].bodies[j].name.clone();
        let mut lost = false;
        if let Some(bio) = self.biospheres.iter_mut().find(|x| x.system == s as u32 && x.body == j as u32) {
            if bio.stage > Stage::Sterile {
                lost = true;
                bio.stage = Stage::Sterile;
                bio.stage_since = t;
                bio.biomass = 0.0;
                bio.biodiversity = 0.0;
                bio.photosynthesis_since = None;
                bio.civilization_present = false;
            }
        }
        if lost {
            self.history.push(Event { time: t, category: Category::Disaster, importance: 5, title: format!("{name} sterilised"), detail: format!("All life is lost: {why}."), system: Some(s as u32), body: Some(j as u32), civ: None, cause: None });
        }
        self.on_body_destroyed(s, j, t, &format!("was sterilised by {why}"));
    }

    /// Radiation fronts reaching other systems.
    fn propagate_blasts(&mut self, t: f64) {
        for bi in 0..self.blasts.len() {
            let (origin, t0, energy) = {
                let b = &self.blasts[bi];
                (self.systems[b.system as usize].position, b.time, b.energy)
            };
            let source = self.systems[self.blasts[bi].system as usize].star.name.clone();
            for s in 0..self.systems.len() {
                if self.blasts[bi].reached.contains(&(s as u32)) {
                    continue;
                }
                let dist = (self.systems[s].position - origin).length();
                if dist > SPEED_OF_LIGHT * (t - t0) {
                    continue;
                }
                self.blasts[bi].reached.push(s as u32);
                let fluence = energy * RADIATION_FRACTION / (4.0 * std::f64::consts::PI * dist * dist);
                if fluence < FLUENCE_HARM {
                    continue;
                }
                let severity = ((fluence / FLUENCE_HARM).log10() / (FLUENCE_STERILE / FLUENCE_HARM).log10()).clamp(0.0, 1.0);
                let peak = crate::impact::blast_winter_peak_k(severity);
                // Climate forcing first (build 61): at arrival every surface world takes
                // the blast winter — deliberately *before* the biosphere gate below, so
                // lifeless and sterile worlds cool too. The Chronicle event stays gated
                // on an active biosphere. Pre-refresh state feeds the event's detail.
                let mut pre: Vec<(usize, f64, f64)> = Vec::new();
                if peak > 0.05 {
                    for j in 0..self.systems[s].bodies.len() {
                        let b = &self.systems[s].bodies[j];
                        if !b.exists() || !b.kind.has_surface() {
                            continue;
                        }
                        let hab = self.biospheres.iter().find(|x| x.system == s as u32 && x.body == j as u32).map(|x| x.habitability.score).unwrap_or(0.0);
                        pre.push((j, b.temperature, hab));
                        // Stack onto an active winter: restart the decay clock with
                        // peak = max(existing instantaneous cooling, new peak), so
                        // simultaneous blasts stay deterministic and within the 30 K
                        // cap. Forcing never touches `climate_bias` (that is the
                        // real-body calibration offset).
                        let body = &mut self.systems[s].bodies[j];
                        let peak_now = body.impact_winter.map(|w| w.cooling_at(t)).unwrap_or(0.0).max(peak);
                        body.impact_winter = Some(ImpactWinter { start: t, peak_k: peak_now, efold_years: ImpactWinter::EFOLD_YEARS });
                        self.refresh_body_environment(s, j, t);
                    }
                    // Static universes schedule no environment task; make sure the
                    // winter still decays and clears there (`ensure_environment_task`
                    // now also watches for active winters).
                    self.ensure_environment_task();
                }
                for j in 0..self.systems[s].bodies.len() {
                    if !self.systems[s].bodies[j].exists() {
                        continue;
                    }
                    let Some(bi2) = self.biospheres.iter().position(|x| x.system == s as u32 && x.body == j as u32 && x.stage > Stage::Sterile) else { continue };
                    let name = self.systems[s].bodies[j].name.clone();
                    if severity >= 1.0 {
                        self.sterilise_world(s, j, t, &format!("radiation from the supernova of {source}"));
                        continue;
                    }
                    // Chronicle detail: the causal chain (climate → habitability →
                    // capacity), read around the arrival refresh above. The civ's
                    // capacity is its cached value, recomputed at the next civ step
                    // (documented one-step staleness in docs/CLIMATE_DESIGN.md).
                    let (temp_before, hab_before) = pre.iter().find(|(jj, _, _)| *jj == j).copied().map(|(_, tp, hb)| (tp, hb)).unwrap_or_else(|| (self.systems[s].bodies[j].temperature, self.biospheres[bi2].habitability.score));
                    let temp_after = self.systems[s].bodies[j].temperature;
                    let hab_after = self.biospheres[bi2].habitability.score;
                    let winter_k = self.systems[s].bodies[j].impact_winter.map(|w| w.peak_k).unwrap_or(peak);
                    let cap_term = self
                        .civ_on(BodyRef { system: s as u32, body: j as u32 })
                        .map(|c| format!(", capacity {}", group_digits(c.capacity)))
                        .unwrap_or_default();
                    let detail = format!(
                        "The explosion of {source}, {:.1} light-years away, strips the ozone layer: {:.0}% of species are lost. Radiation severity {:.0}%, winter −{:.1} K, temperature {:.1} K → {:.1} K ({:+.1} K), habitability {:.0} % → {:.0} %{}.",
                        dist / LIGHT_YEAR,
                        80.0 * severity,
                        severity * 100.0,
                        winter_k,
                        temp_before,
                        temp_after,
                        temp_after - temp_before,
                        hab_before * 100.0,
                        hab_after * 100.0,
                        cap_term
                    );
                    let bio = &mut self.biospheres[bi2];
                    bio.biodiversity *= 1.0 - 0.8 * severity;
                    bio.biomass *= 1.0 - 0.5 * severity;
                    for c in self.civs.iter_mut().filter(|c| c.system == s as u32 && c.body == j as u32 && c.is_alive()) {
                        c.population *= 1.0 - 0.3 * severity;
                        c.pressures.food = c.pressures.food.max(severity);
                        c.pressures.contact = 1.0;
                    }
                    self.history.push(Event {
                        time: t,
                        category: Category::Life,
                        importance: 5,
                        title: format!("Supernova radiation reaches {name}"),
                        detail,
                        system: Some(s as u32),
                        body: Some(j as u32),
                        civ: None, cause: None,
                    });
                }
            }
        }
        self.blasts.retain(|b| b.reached.len() < self.systems.len());
    }
}

/// Energy (J) delivering `severity` (0 = harm threshold, 1 = sterility threshold) at
/// `dist_m`: solved backwards from the real distance from the exact severity formula in
/// `propagate_blasts`, so tests invent no constants. Test-only (shared by the chain
/// tests in `stellar.rs`, `universe.rs` and `sandbox.rs`); pure IEEE ops.
#[cfg(test)]
pub(crate) fn energy_for_severity(dist_m: f64, severity: f64) -> f64 {
    let fluence = FLUENCE_HARM * 10f64.powf(severity * (FLUENCE_STERILE / FLUENCE_HARM).log10());
    fluence * 4.0 * std::f64::consts::PI * dist_m * dist_m / RADIATION_FRACTION
}

/// Index of the system nearest the Sun (asserted to exist by callers).
#[cfg(test)]
pub(crate) fn nearest_to_sun(u: &Universe) -> usize {
    (1..u.systems.len()).min_by(|&a, &b| u.systems[a].position.length().total_cmp(&u.systems[b].position.length())).expect("neighbour system")
}

/// Fire a blast from the nearest neighbour at the Sun's system with the given severity
/// on arrival at system 0; returns the distance (m).
#[cfg(test)]
pub(crate) fn inject_blast(u: &mut Universe, severity: f64) -> f64 {
    let n = nearest_to_sun(u);
    let dist = u.systems[n].position.length();
    u.blasts.push(Blast { system: n as u32, time: u.time, energy: energy_for_severity(dist, severity), reached: vec![n as u32] });
    u.ensure_star_task();
    dist
}

/// The stellar task's yearly grid: a blast's arrival is first stepped at or after
/// `t0 + d/c` (`STAR_PERIOD` = 1 yr).
#[cfg(test)]
pub(crate) fn arrival_grid(t0: f64, dist: f64) -> f64 {
    t0 + (dist / SPEED_OF_LIGHT / SECONDS_PER_YEAR).ceil() * SECONDS_PER_YEAR
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::star::Star;
    use crate::time::{SECONDS_PER_DAY, SECONDS_PER_GYR};
    use crate::{BodyRef, Scenario, UniverseSettings};

    fn lab_kepler() -> Universe {
        let mut u = Universe::new(UniverseSettings { scenario: Scenario::Sol, seed: 3, ..Default::default() });
        u.civs.clear();
        u
    }

    /// A static Sol universe: system 0 is the real Solar System, five procedural
    /// neighbours; no system is dynamic, so only scheduled clocks run.
    fn static_sol(seed: u64) -> Universe {
        Universe::new(UniverseSettings { scenario: Scenario::Sol, seed, system_count: 6, ..Default::default() })
    }

    /// Index of an existing surface body of system 0 with no active biosphere.
    fn lifeless_surface_body(u: &Universe) -> usize {
        (0..u.systems[0].bodies.len())
            .find(|&j| {
                let b = &u.systems[0].bodies[j];
                b.exists() && b.kind.has_surface() && u.biosphere(BodyRef { system: 0, body: j as u32 }).is_none_or(|bio| bio.stage == Stage::Sterile)
            })
            .expect("Sol must have a lifeless surface world")
    }

    #[test]
    fn the_sun_swallows_mercury_and_venus_and_leaves_a_white_dwarf() {
        let mut u = lab_kepler();
        let a_mars = u.systems[0].bodies[u.systems[0].find_body("Mars").unwrap()].orbit.a;
        let end = u.systems[0].star.end_of_life();
        u.advance_to(end + 2.0 * SECONDS_PER_YEAR * 1000.0, None, None);
        let sys = &u.systems[0];
        assert_eq!(sys.star.kind, StarKind::WhiteDwarf);
        let gone = |n: &str| !sys.bodies[sys.find_body(n).unwrap()].exists();
        assert!(gone("Mercury") && gone("Venus"), "inner planets engulfed");
        let mars = &sys.bodies[sys.find_body("Mars").unwrap()];
        assert!(mars.exists());
        // Mass loss from 1 to ~0.5 M☉ roughly doubles surviving orbits.
        let k = mars.orbit.a / a_mars;
        assert!((1.8..2.2).contains(&k), "Mars orbit widened {k}×");
        assert!(sys.nebulae.iter().any(|n| n.kind == NebulaKind::Planetary));
        // Earth sits right at the giant's maximum radius; whatever happened, it's lifeless.
        let e = BodyRef { system: 0, body: sys.find_body("Earth").unwrap() as u32 };
        assert!(u.biosphere(e).is_none_or(|b| b.stage == Stage::Sterile));
    }

    #[test]
    fn a_massive_star_explodes_and_leaves_a_black_hole() {
        let mut u = lab_kepler();
        let life = Star::from_mass("x".into(), 30.0, 0.0, 0.0).lifetime;
        let t = u.time;
        // A 30 M☉ star at the very end of its life in place of the Sun.
        u.systems[0].star = Star::from_mass("Doomed".into(), 30.0, 0.0, t - life * 1.1199);
        u.ensure_star_task();
        u.advance_by(2000.0 * SECONDS_PER_YEAR);
        let sys = &u.systems[0];
        assert_eq!(sys.star.kind, StarKind::BlackHole);
        // The red supergiant (~1400 R☉ ≈ 6.5 AU) swallows the inner system and Jupiter first.
        let earth = &sys.bodies[sys.find_body("Earth").unwrap()];
        assert!(matches!(earth.removed.unwrap().cause, RemovalCause::MergedInto(None)));
        // 30 → ~9.5 M☉ in an instant: more than half the mass gone, so Saturn is unbound.
        let saturn = &sys.bodies[sys.find_body("Saturn").unwrap()];
        assert!(matches!(saturn.removed.map(|r| r.cause), Some(RemovalCause::Ejected)), "{:?}", saturn.removed);
        assert!(u.history.events.iter().any(|e| e.title.starts_with("Supernova!")));
        let _ = SECONDS_PER_GYR;
    }

    // ── Build 61: blast → climate chain ─────────────────────────────────────

    /// Climate forcing must land before the biosphere gate: a world whose biosphere is
    /// sterile still cools, while the Chronicle event stays biosphere-gated.
    #[test]
    fn blast_forcing_reaches_worlds_without_biospheres() {
        let mut u = static_sol(42);
        let lifeless = lifeless_surface_body(&u);
        let name = u.systems[0].bodies[lifeless].name.clone();
        let temp0 = u.systems[0].bodies[lifeless].temperature;
        let t0 = u.time;
        let dist = inject_blast(&mut u, 0.5);
        let arrive = arrival_grid(t0, dist);
        u.advance_to(arrive, None, None);
        assert_eq!(u.time, arrive, "stepped to arrival");
        // The lifeless world took the winter…
        let b = &u.systems[0].bodies[lifeless];
        let w = b.impact_winter.expect("lifeless world must receive blast forcing");
        assert!(w.peak_k > 0.0 && w.peak_k <= 30.0, "{} K", w.peak_k);
        let drop = temp0 - b.temperature;
        assert!(drop > 1.0 && drop <= 30.0, "lifeless {name} cooled {drop} K");
        // …with no Chronicle event (the event is gated on an active biosphere)…
        assert!(!u.history.events.iter().any(|e| e.title == format!("Supernova radiation reaches {name}")), "no event for lifeless {name}");
        // …while living Earth reports it.
        assert!(u.history.events.iter().any(|e| e.title == "Supernova radiation reaches Earth"));
    }

    /// The static-system wiring fix: expiry runs in `step_environment`, which must be
    /// scheduled (and must do its winter branch) even when no system is dynamic.
    #[test]
    fn blast_winter_expires_on_static_system() {
        let mut u = static_sol(42);
        assert!(u.systems.iter().all(|s| !s.is_dynamic()), "static precondition");
        let surface: Vec<usize> = (0..u.systems[0].bodies.len()).filter(|&j| u.systems[0].bodies[j].exists() && u.systems[0].bodies[j].kind.has_surface()).collect();
        assert!(!surface.is_empty());
        let temps0: Vec<f64> = surface.iter().map(|&j| u.systems[0].bodies[j].temperature).collect();
        let t0 = u.time;
        let dist = inject_blast(&mut u, 0.9);
        // At arrival every surface world must carry the winter (forcing is not gated on
        // biospheres)…
        u.advance_to(arrival_grid(t0, dist), None, None);
        for &j in &surface {
            let b = &u.systems[0].bodies[j];
            let w = b.impact_winter.expect("blast winter applied at arrival");
            assert!(w.peak_k > 0.05 && w.peak_k <= 30.0, "{}: {} K", b.name, w.peak_k);
        }
        // …and 25 yr later it has decayed past the 0.05 K expiry and been cleared.
        u.advance_by(25.0 * SECONDS_PER_YEAR);
        // Every surface world: winter gone, temperature back within 0.1 K of pre-blast.
        for (&j, &before) in surface.iter().zip(&temps0) {
            let b = &u.systems[0].bodies[j];
            assert!(b.impact_winter.is_none(), "{} winter still active after 25 yr", b.name);
            assert!((b.temperature - before).abs() < 0.1, "{}: {before} -> {} K", b.name, b.temperature);
        }
    }

    /// A blast saved mid-transit reloads and arrives exactly like an uninterrupted run:
    /// same event times, same temperature drop.
    #[test]
    fn blast_transit_and_winter_survive_save_roundtrip() {
        let mut u = static_sol(42);
        let earth0 = u.systems[0].find_body("Earth").unwrap();
        let temp0 = u.systems[0].bodies[earth0].temperature;
        let dist = inject_blast(&mut u, 0.5);
        assert_eq!(u.blasts.len(), 1, "blast in transit");
        let json = crate::save::to_json(&u, "blast-transit").unwrap();
        let mut loaded = crate::save::from_json(&json).unwrap();
        assert_eq!(loaded.blasts, u.blasts, "in-transit blast persists through save/load");
        let mut straight = u.clone();
        let total = 10.0 * SECONDS_PER_YEAR;
        straight.advance_by(total);
        loaded.advance_by(total);
        let arrivals = |uu: &Universe| -> Vec<f64> {
            uu.history.events.iter().filter(|e| e.title.starts_with("Supernova radiation reaches")).map(|e| e.time).collect()
        };
        assert!(!arrivals(&straight).is_empty(), "arrival happened");
        assert_eq!(arrivals(&straight), arrivals(&loaded), "arrival time identical");
        let earth = straight.systems[0].find_body("Earth").unwrap();
        let t_straight = straight.systems[0].bodies[earth].temperature;
        let t_loaded = loaded.systems[0].bodies[earth].temperature;
        assert_eq!(t_straight, t_loaded, "temperature drop identical");
        assert!(t_straight < temp0, "Earth actually cooled: {temp0} -> {t_straight}");
        assert!(dist > 0.0);
        // Full state (history, winters, scheduler) identical — save/load changed nothing.
        assert_eq!(serde_json::to_string(&straight).unwrap(), serde_json::to_string(&loaded).unwrap());
    }

    /// The full chain in one scenario: a blast from a neighbour system drives Earth's
    /// climate → habitability → civ capacity → Chronicle detail, while a lifeless world
    /// in the same system cools silently.
    #[test]
    fn supernova_blast_drives_climate_habitability_capacity_and_chronicle() {
        use crate::sandbox::ENVIRONMENT_PERIOD;
        let mut u = static_sol(42);
        // Preconditions (asserted, never skipped): neighbour system + living Earth civ.
        assert_eq!(u.systems.len(), 6, "six systems");
        let n = nearest_to_sun(&u);
        let earth = u.systems[0].find_body("Earth").expect("Sol has Earth");
        let er = BodyRef { system: 0, body: earth as u32 };
        assert!(u.biosphere(er).is_some_and(|b| b.stage > Stage::Sterile), "Earth alive");
        assert!(u.civ_on(er).is_some_and(|c| c.is_alive()), "early humans present");
        let lifeless = lifeless_surface_body(&u);
        let lifeless_name = u.systems[0].bodies[lifeless].name.clone();
        let dist = u.systems[n].position.length();
        assert!(dist / SPEED_OF_LIGHT > 1.2 * SECONDS_PER_YEAR, "seed 42: arrival far enough out for a civ step first ({:.2} ly)", dist / LIGHT_YEAR);
        let t0 = u.time;
        let energy = energy_for_severity(dist, 0.5);
        u.blasts.push(Blast { system: n as u32, time: t0, energy, reached: vec![n as u32] });
        u.ensure_star_task();
        let arrive = t0 + dist / SPEED_OF_LIGHT;
        let arrive_grid = arrival_grid(t0, dist);
        let peak = 15.0; // severity 0.5 → 0.5 · 30 K

        // Phase A — just before arrival: baseline climate, habitability, capacity.
        u.advance_to(arrive - SECONDS_PER_DAY, None, None);
        let temp0 = u.systems[0].bodies[earth].temperature;
        let hab0 = u.biosphere(er).unwrap().habitability.score;
        let cap0 = u.civ_on(er).unwrap().capacity;
        assert!(cap0 > 0.0, "civilization stepped before arrival: capacity {cap0}");
        let lifeless0 = u.systems[0].bodies[lifeless].temperature;
        assert!(u.history.events.iter().all(|e| !e.title.starts_with("Supernova radiation reaches")), "not arrived yet");

        // Phase B — arrival + the first environment refresh (10 d later).
        u.advance_to(arrive_grid + ENVIRONMENT_PERIOD, None, None);
        let (ev_time, ev_detail) = {
            let ev = u.history.events.iter().find(|e| e.title == "Supernova radiation reaches Earth").expect("Chronicle event");
            (ev.time, ev.detail.clone())
        };
        assert!(ev_time >= arrive && ev_time <= arrive + SECONDS_PER_YEAR, "event at {:.2} yr after t0, arrival {:.2} yr", (ev_time - t0) / SECONDS_PER_YEAR, (arrive - t0) / SECONDS_PER_YEAR);
        // T4 surfacing: the detail carries the causal chain.
        for term in ["habitability", "capacity", "K", "severity", "winter"] {
            assert!(ev_detail.contains(term), "event detail missing {term:?}: {ev_detail}");
        }
        // ΔT = peak·exp(−(t_assert − t_event)/3 yr) within 1 %; bounded 0 < drop ≤ 30 K.
        let drop = temp0 - u.systems[0].bodies[earth].temperature;
        let expected = peak * (-(u.time - ev_time) / (3.0 * SECONDS_PER_YEAR)).exp();
        assert!(drop > 0.0 && drop <= 30.0, "drop {drop} K");
        assert!((drop - expected).abs() <= 0.01 * expected, "drop {drop} vs expected {expected}");
        // Habitability cache fell by more than 0.01 while the winter is near peak.
        let hab1 = u.biosphere(er).unwrap().habitability.score;
        assert!(hab0 - hab1 > 0.01, "habitability {hab0} -> {hab1}");
        // Lifeless body in the same system cooled with no event.
        let lifeless_drop = lifeless0 - u.systems[0].bodies[lifeless].temperature;
        assert!(lifeless_drop > 1.0 && lifeless_drop <= 30.0, "lifeless {lifeless_name} cooled {lifeless_drop} K");
        assert!(!u.history.events.iter().any(|e| e.title == format!("Supernova radiation reaches {lifeless_name}")), "no event for lifeless {lifeless_name}");

        // Phase C — after ≥1 civ step post-arrival: capacity fell.
        u.advance_to(arrive_grid + SECONDS_PER_YEAR + 20.0 * SECONDS_PER_DAY, None, None);
        let cap1 = u.civ_on(er).expect("humanity survives").capacity;
        assert!(cap1 < cap0, "capacity {cap0} -> {cap1}");
        let drop_end = temp0 - u.systems[0].bodies[earth].temperature;
        let expected_end = peak * (-(u.time - ev_time) / (3.0 * SECONDS_PER_YEAR)).exp();
        assert!((drop_end - expected_end).abs() <= 0.01 * expected_end, "drop {drop_end} vs expected {expected_end}");
    }

    /// The forcing law, pinned as pure arithmetic: severity (the bounded log-ratio of
    /// fluence between the harm and sterility thresholds) maps linearly onto ≤30 K of
    /// surface-air cooling — the same physical cap `winter_peak_cooling` documents for
    /// impacts. Pure IEEE ops (`clamp`, multiply); no DMath or RNG is involved and no
    /// result here feeds a trajectory outside `ImpactWinter`.
    #[test]
    fn blast_winter_is_bounded_and_decays() {
        use crate::impact::{blast_winter_peak_k, ImpactWinter, BLAST_WINTER_MAX_K};
        assert_eq!(blast_winter_peak_k(0.0), 0.0);
        assert_eq!(blast_winter_peak_k(0.5), 15.0);
        assert_eq!(blast_winter_peak_k(1.0), BLAST_WINTER_MAX_K);
        // Out-of-range severities clamp instead of overcooling or heating.
        assert_eq!(blast_winter_peak_k(-0.4), 0.0);
        assert_eq!(blast_winter_peak_k(7.0), BLAST_WINTER_MAX_K);
        for s in [0.1, 0.33, 0.9, 1.0] {
            let peak = blast_winter_peak_k(s);
            assert!(peak > 0.0 && peak <= 30.0, "severity {s}: {peak} K");
            let w = ImpactWinter { start: 0.0, peak_k: peak, efold_years: ImpactWinter::EFOLD_YEARS };
            // Three e-folds leave e⁻³ ≈ 4.98 % of the peak.
            let after = w.cooling_at(3.0 * ImpactWinter::EFOLD_YEARS * SECONDS_PER_YEAR);
            assert!(after < 0.05 * peak, "severity {s}: {after} K is not <5% of {peak}");
        }
    }

    #[test]
    fn nebulae_expand_and_fade() {
        let n = Nebula { kind: NebulaKind::SupernovaRemnant, born: 0.0, speed: 1e7, mass: 10.0, seed: 1 };
        let yr = SECONDS_PER_YEAR;
        assert!(n.radius(1000.0 * yr) > n.radius(100.0 * yr));
        // Sedov phase grows slower than free expansion.
        assert!(n.radius(10_000.0 * yr) < 1e7 * 10_000.0 * yr * 0.5);
        assert!(n.brightness(200_000.0 * yr) == 0.0);
    }
}

#[cfg(test)]
mod what_if_tests {
    use crate::sandbox::apply_what_if;
    use crate::time::SECONDS_PER_YEAR;
    use crate::{Scenario, Universe, UniverseSettings};

    #[test]
    fn the_supernova_experiment_explodes_within_decades() {
        let mut u = Universe::new(UniverseSettings { scenario: Scenario::SolarSystemLab, ..Default::default() });
        apply_what_if(&mut u, "sun_supernova").unwrap();
        u.advance_by(60.0 * SECONDS_PER_YEAR);
        let s = &u.systems[0];
        assert!(s.star.kind != crate::astro::star::StarKind::Normal, "still {:?} phase {:?}", s.star.kind, s.star.phase(u.time));
        assert!(!s.nebulae.is_empty());
        let r = s.nebulae[0].radius(u.time) / crate::astro::AU;
        assert!(r > 20.0, "remnant radius {r} AU");
    }
}
