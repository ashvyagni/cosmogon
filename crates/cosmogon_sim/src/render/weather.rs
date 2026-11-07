//! Atmospheres and weather: clouds, storms, hurricanes where applicable.

use serde::Serialize;
use crate::rng::Rng;
use crate::rng::domain;
use cosmogon_core::dmath::DMath;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeatherFeature { Cloud, Storm, Hurricane, Front, Fog, }

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeatherIntensity { Calm, Breezy, Stormy, Severe, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct WeatherParams {
    pub feature_type: WeatherFeature,
    pub intensity: WeatherIntensity,
    pub coverage_frac: f64,
    pub wind_speed_m_s: f64,
    pub pressure_hpa: f64,
}

pub const DEFAULT_WEATHER: WeatherParams = WeatherParams {
    feature_type: WeatherFeature::Cloud,
    intensity: WeatherIntensity::Calm,
    coverage_frac: 0.3,
    wind_speed_m_s: 5.0,
    pressure_hpa: 1013.25,
};

#[derive(Clone, Debug, PartialEq)]
pub struct WeatherCell {
    pub temperature: f64,
    pub pressure: f64,
    pub humidity: f64,
    pub wind_u: f64,
    pub wind_v: f64,
    pub cloud_cover: f64,
    pub precipitation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeatherGrid {
    pub lat_cells: usize,
    pub lon_cells: usize,
    pub cells: Vec<WeatherCell>,
}

impl WeatherGrid {
    pub fn new(lat_cells: usize, lon_cells: usize, seed: u64) -> Self {
        let mut rng = Rng::stream(seed, domain::PLANETS, &[0x2A3F]);
        let mut cells = Vec::with_capacity(lat_cells * lon_cells);
        for li in 0..lat_cells {
            let lat = (li as f64 / lat_cells as f64) * 180.0 - 90.0;
            let base_temp = 288.0 - 40.0 * (lat / 90.0).abs().powf(1.2);
            for _ in 0..lon_cells {
                cells.push(WeatherCell {
                    temperature: base_temp + rng.range(-5.0, 5.0),
                    pressure: 1013.25 + rng.range(-30.0, 30.0),
                    humidity: rng.range(0.2, 0.9),
                    wind_u: rng.range(-10.0, 10.0),
                    wind_v: rng.range(-10.0, 10.0),
                    cloud_cover: rng.range(0.0, 0.6),
                    precipitation: 0.0,
                });
            }
        }
        Self { lat_cells, lon_cells, cells }
    }
    pub fn step(&mut self, dt: f64, seed: u64) {
        let mut rng = Rng::stream(seed, domain::PLANETS, &[0x4B21]);
        let new: Vec<WeatherCell> = self.cells.iter().enumerate().map(|(i, c)| {
            let li = i / self.lon_cells;
            let lat = (li as f64 / self.lat_cells as f64) * 180.0 - 90.0;
            let mut nc = c.clone();
            nc.wind_u += 2.0 * 7.29e-5 * lat.to_radians().dsin() * c.wind_v * dt * 0.001;
            nc.wind_v -= 2.0 * 7.29e-5 * lat.to_radians().dsin() * c.wind_u * dt * 0.001;
            let eq = 288.0 - 40.0 * (lat / 90.0).abs().powf(1.2);
            nc.temperature += (eq - c.temperature) * dt * 0.0001;
            nc.humidity = (c.humidity + rng.range(-0.05, 0.05)).clamp(0.0, 1.0);
            nc.cloud_cover = (c.humidity * (c.temperature / 288.0).powf(2.0).clamp(0.1, 1.0)).clamp(0.0, 1.0);
            let uplift = c.wind_u.abs() + c.wind_v.abs();
            nc.precipitation = if c.humidity > 0.85 && uplift > 5.0 { (c.humidity - 0.85) * uplift * 0.1 * dt } else { 0.0 };
            nc.wind_u *= 0.99;
            nc.wind_v *= 0.99;
            nc
        }).collect();
        self.cells = new;
    }
    pub fn cell_at(&self, lat: f64, lon: f64) -> &WeatherCell {
        let li = ((lat + 90.0) / 180.0 * self.lat_cells as f64).clamp(0.0, self.lat_cells as f64 - 1.0) as usize;
        let lo = (lon.rem_euclid(360.0) / 360.0 * self.lon_cells as f64).clamp(0.0, self.lon_cells as f64 - 1.0) as usize;
        &self.cells[li * self.lon_cells + lo]
    }
}

pub fn feature_count(i: WeatherIntensity) -> usize {
    match i { WeatherIntensity::Calm => 2, WeatherIntensity::Breezy => 5, WeatherIntensity::Stormy => 15, WeatherIntensity::Severe => 40 }
}

pub fn generate_weather_system(seed: u64, lat: f64, lon: f64, i: WeatherIntensity) -> WeatherParams {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[(lat * 1000.0) as u64, (lon * 1000.0) as u64, 3]);
    let (w, p, c) = match i {
        WeatherIntensity::Calm => (rng.range(0.0, 5.0), 1013.25, rng.range(0.1, 0.4)),
        WeatherIntensity::Breezy => (rng.range(5.0, 15.0), 1010.0, rng.range(0.3, 0.6)),
        WeatherIntensity::Stormy => (rng.range(15.0, 35.0), 1000.0, rng.range(0.6, 0.9)),
        WeatherIntensity::Severe => (rng.range(35.0, 70.0), 980.0, rng.range(0.85, 1.0)),
    };
    let ft = match i {
        WeatherIntensity::Calm => WeatherFeature::Cloud,
        WeatherIntensity::Breezy => if rng.chance(0.4) { WeatherFeature::Storm } else { WeatherFeature::Cloud },
        WeatherIntensity::Stormy => WeatherFeature::Storm,
        WeatherIntensity::Severe => WeatherFeature::Hurricane,
    };
    WeatherParams { feature_type: ft, intensity: i, coverage_frac: c, wind_speed_m_s: w, pressure_hpa: p }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn grid_deterministic() {
        assert_eq!(WeatherGrid::new(10, 20, 42).cells, WeatherGrid::new(10, 20, 42).cells);
    }
    #[test] fn grid_step_changes() {
        let mut g = WeatherGrid::new(8, 16, 7);
        let t0 = g.cells[0].temperature;
        g.step(3600.0, 7);
        assert_ne!(t0, g.cells[0].temperature);
    }
    #[test] fn cell_at_valid() {
        let g = WeatherGrid::new(10, 20, 3);
        let c = g.cell_at(0.0, 0.0);
        assert!((200.0..400.0).contains(&c.temperature));
    }
    #[test] fn weather_system_deterministic() {
        assert_eq!(generate_weather_system(5, 45.0, 90.0, WeatherIntensity::Stormy), generate_weather_system(5, 45.0, 90.0, WeatherIntensity::Stormy));
    }
    #[test] fn severe_high_wind() {
        let w = generate_weather_system(99, 10.0, 20.0, WeatherIntensity::Severe);
        assert!(w.wind_speed_m_s > 35.0);
    }
    #[test] fn fc_matches() {
        assert!(feature_count(WeatherIntensity::Severe) > feature_count(WeatherIntensity::Calm));
    }
}
