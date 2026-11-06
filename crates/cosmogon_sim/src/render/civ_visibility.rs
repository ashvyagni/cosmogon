//! Civilizations and buildings visible from close range.
//!
//! Civilization visibility features for close-up planetary views: cities, roads,
//! lights, infrastructure visible from orbit or surface.

use serde::{Serialize};

/// Civilization visibility feature type.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CivVisibilityFeature {
    City,
    Town,
    Road,
    Port,
    Spaceport,
    IndustrialZone,
    Farmland,
}

/// Civilization visibility level.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CivVisibilityLevel {
    None,
    NightLights,
    SurfaceFeatures,
    IndividualBuildings,
}

/// Civilization visibility parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CivVisibilityParams {
    pub level: CivVisibilityLevel,
    pub feature_type: CivVisibilityFeature,
    pub population: f64,
    pub night_light_brightness: f64,  // 0..1
}

/// Default civilization visibility parameters.
pub const DEFAULT_CIV_VISIBILITY: CivVisibilityParams = CivVisibilityParams {
    level: CivVisibilityLevel::NightLights,
    feature_type: CivVisibilityFeature::City,
    population: 1_000_000.0,
    night_light_brightness: 0.7,
};

/// Feature count for a visibility level.
pub fn feature_count(level: CivVisibilityLevel) -> usize {
    match level {
        CivVisibilityLevel::None => 0,
        CivVisibilityLevel::NightLights => 5,
        CivVisibilityLevel::SurfaceFeatures => 20,
        CivVisibilityLevel::IndividualBuildings => 100,
    }
}
