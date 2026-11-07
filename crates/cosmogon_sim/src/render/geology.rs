//! Geological activity: volcanism, earthquakes and visible consequences.

use serde::Serialize;
use crate::rng::Rng;
use crate::rng::domain;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GeoFeature { Volcano, Earthquake, Rift, Mountain, Fault, }

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GeoActivity { Dormant, Low, Moderate, High, Extreme, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GeoParams {
    pub feature_type: GeoFeature,
    pub activity: GeoActivity,
    pub magnitude: f64,
    pub depth_km: f64,
    pub duration_s: f64,
}

pub const DEFAULT_GEO: GeoParams = GeoParams {
    feature_type: GeoFeature::Volcano,
    activity: GeoActivity::Low,
    magnitude: 3.0,
    depth_km: 10.0,
    duration_s: 3600.0,
};

#[derive(Clone, Debug, PartialEq)]
pub struct TectonicPlate {
    pub id: u32,
    pub center_lat: f64,
    pub center_lon: f64,
    pub radius_deg: f64,
    pub velocity_u: f64,
    pub velocity_v: f64,
    pub plate_type: PlateType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlateType { Oceanic, Continental, }

pub fn generate_plates(seed: u64, lat_cells: usize, lon_cells: usize) -> Vec<TectonicPlate> {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[0x8E12]);
    let n = match lat_cells * lon_cells {
        n if n < 100 => rng.range_u32(3, 7),
        n if n < 1000 => rng.range_u32(7, 15),
        _ => rng.range_u32(15, 25),
    };
    let mut p = Vec::with_capacity(n as usize);
    for i in 0..n {
        p.push(TectonicPlate {
            id: i,
            center_lat: rng.range(-70.0, 70.0),
            center_lon: rng.range(0.0, 360.0),
            radius_deg: rng.range(15.0, 60.0),
            velocity_u: rng.range(-2.0, 2.0),
            velocity_v: rng.range(-2.0, 2.0),
            plate_type: if rng.chance(0.6) { PlateType::Continental } else { PlateType::Oceanic },
        });
    }
    p
}

pub fn plate_at(plates: &[TectonicPlate], lat: f64, lon: f64) -> Option<&TectonicPlate> {
    plates.iter().filter(|p| {
        let dlat = lat - p.center_lat;
        let mut dlon = (lon - p.center_lon).rem_euclid(360.0);
        if dlon > 180.0 { dlon -= 360.0; }
        (dlat * dlat + dlon * dlon) < p.radius_deg * p.radius_deg
    }).min_by_key(|p| {
        let dlat = (lat - p.center_lat).abs() as u64;
        let dlon = ((lon - p.center_lon).rem_euclid(360.0) as i64).abs() as u64;
        dlat + dlon
    })
}

pub fn geological_activity(seed: u64, lat: f64, lon: f64, plates: &[TectonicPlate]) -> GeoActivity {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[(lat * 1000.0) as u64, (lon * 1000.0) as u64, 4]);
    let mut near = false;
    for p in plates {
        let dlat = (lat - p.center_lat).abs();
        let mut dlon = (lon - p.center_lon).rem_euclid(360.0);
        if dlon > 180.0 { dlon -= 360.0; }
        let dist = (dlat * dlat + dlon * dlon).sqrt();
        if dist > p.radius_deg * 0.7 && dist < p.radius_deg * 1.3 { near = true; break; }
    }
    let score = (if near { 0.6 } else { 0.1 } + rng.range(-0.2, 0.2)).clamp(0.0, 1.0);
    if score > 0.8 { GeoActivity::Extreme }
    else if score > 0.6 { GeoActivity::High }
    else if score > 0.35 { GeoActivity::Moderate }
    else if score > 0.15 { GeoActivity::Low }
    else { GeoActivity::Dormant }
}

pub fn generate_event(seed: u64, lat: f64, lon: f64, a: GeoActivity) -> GeoParams {
    let mut rng = Rng::stream(seed, domain::PLANETS, &[(lat * 1000.0) as u64, (lon * 1000.0) as u64, 5]);
    let (ft, mag, depth, dur) = match a {
        GeoActivity::Dormant => (GeoFeature::Mountain, rng.range(0.0, 1.0), rng.range(5.0, 15.0), rng.range(3600.0, 86400.0)),
        GeoActivity::Low => (match rng.range_u32(0, 4) { 0 => GeoFeature::Volcano, 1 => GeoFeature::Earthquake, 2 => GeoFeature::Rift, _ => GeoFeature::Fault }, rng.range(1.0, 3.0), rng.range(10.0, 50.0), rng.range(3600.0, 172800.0)),
        GeoActivity::Moderate => (match rng.range_u32(0, 4) { 0 => GeoFeature::Volcano, 1 => GeoFeature::Earthquake, 2 => GeoFeature::Mountain, _ => GeoFeature::Fault }, rng.range(3.0, 6.0), rng.range(5.0, 30.0), rng.range(86400.0, 604800.0)),
        GeoActivity::High => (match rng.range_u32(0, 5) { 0 => GeoFeature::Volcano, 1 => GeoFeature::Earthquake, 2 => GeoFeature::Rift, 3 => GeoFeature::Mountain, _ => GeoFeature::Fault }, rng.range(5.0, 8.0), rng.range(2.0, 15.0), rng.range(604800.0, 2592000.0)),
        GeoActivity::Extreme => (match rng.range_u32(0, 4) { 0 => GeoFeature::Volcano, 1 => GeoFeature::Earthquake, 2 => GeoFeature::Rift, _ => GeoFeature::Mountain }, rng.range(7.0, 10.0), rng.range(0.0, 10.0), rng.range(2592000.0, 7776000.0)),
    };
    GeoParams { feature_type: ft, activity: a, magnitude: mag, depth_km: depth, duration_s: dur }
}

pub fn feature_count(a: GeoActivity) -> usize {
    match a { GeoActivity::Dormant => 0, GeoActivity::Low => 1, GeoActivity::Moderate => 3, GeoActivity::High => 10, GeoActivity::Extreme => 30 }
}

pub fn richter_energy(m: f64) -> f64 {
    10.0f64.powf(4.8 + 1.5 * m)
}

pub const VEI_VOLUME_KM3: [f64; 9] = [
    0.000_001, 0.000_01, 0.000_1, 0.001, 0.01, 0.1, 1.0, 10.0, 100.0,
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn plates_deterministic() {
        assert_eq!(generate_plates(42, 20, 40), generate_plates(42, 20, 40));
    }
    #[test] fn plate_at_center() {
        let p = generate_plates(7, 30, 60);
        assert!(plate_at(&p, p[0].center_lat, p[0].center_lon).is_some());
    }
    #[test] fn activity_deterministic() {
        let p = generate_plates(3, 20, 40);
        assert_eq!(geological_activity(5, 10.0, 20.0, &p), geological_activity(5, 10.0, 20.0, &p));
    }
    #[test] fn event_matches_activity() {
        assert_eq!(generate_event(9, 0.0, 0.0, GeoActivity::Dormant).activity, GeoActivity::Dormant);
        assert_eq!(generate_event(9, 0.0, 0.0, GeoActivity::Extreme).activity, GeoActivity::Extreme);
    }
    #[test] fn richter_increases() {
        assert!(richter_energy(5.0) > richter_energy(4.0));
    }
    #[test] fn vei_monotonic() {
        for i in 1..VEI_VOLUME_KM3.len() { assert!(VEI_VOLUME_KM3[i] > VEI_VOLUME_KM3[i-1]); }
    }
    #[test] fn fc_matches() {
        assert!(feature_count(GeoActivity::Extreme) > feature_count(GeoActivity::Low));
        assert_eq!(feature_count(GeoActivity::Dormant), 0);
    }
}
