//! Atmospheric scattering.
//!
//! Atmospheric scattering parameters for realistic sky rendering.

use serde::{Serialize};

/// Atmospheric scattering model.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScatteringModel {
    Rayleigh,
    Mie,
    Combined,
}

/// Atmospheric scattering parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ScatteringParams {
    pub model: ScatteringModel,
    pub rayleigh_scale_height_km: f64,
    pub mie_scale_height_km: f64,
    pub ground_albedo: f64,
}

/// Default atmospheric scattering parameters.
pub const DEFAULT_SCATTERING: ScatteringParams = ScatteringParams {
    model: ScatteringModel::Combined,
    rayleigh_scale_height_km: 8.0,
    mie_scale_height_km: 1.2,
    ground_albedo: 0.3,
};
