//! Moons, rings, small bodies and encounter detail.

use serde::Serialize;
use crate::rng::Rng;
use crate::rng::domain;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmallBodyFeature { Moon, Ring, Asteroid, Comet, Meteoroid, }

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncounterDetail { None, Flyby, Rendezvous, Landing, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SmallBodyParams {
    pub feature_type: SmallBodyFeature,
    pub encounter: EncounterDetail,
    pub diameter_m: f64,
    pub distance_km: f64,
    pub relative_velocity_km_s: f64,
}

pub const DEFAULT_SMALL_BODY: SmallBodyParams = SmallBodyParams {
    feature_type: SmallBodyFeature::Moon,
    encounter: EncounterDetail::Flyby,
    diameter_m: 3_474_000.0,
    distance_km: 384_400.0,
    relative_velocity_km_s: 1.0,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Moon {
    pub name: String,
    pub diameter_m: f64,
    pub distance_km: f64,
    pub orbital_period_days: f64,
    pub mass_kg: f64,
}

pub fn generate_moons(seed: u64, planet_radius_km: f64, mass_kg: f64) -> Vec<Moon> {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0x3A7F]);
    let n = if planet_radius_km > 5000.0 { rng.range_u32(1, 8) }
    else if planet_radius_km > 1000.0 { rng.range_u32(0, 3) }
    else { rng.range_u32(0, 1) };
    let mut moons = Vec::with_capacity(n as usize);
    for i in 0..n {
        let d = planet_radius_km * 1000.0 * rng.range(0.02, 0.5);
        let dist = planet_radius_km * rng.range(3.0, 50.0);
        let mu = 6.67430e-11 * mass_kg;
        let period = (2.0 * std::f64::consts::PI * (dist * 1000.0).powf(1.5) / mu.sqrt()) / 86400.0;
        let mass = (d / 2.0).powi(3) * 3000.0 * 4.0 / 3.0 * std::f64::consts::PI;
        moons.push(Moon { name: format!("Moon-{i:02}"), diameter_m: d, distance_km: dist, orbital_period_days: period, mass_kg: mass });
    }
    moons
}

#[derive(Clone, Debug, PartialEq)]
pub struct RingSystem {
    pub inner_radius_km: f64,
    pub outer_radius_km: f64,
    pub particle_count: u64,
    pub opacity: f64,
}

pub fn generate_rings(seed: u64, planet_radius_km: f64) -> Option<RingSystem> {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0x9C21]);
    if planet_radius_km < 20000.0 && !rng.chance(0.1) { return None; }
    Some(RingSystem {
        inner_radius_km: planet_radius_km * rng.range(1.1, 1.5),
        outer_radius_km: planet_radius_km * rng.range(1.8, 3.0),
        particle_count: rng.range(1_000_000.0, 100_000_000.0) as u64,
        opacity: rng.range(0.1, 0.9),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comet {
    pub name: String,
    pub nucleus_diameter_km: f64,
    pub coma_diameter_km: f64,
    pub tail_length_km: f64,
    pub period_years: f64,
}

pub fn generate_comet(seed: u64) -> Comet {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0xB4D2]);
    let n = rng.range(1.0, 50.0);
    let c = n * rng.range(100.0, 10000.0);
    let t = c * rng.range(10.0, 1000.0);
    let p = rng.range(3.0, 100_000.0);
    Comet { name: format!("Comet-{seed:x}"), nucleus_diameter_km: n, coma_diameter_km: c, tail_length_km: t, period_years: p }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncounterTrajectory {
    pub detail: EncounterDetail,
    pub closest_approach_km: f64,
    pub relative_velocity_km_s: f64,
    pub duration_days: f64,
    pub delta_v_m_s: f64,
}

pub fn generate_encounter(seed: u64, detail: EncounterDetail) -> EncounterTrajectory {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0xD5E1]);
    match detail {
        EncounterDetail::None => EncounterTrajectory { detail, closest_approach_km: f64::INFINITY, relative_velocity_km_s: rng.range(5.0, 30.0), duration_days: 0.0, delta_v_m_s: 0.0 },
        EncounterDetail::Flyby => EncounterTrajectory { detail, closest_approach_km: rng.range(100.0, 10_000.0), relative_velocity_km_s: rng.range(5.0, 25.0), duration_days: rng.range(1.0, 30.0), delta_v_m_s: 0.0 },
        EncounterDetail::Rendezvous => EncounterTrajectory { detail, closest_approach_km: 0.0, relative_velocity_km_s: rng.range(0.0, 1.0), duration_days: rng.range(30.0, 365.0), delta_v_m_s: rng.range(100.0, 3000.0) },
        EncounterDetail::Landing => EncounterTrajectory { detail, closest_approach_km: 0.0, relative_velocity_km_s: rng.range(0.0, 0.1), duration_days: rng.range(1.0, 90.0), delta_v_m_s: rng.range(500.0, 4000.0) },
    }
}

pub fn feature_count(d: EncounterDetail) -> usize {
    match d { EncounterDetail::None => 0, EncounterDetail::Flyby => 1, EncounterDetail::Rendezvous => 3, EncounterDetail::Landing => 10 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn moons_deterministic() {
        assert_eq!(generate_moons(42, 6371.0, 5.972e24), generate_moons(42, 6371.0, 5.972e24));
    }
    #[test] fn larger_planet_more_moons() {
        let s = generate_moons(7, 500.0, 1e20).len();
        let l = generate_moons(7, 60000.0, 1e26).len();
        assert!(l >= s);
    }
    #[test] fn period_positive() {
        for m in &generate_moons(3, 6371.0, 5.972e24) { assert!(m.orbital_period_days > 0.0); }
    }
    #[test] fn comet_deterministic() {
        assert_eq!(generate_comet(99), generate_comet(99));
    }
    #[test] fn encounter_matches() {
        let e = generate_encounter(17, EncounterDetail::Landing);
        assert_eq!(e.detail, EncounterDetail::Landing);
        assert_eq!(e.closest_approach_km, 0.0);
    }
    #[test] fn fc_matches() {
        assert!(feature_count(EncounterDetail::Landing) > feature_count(EncounterDetail::Flyby));
        assert_eq!(feature_count(EncounterDetail::None), 0);
    }
}
