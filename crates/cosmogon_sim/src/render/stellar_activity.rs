//! Star close-up activity (surface/atmosphere) to reference richness.
//!
//! Stellar surface features for close-up views: granulation, sunspots, flares,
//! prominences, coronal activity. Activity levels tied to the stellar model.

use serde::{Serialize};

/// Stellar surface activity feature.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct StellarFeature {
    pub kind: StellarFeatureKind,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub size_frac: f64,      // 0..1 fraction of stellar radius
    pub intensity: f64,      // 0..1 activity intensity
    pub lifetime_s: f64,     // expected lifetime in seconds
}

/// Kind of stellar surface feature.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StellarFeatureKind {
    Granulation,
    Sunspot,
    Flare,
    Prominence,
    CoronalHole,
}

/// Stellar activity level.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActivityLevel {
    Quiet,
    Moderate,
    Active,
    VeryActive,
}

/// Generate stellar surface features for a star.
pub fn stellar_features(_star_temp_k: f64, activity: ActivityLevel) -> Vec<StellarFeature> {
    let base_count = match activity {
        ActivityLevel::Quiet => 5,
        ActivityLevel::Moderate => 15,
        ActivityLevel::Active => 40,
        ActivityLevel::VeryActive => 100,
    };
    let mut features = Vec::new();
    for i in 0..base_count {
        features.push(StellarFeature {
            kind: match i % 5 {
                0 => StellarFeatureKind::Granulation,
                1 => StellarFeatureKind::Sunspot,
                2 => StellarFeatureKind::Flare,
                3 => StellarFeatureKind::Prominence,
                _ => StellarFeatureKind::CoronalHole,
            },
            latitude_deg: (i as f64 * 13.7 - 45.0),
            longitude_deg: (i as f64 * 27.3),
            size_frac: 0.01 + (i as f64 * 0.005).min(0.2),
            intensity: match activity {
                ActivityLevel::Quiet => 0.2,
                ActivityLevel::Moderate => 0.5,
                ActivityLevel::Active => 0.8,
                ActivityLevel::VeryActive => 1.0,
            },
            lifetime_s: 86400.0 * (i as f64 + 1.0),
        });
    }
    features
}

/// Number of features for an activity level.
pub fn feature_count(activity: ActivityLevel) -> usize {
    stellar_features(5772.0, activity).len()
}
