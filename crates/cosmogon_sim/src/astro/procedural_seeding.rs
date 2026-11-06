//! Procedural-body seeding with honest provenance labels.
//!
//! Procedural body generation (asteroids, comets, dwarf planets) with provenance
//! labels distinguishing real data from generated data.

use serde::{Serialize};

/// Procedural body generation parameters.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ProceduralBodyParams {
    pub seed: u64,
    pub mass_range: (f64, f64),    // kg
    pub radius_range: (f64, f64),  // m
    pub albedo_range: (f64, f64),  // 0..=1
    pub body_count: usize,
}

/// Default procedural body parameters.
pub const DEFAULT_PROCEDURAL_PARAMS: ProceduralBodyParams = ProceduralBodyParams {
    seed: 0x9E37_79B9_7F4A_7C15,
    mass_range: (1e15, 1e21),
    radius_range: (1e3, 1e5),
    albedo_range: (0.05, 0.4),
    body_count: 100,
};

/// Provenance label for generated bodies.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProvenanceLabel {
    Real,        // From measured data
    Modelled,    // From physical model
    Procedural,  // Generated procedurally
    Hybrid,      // Mix of real and generated
}

/// Number of procedural bodies.
pub fn count() -> usize {
    DEFAULT_PROCEDURAL_PARAMS.body_count
}
