//! Render pipeline integration.
//!
//! Integration of close-up rendering with the main render pipeline.

use serde::{Serialize};

/// Render pipeline stage.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PipelineStage {
    Culling,
    LODSelection,
    GeometryGeneration,
    Shading,
    PostProcessing,
}

/// Pipeline stage configuration.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PipelineStageConfig {
    pub stage: PipelineStage,
    pub enabled: bool,
    pub max_time_ms: f64,
}

/// Default pipeline stage configs.
pub const DEFAULT_PIPELINE_STAGES: &[PipelineStageConfig] = &[
    PipelineStageConfig { stage: PipelineStage::Culling, enabled: true, max_time_ms: 1.0 },
    PipelineStageConfig { stage: PipelineStage::LODSelection, enabled: true, max_time_ms: 2.0 },
    PipelineStageConfig { stage: PipelineStage::GeometryGeneration, enabled: true, max_time_ms: 5.0 },
    PipelineStageConfig { stage: PipelineStage::Shading, enabled: true, max_time_ms: 8.0 },
    PipelineStageConfig { stage: PipelineStage::PostProcessing, enabled: true, max_time_ms: 3.0 },
];

pub fn count() -> usize {
    DEFAULT_PIPELINE_STAGES.len()
}
