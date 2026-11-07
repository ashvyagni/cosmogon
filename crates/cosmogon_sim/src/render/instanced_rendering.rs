//! Instanced rendering.
//!
//! Batches identical meshes (craters, city lights, settlements) into single
//! draw calls to minimize CPU overhead and GPU command submission.


/// An instanced draw call: one mesh, many transforms.
#[derive(Clone, Debug, PartialEq)]
pub struct InstancedDrawCall {
    pub mesh_id: u32,
    pub material_id: u32,
    pub instance_count: u32,
    pub instance_data_offset: u32,
}

/// Instancing manager: batches objects into instanced draw calls.
#[derive(Clone, Debug, Default)]
pub struct InstancingManager {
    pub draw_calls: Vec<InstancedDrawCall>,
    pub total_instances: u32,
}

impl InstancingManager {
    pub fn new() -> Self {
        Self::default()
    }
    /// Add instances for a mesh/material pair.
    pub fn add_instances(&mut self, mesh_id: u32, material_id: u32, count: u32) {
        if let Some(dc) = self.draw_calls.iter_mut().find(|dc| dc.mesh_id == mesh_id && dc.material_id == material_id) {
            dc.instance_count += count;
        } else {
            self.draw_calls.push(InstancedDrawCall {
                mesh_id,
                material_id,
                instance_count: count,
                instance_data_offset: self.total_instances,
            });
        }
        self.total_instances += count;
    }
    /// Returns the number of draw calls saved by instancing.
    pub fn draw_calls_saved(&self, total_objects: u32) -> u32 {
        total_objects.saturating_sub(self.draw_calls.len() as u32)
    }
    /// Returns the instancing efficiency (0..1).
    pub fn efficiency(&self) -> f64 {
        if self.total_instances == 0 { return 1.0; }
        self.draw_calls.len() as f64 / self.total_instances as f64
    }
    pub fn reset(&mut self) {
        self.draw_calls.clear();
        self.total_instances = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn instancing_batches() {
        let mut im = InstancingManager::new();
        im.add_instances(1, 1, 100);
        im.add_instances(1, 1, 50);
        im.add_instances(2, 1, 30);
        assert_eq!(im.draw_calls.len(), 2);
        assert_eq!(im.draw_calls[0].instance_count, 150);
        assert_eq!(im.draw_calls[1].instance_count, 30);
        assert_eq!(im.total_instances, 180);
    }
    #[test] fn draw_calls_saved() {
        let mut im = InstancingManager::new();
        im.add_instances(1, 1, 100);
        assert_eq!(im.draw_calls_saved(100), 99);
    }
    #[test] fn efficiency() {
        let mut im = InstancingManager::new();
        im.add_instances(1, 1, 100);
        assert!((im.efficiency() - 0.01).abs() < 0.001);
    }
    #[test] fn reset_clears() {
        let mut im = InstancingManager::new();
        im.add_instances(1, 1, 100);
        im.reset();
        assert_eq!(im.draw_calls.len(), 0);
        assert_eq!(im.total_instances, 0);
    }
}
