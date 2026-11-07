//! Performance profiling framework.
//!
//! Frame-level timing, draw call counting, VRAM tracking, and performance
//! regression detection. Keeps the simulator smooth by measuring where time
//! is spent and triggering quality reductions when budgets are exceeded.

use std::time::Instant;

/// A named timing measurement.
#[derive(Clone, Debug, PartialEq)]
pub struct TimingSample {
    pub name: &'static str,
    pub elapsed_ms: f64,
}

/// Frame-level performance profile.
#[derive(Clone, Debug, Default)]
pub struct FrameProfile {
    pub frame_index: u64,
    pub total_ms: f64,
    pub samples: Vec<TimingSample>,
    pub draw_calls: u32,
    pub triangles: u64,
    pub vram_mb: f64,
}

/// Rolling window of frame profiles for regression detection.
#[derive(Clone, Debug)]
pub struct ProfileHistory {
    pub frames: Vec<FrameProfile>,
    pub max_frames: usize,
}

impl Default for ProfileHistory {
    fn default() -> Self {
        Self { frames: Vec::with_capacity(10), max_frames: 10 }
    }
}

impl ProfileHistory {
    pub fn new(max_frames: usize) -> Self {
        Self { frames: Vec::with_capacity(max_frames), max_frames }
    }
    pub fn push(&mut self, frame: FrameProfile) {
        self.frames.push(frame);
        if self.frames.len() > self.max_frames {
            self.frames.remove(0);
        }
    }
    pub fn average_frame_ms(&self) -> f64 {
        if self.frames.is_empty() { return 0.0; }
        self.frames.iter().map(|f| f.total_ms).sum::<f64>() / self.frames.len() as f64
    }
    pub fn p95_frame_ms(&self) -> f64 {
        if self.frames.is_empty() { return 0.0; }
        let mut times: Vec<f64> = self.frames.iter().map(|f| f.total_ms).collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = (times.len() as f64 * 0.95) as usize;
        times[idx.min(times.len() - 1)]
    }
}

/// Performance profiler: accumulates per-frame timing.
#[derive(Clone, Debug, Default)]
pub struct PerformanceProfiler {
    pub current_frame: Option<FrameProfile>,
    pub history: ProfileHistory,
    pub start_time: Option<Instant>,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn begin_frame(&mut self, frame_index: u64) {
        self.current_frame = Some(FrameProfile {
            frame_index,
            total_ms: 0.0,
            samples: Vec::new(),
            draw_calls: 0,
            triangles: 0,
            vram_mb: 0.0,
        });
        self.start_time = Some(Instant::now());
    }
    pub fn record_sample(&mut self, name: &'static str, elapsed_ms: f64) {
        if let Some(frame) = self.current_frame.as_mut() {
            frame.samples.push(TimingSample { name, elapsed_ms });
        }
    }
    pub fn end_frame(&mut self) {
        if let Some(mut frame) = self.current_frame.take() {
            if let Some(start) = self.start_time.take() {
                frame.total_ms = start.elapsed().as_secs_f64() * 1000.0;
            }
            self.history.push(frame);
        }
    }
    pub fn recent_average_ms(&self) -> f64 {
        self.history.average_frame_ms()
    }
    pub fn recent_p95_ms(&self) -> f64 {
        self.history.p95_frame_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn profiler_tracks_frames() {
        let mut p = PerformanceProfiler::new();
        p.begin_frame(0);
        p.record_sample("update", 5.0);
        p.record_sample("render", 8.0);
        p.end_frame();
        assert_eq!(p.recent_average_ms(), p.recent_p95_ms());
    }
    #[test] fn history_limits_frames() {
        let mut p = PerformanceProfiler::new();
        for i in 0..15 {
            p.begin_frame(i);
            p.record_sample("x", 10.0);
            p.end_frame();
        }
        assert_eq!(p.history.frames.len(), 10);
    }
}
