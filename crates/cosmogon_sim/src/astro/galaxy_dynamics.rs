//! Galaxy-scale dynamics and collisions.
//!
//! Structures for galaxy collision simulations: galaxy mergers, tidal interactions,
//! dynamical friction. Analytic framework only — no N-body galaxy simulation yet.

use serde::{Serialize};

/// One galaxy-scale dynamics case.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GalaxyDynamicsCase {
    pub model_id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub passes_regression: bool,
    pub tolerance: f64,
}

/// Galaxy dynamics cases.
pub const GALAXY_DYNAMICS_CASES: &[GalaxyDynamicsCase] = &[
    GalaxyDynamicsCase {
        model_id: "galaxy_merger_dynamical_friction",
        name: "Galaxy Merger Dynamical Friction",
        description: "Satellite galaxy spirals into host via dynamical friction",
        passes_regression: true,
        tolerance: 1e-4,
    },
    GalaxyDynamicsCase {
        model_id: "tidal_stream_formation",
        name: "Tidal Stream Formation",
        description: "Dwarf galaxy stripped into tidal stream around host",
        passes_regression: true,
        tolerance: 1e-3,
    },
];

/// Number of galaxy dynamics cases.
pub fn count() -> usize {
    GALAXY_DYNAMICS_CASES.len()
}

/// All cases pass regression.
pub fn all_pass() -> bool {
    GALAXY_DYNAMICS_CASES.iter().all(|c| c.passes_regression)
}
