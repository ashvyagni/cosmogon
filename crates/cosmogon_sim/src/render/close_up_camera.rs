//! Close-up camera controls.

use serde::Serialize;
use crate::Vec3d;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CameraMode { Orbit, Free, Focus, Track, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CameraParams {
    pub mode: CameraMode,
    pub distance_km: f64,
    pub fov_deg: f64,
    pub target_lat_deg: f64,
    pub target_lon_deg: f64,
}

pub const DEFAULT_CAMERA: CameraParams = CameraParams {
    mode: CameraMode::Orbit,
    distance_km: 1000.0,
    fov_deg: 45.0,
    target_lat_deg: 0.0,
    target_lon_deg: 0.0,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CameraState {
    pub position: Vec3d,
    pub target: Vec3d,
    pub up: Vec3d,
    pub fov_deg: f64,
}

#[derive(Clone, Debug)]
pub struct CameraController {
    pub params: CameraParams,
    pub state: CameraState,
    pub smooth_factor: f64,
    pub min_distance_km: f64,
    pub max_distance_km: f64,
    pub collision_avoidance: bool,
}

impl CameraController {
    pub fn new(params: CameraParams) -> Self {
        let state = camera_state_from_params(&params);
        Self { params, state, smooth_factor: 0.15, min_distance_km: 5.0, max_distance_km: 50_000.0, collision_avoidance: true }
    }
    pub fn update(&mut self, dt: f64, body_radius_km: f64) {
        self.params.distance_km = self.params.distance_km.clamp(
            self.min_distance_km.max(body_radius_km * 1.05),
            self.max_distance_km,
        );
        let ideal = ideal_camera_position(&self.params, body_radius_km);
        let t = (self.smooth_factor * dt * 10.0).clamp(0.0, 1.0);
        self.state.position = self.state.position.lerp(ideal, t);
        self.state.target = params_to_target_vec(&self.params, body_radius_km);
        self.state.fov_deg = self.params.fov_deg;
        if self.collision_avoidance {
            let to_camera = (self.state.position - self.state.target).normalize();
            let dist = (self.state.position - self.state.target).length();
            if dist < body_radius_km * 1.02 {
                self.state.position = self.state.target + to_camera * body_radius_km * 1.02;
                self.params.distance_km = body_radius_km * 1.02;
            }
        }
    }
    pub fn orbit(&mut self, _d_lat: f64, _d_lon: f64) {}
    pub fn zoom(&mut self, factor: f64) {
        self.params.distance_km /= factor;
        self.params.distance_km = self.params.distance_km.clamp(self.min_distance_km, self.max_distance_km);
    }
    pub fn pan(&mut self, d_lat: f64, d_lon: f64) {
        self.params.target_lat_deg = (self.params.target_lat_deg + d_lat).clamp(-90.0, 90.0);
        self.params.target_lon_deg = (self.params.target_lon_deg + d_lon).rem_euclid(360.0);
    }
    pub fn focus(&mut self, lat_deg: f64, lon_deg: f64, distance_km: f64) {
        self.params.target_lat_deg = lat_deg.clamp(-90.0, 90.0);
        self.params.target_lon_deg = lon_deg.rem_euclid(360.0);
        self.params.distance_km = distance_km;
    }
}

fn camera_state_from_params(params: &CameraParams) -> CameraState {
    let target = params_to_target_vec(params, 1.0);
    let pos = params_to_camera_pos(params, 1.0);
    CameraState { position: pos, target, up: Vec3d::new(0.0, 0.0, 1.0), fov_deg: params.fov_deg }
}

fn params_to_target_vec(params: &CameraParams, radius_km: f64) -> Vec3d {
    let lat = params.target_lat_deg.to_radians();
    let lon = params.target_lon_deg.to_radians();
    Vec3d::new(lat.cos() * lon.cos() * radius_km, lat.cos() * lon.sin() * radius_km, lat.sin() * radius_km)
}

fn params_to_camera_pos(params: &CameraParams, radius_km: f64) -> Vec3d {
    let target = params_to_target_vec(params, radius_km);
    let radial = target.normalize();
    target + radial * params.distance_km
}

fn ideal_camera_position(params: &CameraParams, radius_km: f64) -> Vec3d {
    params_to_camera_pos(params, radius_km)
}

pub fn project(point: Vec3d, camera: &CameraState, screen_w: f64, screen_h: f64) -> Option<(f64, f64)> {
    let view_dir = (camera.target - camera.position).normalize();
    let right = view_dir.cross(Vec3d::new(0.0, 0.0, 1.0)).normalize();
    let up = right.cross(view_dir).normalize();
    let to_point = point - camera.position;
    let depth = to_point.dot(view_dir);
    if depth <= 0.0 { return None; }
    let fov_rad = camera.fov_deg.to_radians();
    let sx = to_point.dot(right) / depth * (screen_w / fov_rad.tan());
    let sy = to_point.dot(up) / depth * (screen_h / fov_rad.tan());
    Some((screen_w / 2.0 + sx, screen_h / 2.0 - sy))
}

pub fn is_visible(point: Vec3d, camera: &CameraState, screen_w: f64, screen_h: f64) -> bool {
    match project(point, camera, screen_w, screen_h) {
        Some((x, y)) => x >= 0.0 && x <= screen_w && y >= 0.0 && y <= screen_h,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn camera_deterministic() {
        let p = CameraParams { mode: CameraMode::Orbit, distance_km: 1000.0, fov_deg: 45.0, target_lat_deg: 0.0, target_lon_deg: 0.0 };
        assert_eq!(CameraController::new(p.clone()).state.position, CameraController::new(p).state.position);
    }
    #[test] fn zoom_clamps() {
        let mut cc = CameraController::new(DEFAULT_CAMERA);
        cc.params.distance_km = 100.0;
        cc.zoom(10.0);
        assert!(cc.params.distance_km >= cc.min_distance_km);
        cc.zoom(0.001);
        assert!(cc.params.distance_km <= cc.max_distance_km);
    }
    #[test] fn pan_changes_target() {
        let mut cc = CameraController::new(DEFAULT_CAMERA);
        cc.pan(10.0, 20.0);
        assert!((cc.params.target_lat_deg - 10.0).abs() < 0.001);
        assert!((cc.params.target_lon_deg - 20.0).abs() < 0.001);
    }
    #[test] fn focus_sets_all() {
        let mut cc = CameraController::new(DEFAULT_CAMERA);
        cc.focus(45.0, 90.0, 500.0);
        assert!((cc.params.target_lat_deg - 45.0).abs() < 0.001);
        assert!((cc.params.target_lon_deg - 90.0).abs() < 0.001);
        assert!((cc.params.distance_km - 500.0).abs() < 0.001);
    }
    #[test] fn collision_avoidance() {
        let mut params = DEFAULT_CAMERA;
        params.distance_km = 1.0;
        let mut cc = CameraController::new(params);
        cc.update(0.1, 6371.0);
        assert!(cc.params.distance_km >= 6371.0 * 1.02);
    }
    #[test] fn projection_works() {
        let cc = CameraController::new(DEFAULT_CAMERA);
        let point = cc.state.target + Vec3d::new(100.0, 0.0, 0.0);
        let r = project(point, &cc.state, 1920.0, 1080.0);
        assert!(r.is_some());
        let (px, py) = r.unwrap();
        assert!(px.is_finite() && py.is_finite());
    }
    #[test] fn behind_camera_invisible() {
        let cc = CameraController::new(DEFAULT_CAMERA);
        let view_dir = (cc.state.target - cc.state.position).normalize();
        let behind = cc.state.position - view_dir * 1000.0;
        assert!(!is_visible(behind, &cc.state, 1920.0, 1080.0));
    }
}
