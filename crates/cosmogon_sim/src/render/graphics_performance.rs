//! Graphics performance optimization.

use serde::Serialize;
use crate::render::lod_budget::{AdaptiveQuality, LodTier};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GraphicsMetric {
    pub name: &'static str,
    pub value: f64,
    pub unit: &'static str,
    pub target: f64,
}

pub const GRAPHICS_METRICS: &[GraphicsMetric] = &[
    GraphicsMetric { name: "frame_time_ms", value: 16.67, unit: "ms", target: 16.67 },
    GraphicsMetric { name: "draw_calls_per_frame", value: 5000.0, unit: "count", target: 10000.0 },
    GraphicsMetric { name: "triangle_count", value: 1_000_000.0, unit: "count", target: 10_000_000.0 },
    GraphicsMetric { name: "vram_usage_mb", value: 512.0, unit: "MB", target: 2048.0 },
];

#[derive(Clone, Debug)]
pub struct FrameBudget {
    pub target_frame_ms: f64,
    pub frame_times: [f64; 60],
    pub frame_index: usize,
    pub frames_recorded: usize,
    pub adaptive_quality: AdaptiveQuality,
}

impl Default for FrameBudget {
    fn default() -> Self {
        Self { target_frame_ms: 16.67, frame_times: [16.67; 60], frame_index: 0, frames_recorded: 0, adaptive_quality: AdaptiveQuality::default() }
    }
}

impl FrameBudget {
    pub fn record_frame(&mut self, frame_ms: f64, base_tier: LodTier) -> LodTier {
        self.frame_times[self.frame_index] = frame_ms;
        self.frame_index = (self.frame_index + 1) % 60;
        if self.frames_recorded < 60 { self.frames_recorded += 1; }
        self.adaptive_quality.update(frame_ms, base_tier)
    }
    pub fn average_frame_ms(&self) -> f64 {
        let n = self.frames_recorded.min(60);
        if n == 0 { return self.target_frame_ms; }
        self.frame_times.iter().take(n).sum::<f64>() / n as f64
    }
    pub fn fps(&self) -> f64 {
        let avg = self.average_frame_ms();
        if avg > 0.0 { 1000.0 / avg } else { 60.0 }
    }
    pub fn over_budget(&self) -> bool {
        self.average_frame_ms() > self.target_frame_ms
    }
    pub fn downgrade_level(&self) -> u32 {
        self.adaptive_quality.downgrade_level
    }
}

#[derive(Clone, Debug, Default)]
pub struct DrawCallTracker {
    pub total_calls: u32,
    pub instanced_calls: u32,
    pub culled_calls: u32,
    pub max_calls: u32,
}

impl DrawCallTracker {
    pub fn new(max_calls: u32) -> Self {
        Self { max_calls, ..Default::default() }
    }
    pub fn record(&mut self, instanced: bool, culled: bool) {
        self.total_calls += 1;
        if instanced { self.instanced_calls += 1; }
        if culled { self.culled_calls += 1; }
    }
    pub fn reset(&mut self) {
        self.total_calls = 0;
        self.instanced_calls = 0;
        self.culled_calls = 0;
    }
    pub fn instancing_ratio(&self) -> f64 {
        if self.total_calls == 0 { 0.0 } else { self.instanced_calls as f64 / self.total_calls as f64 }
    }
    pub fn culling_ratio(&self) -> f64 {
        if self.total_calls == 0 { 0.0 } else { self.culled_calls as f64 / self.total_calls as f64 }
    }
    pub fn over_budget(&self) -> bool {
        self.total_calls > self.max_calls
    }
}

#[derive(Clone, Debug, Default)]
pub struct VramManager {
    pub used_mb: f64,
    pub budget_mb: f64,
    pub texture_mb: f64,
    pub buffer_mb: f64,
    pub atlas_mb: f64,
}

impl VramManager {
    pub fn new(budget_mb: f64) -> Self {
        Self { budget_mb, ..Default::default() }
    }
    pub fn allocate(&mut self, mb: f64) -> bool {
        if self.used_mb + mb > self.budget_mb { return false; }
        self.used_mb += mb;
        true
    }
    pub fn deallocate(&mut self, mb: f64) {
        self.used_mb = (self.used_mb - mb).max(0.0);
    }
    pub fn usage_ratio(&self) -> f64 {
        if self.budget_mb == 0.0 { 0.0 } else { self.used_mb / self.budget_mb }
    }
    pub fn over_budget(&self) -> bool {
        self.used_mb > self.budget_mb
    }
}

#[derive(Clone, Debug)]
pub struct PerformanceSummary {
    pub fps: f64,
    pub frame_ms: f64,
    pub draw_calls: u32,
    pub instancing_ratio: f64,
    pub vram_mb: f64,
    pub vram_usage_ratio: f64,
    pub downgrade_level: u32,
}

impl PerformanceSummary {
    pub fn from(frame: &FrameBudget, draws: &DrawCallTracker, vram: &VramManager) -> Self {
        Self {
            fps: frame.fps(),
            frame_ms: frame.average_frame_ms(),
            draw_calls: draws.total_calls,
            instancing_ratio: draws.instancing_ratio(),
            vram_mb: vram.used_mb,
            vram_usage_ratio: vram.usage_ratio(),
            downgrade_level: frame.downgrade_level(),
        }
    }
}

pub fn count() -> usize { GRAPHICS_METRICS.len() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn frame_budget_tracks() {
        let mut fb = FrameBudget::default();
        for i in 0..60 { fb.record_frame(16.67 + i as f64 * 0.01, LodTier::Extreme); }
        let avg = fb.average_frame_ms();
        assert!(avg > 16.67 && avg < 17.3);
    }
    #[test] fn frame_budget_over() {
        let mut fb = FrameBudget::default();
        for _ in 0..60 { fb.record_frame(50.0, LodTier::Extreme); }
        assert!(fb.over_budget());
        assert!(fb.fps() < 30.0);
    }
    #[test] fn draw_tracker() {
        let mut dt = DrawCallTracker::new(1000);
        dt.record(true, false);
        dt.record(false, true);
        dt.record(true, true);
        assert_eq!(dt.total_calls, 3);
        assert_eq!(dt.instanced_calls, 2);
        assert_eq!(dt.culled_calls, 2);
        assert!((dt.instancing_ratio() - 2.0/3.0).abs() < 0.001);
    }
    #[test] fn draw_over_budget() {
        let mut dt = DrawCallTracker::new(5);
        for _ in 0..6 { dt.record(false, false); }
        assert!(dt.over_budget());
    }
    #[test] fn vram_tracks() {
        let mut vm = VramManager::new(1024.0);
        assert!(vm.allocate(500.0));
        assert!((vm.usage_ratio() - 500.0/1024.0).abs() < 0.001);
        vm.deallocate(200.0);
        assert!((vm.usage_ratio() - 300.0/1024.0).abs() < 0.001);
    }
    #[test] fn vram_refuses_over() {
        let mut vm = VramManager::new(100.0);
        assert!(vm.allocate(60.0));
        assert!(!vm.allocate(50.0));
    }
    #[test] fn metrics_count() {
        assert_eq!(count(), 4);
    }
}
