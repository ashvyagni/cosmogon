//! Graphics supplement completion.
//!
//! Final integration and validation of the graphics supplement.

use serde::{Serialize};

/// Graphics supplement completion status.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GraphicsCompletion {
    pub builds_completed: u32,
    pub builds_total: u32,
    pub rendering_architecture: bool,
    pub stellar_activity: bool,
    pub terrain_detail: bool,
    pub weather: bool,
    pub geology: bool,
    pub civ_visibility: bool,
    pub small_bodies: bool,
}

/// Default completion status.
pub const DEFAULT_COMPLETION: GraphicsCompletion = GraphicsCompletion {
    builds_completed: 7,
    builds_total: 7,
    rendering_architecture: true,
    stellar_activity: true,
    terrain_detail: true,
    weather: true,
    geology: true,
    civ_visibility: true,
    small_bodies: true,
};
