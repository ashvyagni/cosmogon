//! Night lights and city lights.
//!
//! Night light rendering parameters for civilization visibility on the night side.

use serde::{Serialize};

/// Night light feature.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NightLightFeature {
    CityLights,
    RoadLights,
    PortLights,
    IndustrialLights,
}

/// Night light parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct NightLightParams {
    pub feature: NightLightFeature,
    pub brightness: f64,  // 0..1
    pub coverage: f64,    // 0..1
}

/// Default night light parameters.
pub const DEFAULT_NIGHT_LIGHTS: NightLightParams = NightLightParams {
    feature: NightLightFeature::CityLights,
    brightness: 0.8,
    coverage: 0.5,
};
