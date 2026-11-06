//! Moons, rings, small bodies and encounter detail.
//!
//! Small body and encounter features for close-up views: moons, ring systems,
//! asteroid encounters, comet comas.

use serde::{Serialize};

/// Small body feature type.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmallBodyFeature {
    Moon,
    Ring,
    Asteroid,
    Comet,
    Meteoroid,
}

/// Encounter detail level.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncounterDetail {
    None,
    Flyby,
    Rendezvous,
    Landing,
}

/// Small body parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SmallBodyParams {
    pub feature_type: SmallBodyFeature,
    pub encounter: EncounterDetail,
    pub diameter_m: f64,
    pub distance_km: f64,
    pub relative_velocity_km_s: f64,
}

/// Default small body parameters.
pub const DEFAULT_SMALL_BODY: SmallBodyParams = SmallBodyParams {
    feature_type: SmallBodyFeature::Moon,
    encounter: EncounterDetail::Flyby,
    diameter_m: 3_474_000.0,
    distance_km: 384_400.0,
    relative_velocity_km_s: 1.0,
};

/// Feature count for an encounter detail level.
pub fn feature_count(detail: EncounterDetail) -> usize {
    match detail {
        EncounterDetail::None => 0,
        EncounterDetail::Flyby => 1,
        EncounterDetail::Rendezvous => 3,
        EncounterDetail::Landing => 10,
    }
}
