//! Night lights and city lights.

use serde::Serialize;
use crate::rng::Rng;
use crate::rng::domain;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NightLightFeature { CityLights, RoadLights, PortLights, IndustrialLights, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct NightLightParams {
    pub feature: NightLightFeature,
    pub brightness: f64,
    pub coverage: f64,
}

pub const DEFAULT_NIGHT_LIGHTS: NightLightParams = NightLightParams {
    feature: NightLightFeature::CityLights,
    brightness: 0.8,
    coverage: 0.5,
};

#[derive(Clone, Debug, PartialEq)]
pub struct LightSource {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub intensity: f64,
    pub color_temp_k: f64,
    pub feature: NightLightFeature,
}

pub fn generate_night_lights(seed: u64, num_lights: u32, lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64) -> Vec<LightSource> {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0xE6F7]);
    let mut lights = Vec::with_capacity(num_lights as usize);
    for _ in 0..num_lights {
        let lat = rng.range(lat_min, lat_max);
        let lon = rng.range(lon_min, lon_max);
        let mut intensity = rng.range(0.1, 1.0);
        if lat.abs() < 10.0 { intensity = (intensity + 0.2).clamp(0.0, 1.0); }
        lights.push(LightSource {
            lat_deg: lat, lon_deg: lon, intensity,
            color_temp_k: rng.range(2700.0, 6500.0),
            feature: match rng.range_u32(0, 4) { 0 => NightLightFeature::CityLights, 1 => NightLightFeature::RoadLights, 2 => NightLightFeature::PortLights, _ => NightLightFeature::IndustrialLights },
        });
    }
    lights
}

pub fn clustered_lights(seed: u64, num_clusters: u32, lights_per_cluster: u32, lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64) -> Vec<LightSource> {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0x1F2E]);
    let mut centers = Vec::new();
    for _ in 0..num_clusters {
        centers.push((rng.range(lat_min, lat_max), rng.range(lon_min, lon_max), rng.range(0.5, 1.0)));
    }
    let mut lights = Vec::new();
    for (clat, clon, cint) in centers {
        for _ in 0..lights_per_cluster {
            let lat = (clat + rng.range(-3.0, 3.0)).clamp(lat_min, lat_max);
            let lon = (clon + rng.range(-3.0, 3.0)).rem_euclid(360.0);
            let intensity = (cint + rng.range(-0.2, 0.2)).clamp(0.0, 1.0);
            lights.push(LightSource { lat_deg: lat, lon_deg: lon, intensity, color_temp_k: rng.range(2700.0, 5000.0), feature: NightLightFeature::CityLights });
        }
    }
    lights
}

pub fn total_light_output(lights: &[LightSource]) -> f64 {
    lights.iter().map(|l| l.intensity).sum()
}
pub fn avg_color_temperature(lights: &[LightSource]) -> f64 {
    if lights.is_empty() { return 4000.0; }
    lights.iter().map(|l| l.color_temp_k).sum::<f64>() / lights.len() as f64
}
pub fn light_density(lights: &[LightSource], lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64) -> f64 {
    let area = (lat_max - lat_min).abs() * (lon_max - lon_min).abs();
    if area == 0.0 { return 0.0; }
    lights.len() as f64 / area
}
pub fn brightness_at_distance(base: f64, distance_km: f64, scale: f64) -> f64 {
    base * (-distance_km / scale).exp()
}
pub fn visible_lights(lights: &[LightSource], min_intensity: f64) -> Vec<&LightSource> {
    lights.iter().filter(|l| l.intensity >= min_intensity).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn lights_deterministic() {
        assert_eq!(generate_night_lights(42, 50, -60.0, 60.0, -100.0, 100.0), generate_night_lights(42, 50, -60.0, 60.0, -100.0, 100.0));
    }
    #[test] fn clustered_has_right_count() {
        let lights = clustered_lights(7, 5, 10, -60.0, 60.0, -100.0, 100.0);
        assert_eq!(lights.len(), 50);
    }
    #[test] fn total_output_sums() {
        let lights = vec![
            LightSource { lat_deg: 0.0, lon_deg: 0.0, intensity: 0.5, color_temp_k: 3000.0, feature: NightLightFeature::CityLights },
            LightSource { lat_deg: 1.0, lon_deg: 1.0, intensity: 0.3, color_temp_k: 4000.0, feature: NightLightFeature::RoadLights },
        ];
        assert!((total_light_output(&lights) - 0.8).abs() < 0.001);
    }
    #[test] fn avg_temp_valid() {
        let lights = vec![
            LightSource { lat_deg: 0.0, lon_deg: 0.0, intensity: 1.0, color_temp_k: 3000.0, feature: NightLightFeature::CityLights },
            LightSource { lat_deg: 1.0, lon_deg: 1.0, intensity: 1.0, color_temp_k: 5000.0, feature: NightLightFeature::CityLights },
        ];
        assert!((avg_color_temperature(&lights) - 4000.0).abs() < 0.001);
        assert_eq!(avg_color_temperature(&[]), 4000.0);
    }
    #[test] fn density_decreases() {
        let lights = vec![LightSource { lat_deg: 0.0, lon_deg: 0.0, intensity: 1.0, color_temp_k: 3000.0, feature: NightLightFeature::CityLights }];
        assert!(light_density(&lights, -10.0, 10.0, -10.0, 10.0) > light_density(&lights, -90.0, 90.0, -180.0, 180.0));
    }
    #[test] fn brightness_falloff() {
        assert_eq!(brightness_at_distance(1.0, 0.0, 100.0), 1.0);
        assert!(brightness_at_distance(1.0, 100.0, 100.0) < 1.0);
    }
    #[test] fn visible_filters() {
        let lights = vec![
            LightSource { lat_deg: 0.0, lon_deg: 0.0, intensity: 0.9, color_temp_k: 3000.0, feature: NightLightFeature::CityLights },
            LightSource { lat_deg: 1.0, lon_deg: 1.0, intensity: 0.3, color_temp_k: 4000.0, feature: NightLightFeature::CityLights },
        ];
        assert_eq!(visible_lights(&lights, 0.5).len(), 1);
    }
    #[test] fn intensities_valid() {
        let lights = generate_night_lights(9, 100, -90.0, 90.0, -180.0, 180.0);
        for l in &lights { assert!((0.0..=1.0).contains(&l.intensity)); }
    }
}
