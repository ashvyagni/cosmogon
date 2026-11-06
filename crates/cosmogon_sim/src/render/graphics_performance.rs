//! Graphics performance optimization.
//!
//! Render performance tracking and optimization for close-up cosmic body views.

use serde::{Serialize};

/// Graphics performance metric.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GraphicsMetric {
    pub name: &'static str,
    pub value: f64,
    pub unit: &'static str,
    pub target: f64,
}

/// Graphics performance metrics.
pub const GRAPHICS_METRICS: &[GraphicsMetric] = &[
    GraphicsMetric { name: "frame_time_ms", value: 16.67, unit: "ms", target: 16.67 },
    GraphicsMetric { name: "draw_calls_per_frame", value: 5000.0, unit: "count", target: 10000.0 },
    GraphicsMetric { name: "triangle_count", value: 1_000_000.0, unit: "count", target: 10_000_000.0 },
    GraphicsMetric { name: "vram_usage_mb", value: 512.0, unit: "MB", target: 2048.0 },
];

pub fn count() -> usize {
    GRAPHICS_METRICS.len()
}
