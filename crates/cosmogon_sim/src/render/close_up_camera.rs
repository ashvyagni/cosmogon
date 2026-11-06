//! Close-up camera controls.
//!
//! Camera controls for close-up cosmic body views: orbit, zoom, pan, focus.

use serde::{Serialize};

/// Camera control mode.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CameraMode {
    Orbit,
    Free,
    Focus,
    Track,
}

/// Camera parameters for close-up views.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CameraParams {
    pub mode: CameraMode,
    pub distance_km: f64,
    pub fov_deg: f64,
    pub target_lat_deg: f64,
    pub target_lon_deg: f64,
}

/// Default camera parameters.
pub const DEFAULT_CAMERA: CameraParams = CameraParams {
    mode: CameraMode::Orbit,
    distance_km: 1000.0,
    fov_deg: 45.0,
    target_lat_deg: 0.0,
    target_lon_deg: 0.0,
};
