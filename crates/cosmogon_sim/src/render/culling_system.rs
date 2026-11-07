//! Culling system.
//!
//! Frustum culling and occlusion culling to skip off-screen and hidden objects.

use crate::Vec3d;

/// An axis-aligned bounding box in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AABB {
    pub min: Vec3d,
    pub max: Vec3d,
}

impl AABB {
    pub fn center(&self) -> Vec3d {
        Vec3d::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
            (self.min.z + self.max.z) / 2.0,
        )
    }
    pub fn radius(&self) -> f64 {
        let c = self.center();
        (self.max - c).length()
    }
    pub fn contains_point(&self, p: Vec3d) -> bool {
        p.x >= self.min.x && p.x <= self.max.x &&
        p.y >= self.min.y && p.y <= self.max.y &&
        p.z >= self.min.z && p.z <= self.max.z
    }
}

/// Frustum planes (left, right, bottom, top, near, far).
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

/// A plane: normal dot point + distance = 0.
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3d,
    pub distance: f64,
}

impl Plane {
    pub fn signed_distance(&self, p: Vec3d) -> f64 {
        self.normal.dot(p) + self.distance
    }
}

impl Frustum {
    pub fn contains_aabb(&self, bounds: &AABB) -> bool {
        for plane in &self.planes {
            let center = bounds.center();
            let radius = bounds.radius();
            let dist = plane.signed_distance(center);
            if dist > radius {
                return false; // completely outside this plane
            }
        }
        true
    }
}

/// Culling system: manages frustum and occlusion culling.
#[derive(Clone, Debug, Default)]
pub struct CullingSystem {
    pub frustum: Option<Frustum>,
    pub culled_count: u32,
    pub total_count: u32,
    pub occlusion_queries: u32,
}

impl CullingSystem {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_frustum(&mut self, frustum: Frustum) {
        self.frustum = Some(frustum);
    }
    /// Test if a bounding box is visible. Returns true if visible.
    pub fn is_visible(&mut self, bounds: &AABB) -> bool {
        self.total_count += 1;
        if let Some(frustum) = &self.frustum {
            if !frustum.contains_aabb(bounds) {
                self.culled_count += 1;
                return false;
            }
        }
        true
    }
    /// Culling ratio (0..1): fraction of objects culled.
    pub fn culling_ratio(&self) -> f64 {
        if self.total_count == 0 { return 0.0; }
        self.culled_count as f64 / self.total_count as f64
    }
    pub fn reset(&mut self) {
        self.culled_count = 0;
        self.total_count = 0;
        self.occlusion_queries = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn frustum_culls_outside() {
        let mut cs = CullingSystem::new();
        let frustum = Frustum {
            planes: [
                Plane { normal: Vec3d::new(1.0, 0.0, 0.0), distance: -100.0 }, // x >= -100
                Plane { normal: Vec3d::new(-1.0, 0.0, 0.0), distance: -100.0 }, // x <= 100
                Plane { normal: Vec3d::new(0.0, 1.0, 0.0), distance: -100.0 },
                Plane { normal: Vec3d::new(0.0, -1.0, 0.0), distance: -100.0 },
                Plane { normal: Vec3d::new(0.0, 0.0, 1.0), distance: -1000.0 },
                Plane { normal: Vec3d::new(0.0, 0.0, -1.0), distance: -1000.0 },
            ],
        };
        cs.set_frustum(frustum);
        let inside = AABB { min: Vec3d::new(0.0, 0.0, 0.0), max: Vec3d::new(10.0, 10.0, 10.0) };
        let outside = AABB { min: Vec3d::new(200.0, 0.0, 0.0), max: Vec3d::new(210.0, 10.0, 10.0) };
        assert!(cs.is_visible(&inside));
        assert!(!cs.is_visible(&outside));
        assert!(cs.culling_ratio() > 0.0);
    }
    #[test] fn culling_reset() {
        let mut cs = CullingSystem::new();
        cs.total_count = 100;
        cs.culled_count = 50;
        cs.reset();
        assert_eq!(cs.total_count, 0);
        assert_eq!(cs.culled_count, 0);
    }
}
