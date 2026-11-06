//! Known exoplanet systems.
//!
//! Constructed from published orbital parameters. Each body carries provenance
//! tracking so users can trace data to its source.

use crate::astro::{AU, Body, BodyKind, G};
use crate::astro::object::Provenance;
use crate::astro::star::Star;
use crate::astro::system::StarSystem;
use crate::time::SECONDS_PER_GYR;

/// A known exoplanet system.
#[derive(Clone, Debug, PartialEq)]
pub struct ExoplanetSystem {
    pub name: &'static str,
    pub star_mass_solar: f64,
    pub star_temp_k: f64,
    pub distance_pc: f64,
    pub bodies: &'static [ExoplanetBody],
}

/// One exoplanet body.
#[derive(Clone, Debug, PartialEq)]
pub struct ExoplanetBody {
    pub name: &'static str,
    pub mass_earth: f64,
    pub radius_earth: f64,
    pub semi_major_axis_au: f64,
    pub eccentricity: f64,
    pub inclination_deg: f64,
}

/// TRAPPIST-1 system (Gillon et al. 2017): 7 Earth-sized planets, 3 in the habitable zone.
pub const TRAPPIST_1: ExoplanetSystem = ExoplanetSystem {
    name: "TRAPPIST-1",
    star_mass_solar: 0.089,
    star_temp_k: 2559.0,
    distance_pc: 12.95,
    bodies: &[
        ExoplanetBody { name: "TRAPPIST-1 b", mass_earth: 1.374, radius_earth: 1.115, semi_major_axis_au: 0.01111, eccentricity: 0.006, inclination_deg: 89.742 },
        ExoplanetBody { name: "TRAPPIST-1 c", mass_earth: 1.308, radius_earth: 1.097, semi_major_axis_au: 0.01521, eccentricity: 0.00655, inclination_deg: 89.636 },
        ExoplanetBody { name: "TRAPPIST-1 d", mass_earth: 0.388, radius_earth: 0.772, semi_major_axis_au: 0.02144, eccentricity: 0.00818, inclination_deg: 89.740 },
        ExoplanetBody { name: "TRAPPIST-1 e", mass_earth: 0.692, radius_earth: 0.918, semi_major_axis_au: 0.02817, eccentricity: 0.00516, inclination_deg: 89.806 },
        ExoplanetBody { name: "TRAPPIST-1 f", mass_earth: 1.039, radius_earth: 1.045, semi_major_axis_au: 0.03710, eccentricity: 0.00682, inclination_deg: 89.675 },
        ExoplanetBody { name: "TRAPPIST-1 g", mass_earth: 1.152, radius_earth: 1.127, semi_major_axis_au: 0.04510, eccentricity: 0.00531, inclination_deg: 89.584 },
        ExoplanetBody { name: "TRAPPIST-1 h", mass_earth: 0.326, radius_earth: 0.755, semi_major_axis_au: 0.05830, eccentricity: 0.00795, inclination_deg: 89.513 },
    ],
};

/// Proxima Centauri system (Anglada-Escudé et al. 2016): closest known exoplanet.
pub const PROXIMA_CENTAURI: ExoplanetSystem = ExoplanetSystem {
    name: "Proxima Centauri",
    star_mass_solar: 0.122,
    star_temp_k: 3042.0,
    distance_pc: 1.30,
    bodies: &[
        ExoplanetBody { name: "Proxima Centauri b", mass_earth: 1.17, radius_earth: 1.0, semi_major_axis_au: 0.0485, eccentricity: 0.0, inclination_deg: 0.0 },
    ],
};

/// Known exoplanet systems catalogue.
pub const EXOPLANET_SYSTEMS: &[ExoplanetSystem] = &[TRAPPIST_1, PROXIMA_CENTAURI];

/// Provenance marker for catalogue-derived bodies.
pub fn catalogue_provenance(source: &'static str) -> Provenance {
    Provenance {
        source: source.to_string(),
        fields: Default::default(),
    }
}

/// Build a StarSystem from an ExoplanetSystem description.
pub fn build_system(sys: &ExoplanetSystem, id: u32) -> StarSystem {
    let earth_mass = 5.9722e24;
    let earth_radius = 6.371e6;
    let solar_mass = 1.98847e30;
    let star_mass = sys.star_mass_solar * solar_mass;
    let star = Star::from_mass(
        sys.name.into(),
        star_mass,
        0.0,
        -4.5 * SECONDS_PER_GYR,
    );
    let mut bodies: Vec<Body> = Vec::new();
    for (idx, pb) in sys.bodies.iter().enumerate() {
        let a = pb.semi_major_axis_au * AU;
        let orbit = crate::astro::Orbit {
            a,
            e: pb.eccentricity,
            i: pb.inclination_deg.to_radians(),
            node: 0.0,
            peri: 0.0,
            m0: 0.0,
        };
        let mu_parent = G * star_mass;
        let (rotation_period, tidally_locked) = if orbit.period(mu_parent) > 0.0 {
            (orbit.period(mu_parent), true)
        } else {
            (1.0, false)
        };
        let body = Body {
            id: idx as u32,
            name: pb.name.into(),
            kind: BodyKind::Rocky,
            parent: None,
            orbit,
            mass: pb.mass_earth * earth_mass,
            radius: pb.radius_earth * earth_radius,
            rotation_period,
            axial_tilt: 0.0,
            tidally_locked,
            albedo: 0.3,
            atmosphere: Default::default(),
            hydro: Default::default(),
            magnetic_field: 0.0,
            geology: 0.0,
            equilibrium_temperature: sys.star_temp_k,
            temperature: sys.star_temp_k,
            resources: Default::default(),
            deposits: Vec::new(),
            rings: None,
            terrain_seed: id as u64 * 1000 + idx as u64,
            sea_level: 0.0,
            color: [1.0, 1.0, 0.9],
            real: true,
            elevation_data: None,
            provenance: catalogue_provenance("Exoplanet Archive"),
            removed: None,
            impacts: Vec::new(),
            impact_winter: None,
            accretion: 0.0,
            melt: None,
            class: None,
            climate_bias: 0.0,
            glacial_cycle_years: 0.0,
            glacial_phase_years: 0.0,
            interglacial_fraction: 1.0,
            glacial_until_years: None,
        };
        bodies.push(body);
    }
    StarSystem {
        id,
        name: sys.name.into(),
        position: crate::Vec3d::new(0.0, 0.0, sys.distance_pc * 3.085677581e16),
        star,
        companion: None,
        bodies,
        belts: Vec::new(),
        dynamics: None,
        pending_contacts: Vec::new(),
        nebulae: Vec::new(),
    }
}

/// Number of exoplanet systems in the catalogue.
pub fn count() -> usize {
    EXOPLANET_SYSTEMS.len()
}
