//! Cosmology/large-scale structure presentation.
//!
//! Cosmological parameters and large-scale structure: cosmic microwave background,
//! baryon acoustic oscillations, large-scale structure formation. Presentation layer only.

use serde::{Serialize};

/// Cosmological parameter set.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CosmologicalParameters {
    pub hubble_constant: f64,        // km/s/Mpc
    pub matter_density: f64,         // Omega_m
    pub dark_energy_density: f64,    // Omega_Lambda
    pub radiation_density: f64,      // Omega_r
    pub baryon_density: f64,         // Omega_b
    pub scalar_amplitude: f64,       // A_s
    pub scalar_index: f64,           // n_s
}

/// Default Planck 2018 cosmological parameters.
pub const PLANCK_2018: CosmologicalParameters = CosmologicalParameters {
    hubble_constant: 67.4,
    matter_density: 0.315,
    dark_energy_density: 0.685,
    radiation_density: 9.1e-5,
    baryon_density: 0.0493,
    scalar_amplitude: 2.1e-9,
    scalar_index: 0.965,
};

/// Large-scale structure presentation case.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct LargeScaleStructureCase {
    pub model_id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub passes_regression: bool,
}

pub const LARGE_SCALE_STRUCTURE_CASES: &[LargeScaleStructureCase] = &[
    LargeScaleStructureCase {
        model_id: "cmb_power_spectrum",
        name: "CMB Power Spectrum",
        description: "Angular power spectrum of cosmic microwave background anisotropies",
        passes_regression: true,
    },
    LargeScaleStructureCase {
        model_id: "baryon_acoustic_oscillations",
        name: "Baryon Acoustic Oscillations",
        description: "Standard ruler in galaxy correlation function",
        passes_regression: true,
    },
];

pub fn count() -> usize {
    LARGE_SCALE_STRUCTURE_CASES.len()
}
