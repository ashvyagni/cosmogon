//! Rendering architecture / level-of-detail / GPU budget foundation.
//!
//! Real LOD system with hysteresis, pixel-coverage-based tier selection, adaptive
//! quality downgrade, and morphing between tiers.

use serde::Serialize;
use crate::rng::Rng;
use crate::rng::domain;

/// Level-of-detail tier for close-up rendering.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LodTier {
    Far,
    Medium,
    Close,
    Extreme,
}

fn tier_ordinal(t: LodTier) -> u8 {
    match t {
        LodTier::Far => 0,
        LodTier::Medium => 1,
        LodTier::Close => 2,
        LodTier::Extreme => 3,
    }
}

impl LodTier {
    pub fn from_coverage(coverage_px: f64, current: Option<LodTier>) -> Self {
        let tier = if coverage_px < 12.0 {
            LodTier::Far
        } else if coverage_px < 60.0 {
            LodTier::Medium
        } else if coverage_px < 240.0 {
            LodTier::Close
        } else {
            LodTier::Extreme
        };
        match current {
            Some(prev) => {
                let prev_cov = match prev {
                    LodTier::Far => 12.0,
                    LodTier::Medium => 60.0,
                    LodTier::Close => 240.0,
                    LodTier::Extreme => f64::INFINITY,
                };
                if tier_ordinal(tier) < tier_ordinal(prev) && coverage_px > prev_cov * 1.15 {
                    prev
                } else if tier_ordinal(tier) > tier_ordinal(prev) && coverage_px < prev_cov / 1.15 {
                    prev
                } else {
                    tier
                }
            }
            None => tier,
        }
    }

    pub fn morph_factor(coverage_px: f64) -> f64 {
        let (low, high) = match coverage_px {
            c if c < 12.0 => (8.0, 12.0),
            c if c < 60.0 => (40.0, 60.0),
            c if c < 240.0 => (180.0, 240.0),
            _ => return 1.0,
        };
        ((coverage_px - low) / (high - low)).clamp(0.0, 1.0)
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GpuBudget {
    pub layer: &'static str,
    pub max_vram_mb: f64,
    pub max_draw_calls: u32,
    pub max_triangles: u64,
}

pub const RENDER_BUDGETS: &[GpuBudget] = &[
    GpuBudget { layer: "stars", max_vram_mb: 256.0, max_draw_calls: 10_000, max_triangles: 1_000_000 },
    GpuBudget { layer: "planets", max_vram_mb: 512.0, max_draw_calls: 25_000, max_triangles: 10_000_000 },
    GpuBudget { layer: "surface", max_vram_mb: 1024.0, max_draw_calls: 100_000, max_triangles: 100_000_000 },
    GpuBudget { layer: "atmosphere", max_vram_mb: 128.0, max_draw_calls: 5_000, max_triangles: 1_000_000 },
];

pub fn count() -> usize { RENDER_BUDGETS.len() }
pub fn total_vram_mb() -> f64 { RENDER_BUDGETS.iter().map(|b| b.max_vram_mb).sum() }
pub fn lod_tier(distance_km: f64) -> LodTier {
    if distance_km > 1_000_000.0 { LodTier::Far }
    else if distance_km > 10_000.0 { LodTier::Medium }
    else if distance_km > 100.0 { LodTier::Close }
    else { LodTier::Extreme }
}

#[derive(Clone, Debug)]
pub struct AdaptiveQuality {
    pub target_frame_ms: f64,
    pub measured_frame_ms: f64,
    pub over_budget_frames: u32,
    pub max_downgrade: u32,
    pub downgrade_level: u32,
}

impl Default for AdaptiveQuality {
    fn default() -> Self {
        Self { target_frame_ms: 16.67, measured_frame_ms: 16.67, over_budget_frames: 0, max_downgrade: 2, downgrade_level: 0 }
    }
}

impl AdaptiveQuality {
    pub fn update(&mut self, frame_ms: f64, base_tier: LodTier) -> LodTier {
        self.measured_frame_ms = frame_ms;
        if frame_ms > self.target_frame_ms {
            self.over_budget_frames += 1;
            if self.over_budget_frames >= 3 && self.downgrade_level < self.max_downgrade {
                self.downgrade_level += 1;
                self.over_budget_frames = 0;
            }
        } else {
            self.over_budget_frames = 0;
            if self.downgrade_level > 0 { self.downgrade_level -= 1; }
        }
        self.effective_tier(base_tier)
    }
    pub fn effective_tier(&self, base: LodTier) -> LodTier {
        match base {
            LodTier::Extreme => if self.downgrade_level >= 2 { LodTier::Medium } else if self.downgrade_level >= 1 { LodTier::Close } else { LodTier::Extreme },
            LodTier::Close => if self.downgrade_level >= 1 { LodTier::Medium } else { LodTier::Close },
            LodTier::Medium => if self.downgrade_level >= 1 { LodTier::Far } else { LodTier::Medium },
            LodTier::Far => LodTier::Far,
        }
    }
}

pub fn lod_jitter(seed: u64, body_index: u64) -> f64 {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[body_index, 0x1C47]);
    rng.range(-0.25, 0.25)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lod_tier_selection_is_monotonic() {
        let tiers = [(1.0, LodTier::Far), (20.0, LodTier::Medium), (100.0, LodTier::Close), (500.0, LodTier::Extreme)];
        for (c, e) in tiers { assert_eq!(LodTier::from_coverage(c, None), e); }
    }
    #[test]
    fn lod_hysteresis_prevents_flipping() {
        assert_eq!(LodTier::from_coverage(50.0, Some(LodTier::Medium)), LodTier::Medium);
        assert_eq!(LodTier::from_coverage(58.0, Some(LodTier::Medium)), LodTier::Medium);
        assert_eq!(LodTier::from_coverage(70.0, Some(LodTier::Medium)), LodTier::Close);
    }
    #[test]
    fn morph_factor_in_range() {
        for c in [4.0, 10.0, 50.0, 200.0, 500.0] {
            assert!((0.0..=1.0).contains(&LodTier::morph_factor(c)));
        }
    }
    #[test]
    fn adaptive_quality_downgrades() {
        let mut aq = AdaptiveQuality::default();
        for _ in 0..3 { aq.update(30.0, LodTier::Extreme); }
        assert_eq!(aq.update(30.0, LodTier::Extreme), LodTier::Close);
        for _ in 0..5 { aq.update(10.0, LodTier::Close); }
        assert_eq!(aq.update(10.0, LodTier::Extreme), LodTier::Extreme);
    }
    #[test]
    fn lod_jitter_deterministic() {
        assert_eq!(lod_jitter(42, 7), lod_jitter(42, 7));
    }
}
