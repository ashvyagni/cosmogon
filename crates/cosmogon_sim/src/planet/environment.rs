//! A zero-dimensional climate model.
//!
//! Surface temperature = radiative equilibrium temperature raised by a grey greenhouse
//! with optical depth τ: `T_s = T_eq · (1 + 0.75 τ)^¼`. τ is a parametric fit with a CO₂
//! term calibrated on Venus and Mars and a water-vapour term calibrated on Earth. This
//! reproduces Venus (~740 K), Earth (~288 K) and Mars (~213 K) to within a few kelvin and
//! responds plausibly to CO₂ changes, stellar brightening and water loss. It is *not* a
//! GCM; latitude structure is added procedurally by `terrain`.

#[allow(unused_imports)]
use cosmogon_core::dmath::DMath;
use crate::astro::{Body, BodyKind, Star, AU};

/// Stellar flux relative to Earth's (S⊕).
pub fn insolation(star: &Star, t: f64, distance_m: f64) -> f64 {
    let d = distance_m / AU;
    star.luminosity(t) / (d * d)
}

pub fn equilibrium_temperature(luminosity: f64, distance_au: f64, albedo: f64) -> f64 {
    278.6 * luminosity.dpowf(0.25) / distance_au.sqrt() * (1.0 - albedo).max(0.0).dpowf(0.25)
}

/// Boiling point of water (K) at pressure `p_bar` (Clausius–Clapeyron).
pub fn boiling_point(p_bar: f64) -> f64 {
    if p_bar <= 0.0 {
        return 0.0;
    }
    1.0 / (1.0 / 373.15 - (8.314 / 40_660.0) * p_bar.dln())
}

/// Whether liquid water is stable at temperature `t` and pressure `p_bar`.
pub fn liquid_water_stable(t: f64, p_bar: f64) -> bool {
    p_bar > 0.006 && t > 273.15 && t < boiling_point(p_bar)
}

pub fn greenhouse_tau(body: &Body, water_vapour: bool) -> f64 {
    let a = &body.atmosphere;
    if !a.is_present() {
        return 0.0;
    }
    // Thick-atmosphere power law (fit to Venus/Mars) plus the logarithmic forcing of a
    // trace gas in a pressure-broadened atmosphere (≈3 K per doubling on Earth).
    let p_co2 = a.partial(a.co2).max(0.0);
    let co2 = 4.24 * p_co2.dpowf(0.777) + 0.16 * (1.0 + p_co2 / 1e-4).dln() * a.pressure_bar.min(1.0);
    let ch4 = 2.0 * a.partial(a.ch4).max(0.0).dpowf(0.6);
    let h2o = if water_vapour { 0.61 * a.pressure_bar.min(4.0).sqrt() } else { 0.0 };
    // Pressure-induced absorption in thick H₂ envelopes (super-Earths with primordial air).
    let h2 = 0.5 * a.partial(a.h2he).max(0.0).dpowf(0.8);
    co2 + ch4 + h2o + h2
}

/// Recompute temperature, water state and albedo for a body.
pub fn update_climate(body: &mut Body, star: &Star, t: f64, stellar_distance_m: f64) {
    let lum = star.luminosity(t);
    let d_au = stellar_distance_m / AU;
    if body.kind.is_stellar() {
        // Stars and remnants keep their own temperature (set when created).
        return;
    }
    if !body.kind.has_surface() {
        body.equilibrium_temperature = equilibrium_temperature(lum, d_au, body.albedo);
        // Giants radiate internal heat; quote a 1-bar-level temperature.
        body.temperature = body.equilibrium_temperature * 1.35 + body.climate_bias;
        return;
    }

    let w = body.hydro.water_inventory;
    let p = body.atmosphere.pressure_bar;
    let mut temp = body.temperature.max(30.0);
    let mut albedo = body.albedo;
    for _ in 0..4 {
        let t_eq = equilibrium_temperature(lum, d_au, albedo);
        let vapour = w > 0.01 && t_eq > 200.0 && p > 0.05;
        let tau = greenhouse_tau(body, vapour);
        temp = t_eq * (1.0 + 0.75 * tau).dpowf(0.25) + body.climate_bias;
        body.equilibrium_temperature = t_eq;

        // Water phase and surface coverage.
        let cover = (0.71 * w.max(0.0).dpowf(0.3)).clamp(0.0, 1.0);
        if w < 1e-4 {
            body.hydro.ocean_fraction = 0.0;
            body.hydro.ice_fraction = 0.0;
        } else if liquid_water_stable(temp, p) {
            body.hydro.ocean_fraction = cover;
            let ice = ((295.0 - temp) / 120.0).clamp(0.0, 0.6) * (w * 4.0).min(1.0);
            body.hydro.ice_fraction = ice.min(1.0 - cover);
            body.hydro.ocean_fraction = (cover - ice * 0.5).max(0.0);
        } else if temp <= 273.15 {
            // Ice sheets are kilometres thick, so frozen water covers far less area than
            // the same water as an ocean would.
            body.hydro.ocean_fraction = 0.0;
            body.hydro.ice_fraction = (0.71 * w.max(0.0).powf(0.6)).min(1.0);
        } else {
            // Too hot / too thin: water is vapour or lost.
            body.hydro.ocean_fraction = 0.0;
            body.hydro.ice_fraction = 0.0;
        }

        if !body.real {
            let base = if body.kind == BodyKind::Icy { 0.55 } else { 0.18 };
            let clouds = if vapour && body.hydro.ocean_fraction > 0.0 { 0.12 } else { 0.0 };
            let thick = if p > 20.0 { 0.45 } else { 0.0 };
            albedo = (base + 0.45 * body.hydro.ice_fraction + clouds + thick).min(0.9);
        }
    }
    body.albedo = albedo;
    // Impact winter: surface air cooling only (see `impact::ImpactWinter`).
    let winter = body.impact_winter.map(|w| w.cooling_at(t)).unwrap_or(0.0);
    body.temperature = temp - winter;
}

/// The carbonate–silicate cycle over `dt_myr` million years: on geologically active worlds
/// with water, volcanic CO₂ accumulates while the surface is frozen (weathering stops) and is
/// drawn down by weathering when it is hot. This is what lets a world escape a snowball state
/// under a faint young star, and why "climate regulation" is a habitability factor.
/// Only meaningful on geological timescales — never call it on civilization timesteps.
pub fn carbon_cycle(body: &mut Body, dt_myr: f64) {
    if body.real || body.kind != BodyKind::Rocky || body.geology < 0.2 || body.hydro.water_inventory < 0.01 || !body.atmosphere.is_present() || body.atmosphere.pressure_bar > 20.0 {
        return;
    }
    let a = &mut body.atmosphere;
    let strength = body.geology.min(1.5);
    if body.temperature < 278.0 {
        a.co2 = (a.co2 * 1.25f64.dpowf(dt_myr * strength)).min(0.6);
    } else if body.temperature > 300.0 && body.hydro.ocean_fraction > 0.0 {
        a.co2 = (a.co2 / 1.15f64.dpowf(dt_myr * strength)).max(1e-5);
    } else {
        return;
    }
    a.normalise();
}

/// Calibrate a real body so the model reproduces its measured temperature now.
pub fn calibrate_real_body(body: &mut Body, star: &Star, t: f64, stellar_distance_m: f64, observed: f64) {
    body.climate_bias = 0.0;
    body.temperature = observed;
    let (ocean, ice) = (body.hydro.ocean_fraction, body.hydro.ice_fraction);
    update_climate(body, star, t, stellar_distance_m);
    body.climate_bias = observed - body.temperature;
    body.temperature = observed;
    body.hydro.ocean_fraction = ocean;
    body.hydro.ice_fraction = ice;
}

/// A star whose luminosity at t = 0 is exactly `luminosity` L☉: zero age puts it at
/// the start of the main sequence, where `luminosity(0) = luminosity_ms · 0.72`.
fn lab_star(luminosity: f64) -> Star {
    let mut s = Star::from_mass("Lab".into(), 1.0, 0.0, 0.0);
    s.luminosity_ms = luminosity / 0.72;
    s.formed_at = 0.0;
    s.lifetime = 1.0e30;
    s
}

/// The controlled Earth-like fixture of the ice-albedo sweep: a real-body template with
/// the calibration offset, the real-body albedo lock and the observed water inventory
/// overridden so the ice/albedo feedback path (`update_climate`, `!body.real`) runs.
fn ice_albedo_body() -> Body {
    let sys = crate::astro::sol::sol_system();
    let i = sys.find_body("Earth").expect("Sol has Earth");
    let mut b = sys.bodies[i].clone();
    b.real = false;
    b.climate_bias = 0.0;
    b.albedo = 0.3;
    b.hydro.water_inventory = 1.0;
    b.impact_winter = None;
    b
}

/// Ice–albedo sweep shared with the `climate-analytic-greenhouse` validation case: an
/// Earth-like body at 1 AU, luminosity 1.00 → 0.70 L☉ in steps of 0.01, each step
/// continuing from the previous climate state (the collapse is a hysteresis jump, not a
/// per-step independent equilibrium). Returns `(luminosity, temperature_k, albedo)` per
/// step, brightest first. Test/validation fixture — never called by the simulation.
pub(crate) fn ice_albedo_luminosity_sweep() -> Vec<(f64, f64, f64)> {
    let mut body = ice_albedo_body();
    let mut out = Vec::with_capacity(31);
    for k in 0..31 {
        let lam = 1.0 - 0.01 * k as f64;
        update_climate(&mut body, &lab_star(lam), 0.0, AU);
        out.push((lam, body.temperature, body.albedo));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::sol;

    #[test]
    fn model_reproduces_terrestrial_planets_without_calibration() {
        let sys = sol::sol_system();
        for (name, observed, tol) in [("Venus", 737.0, 40.0), ("Earth", 288.0, 8.0), ("Mars", 210.0, 12.0)] {
            let i = sys.find_body(name).unwrap();
            let mut b = sys.bodies[i].clone();
            b.climate_bias = 0.0;
            update_climate(&mut b, &sys.star, 0.0, sys.stellar_distance(i));
            assert!((b.temperature - observed).abs() < tol, "{name}: {} K", b.temperature);
        }
    }

    #[test]
    fn more_co2_warms_earth() {
        let sys = sol::sol_system();
        let i = sys.find_body("Earth").unwrap();
        let mut b = sys.bodies[i].clone();
        let before = b.temperature;
        b.atmosphere.co2 *= 2.0;
        update_climate(&mut b, &sys.star, 0.0, sys.stellar_distance(i));
        assert!(b.temperature > before + 1.5 && b.temperature < before + 6.0, "{} -> {}", before, b.temperature);
    }

    #[test]
    fn boiling_point_at_one_bar() {
        assert!((boiling_point(1.01325) - 373.15).abs() < 0.5);
        assert!(!liquid_water_stable(280.0, 0.006));
    }

    /// Hand-computed radiative-equilibrium fixtures. The expression under test is pure
    /// IEEE-754 arithmetic (constant multiply, `sqrt`, `powf(0.25)`); no DMath functions
    /// are needed because no result here feeds a simulation trajectory and libm's own
    /// correctly-rounded behaviour is exactly what is being pinned.
    #[test]
    fn equilibrium_temperature_matches_hand_computed_fixtures() {
        // T_eq(1 L☉, 1 AU, A=0) = 278.6 K exactly by definition of the constant.
        assert!((equilibrium_temperature(1.0, 1.0, 0.0) - 278.6).abs() < 1e-9);
        // T_eq(1, 1, 0.3) = 278.6 · 0.7^¼ ≈ 254.83 K (hand-computed).
        assert!((equilibrium_temperature(1.0, 1.0, 0.3) - 254.83).abs() < 0.05);
        // Homogeneity of degree ¼ in L: T_eq(4L, d, A) = √2 · T_eq(L, d, A).
        let one = equilibrium_temperature(1.0, 1.0, 0.3);
        let four = equilibrium_temperature(4.0, 1.0, 0.3);
        assert!((four - std::f64::consts::SQRT_2 * one).abs() < 1e-9);
    }

    /// More CO₂ must never cool the model: τ and the resulting surface temperature are
    /// both non-decreasing over a 1e-5 → 0.5 bar CO₂ sweep in a fixed 1-bar air.
    /// `real = true` locks the albedo so this isolates the greenhouse response itself
    /// (the cloud/ice albedo feedback is covered by the runaway test below).
    #[test]
    fn greenhouse_tau_is_monotone_in_co2() {
        let sys = sol::sol_system();
        let i = sys.find_body("Earth").unwrap();
        let base = sys.bodies[i].clone();
        let d = sys.stellar_distance(i);
        let star = sys.star.clone();
        let (mut last_tau, mut last_t) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for k in 0..9 {
            // Log-spaced partial pressures: 1e-5 … 0.5 bar in a fixed 1-bar air.
            let p_co2 = 1.0e-5_f64 * (0.5_f64 / 1.0e-5).powf(k as f64 / 8.0);
            let mut b = base.clone();
            b.real = true;
            b.climate_bias = 0.0;
            b.atmosphere.pressure_bar = 1.0;
            b.atmosphere.co2 = p_co2;
            let tau = greenhouse_tau(&b, true);
            update_climate(&mut b, &star, 0.0, d);
            assert!(tau >= last_tau, "τ decreased at {p_co2} bar: {tau} < {last_tau}");
            assert!(b.temperature >= last_t, "temperature decreased at {p_co2} bar: {} < {last_t}", b.temperature);
            (last_tau, last_t) = (tau, b.temperature);
        }
    }

    /// The ice–albedo feedback must have a real threshold: cooling a sun-like star in
    /// 0.01 L☉ steps drives the Earth-like fixture across a sharp warm → frozen jump.
    /// `real = false` because real bodies bypass the feedback (`update_climate`).
    #[test]
    fn ice_albedo_runaway_has_a_threshold() {
        let sweep = ice_albedo_luminosity_sweep();
        // Largest adjacent temperature drop across the sweep.
        let mut jump = 0.0;
        let mut at = 0;
        for k in 1..sweep.len() {
            let drop = sweep[k - 1].1 - sweep[k].1;
            if drop > jump {
                jump = drop;
                at = k;
            }
        }
        let (lam_w, t_w, a_w) = sweep[at - 1];
        let (lam_f, t_f, a_f) = sweep[at];
        println!("ice-albedo collapse between λ={lam_w:.2} ({t_w:.1} K, A={a_w:.3}) and λ={lam_f:.2} ({t_f:.1} K, A={a_f:.3}), jump {jump:.1} K");
        assert!(t_w >= 265.0, "warm side {t_w:.1} K < 265");
        assert!(a_w <= 0.42, "warm albedo {a_w:.3} > 0.42");
        // The frozen floor sits near 260 K, not the ~235 K a dry model would give: with a
        // 1-bar Earth air the water-vapour term (0.61·√p) stays on while t_eq > 200 K, so
        // the frozen side keeps most of the greenhouse. The warm→frozen gap is therefore
        // ~15 K, not ≥20 K (see docs/CLIMATE_VALIDATION.md).
        assert!(t_f <= 260.0, "frozen side {t_f:.1} K > 260");
        assert!(a_f >= 0.45, "frozen albedo {a_f:.3} < 0.45");
        assert!(jump >= 10.0, "largest jump {jump:.1} K < 10");
    }
}
