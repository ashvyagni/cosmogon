//! Atmospheric scattering.

use serde::Serialize;
use crate::Vec3d;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScatteringModel { Rayleigh, Mie, Combined, }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ScatteringParams {
    pub model: ScatteringModel,
    pub rayleigh_scale_height_km: f64,
    pub mie_scale_height_km: f64,
    pub ground_albedo: f64,
}

pub const DEFAULT_SCATTERING: ScatteringParams = ScatteringParams {
    model: ScatteringModel::Combined,
    rayleigh_scale_height_km: 8.0,
    mie_scale_height_km: 1.2,
    ground_albedo: 0.3,
};

const WAVELENGTH_NM: [f64; 3] = [680.0, 550.0, 440.0];

fn rayleigh_coeff(lambda_nm: f64) -> f64 {
    let l = lambda_nm / 550.0;
    1.0 / l.powf(4.0)
}
fn mie_coeff(lambda_nm: f64) -> f64 {
    let l = lambda_nm / 550.0;
    1.0 / l.powf(0.75)
}
fn density(h_km: f64, scale: f64) -> f64 {
    (-h_km / scale).exp()
}

pub fn sky_color(params: &ScatteringParams, sun_dir: Vec3d, view_dir: Vec3d, camera_alt_km: f64) -> [f64; 3] {
    let mu = sun_dir.dot(view_dir).clamp(-1.0, 1.0);
    let phase_r = 0.75 * (1.0 + mu * mu);
    let phase_m = 0.75 * (1.0 - mu * mu) / (1.0 + mu * mu + 1e-6).powf(1.5);
    let mut color = [0.0f64; 3];
    for (i, &lambda) in WAVELENGTH_NM.iter().enumerate() {
        let coeff;
        match params.model {
            ScatteringModel::Rayleigh => coeff = rayleigh_coeff(lambda),
            ScatteringModel::Mie => coeff = mie_coeff(lambda),
            ScatteringModel::Combined => coeff = rayleigh_coeff(lambda) * 0.6 + mie_coeff(lambda) * 0.4,
        }
        let pd = density(camera_alt_km, params.rayleigh_scale_height_km) * params.rayleigh_scale_height_km;
        let md = density(camera_alt_km, params.mie_scale_height_km) * params.mie_scale_height_km;
        let att = (-pd * 0.001).exp();
        let scattered = coeff * pd * phase_r * 0.1;
        let mie_s = mie_coeff(lambda) * md * phase_m * 0.05;
        color[i] = (scattered + mie_s) * att + params.ground_albedo * 0.1;
    }
    let max = color.iter().fold(0.0f64, |a, &b| a.max(b));
    if max > 0.0 {
        color.iter_mut().for_each(|c| *c = (*c / max).min(1.0));
    } else {
        color = [0.1, 0.1, 0.2];
    }
    color
}

pub fn zenith_color(params: &ScatteringParams, sun_elevation_deg: f64, camera_alt_km: f64) -> [f64; 3] {
    let sun = Vec3d::new(sun_elevation_deg.to_radians().cos(), 0.0, sun_elevation_deg.to_radians().sin()).normalize();
    sky_color(params, sun, Vec3d::new(0.0, 0.0, 1.0), camera_alt_km)
}

pub fn horizon_color(params: &ScatteringParams, sun_elevation_deg: f64, camera_alt_km: f64) -> [f64; 3] {
    let sun = Vec3d::new(sun_elevation_deg.to_radians().cos(), 0.0, sun_elevation_deg.to_radians().sin()).normalize();
    sky_color(params, sun, Vec3d::new(1.0, 0.0, 0.0), camera_alt_km)
}

pub fn sunset_color(params: &ScatteringParams, camera_alt_km: f64) -> [f64; 3] {
    zenith_color(params, -5.0, camera_alt_km)
}

pub fn extinction(params: &ScatteringParams, path_length_km: f64, wavelength_nm: f64) -> f64 {
    let coeff = match params.model {
        ScatteringModel::Rayleigh => rayleigh_coeff(wavelength_nm),
        ScatteringModel::Mie => mie_coeff(wavelength_nm),
        ScatteringModel::Combined => rayleigh_coeff(wavelength_nm) * 0.6 + mie_coeff(wavelength_nm) * 0.4,
    };
    (-coeff * path_length_km * 0.0005).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn scattering_deterministic() {
        let p = DEFAULT_SCATTERING;
        let sun = Vec3d::new(1.0, 0.0, 0.5).normalize();
        let view = Vec3d::new(0.0, 1.0, 0.0).normalize();
        assert_eq!(sky_color(&p, sun, view, 0.0), sky_color(&p, sun, view, 0.0));
    }
    #[test] fn components_in_range() {
        let p = DEFAULT_SCATTERING;
        let sun = Vec3d::new(1.0, 0.0, 0.5).normalize();
        let view = Vec3d::new(0.0, 1.0, 0.0).normalize();
        let c = sky_color(&p, sun, view, 0.0);
        for &v in &c { assert!((0.0..=1.0).contains(&v)); }
    }
    #[test] fn models_differ() {
        let sun = Vec3d::new(1.0, 0.0, 0.5).normalize();
        let view = Vec3d::new(0.0, 1.0, 0.0).normalize();
        let r = sky_color(&ScatteringParams { model: ScatteringModel::Rayleigh, ..DEFAULT_SCATTERING }, sun, view, 0.0);
        let m = sky_color(&ScatteringParams { model: ScatteringModel::Mie, ..DEFAULT_SCATTERING }, sun, view, 0.0);
        let c = sky_color(&ScatteringParams { model: ScatteringModel::Combined, ..DEFAULT_SCATTERING }, sun, view, 0.0);
        assert!(r.iter().zip(m.iter()).zip(c.iter()).any(|((a, b), c)| (a - b).abs() > 0.01 || (a - c).abs() > 0.01));
    }
    #[test] fn extinction_decreases_with_wavelength() {
        let p = DEFAULT_SCATTERING;
        assert!(extinction(&p, 100.0, 440.0) < extinction(&p, 100.0, 680.0));
    }
    #[test] fn sunset_redder() {
        let p = DEFAULT_SCATTERING;
        let noon = zenith_color(&p, 45.0, 0.0);
        let sunset = sunset_color(&p, 0.0);
        assert!(sunset[0] / (sunset[2] + 0.01) >= noon[0] / (noon[2] + 0.01));
    }
    #[test] fn extinction_valid() {
        let p = DEFAULT_SCATTERING;
        let e = extinction(&p, 100.0, 550.0);
        assert!(e > 0.0 && e <= 1.0);
    }
}
