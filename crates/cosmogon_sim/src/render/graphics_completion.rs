//! Graphics supplement completion.

use serde::Serialize;


#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct GraphicsCompletion {
    pub builds_completed: u32,
    pub builds_total: u32,
    pub rendering_architecture: bool,
    pub stellar_activity: bool,
    pub terrain_detail: bool,
    pub weather: bool,
    pub geology: bool,
    pub civ_visibility: bool,
    pub small_bodies: bool,
    pub graphics_performance: bool,
    pub render_pipeline: bool,
    pub close_up_camera: bool,
    pub surface_detail: bool,
    pub atmospheric_scattering: bool,
    pub night_lights: bool,
}

pub const DEFAULT_COMPLETION: GraphicsCompletion = GraphicsCompletion {
    builds_completed: 7,
    builds_total: 7,
    rendering_architecture: true,
    stellar_activity: true,
    terrain_detail: true,
    weather: true,
    geology: true,
    civ_visibility: true,
    small_bodies: true,
    graphics_performance: false,
    render_pipeline: false,
    close_up_camera: false,
    surface_detail: false,
    atmospheric_scattering: false,
    night_lights: false,
};

pub fn integration_check(
    lod: &crate::render::lod_budget::AdaptiveQuality,
    _terrain: &crate::render::surface_detail::SurfaceDetailParams,
    camera: &crate::render::close_up_camera::CameraController,
    weather: &crate::render::weather::WeatherGrid,
) -> IntegrationResult {
    let mut issues = Vec::new();
    if lod.downgrade_level > 2 {
        issues.push("LOD downgrade too aggressive".to_string());
    }
    if camera.params.distance_km < camera.min_distance_km {
        issues.push("Camera distance below minimum".to_string());
    }
    if weather.lat_cells == 0 || weather.lon_cells == 0 {
        issues.push("Weather grid has zero dimensions".to_string());
    }
    IntegrationResult { passed: issues.is_empty(), issues }
}

#[derive(Clone, Debug)]
pub struct IntegrationResult {
    pub passed: bool,
    pub issues: Vec<String>,
}

pub fn completion_percentage(completion: &GraphicsCompletion) -> f64 {
    let total = completion.builds_total as f64;
    if total == 0.0 { return 0.0; }
    completion.builds_completed as f64 / total * 100.0
}

pub fn is_critical_complete(completion: &GraphicsCompletion) -> bool {
    completion.rendering_architecture
        && completion.stellar_activity
        && completion.terrain_detail
        && completion.civ_visibility
}

pub fn missing_modules(completion: &GraphicsCompletion) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !completion.rendering_architecture { missing.push("rendering_architecture"); }
    if !completion.stellar_activity { missing.push("stellar_activity"); }
    if !completion.terrain_detail { missing.push("terrain_detail"); }
    if !completion.weather { missing.push("weather"); }
    if !completion.geology { missing.push("geology"); }
    if !completion.civ_visibility { missing.push("civ_visibility"); }
    if !completion.small_bodies { missing.push("small_bodies"); }
    if !completion.graphics_performance { missing.push("graphics_performance"); }
    if !completion.render_pipeline { missing.push("render_pipeline"); }
    if !completion.close_up_camera { missing.push("close_up_camera"); }
    if !completion.surface_detail { missing.push("surface_detail"); }
    if !completion.atmospheric_scattering { missing.push("atmospheric_scattering"); }
    if !completion.night_lights { missing.push("night_lights"); }
    missing
}

#[cfg(test)]
mod tests {
    use super::super::surface_detail::DEFAULT_SURFACE_DETAIL;
    use super::*;
    #[test] fn default_completion_has_7() {
        assert_eq!(DEFAULT_COMPLETION.builds_completed, 7);
        assert_eq!(DEFAULT_COMPLETION.builds_total, 7);
    }
    #[test] fn completion_percentage() {
        let c = GraphicsCompletion { builds_completed: 5, builds_total: 10, ..DEFAULT_COMPLETION };
        assert!((super::completion_percentage(&c) - 50.0).abs() < 0.001);
    }
    #[test] fn completion_zero() {
        let c = GraphicsCompletion { builds_completed: 0, builds_total: 10, ..DEFAULT_COMPLETION };
        assert_eq!(super::completion_percentage(&c), 0.0);
    }
    #[test] fn completion_100() {
        let c = GraphicsCompletion {
            builds_completed: 14, builds_total: 14,
            rendering_architecture: true, stellar_activity: true, terrain_detail: true,
            weather: true, geology: true, civ_visibility: true, small_bodies: true,
            graphics_performance: true, render_pipeline: true, close_up_camera: true,
            surface_detail: true, atmospheric_scattering: true, night_lights: true,
        };
        assert!((super::completion_percentage(&c) - 100.0).abs() < 0.001);
    }
    #[test] fn integration_passes() {
        let lod = crate::render::lod_budget::AdaptiveQuality::default();
        let terrain = crate::render::surface_detail::SurfaceDetailParams { level: crate::render::surface_detail::SurfaceDetailLevel::Detailed, ..DEFAULT_SURFACE_DETAIL };
        let camera = crate::render::close_up_camera::CameraController::new(crate::render::close_up_camera::DEFAULT_CAMERA);
        let weather = crate::render::weather::WeatherGrid::new(10, 20, 42);
        let r = integration_check(&lod, &terrain, &camera, &weather);
        assert!(r.passed, "{:?}", r.issues);
    }
    #[test] fn integration_fails_bad_camera() {
        let mut params = crate::render::close_up_camera::DEFAULT_CAMERA;
        params.distance_km = 1.0;
        let lod = crate::render::lod_budget::AdaptiveQuality::default();
        let terrain = crate::render::surface_detail::SurfaceDetailParams { level: crate::render::surface_detail::SurfaceDetailLevel::Detailed, ..DEFAULT_SURFACE_DETAIL };
        let camera = crate::render::close_up_camera::CameraController::new(params);
        let weather = crate::render::weather::WeatherGrid::new(10, 20, 42);
        let r = integration_check(&lod, &terrain, &camera, &weather);
        assert!(!r.passed);
    }
    #[test] fn missing_all() {
        let c = GraphicsCompletion { builds_completed: 0, builds_total: 14, ..Default::default() };
        assert_eq!(missing_modules(&c).len(), 13);
    }
    #[test] fn critical_complete() {
        let mut c = DEFAULT_COMPLETION;
        assert!(is_critical_complete(&c));
        c.rendering_architecture = false;
        assert!(!is_critical_complete(&c));
    }
    #[test] fn default_critical() {
        assert!(is_critical_complete(&DEFAULT_COMPLETION));
    }
}
