//! Render pipeline integration.

use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PipelineStage { Culling, LODSelection, GeometryGeneration, Shading, PostProcessing, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PipelineStageConfig {
    pub stage: PipelineStage,
    pub enabled: bool,
    pub max_time_ms: f64,
    pub actual_time_ms: f64,
}

pub fn count() -> usize {
    DEFAULT_PIPELINE_STAGES.len()
}

pub const DEFAULT_PIPELINE_STAGES: &[PipelineStageConfig] = &[
    PipelineStageConfig { stage: PipelineStage::Culling, enabled: true, max_time_ms: 1.0, actual_time_ms: 0.0 },
    PipelineStageConfig { stage: PipelineStage::LODSelection, enabled: true, max_time_ms: 2.0, actual_time_ms: 0.0 },
    PipelineStageConfig { stage: PipelineStage::GeometryGeneration, enabled: true, max_time_ms: 5.0, actual_time_ms: 0.0 },
    PipelineStageConfig { stage: PipelineStage::Shading, enabled: true, max_time_ms: 8.0, actual_time_ms: 0.0 },
    PipelineStageConfig { stage: PipelineStage::PostProcessing, enabled: true, max_time_ms: 3.0, actual_time_ms: 0.0 },
];

#[derive(Clone, Debug, Default)]
pub struct RenderPipeline {
    pub stages: Vec<PipelineStageConfig>,
    pub total_frame_ms: f64,
    pub instancing_enabled: bool,
    pub frustum_culling_enabled: bool,
    pub occlusion_culling_enabled: bool,
}

impl RenderPipeline {
    pub fn new() -> Self {
        Self { stages: DEFAULT_PIPELINE_STAGES.to_vec(), ..Default::default() }
    }
    pub fn record_stage(&mut self, stage: PipelineStage, time_ms: f64) {
        if let Some(s) = self.stages.iter_mut().find(|s| s.stage == stage) {
            s.actual_time_ms = time_ms;
        }
        self.total_frame_ms = self.stages.iter().map(|s| s.actual_time_ms).sum();
    }
    pub fn stage_over_budget(&self, stage: PipelineStage) -> bool {
        self.stages.iter().find(|s| s.stage == stage).map(|s| s.enabled && s.actual_time_ms > s.max_time_ms).unwrap_or(false)
    }
    pub fn over_budget_stages(&self) -> Vec<PipelineStage> {
        self.stages.iter().filter(|s| s.enabled && s.actual_time_ms > s.max_time_ms).map(|s| s.stage).collect()
    }
    pub fn total_time_ms(&self) -> f64 { self.total_frame_ms }
    pub fn set_instancing(&mut self, enabled: bool) { self.instancing_enabled = enabled; }
    pub fn set_frustum_culling(&mut self, enabled: bool) { self.frustum_culling_enabled = enabled; }
    pub fn set_occlusion_culling(&mut self, enabled: bool) { self.occlusion_culling_enabled = enabled; }
    pub fn estimated_culled_calls(&self, total_objects: u32) -> u32 {
        let mut savings = 0.0f64;
        if self.frustum_culling_enabled { savings += total_objects as f64 * 0.4; }
        if self.occlusion_culling_enabled { savings += total_objects as f64 * 0.25 * (1.0 - savings / total_objects as f64); }
        savings.min(total_objects as f64) as u32
    }
    pub fn instanced_batch_count(&self, total_objects: u32, unique_meshes: u32) -> u32 {
        if !self.instancing_enabled { return total_objects; }
        unique_meshes.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn pipeline_records_stage() {
        let mut rp = RenderPipeline::new();
        rp.record_stage(PipelineStage::Shading, 4.5);
        assert_eq!(rp.stages.iter().find(|s| s.stage == PipelineStage::Shading).unwrap().actual_time_ms, 4.5);
        assert!((rp.total_frame_ms - 4.5).abs() < 0.001);
    }
    #[test] fn stage_over_budget() {
        let mut rp = RenderPipeline::new();
        rp.record_stage(PipelineStage::Shading, 10.0);
        assert!(rp.stage_over_budget(PipelineStage::Shading));
        assert!(!rp.stage_over_budget(PipelineStage::Culling));
    }
    #[test] fn over_budget_stages() {
        let mut rp = RenderPipeline::new();
        rp.record_stage(PipelineStage::Shading, 10.0);
        rp.record_stage(PipelineStage::GeometryGeneration, 6.0);
        let over = rp.over_budget_stages();
        assert!(over.contains(&PipelineStage::Shading));
        assert!(over.contains(&PipelineStage::GeometryGeneration));
        assert!(!over.contains(&PipelineStage::Culling));
    }
    #[test] fn instancing_reduces_calls() {
        let rp = RenderPipeline::new();
        assert_eq!(rp.instanced_batch_count(1000, 5), 1000);
        let mut rp_i = RenderPipeline::new();
        rp_i.set_instancing(true);
        assert_eq!(rp_i.instanced_batch_count(1000, 5), 5);
    }
    #[test] fn frustum_culling_saves() {
        let mut rp = RenderPipeline::new();
        assert_eq!(rp.estimated_culled_calls(1000), 0);
        rp.set_frustum_culling(true);
        let c = rp.estimated_culled_calls(1000);
        assert!(c > 300 && c < 500);
    }
    #[test] fn lod_reduces_triangles() {
        use crate::render::lod_budget::LodTier;
        let base = 1_000_000u64;
        let far = super::GeometryLOD::new(base, LodTier::Far);
        let ext = super::GeometryLOD::new(base, LodTier::Extreme);
        assert!(far.effective_triangles() < ext.effective_triangles());
        assert_eq!(ext.effective_triangles(), base);
    }
    #[test] fn pipeline_total_sums() {
        let mut rp = RenderPipeline::new();
        rp.record_stage(PipelineStage::Culling, 1.0);
        rp.record_stage(PipelineStage::Shading, 5.0);
        assert!((rp.total_frame_ms - 6.0).abs() < 0.001);
    }
}

// Geometry LOD: reduce triangle count based on distance.
#[derive(Clone, Debug)]
pub struct GeometryLOD {
    pub base_triangles: u64,
    pub lod_tier: crate::render::lod_budget::LodTier,
}

impl GeometryLOD {
    pub fn new(base_triangles: u64, tier: crate::render::lod_budget::LodTier) -> Self {
        Self { base_triangles, lod_tier: tier }
    }
    pub fn effective_triangles(&self) -> u64 {
        let mult = match self.lod_tier {
            crate::render::lod_budget::LodTier::Far => 0.05,
            crate::render::lod_budget::LodTier::Medium => 0.25,
            crate::render::lod_budget::LodTier::Close => 0.7,
            crate::render::lod_budget::LodTier::Extreme => 1.0,
        };
        (self.base_triangles as f64 * mult) as u64
    }
}
