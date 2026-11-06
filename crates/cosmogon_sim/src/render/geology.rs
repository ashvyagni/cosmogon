//! Geological activity: volcanism, earthquakes and visible consequences.
//!
//! Geological activity features for close-up planetary views: volcanoes, earthquakes,
//! tectonic features.

use serde::{Serialize};

/// Geological feature type.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GeoFeature {
    Volcano,
    Earthquake,
    Rift,
    Mountain,
    Fault,
}

/// Geological activity level.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GeoActivity {
    Dormant,
    Low,
    Moderate,
    High,
    Extreme,
}

/// Geological activity parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GeoParams {
    pub feature_type: GeoFeature,
    pub activity: GeoActivity,
    pub magnitude: f64,     // Richter scale for earthquakes, VEI for volcanoes
    pub depth_km: f64,
    pub duration_s: f64,
}

/// Default geological parameters.
pub const DEFAULT_GEO: GeoParams = GeoParams {
    feature_type: GeoFeature::Volcano,
    activity: GeoActivity::Low,
    magnitude: 3.0,
    depth_km: 10.0,
    duration_s: 3600.0,
};

/// Feature count for an activity level.
pub fn feature_count(activity: GeoActivity) -> usize {
    match activity {
        GeoActivity::Dormant => 0,
        GeoActivity::Low => 1,
        GeoActivity::Moderate => 3,
        GeoActivity::High => 10,
        GeoActivity::Extreme => 30,
    }
}
