//! Surface detail rendering.
//!
//! Surface detail rendering parameters for close-up planetary views.

use serde::{Serialize};

/// Surface detail feature.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SurfaceDetail {
    Craters,
    Mountains,
    Rivers,
    Cities,
    Lights,
}

/// Surface detail level.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SurfaceDetailLevel {
    None,
    Basic,
    Detailed,
    Ultra,
}

/// Surface detail parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SurfaceDetailParams {
    pub feature: SurfaceDetail,
    pub level: SurfaceDetailLevel,
    pub max_features: u32,
}

/// Default surface detail parameters.
pub const DEFAULT_SURFACE_DETAIL: SurfaceDetailParams = SurfaceDetailParams {
    feature: SurfaceDetail::Craters,
    level: SurfaceDetailLevel::Basic,
    max_features: 1000,
};
