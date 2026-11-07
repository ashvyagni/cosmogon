//! Adaptive quality system.
//!
//! Dynamically reduces rendering quality (LOD, particle count, shadow resolution)
//! when frame budget is exceeded, and restores quality when headroom returns.



/// Quality tier for adaptive rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QualityTier {
    Ultra,   // Maximum quality, no compromises
    High,    // Minor quality reductions
    Medium,  // Noticeable reductions
    Low,     // Aggressive reductions
    Minimal, // Bare minimum, performance first
}

/// Adaptive quality controller.
#[derive(Clone, Debug)]
pub struct AdaptiveQualityController {
    pub target_fps: f64,
    pub target_frame_ms: f64,
    pub current_tier: QualityTier,
    pub consecutive_good_frames: u32,
    pub consecutive_bad_frames: u32,
    pub frames_needed_to_upgrade: u32,
    pub frames_needed_to_downgrade: u32,
}

impl Default for AdaptiveQualityController {
    fn default() -> Self {
        Self {
            target_fps: 60.0,
            target_frame_ms: 16.67,
            current_tier: QualityTier::High,
            consecutive_good_frames: 0,
            consecutive_bad_frames: 0,
            frames_needed_to_upgrade: 30,
            frames_needed_to_downgrade: 5,
        }
    }
}

impl AdaptiveQualityController {
    pub fn update(&mut self, frame_ms: f64) -> QualityTier {
        if frame_ms > self.target_frame_ms {
            self.consecutive_bad_frames += 1;
            self.consecutive_good_frames = 0;
            if self.consecutive_bad_frames >= self.frames_needed_to_downgrade {
                self.downgrade();
                self.consecutive_bad_frames = 0;
            }
        } else {
            self.consecutive_good_frames += 1;
            self.consecutive_bad_frames = 0;
            if self.consecutive_good_frames >= self.frames_needed_to_upgrade {
                self.upgrade();
                self.consecutive_good_frames = 0;
            }
        }
        self.current_tier
    }
    fn downgrade(&mut self) {
        self.current_tier = match self.current_tier {
            QualityTier::Ultra => QualityTier::High,
            QualityTier::High => QualityTier::Medium,
            QualityTier::Medium => QualityTier::Low,
            QualityTier::Low => QualityTier::Minimal,
            QualityTier::Minimal => QualityTier::Minimal,
        };
    }
    fn upgrade(&mut self) {
        self.current_tier = match self.current_tier {
            QualityTier::Minimal => QualityTier::Low,
            QualityTier::Low => QualityTier::Medium,
            QualityTier::Medium => QualityTier::High,
            QualityTier::High => QualityTier::Ultra,
            QualityTier::Ultra => QualityTier::Ultra,
        };
    }
    /// Maps quality tier to LOD tier.
    pub fn lod_tier(&self) -> crate::render::lod_budget::LodTier {
        match self.current_tier {
            QualityTier::Ultra => crate::render::lod_budget::LodTier::Extreme,
            QualityTier::High => crate::render::lod_budget::LodTier::Close,
            QualityTier::Medium => crate::render::lod_budget::LodTier::Medium,
            QualityTier::Low => crate::render::lod_budget::LodTier::Far,
            QualityTier::Minimal => crate::render::lod_budget::LodTier::Far,
        }
    }
    /// Returns the particle count multiplier (0..1).
    pub fn particle_multiplier(&self) -> f64 {
        match self.current_tier {
            QualityTier::Ultra => 1.0,
            QualityTier::High => 0.75,
            QualityTier::Medium => 0.5,
            QualityTier::Low => 0.25,
            QualityTier::Minimal => 0.1,
        }
    }
    /// Returns the shadow map resolution multiplier (0..1).
    pub fn shadow_multiplier(&self) -> f64 {
        match self.current_tier {
            QualityTier::Ultra => 1.0,
            QualityTier::High => 0.75,
            QualityTier::Medium => 0.5,
            QualityTier::Low => 0.25,
            QualityTier::Minimal => 0.125,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn downgrade_on_bad_frames() {
        let mut aq = AdaptiveQualityController::default();
        for _ in 0..5 {
            aq.update(50.0); // way over 16.67ms
        }
        assert_eq!(aq.current_tier, QualityTier::Medium);
    }
    #[test] fn upgrade_on_good_frames() {
        let mut aq = AdaptiveQualityController::default();
        aq.current_tier = QualityTier::Low;
        aq.consecutive_good_frames = 0;
        for _ in 0..30 {
            aq.update(8.0); // well under budget
        }
        assert_eq!(aq.current_tier, QualityTier::Medium);
    }
    #[test] fn lod_tier_maps() {
        let aq = AdaptiveQualityController {
            current_tier: QualityTier::Ultra,
            ..Default::default()
        };
        assert_eq!(aq.lod_tier(), crate::render::lod_budget::LodTier::Extreme);
        let aq = AdaptiveQualityController {
            current_tier: QualityTier::Minimal,
            ..Default::default()
        };
        assert_eq!(aq.lod_tier(), crate::render::lod_budget::LodTier::Far);
    }
    #[test] fn particle_multiplier_decreases() {
        let ultra = AdaptiveQualityController {
            current_tier: QualityTier::Ultra,
            ..Default::default()
        };
        let minimal = AdaptiveQualityController {
            current_tier: QualityTier::Minimal,
            ..Default::default()
        };
        assert!(ultra.particle_multiplier() > minimal.particle_multiplier());
    }
}
