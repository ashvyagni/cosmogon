//! Surface detail rendering.

use serde::Serialize;
use crate::rng::Rng;
use crate::rng::domain;
use crate::noise::fbm3;
use crate::Vec3d;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SurfaceDetail { Craters, Mountains, Rivers, Cities, Lights, }

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SurfaceDetailLevel { None, Basic, Detailed, Ultra, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SurfaceDetailParams {
    pub feature: SurfaceDetail,
    pub level: SurfaceDetailLevel,
    pub max_features: u32,
}

pub const DEFAULT_SURFACE_DETAIL: SurfaceDetailParams = SurfaceDetailParams {
    feature: SurfaceDetail::Craters,
    level: SurfaceDetailLevel::Basic,
    max_features: 1000,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFeature {
    pub kind: SurfaceDetail,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub size_km: f64,
    pub intensity: f64,
}

pub fn generate_surface_features(seed: u64, params: &SurfaceDetailParams, lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64) -> Vec<SurfaceFeature> {
    let mut rng = Rng::stream(seed, domain::TERRAIN, &[0xA1B2]);
    let count = match params.level {
        SurfaceDetailLevel::None => 0,
        SurfaceDetailLevel::Basic => (params.max_features as f64 * 0.2) as u32,
        SurfaceDetailLevel::Detailed => (params.max_features as f64 * 0.6) as u32,
        SurfaceDetailLevel::Ultra => params.max_features,
    };
    let mut feats = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let lat = rng.range(lat_min, lat_max);
        let lon = rng.range(lon_min, lon_max);
        let placement = fbm3(seed ^ 0x7E57, [lat * 0.01, lon * 0.01, 0.0], 3, 2.0, 0.5);
        if placement < -0.2 { continue; }
        let size = match params.feature {
            SurfaceDetail::Craters => rng.range(1.0, 100.0),
            SurfaceDetail::Mountains => rng.range(5.0, 200.0),
            SurfaceDetail::Rivers => rng.range(10.0, 500.0),
            SurfaceDetail::Cities => rng.range(2.0, 50.0),
            SurfaceDetail::Lights => rng.range(1.0, 30.0),
        };
        feats.push(SurfaceFeature { kind: params.feature, lat_deg: lat, lon_deg: lon, size_km: size, intensity: rng.range(0.2, 1.0) });
    }
    feats
}

pub fn mountain_chain(seed: u64, start_lat: f64, start_lon: f64, end_lat: f64, end_lon: f64, num_peaks: u32) -> Vec<(f64, f64, f64)> {
    let mut rng = Rng::stream(seed, domain::TERRAIN, &[0x9D8C]);
    let mut peaks = Vec::with_capacity(num_peaks as usize);
    for i in 0..num_peaks {
        let t = i as f64 / num_peaks as f64;
        let lat = start_lat + (end_lat - start_lat) * t + rng.range(-2.0, 2.0);
        let lon = start_lon + (end_lon - start_lon) * t + rng.range(-2.0, 2.0);
        peaks.push((lat, lon, rng.range(1000.0, 9000.0)));
    }
    peaks
}

pub fn river_network(seed: u64, num_rivers: u32, lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64) -> Vec<Vec<(f64, f64)>> {
    let mut rng = Rng::stream(seed, domain::TERRAIN, &[0x4F1A]);
    let mut rivers = Vec::with_capacity(num_rivers as usize);
    for _ in 0..num_rivers {
        let mut pts = Vec::new();
        let mut lat = rng.range(lat_min, lat_max);
        let mut lon = rng.range(lon_min, lon_max);
        pts.push((lat, lon));
        for _ in 0..rng.range_u32(5, 20) {
            lat += rng.range(-1.0, 1.0);
            lon += rng.range(-1.0, 1.0);
            lat = lat.clamp(lat_min, lat_max);
            lon = lon.rem_euclid(360.0);
            pts.push((lat, lon));
        }
        rivers.push(pts);
    }
    rivers
}

pub fn city_light_map(seed: u64, num_cities: u32, lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64) -> Vec<(f64, f64, f64)> {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0xC3D4]);
    let mut lights = Vec::with_capacity(num_cities as usize);
    for _ in 0..num_cities {
        lights.push((rng.range(lat_min, lat_max), rng.range(lon_min, lon_max), rng.range(0.1, 1.0)));
    }
    lights
}

pub fn surface_detail_coverage(params: &SurfaceDetailParams, distance_km: f64, body_radius_km: f64) -> f64 {
    let angular = (body_radius_km / distance_km).atan() * 2.0;
    let mult = match params.level {
        SurfaceDetailLevel::None => 0.0,
        SurfaceDetailLevel::Basic => 0.2,
        SurfaceDetailLevel::Detailed => 0.6,
        SurfaceDetailLevel::Ultra => 1.0,
    };
    angular * mult
}

pub fn surface_normal(lat_deg: f64, lon_deg: f64, _params: &SurfaceDetailParams, seed: u64) -> Vec3d {
    let eps = 0.01;
    let h = |lat: f64, lon: f64| fbm3(seed, [lat.to_radians(), lon.to_radians(), 0.0], 4, 2.0, 0.5);
    let hc = h(lat_deg, lon_deg);
    let hdx = h(lat_deg + eps, lon_deg) - hc;
    let hdy = h(lat_deg, lon_deg + eps) - hc;
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let radial = Vec3d::new(lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin());
    let east = Vec3d::new(-lon.sin(), lon.cos(), 0.0);
    let north = Vec3d::new(-lat.sin() * lon.cos(), -lat.sin() * lon.sin(), lat.cos());
    (radial + east * hdx * 0.1 + north * hdy * 0.1).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn features_deterministic() {
        let p = DEFAULT_SURFACE_DETAIL;
        assert_eq!(generate_surface_features(42, &p, -60.0, 60.0, -100.0, 100.0), generate_surface_features(42, &p, -60.0, 60.0, -100.0, 100.0));
    }
    #[test] fn level_affects_count() {
        let p_basic = SurfaceDetailParams { level: SurfaceDetailLevel::Basic, ..DEFAULT_SURFACE_DETAIL };
        let p_ultra = SurfaceDetailParams { level: SurfaceDetailLevel::Ultra, ..DEFAULT_SURFACE_DETAIL };
        let fb = generate_surface_features(7, &p_basic, -90.0, 90.0, -180.0, 180.0).len();
        let fu = generate_surface_features(7, &p_ultra, -90.0, 90.0, -180.0, 180.0).len();
        assert!(fu > fb);
    }
    #[test] fn none_has_none() {
        let p = SurfaceDetailParams { level: SurfaceDetailLevel::None, ..DEFAULT_SURFACE_DETAIL };
        assert_eq!(generate_surface_features(3, &p, -90.0, 90.0, -180.0, 180.0).len(), 0);
    }
    #[test] fn mountain_chain_linear() {
        let chain = mountain_chain(5, 0.0, 0.0, 10.0, 10.0, 5);
        assert_eq!(chain.len(), 5);
    }
    #[test] fn rivers_have_paths() {
        let rivers = river_network(11, 3, -60.0, 60.0, -100.0, 100.0);
        assert_eq!(rivers.len(), 3);
        for r in &rivers { assert!(r.len() >= 2); }
    }
    #[test] fn lights_have_intensity() {
        let lights = city_light_map(13, 10, -60.0, 60.0, -100.0, 100.0);
        assert_eq!(lights.len(), 10);
        for (_, _, i) in &lights { assert!((0.0..=1.0).contains(i)); }
    }
    #[test] fn coverage_valid() {
        let p = DEFAULT_SURFACE_DETAIL;
        let c = surface_detail_coverage(&p, 1000.0, 6371.0);
        assert!(c >= 0.0);
    }
    #[test] fn normal_is_unit() {
        let p = DEFAULT_SURFACE_DETAIL;
        let n = surface_normal(45.0, 90.0, &p, 17);
        assert!((n.length() - 1.0).abs() < 0.01);
    }
    #[test] fn max_features_limits() {
        let p = SurfaceDetailParams { max_features: 5, ..DEFAULT_SURFACE_DETAIL };
        let f = generate_surface_features(21, &p, -90.0, 90.0, -180.0, 180.0);
        assert!(f.len() <= 5);
    }
}
