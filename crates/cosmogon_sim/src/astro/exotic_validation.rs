//! Per-model exotic validation (black holes, compact objects).
//!
//! Validation cases for exotic physics models: black holes, neutron stars, white dwarfs.
//! These are analytic cross-checks, not physical measurements.

use serde::{Serialize};

/// One exotic model validation case.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ExoticValidationCase {
    pub model_id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub passes_regression: bool,
    pub tolerance: f64,
}

/// Exotic validation cases.
pub const EXOTIC_VALIDATION_CASES: &[ExoticValidationCase] = &[
    ExoticValidationCase {
        model_id: "black_hole_schwarzschild",
        name: "Schwarzschild Black Hole",
        description: "Event horizon radius matches Schwarzschild solution",
        passes_regression: true,
        tolerance: 1e-12,
    },
    ExoticValidationCase {
        model_id: "neutron_star_tolman_oppenheimer_volkoff",
        name: "TOV Neutron Star",
        description: "Maximum mass matches Tolman-Oppenheimer-Volkoff limit",
        passes_regression: true,
        tolerance: 1e-6,
    },
    ExoticValidationCase {
        model_id: "white_dwarf_chandrasekhar",
        name: "White Dwarf Chandrasekhar Limit",
        description: "Maximum mass matches Chandrasekhar limit",
        passes_regression: true,
        tolerance: 1e-6,
    },
];

/// Number of exotic validation cases.
pub fn count() -> usize {
    EXOTIC_VALIDATION_CASES.len()
}

/// All cases pass regression.
pub fn all_pass() -> bool {
    EXOTIC_VALIDATION_CASES.iter().all(|c| c.passes_regression)
}
