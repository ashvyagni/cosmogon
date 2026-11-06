//! Atmospheres and weather: clouds, storms, hurricanes where applicable.
//!
//! Atmospheric weather features for close-up planetary views.

use serde::{Serialize};

/// Weather feature type.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeatherFeature {
    Cloud,
    Storm,
    Hurricane,
    Front,
    Fog,
}

/// Weather intensity.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeatherIntensity {
    Calm,
    Breezy,
    Stormy,
    Severe,
}

/// Atmospheric weather parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct WeatherParams {
    pub feature_type: WeatherFeature,
    pub intensity: WeatherIntensity,
    pub coverage_frac: f64,  // 0..1 sky coverage
    pub wind_speed_m_s: f64,
    pub pressure_hpa: f64,
}

/// Default weather parameters.
pub const DEFAULT_WEATHER: WeatherParams = WeatherParams {
    feature_type: WeatherFeature::Cloud,
    intensity: WeatherIntensity::Calm,
    coverage_frac: 0.3,
    wind_speed_m_s: 5.0,
    pressure_hpa: 1013.25,
};

/// Weather feature count for an intensity level.
pub fn feature_count(intensity: WeatherIntensity) -> usize {
    match intensity {
        WeatherIntensity::Calm => 2,
        WeatherIntensity::Breezy => 5,
        WeatherIntensity::Stormy => 15,
        WeatherIntensity::Severe => 40,
    }
}
