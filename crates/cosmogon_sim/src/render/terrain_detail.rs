//! Planet terrain close-ups: mountains, water, erosion detail.
//!
//! Terrain feature types for close-up planetary surface rendering.

use serde::{Serialize};

/// Terrain feature type.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerrainFeature {
    Mountain,
    Valley,
    Crater,
    Ocean,
    River,
    Desert,
    Forest,
    Ice,
    Volcano,
}

/// Terrain detail level.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerrainDetail {
    Low,
    Medium,
    High,
    Ultra,
}

/// Procedural terrain parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct TerrainParams {
    pub seed: u64,
    pub detail: TerrainDetail,
    pub mountain_height_m: f64,
    pub ocean_depth_m: f64,
    pub erosion_eras: u32,
}

/// Default terrain parameters.
pub const DEFAULT_TERRAIN: TerrainParams = TerrainParams {
    seed: 0xDEAD_BEEF,
    detail: TerrainDetail::High,
    mountain_height_m: 8848.0,
    ocean_depth_m: 10911.0,
    erosion_eras: 3,
};

/// Feature density per terrain detail level.
pub fn feature_density(detail: TerrainDetail) -> f64 {
    match detail {
        TerrainDetail::Low => 0.1,
        TerrainDetail::Medium => 1.0,
        TerrainDetail::High => 10.0,
        TerrainDetail::Ultra => 100.0,
    }
}
