//! Star close-up activity (surface/atmosphere) to reference richness.

use serde::Serialize;
use crate::rng::Rng;
use crate::rng::domain;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct StellarFeature {
    pub kind: StellarFeatureKind,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub size_frac: f64,
    pub intensity: f64,
    pub lifetime_s: f64,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StellarFeatureKind {
    Granulation, Sunspot, Flare, Prominence, CoronalHole,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActivityLevel {
    Quiet, Moderate, Active, VeryActive,
}

pub fn activity_level(star_temp_k: f64, age_yr: f64) -> ActivityLevel {
    let youth = (1.0 - (age_yr / 5.0e9).clamp(0.0, 1.0)) * 0.6;
    let cool = if star_temp_k < 3500.0 { 0.4 } else { 0.0 };
    let score = youth + cool;
    if score > 0.7 { ActivityLevel::VeryActive }
    else if score > 0.4 { ActivityLevel::Active }
    else if score > 0.15 { ActivityLevel::Moderate }
    else { ActivityLevel::Quiet }
}

pub fn granulation_scale(star_temp_k: f64) -> f64 {
    0.002 + (star_temp_k / 5772.0).powf(0.5) * 0.008
}

pub fn sunspot_cycle_s(star_temp_k: f64) -> f64 {
    11.0 * 365.25 * 86400.0 * (5772.0 / star_temp_k).powf(0.3).clamp(0.5, 3.0)
}

pub fn stellar_features(seed: u64, star_temp_k: f64, age_yr: f64, time_s: f64) -> Vec<StellarFeature> {
    let activity = activity_level(star_temp_k, age_yr);
    let cycle = (time_s / sunspot_cycle_s(star_temp_k)) % 1.0;
    let spot_amp = (cycle * std::f64::consts::TAU).sin().abs();
    let mut rng = Rng::stream(seed, domain::STAR, &[0x7F3A]);
    let base = match activity { ActivityLevel::Quiet => 8, ActivityLevel::Moderate => 24, ActivityLevel::Active => 64, ActivityLevel::VeryActive => 160 };
    let mut feats = Vec::with_capacity(base);
    let granule_n = match activity { ActivityLevel::Quiet => 200, ActivityLevel::Moderate => 500, ActivityLevel::Active => 1200, ActivityLevel::VeryActive => 2500 };
    for _ in 0..granule_n {
        feats.push(StellarFeature { kind: StellarFeatureKind::Granulation, latitude_deg: rng.range(-85.0, 85.0), longitude_deg: rng.range(0.0, 360.0), size_frac: granulation_scale(star_temp_k) * rng.range(0.5, 1.5), intensity: rng.range(0.1, 0.4), lifetime_s: rng.range(600.0, 3600.0) });
    }
    let spot_n = ((base as f64) * 0.15 * spot_amp) as usize;
    for _ in 0..spot_n {
        feats.push(StellarFeature { kind: StellarFeatureKind::Sunspot, latitude_deg: rng.range(-35.0, 35.0), longitude_deg: rng.range(0.0, 360.0), size_frac: rng.range(0.005, 0.04) * spot_amp, intensity: rng.range(0.3, 0.9), lifetime_s: rng.range(3.0 * 86400.0, 30.0 * 86400.0) });
    }
    let flare_n = match activity { ActivityLevel::Quiet => 0, ActivityLevel::Moderate => 2, ActivityLevel::Active => 8, ActivityLevel::VeryActive => 25 };
    for _ in 0..flare_n {
        if rng.chance(0.3) {
            feats.push(StellarFeature { kind: StellarFeatureKind::Flare, latitude_deg: rng.range(-60.0, 60.0), longitude_deg: rng.range(0.0, 360.0), size_frac: rng.range(0.001, 0.01), intensity: rng.range(0.6, 1.0), lifetime_s: rng.range(60.0, 7200.0) });
        }
    }
    let prom_n = match activity { ActivityLevel::Quiet => 1, ActivityLevel::Moderate => 4, ActivityLevel::Active => 12, ActivityLevel::VeryActive => 30 };
    for _ in 0..prom_n {
        feats.push(StellarFeature { kind: StellarFeatureKind::Prominence, latitude_deg: rng.range(-50.0, 50.0), longitude_deg: rng.range(0.0, 360.0), size_frac: rng.range(0.01, 0.08), intensity: rng.range(0.4, 0.8), lifetime_s: rng.range(1.0 * 86400.0, 60.0 * 86400.0) });
    }
    let hole_n = match activity { ActivityLevel::Quiet => 4, ActivityLevel::Moderate => 2, ActivityLevel::Active => 1, ActivityLevel::VeryActive => 0 };
    for _ in 0..hole_n {
        let lat = if rng.chance(0.5) { rng.range(55.0, 85.0) } else { rng.range(-85.0, -55.0) };
        feats.push(StellarFeature { kind: StellarFeatureKind::CoronalHole, latitude_deg: lat, longitude_deg: rng.range(0.0, 360.0), size_frac: rng.range(0.05, 0.15), intensity: rng.range(0.2, 0.6), lifetime_s: rng.range(7.0 * 86400.0, 90.0 * 86400.0) });
    }
    feats
}

pub fn feature_count(a: ActivityLevel) -> usize {
    match a { ActivityLevel::Quiet => 230, ActivityLevel::Moderate => 550, ActivityLevel::Active => 1300, ActivityLevel::VeryActive => 2700 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn activity_levels() {
        assert_eq!(activity_level(3000.0, 1.0e6), ActivityLevel::VeryActive);
        assert_eq!(activity_level(5000.0, 8.0e9), ActivityLevel::Quiet);
        assert_eq!(activity_level(3000.0, 8.0e9), ActivityLevel::Moderate);
    }
    #[test] fn features_deterministic() {
        assert_eq!(stellar_features(42, 5772.0, 4.6e9, 0.0), stellar_features(42, 5772.0, 4.6e9, 0.0));
    }
    #[test] fn more_activity_more_features() {
        assert!(stellar_features(7, 6000.0, 1e8, 0.0).len() > stellar_features(7, 4000.0, 1e10, 0.0).len());
    }
    #[test] fn cycle_positive() { assert!(sunspot_cycle_s(5772.0) > 0.0); }
}
