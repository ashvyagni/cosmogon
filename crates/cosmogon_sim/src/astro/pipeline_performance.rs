//! Data pipeline performance and memory budgets.
//!
//! Performance tracking for data pipeline: memory usage, load times, cache hit rates.

use serde::{Serialize};

/// Performance budget for one pipeline stage.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PipelineBudget {
    pub stage: &'static str,
    pub max_memory_mb: f64,
    pub max_load_time_s: f64,
    pub max_cache_miss_rate: f64,
}

/// Data pipeline budgets.
pub const PIPELINE_BUDGETS: &[PipelineBudget] = &[
    PipelineBudget {
        stage: "catalogue_load",
        max_memory_mb: 512.0,
        max_load_time_s: 5.0,
        max_cache_miss_rate: 0.05,
    },
    PipelineBudget {
        stage: "deep_sky_render",
        max_memory_mb: 256.0,
        max_load_time_s: 2.0,
        max_cache_miss_rate: 0.10,
    },
    PipelineBudget {
        stage: "exoplanet_query",
        max_memory_mb: 128.0,
        max_load_time_s: 1.0,
        max_cache_miss_rate: 0.02,
    },
];

/// Number of pipeline stages with budgets.
pub fn count() -> usize {
    PIPELINE_BUDGETS.len()
}

/// Total memory budget across all stages.
pub fn total_memory_mb() -> f64 {
    PIPELINE_BUDGETS.iter().map(|b| b.max_memory_mb).sum()
}
