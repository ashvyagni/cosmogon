//! Rendering architecture / level-of-detail / GPU budget foundation.
//!
//! Foundational structures for the graphics supplement: render LOD levels,
//! GPU memory budgets, and draw-call budgets for close-up cosmic body rendering.

use serde::{Serialize};

/// Level-of-detail tier for close-up rendering.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LodTier {
    /// Far view: whole body as a point or small sprite.
    Far,
    /// Medium view: body with basic surface features.
    Medium,
    /// Close view: surface detail visible.
    Close,
    /// Extreme close-up: individual features (craters, clouds, cities).
    Extreme,
}

/// GPU memory budget for one render layer.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GpuBudget {
    pub layer: &'static str,
    pub max_vram_mb: f64,
    pub max_draw_calls: u32,
    pub max_triangles: u64,
}

/// Render architecture budgets.
pub const RENDER_BUDGETS: &[GpuBudget] = &[
    GpuBudget { layer: "stars", max_vram_mb: 256.0, max_draw_calls: 10_000, max_triangles: 1_000_000 },
    GpuBudget { layer: "planets", max_vram_mb: 512.0, max_draw_calls: 25_000, max_triangles: 10_000_000 },
    GpuBudget { layer: "surface", max_vram_mb: 1024.0, max_draw_calls: 100_000, max_triangles: 100_000_000 },
    GpuBudget { layer: "atmosphere", max_vram_mb: 128.0, max_draw_calls: 5_000, max_triangles: 1_000_000 },
];

/// Number of render layers with budgets.
pub fn count() -> usize {
    RENDER_BUDGETS.len()
}

/// Total VRAM budget across all layers.
pub fn total_vram_mb() -> f64 {
    RENDER_BUDGETS.iter().map(|b| b.max_vram_mb).sum()
}

/// LOD tier for a given distance.
pub fn lod_tier(distance_km: f64) -> LodTier {
    if distance_km > 1_000_000.0 { LodTier::Far }
    else if distance_km > 10_000.0 { LodTier::Medium }
    else if distance_km > 100.0 { LodTier::Close }
    else { LodTier::Extreme }
}
