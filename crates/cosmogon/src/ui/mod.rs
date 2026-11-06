//! User interface (egui). The UI only *reads* simulation state and issues commands
//! (select, focus, speed, save…); it never mutates the simulation model directly.

pub mod alerts;
mod charts;
mod dock;
pub mod home;
mod hud;
mod inspect;
mod markers;
mod performance;
mod research;
mod research_chart;
mod surface_study;
mod habitat;
pub(crate) mod vehicle;
pub(crate) mod vehicle_cargo;
pub(crate) mod world_vehicle;
pub use vehicle::VehicleUi;
mod habitat_oxygen;
mod habitat_water;
mod habitat_humidity;
mod habitat_breach;
mod habitat_maintenance;
mod habitat_calibration;
mod habitat_radiation;
mod habitat_settlement;
mod habitat_crop;
mod habitat_why;
pub(crate) mod habitat_recordings;
pub(crate) mod habitat_pressure;
mod habitat_replay;
mod habitat_figure;
mod habitat_balance;
pub(crate) mod habitat_graph;
mod greenhouse;
mod greenhouse_supplies;
mod crop_chemistry;
pub use habitat::HabitatUi;
pub(crate) use surface_study::Study;
mod navigation;
pub(crate) mod cinematic;
mod input;
mod tools;
pub mod tour;
pub mod units;

pub use home::MenuScreen;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::sim::Sim;
use crate::state::AppState;

pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(232, 176, 75);
pub const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(150, 112, 50);
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(214, 220, 232);
pub const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 140, 160);
pub const LIFE: egui::Color32 = egui::Color32::from_rgb(110, 210, 140);
pub const CIV: egui::Color32 = egui::Color32::from_rgb(255, 196, 92);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(235, 100, 90);
pub const PANEL: egui::Color32 = egui::Color32::from_rgb(9, 12, 20);

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftTab {
    #[default]
    Systems,
    Chronicle,
    Civilizations,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyTab {
    #[default]
    Overview,
    Orbit,
    Physics,
    Environment,
    Surface,
    Life,
    Civilization,
    History,
    Data,
}

#[derive(Resource)]
pub struct UiState {
    pub show_left: bool,
    pub show_right: bool,
    pub hidden: bool,
    pub debug: bool,
    pub performance_open: bool,
    pub research_requested: Option<usize>,
    pub alerts_open: bool,
    pub alerts_tab: usize,
    pub notification_query: String,
    pub notice_severity: crate::notifications::Severity,
    pub notice_category: Option<cosmogon_sim::history::Category>,
    pub notice_unread: bool,
    pub camera_open: bool,
    pub camera_tab: usize,
    pub viewpoint_name: String,
    pub remove_viewpoint: Option<usize>,
    pub help: bool,
    pub pause_menu: bool,
    pub left_tab: LeftTab,
    pub body_tab: BodyTab,
    pub surface_study_open: bool,
    pub habitat_open: bool,
    pub vehicle_open: bool,
    pub habitat_requested: Option<u64>,
    pub surface_latitude: f64,
    pub surface_longitude: f64,
    pub surface_site: Option<String>,
    pub surface_altitude: f64,
    pub surface_tracked: bool,
    pub surface_horizon: bool,
    pub surface_export_path: String,
    pub surface_export_status: Option<String>,
    pub search: String,
    pub chronicle_min_importance: u8,
    /// Eight category toggles indexed by `Category::index()`; all on by default.
    pub chronicle_categories: [bool; 8],
    /// Case-insensitive substring search over title + detail (build 62).
    pub chronicle_search: String,
    /// Scope the Timeline list to `sim.selected` (build 62).
    pub chronicle_scope: bool,
    /// false = Timeline (newest first), true = Legends (era-grouped narrative).
    pub chronicle_mode: bool,
    pub chronicle_export_path: String,
    pub chronicle_export_status: Option<String>,
    pub menu: MenuScreen,
    pub form: home::NewForm,
    pub saves: Vec<crate::persistence::SaveEntry>,
    pub sandboxes: Vec<crate::persistence::SandboxEntry>,
    pub rename: Option<(String, String)>,
    pub confirm_delete: Option<String>,
    pub open_checkpoints: Option<String>,
    pub intro_started: Option<f64>,
    pub intro_skipped: bool,
    pub tech_filter_blocked: bool,
    pub share_domain: u8,
    /// Sandbox tool windows.
    pub create_open: bool,
    pub create: tools::CreateForm,
    pub physics_open: bool,
    pub palette_open: bool,
    pub palette_query: String,
    pub palette_index: usize,
    pub clone_dialog: Option<String>,
    pub save_as_dialog: Option<String>,
    pub checkpoint_label: String,
    pub impulse: [f64; 3],
    /// Set by the photo button / P; forwarded to `capture::Photo`.
    pub photo_requested: bool,
    /// A photo is being taken: draw nothing but the universe.
    pub photo_mode: bool,
}

impl UiState {
    pub fn show_surface_anchor(&mut self, a: &crate::surface::SurfaceAnchor) {
        self.surface_site=a.site_id.clone();self.surface_latitude=a.latitude_deg;self.surface_longitude=a.longitude_east_deg;
        self.surface_altitude=a.altitude_m;self.surface_tracked=a.tracked;
        let radial=bevy::math::DVec3::from_array(cosmogon_sim::planet::terrain::dir_from_lat_lon(a.latitude_deg.to_radians(),a.longitude_east_deg.to_radians()));
        let forward=bevy::math::DQuat::from_array(a.local_orientation_xyzw)*bevy::math::DVec3::NEG_Z;
        self.surface_horizon=forward.dot(-radial)<0.7;
        self.surface_export_status=None;self.body_tab=BodyTab::Surface;self.show_right=true;
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_left: true,
            show_right: true,
            hidden: false,
            debug: false,
            performance_open: false,
            research_requested: None,
            alerts_open: false,
            alerts_tab: 0,
            notification_query: String::new(),
            notice_severity: crate::notifications::Severity::Information,
            notice_category: None,
            notice_unread: false,
            camera_open: false,
            camera_tab: 0,
            viewpoint_name: String::new(),
            remove_viewpoint: None,
            help: false,
            pause_menu: false,
            left_tab: LeftTab::Systems,
            body_tab: BodyTab::Overview,
            surface_study_open: false,
            habitat_open: false,
            vehicle_open: false,
            habitat_requested: None,
            surface_latitude: 18.65,
            surface_longitude: 226.2,
            surface_site: None,
            surface_altitude: 100_000.0,
            surface_tracked: true,
            surface_horizon: false,
            surface_export_path: String::new(),
            surface_export_status: None,
            search: String::new(),
            chronicle_min_importance: 4,
            chronicle_categories: [true; 8],
            chronicle_search: String::new(),
            chronicle_scope: false,
            chronicle_mode: false,
            chronicle_export_path: String::new(),
            chronicle_export_status: None,
            menu: MenuScreen::Home,
            form: home::NewForm::default(),
            saves: Vec::new(),
            sandboxes: Vec::new(),
            rename: None,
            confirm_delete: None,
            open_checkpoints: None,
            intro_started: None,
            intro_skipped: false,
            tech_filter_blocked: false,
            share_domain: 0,
            create_open: false,
            create: tools::CreateForm::default(),
            physics_open: false,
            palette_open: false,
            palette_query: String::new(),
            palette_index: 0,
            clone_dialog: None,
            save_as_dialog: None,
            checkpoint_label: String::new(),
            impulse: [0.0; 3],
            photo_requested: false,
            photo_mode: false,
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .init_resource::<surface_study::Study>()
            .init_resource::<habitat::HabitatUi>()
            .init_resource::<VehicleUi>()
            .add_systems(Startup, (init_ui_state,habitat::init,vehicle::init))
            .add_systems(EguiPrimaryContextPass, apply_theme)
            .add_systems(EguiPrimaryContextPass, (home::main_menu).after(apply_theme).run_if(in_state(AppState::MainMenu)))
            .add_systems(EguiPrimaryContextPass, (home::generating).after(apply_theme).run_if(in_state(AppState::Generating)))
            .add_systems(
                EguiPrimaryContextPass,
                (markers::draw_markers, hud::keyboard, hud::top_bar, hud::bottom_bar, crate::timeline::timeline_strip, dock::tool_dock, inspect::left_panel, inspect::right_panel, tools::tool_windows, hud::toasts, hud::overlays_windows, performance::panel, navigation::panel, alerts::panel, research::panel, tour::tour_card)
                    .chain()
                    .after(apply_theme)
                    .run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)),
            )
            .add_systems(EguiPrimaryContextPass,surface_study::panel.after(research::panel).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)))
            .add_systems(EguiPrimaryContextPass,habitat::panel.after(surface_study::panel).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)))
            .add_systems(EguiPrimaryContextPass,vehicle::panel.after(habitat::panel).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)))
            .add_plugins(home::MenuScenePlugin);
    }
}

fn init_ui_state(mut ui: ResMut<UiState>, args: Res<crate::args::Args>) {
    ui.hidden = args.hide_ui;
    ui.debug = args.debug;
    if args.select_civ {
        ui.body_tab = BodyTab::Civilization;
    }
    ui.menu = match args.menu.as_deref() {
        Some("new") => MenuScreen::NewSandbox,
        Some("load") => MenuScreen::Load,
        Some("scenarios") => MenuScreen::Scenarios,
        Some("settings") => MenuScreen::Settings,
        Some("credits") => MenuScreen::Credits,
        _ => MenuScreen::Home,
    };
    if ui.menu == MenuScreen::Load {
        ui.sandboxes = crate::persistence::list_sandboxes();
        ui.saves = crate::persistence::list_saves();
    }
    if args.menu.is_some() {
        ui.intro_skipped = true;
    }
    if let Some(site) = args.surface_site.as_deref().and_then(cosmogon_sim::planet::landing::find) {
        ui.surface_site = Some(site.id.into()); ui.surface_latitude = site.latitude_north_deg; ui.surface_longitude = site.longitude_east_deg;
        ui.surface_altitude = args.surface_altitude; ui.surface_tracked = !args.surface_snapshot;
        ui.surface_horizon = args.surface_horizon;
        ui.body_tab = BodyTab::Surface;
    }
    if args.habitat_definition.is_some() || args.greenhouse_definition.is_some() || args.crop_chemistry.is_some() || args.greenhouse_supplies.is_some() || args.habitat_sweep.is_some() || args.habitat_recording.is_some() {ui.habitat_open=true;}
    if args.flight_definition.is_some() || args.flight_recording.is_some() || args.vehicle_plan.is_some() || args.vehicle_recording.is_some() || args.cargo_plan.is_some() || args.cargo_recording.is_some() {ui.vehicle_open=true;}
    if args.support_recording.is_some() || args.support_output.is_some() {ui.vehicle_open=true;}
    match args.panel.as_deref() {
        Some("vehicle"|"vehicle-design"|"vehicle-burns"|"vehicle-results"|"vehicle-model"|"vehicle-files"|"vehicle-flight"|"vehicle-flight-results"|"vehicle-flight-files"|"vehicle-arrival"|"vehicle-support"|"cargo-epochs") => ui.vehicle_open=true,
        Some("create") => ui.create_open = true,
        Some("physics") => ui.physics_open = true,
        Some("surface") => ui.body_tab = BodyTab::Surface,
        Some("civilization") => ui.body_tab = BodyTab::Civilization,
        Some("habitat-crop"|"habitat-crop-settings"|"habitat-crop-model"|"habitat-crop-build"|"habitat-recording-balance-heat"|"habitat-recording-balance"|"habitat-recording-balance-electricity"|"habitat-recording-balance-temperature"|"habitat-recording-balance-counters"|"habitat-recording-balance-files"|"habitat-figure" | "habitat" | "habitat-controls" | "habitat-build" | "habitat-model" | "habitat-oxygen" | "habitat-water" | "habitat-humidity" | "habitat-humidity-settings" | "habitat-humidity-model" | "habitat-breach" | "habitat-breach-settings" | "habitat-breach-model" | "habitat-maintenance" | "habitat-maintenance-settings" | "habitat-maintenance-model" | "habitat-settlement" | "habitat-settlement-settings" | "habitat-settlement-model" | "habitat-paired" | "habitat-paired-table" | "habitat-paired-files" | "habitat-paired-replay"|"habitat-graph-layout" | "habitat-recordings" | "habitat-recording-pressure" | "habitat-recording-pressure-change" | "habitat-recording-pressure-breach" | "habitat-recording-pressure-files" | "habitat-recording-why" | "habitat-recording-why-water" | "habitat-recording-why-heat" | "habitat-recording-why-air" | "habitat-recording-why-evidence" | "habitat-experiments" | "habitat-why" | "habitat-why-water" | "habitat-why-heat" | "habitat-why-air" | "habitat-why-evidence" | "greenhouse-chemistry" | "greenhouse-chemistry-model" | "greenhouse-chemistry-files" | "greenhouse-supplies" | "greenhouse-supply-inputs" | "greenhouse-supply-model" | "greenhouse-supply-files" | "greenhouse" | "greenhouse-water" | "greenhouse-daily-water" | "greenhouse-inputs" | "greenhouse-compare" | "greenhouse-model" | "greenhouse-files") => ui.habitat_open = true,
        Some("section" | "section-map") => ui.surface_study_open = true,
        Some("chronicle") => { ui.left_tab = LeftTab::Chronicle; ui.chronicle_mode = false; },
        Some("chronicle-legends") => { ui.left_tab = LeftTab::Chronicle; ui.chronicle_mode = true; },
        Some("alerts") => ui.alerts_open = true,
        Some("sound"|"habitat-sound"|"habitat-sound-export") => { ui.alerts_open = true; ui.alerts_tab = 1; }
        Some("routing") => { ui.alerts_open = true; ui.alerts_tab = 2; }
        Some("performance") => ui.performance_open = true,
        Some("camera") => ui.camera_open = true,
        Some("photo") => { ui.camera_open = true; ui.camera_tab = 1; }
        Some("paths"|"paths-keys"|"paths-files") => {ui.camera_open=true;ui.camera_tab=3;}
        Some("controls"|"controls-everyday"|"controls-files") => {ui.camera_open=true;ui.camera_tab=4;}
        Some("help") => ui.help=true,
        Some("bookmarks") => { ui.camera_open = true; ui.camera_tab = 2; }
        Some("palette") => ui.palette_open = true,
        _ => {}
    }
    // Automated captures and dev starts skip the intro.
    if args.capture.is_some() || args.new.is_some() || args.load.is_some() {
        ui.intro_skipped = true;
    }
}

/// Icon glyphs (Phosphor, MIT) — use inside any text.
pub use egui_phosphor::regular as icon;

/// Inter for text (SIL Open Font License), Phosphor for icons, egui's fonts as fallbacks.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("inter".into(), std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-Regular.ttf"))));
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        f.insert(0, "inter".into());
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}

fn apply_theme(mut contexts: EguiContexts, mut done: Local<Option<f32>>, settings: Res<crate::persistence::UserSettings>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if *done == Some(settings.ui_scale) {
        return Ok(());
    }
    if done.is_none() {
        install_fonts(ctx);
    }
    *done = Some(settings.ui_scale);
    ctx.set_zoom_factor(settings.ui_scale);
    let mut style = (*ctx.style()).clone();
    use egui::{FontFamily::*, FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(19.0, Proportional)),
        (TextStyle::Body, FontId::new(13.0, Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, Monospace)),
        (TextStyle::Button, FontId::new(13.0, Proportional)),
        (TextStyle::Small, FontId::new(10.5, Proportional)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = egui::Color32::from_rgba_premultiplied(11, 14, 24, 242);
    v.window_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(36, 44, 62));
    v.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: egui::Color32::from_black_alpha(140) };
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 6], blur: 18, spread: 0, color: egui::Color32::from_black_alpha(120) };
    v.extreme_bg_color = egui::Color32::from_rgb(6, 8, 14);
    v.faint_bg_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 6);
    v.selection.bg_fill = egui::Color32::from_rgb(92, 70, 32);
    v.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.hyperlink_color = ACCENT;
    v.override_text_color = Some(TEXT);
    let r = egui::CornerRadius::same(6);
    v.window_corner_radius = egui::CornerRadius::same(10);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = r;
    }
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(24, 31, 48);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(24, 31, 48);
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(38, 48, 72);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT_DIM);
    v.widgets.active.weak_bg_fill = egui::Color32::from_rgb(70, 56, 30);
    ctx.set_style(style);
    Ok(())
}

/// Section heading in the house style.
pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text.to_uppercase()).size(11.0).color(ACCENT).strong());
}

pub fn kv(ui: &mut egui::Ui, k: &str, v: impl Into<egui::WidgetText>) {
    ui.label(egui::RichText::new(k).color(MUTED));
    ui.label(v);
    ui.end_row();
}

/// Format a large count compactly: 1.2 M, 3.4 bn.
pub fn compact(n: f64) -> String {
    let a = n.abs();
    if a >= 1e12 {
        format!("{:.2} tn", n / 1e12)
    } else if a >= 1e9 {
        format!("{:.2} bn", n / 1e9)
    } else if a >= 1e6 {
        format!("{:.2} M", n / 1e6)
    } else if a >= 1e4 {
        format!("{:.1} k", n / 1e3)
    } else {
        format!("{n:.0}")
    }
}

pub fn power(w: f64) -> String {
    let a = w.abs();
    if a >= 1e15 {
        format!("{:.1} PW", w / 1e15)
    } else if a >= 1e12 {
        format!("{:.1} TW", w / 1e12)
    } else if a >= 1e9 {
        format!("{:.1} GW", w / 1e9)
    } else if a >= 1e6 {
        format!("{:.1} MW", w / 1e6)
    } else {
        format!("{:.0} kW", w / 1e3)
    }
}

pub fn distance(m: f64) -> String {
    use cosmogon_sim::astro::{AU, LIGHT_YEAR};
    if m >= 0.05 * LIGHT_YEAR {
        format!("{:.2} ly", m / LIGHT_YEAR)
    } else if m >= 0.01 * AU {
        format!("{:.3} AU", m / AU)
    } else if m >= 1e4 {
        format!("{} km", cosmogon_sim::time::group_digits(m / 1000.0))
    } else {
        format!("{m:.0} m")
    }
}
