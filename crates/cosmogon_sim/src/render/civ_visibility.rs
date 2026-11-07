//! Civilizations and buildings visible from close range.

use cosmogon_core::dmath::DMath;
use serde::Serialize;
use crate::rng::Rng;
use crate::rng::domain;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CivVisibilityFeature { City, Town, Road, Port, Spaceport, IndustrialZone, Farmland, }

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CivVisibilityLevel { None, NightLights, SurfaceFeatures, IndividualBuildings, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CivVisibilityParams {
    pub level: CivVisibilityLevel,
    pub feature_type: CivVisibilityFeature,
    pub population: f64,
    pub night_light_brightness: f64,
}

pub const DEFAULT_CIV_VISIBILITY: CivVisibilityParams = CivVisibilityParams {
    level: CivVisibilityLevel::NightLights,
    feature_type: CivVisibilityFeature::City,
    population: 1_000_000.0,
    night_light_brightness: 0.7,
};

#[derive(Clone, Debug, PartialEq)]
pub struct City {
    pub name: String,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub population: f64,
    pub tier: CityTier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CityTier { Hamlet, Village, Town, City, Metropolis, Megacity, }

pub fn place_cities(seed: u64, population: f64, lat_min: f64, lat_max: f64, lon_min: f64, lon_max: f64) -> Vec<City> {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0xF00D]);
    let n = city_count(population);
    let mut cities = Vec::with_capacity(n);
    let mut remaining = population;
    for i in 0..n {
        let rank = (i + 1) as f64;
        let share = rank.powf(-1.5);
        let pop = remaining * share * 0.7;
        remaining -= pop;
        let tier = if pop > 10_000_000.0 { CityTier::Megacity }
        else if pop > 1_000_000.0 { CityTier::Metropolis }
        else if pop > 100_000.0 { CityTier::City }
        else if pop > 10_000.0 { CityTier::Town }
        else if pop > 1_000.0 { CityTier::Village }
        else { CityTier::Hamlet };
        cities.push(City { name: format!("City-{i:04}"), lat_deg: rng.range(lat_min, lat_max), lon_deg: rng.range(lon_min, lon_max), population: pop, tier });
    }
    cities
}

fn city_count(pop: f64) -> usize {
    if pop < 1_000.0 { return 1; }
    (pop.ln() * 2.0).clamp(1.0, 1000.0) as usize
}

pub fn city_light_intensity(pop: f64, tier: CityTier) -> f64 {
    let base = match tier { CityTier::Hamlet => 0.05, CityTier::Village => 0.15, CityTier::Town => 0.35, CityTier::City => 0.6, CityTier::Metropolis => 0.8, CityTier::Megacity => 1.0 };
    (base * (pop / 1_000_000.0_f64).dln1p().clamp(0.0f64, 1.0f64)).clamp(0.0f64, 1.0f64)
}

pub fn road_network(cities: &[City]) -> Vec<(usize, usize, f64)> {
    let mut roads = Vec::new();
    let n = cities.len();
    if n < 2 { return roads; }
    for i in 0..n {
        let mut d: Vec<(f64, usize)> = (0..n).filter(|&j| j != i).map(|j| -> (f64, usize) {
            let dlat = cities[i].lat_deg - cities[j].lat_deg;
            let mut dlon = (cities[i].lon_deg - cities[j].lon_deg).rem_euclid(360.0);
            if dlon > 180.0 { dlon -= 360.0; }
            let dist = (dlat * dlat + dlon * dlon).sqrt();
            (dist, j)
        }).collect();
        d.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for &(dist, j) in d.iter().take(3) { roads.push((i, j, dist)); }
    }
    roads
}

pub fn feature_count(l: CivVisibilityLevel) -> usize {
    match l { CivVisibilityLevel::None => 0, CivVisibilityLevel::NightLights => 5, CivVisibilityLevel::SurfaceFeatures => 20, CivVisibilityLevel::IndividualBuildings => 100 }
}

pub fn visible_city_count(pop: f64, l: CivVisibilityLevel) -> usize {
    let total = city_count(pop);
    match l {
        CivVisibilityLevel::None => 0,
        CivVisibilityLevel::NightLights => (total as f64 * 0.3).clamp(1.0, total as f64) as usize,
        CivVisibilityLevel::SurfaceFeatures => (total as f64 * 0.7).clamp(1.0, total as f64) as usize,
        CivVisibilityLevel::IndividualBuildings => total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn cities_deterministic() {
        assert_eq!(place_cities(42, 5_000_000.0, -60.0, 60.0, -100.0, 100.0), place_cities(42, 5_000_000.0, -60.0, 60.0, -100.0, 100.0));
    }
    #[test] fn more_pop_more_cities() {
        let s = place_cities(7, 100_000.0, -90.0, 90.0, -180.0, 180.0).len();
        let l = place_cities(7, 10_000_000.0, -90.0, 90.0, -180.0, 180.0).len();
        assert!(l > s);
    }
    #[test] fn has_megacity() {
        let c = place_cities(11, 50_000_000.0, -90.0, 90.0, -180.0, 180.0);
        assert!(c.iter().any(|x| x.tier == CityTier::Megacity));
    }
    #[test] fn road_network_nonempty() {
        let c = place_cities(13, 1_000_000.0, -60.0, 60.0, -100.0, 100.0);
        if c.len() >= 2 { assert!(!road_network(&c).is_empty()); }
    }
    #[test] fn light_intensity_range() {
        for pop in [1_000.0, 100_000.0, 1_000_000.0, 10_000_000.0] {
            for t in [CityTier::Hamlet, CityTier::Village, CityTier::Town, CityTier::City, CityTier::Metropolis, CityTier::Megacity] {
                assert!((0.0..=1.0).contains(&city_light_intensity(pop, t)));
            }
        }
    }
    #[test] fn visible_city_scales() {
        let s = visible_city_count(100_000.0, CivVisibilityLevel::SurfaceFeatures);
        let l = visible_city_count(10_000_000.0, CivVisibilityLevel::SurfaceFeatures);
        assert!(l > s);
    }
    #[test] fn fc_matches() {
        assert!(feature_count(CivVisibilityLevel::IndividualBuildings) > feature_count(CivVisibilityLevel::NightLights));
        assert_eq!(feature_count(CivVisibilityLevel::None), 0);
    }
}
