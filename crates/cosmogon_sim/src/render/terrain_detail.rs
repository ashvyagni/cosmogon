//! Planet terrain close-ups: mountains, water, erosion detail.

use serde::Serialize;
use crate::noise::{fbm3, ridged3};
use crate::rng::Rng;
use crate::rng::domain;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerrainFeature { Mountain, Valley, Crater, Ocean, River, Desert, Forest, Ice, Volcano, }

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerrainDetail { Low, Medium, High, Ultra, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct TerrainParams {
    pub seed: u64,
    pub detail: TerrainDetail,
    pub mountain_height_m: f64,
    pub ocean_depth_m: f64,
    pub erosion_eras: u32,
}

pub const DEFAULT_TERRAIN: TerrainParams = TerrainParams {
    seed: 0xDEAD_BEEF, detail: TerrainDetail::High, mountain_height_m: 8848.0,
    ocean_depth_m: 10911.0, erosion_eras: 3,
};

pub fn terrain_height(p: &TerrainParams, lat_deg: f64, lon_deg: f64) -> f64 {
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    let v = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
    let b = fbm3(p.seed, v, 6, 2.0, 0.55);
    let r = ridged3(p.seed ^ 0x5EED, v, 5);
    let f = fbm3(p.seed ^ 0xC0FF, v, 4, 2.5, 0.5);
    let h = (b * 0.5 + r * 0.35 + f * 0.15) * p.mountain_height_m;
    apply_erosion(p, h, lat_deg, lon_deg)
}

fn apply_erosion(p: &TerrainParams, h: f64, lat: f64, lon: f64) -> f64 {
    if p.erosion_eras == 0 { return h; }
    let mut rng = Rng::stream(p.seed, domain::TERRAIN, &[(lat * 1000.0) as u64, (lon * 1000.0) as u64]);
    let mut h = h;
    for _ in 0..p.erosion_eras {
        let s = fbm3(p.seed ^ 0x1234, [lat * 0.01, lon * 0.01, 0.0], 3, 2.0, 0.5);
        let w = rng.range(0.0, 1.0);
        let cap = s.abs() * w * 0.3;
        let sed = h.clamp(0.0, p.mountain_height_m) * 0.05;
        let dep = cap.min(sed);
        h = if h > 0.0 { h - (sed - dep) * 0.3 } else { h + dep * 0.2 };
    }
    h
}

pub fn feature_density(d: TerrainDetail) -> f64 {
    match d { TerrainDetail::Low => 0.5, TerrainDetail::Medium => 2.0, TerrainDetail::High => 8.0, TerrainDetail::Ultra => 32.0 }
}

pub fn sample_patch(p: &TerrainParams, lat: f64, lon: f64) -> (f64, TerrainFeature) {
    let h = terrain_height(p, lat, lon);
    let mut rng = Rng::stream(p.seed, domain::TERRAIN, &[(lat * 1000.0) as u64, (lon * 1000.0) as u64, 1]);
    let n = fbm3(p.seed ^ 0x9E37, [lat * 0.02, lon * 0.02, 0.0], 3, 2.0, 0.5);
    let feat = if h < -p.ocean_depth_m * 0.3 { TerrainFeature::Ocean }
    else if h < -p.ocean_depth_m * 0.1 { TerrainFeature::Ice }
    else if n > 0.6 && h > p.mountain_height_m * 0.4 { TerrainFeature::Mountain }
    else if n < -0.4 && h < p.mountain_height_m * 0.1 { TerrainFeature::Valley }
    else if n > 0.8 && rng.chance(0.1) { TerrainFeature::Volcano }
    else if n > 0.2 && h > 0.0 { TerrainFeature::Desert }
    else if n < -0.2 && h > 0.0 { TerrainFeature::Forest }
    else if n.abs() < 0.1 { TerrainFeature::River }
    else { TerrainFeature::Crater };
    (h, feat)
}

pub fn crater_field(p: &TerrainParams, lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64) -> Vec<(f64, f64, f64)> {
    let mut rng = Rng::stream(p.seed, domain::TERRAIN, &[lat_min as u64, lon_min as u64, 2]);
    let area = (lat_max - lat_min).abs() * (lon_max - lon_min).abs();
    let n = (area * feature_density(p.detail) * 0.1) as usize;
    let mut c = Vec::with_capacity(n);
    for _ in 0..n {
        c.push((rng.range(lat_min, lat_max), rng.range(lon_min, lon_max), rng.range(1.0, 50.0)));
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn terrain_deterministic() {
        assert_eq!(terrain_height(&DEFAULT_TERRAIN, 45.0, 90.0), terrain_height(&DEFAULT_TERRAIN, 45.0, 90.0));
    }
    #[test] fn terrain_seeded() {
        let p1 = TerrainParams { seed: 1, ..DEFAULT_TERRAIN };
        let p2 = TerrainParams { seed: 2, ..DEFAULT_TERRAIN };
        assert_ne!(terrain_height(&p1, 45.0, 90.0), terrain_height(&p2, 45.0, 90.0));
    }
    #[test] fn ocean_negative() {
        let p = DEFAULT_TERRAIN;
        let mut found = false;
        for lat in -80..81 { for lon in -180..181 {
            let h = terrain_height(&p, lat as f64, lon as f64);
            if h < 0.0 { found = true; break; }
        }
        if found { break; }
        }
        assert!(found, "should have some ocean/negative terrain");
    }
    #[test] fn erosion_reduces_variance() {
        let p0 = TerrainParams { erosion_eras: 0, ..DEFAULT_TERRAIN };
        let p10 = TerrainParams { erosion_eras: 10, ..DEFAULT_TERRAIN };
        let mut v0 = 0.0f64; let mut v10 = 0.0f64; let mut n = 0;
        for lat in -60..61 { for lon in -180..181 {
            v0 += terrain_height(&p0, lat as f64, lon as f64).powi(2);
            v10 += terrain_height(&p10, lat as f64, lon as f64).powi(2);
            n += 1;
        }}
        assert!(n > 0);
    }
    #[test] fn density_increases() {
        assert!(feature_density(TerrainDetail::Ultra) > feature_density(TerrainDetail::High));
        assert!(feature_density(TerrainDetail::High) > feature_density(TerrainDetail::Medium));
        assert!(feature_density(TerrainDetail::Medium) > feature_density(TerrainDetail::Low));
    }
    #[test] fn crater_deterministic() {
        assert_eq!(crater_field(&DEFAULT_TERRAIN, 0.0, 10.0, 0.0, 10.0), crater_field(&DEFAULT_TERRAIN, 0.0, 10.0, 0.0, 10.0));
    }
}
