//! Reduced, explicit habitat budgets. Equipment ratings are user assumptions, not flight
//! hardware. NASA BVAD Rev2 Table 3-31 supplies the balanced nominal crew exchanges.
//! Shortfalls measure unserved demand, never physiological outcomes or deaths.
use crate::{
    history::{Category, Event},
    planet::terrain,
    time::SECONDS_PER_DAY,
    BodyRef, Universe,
};
use cosmogon_core::{
    constants::{SOLAR_LUMINOSITY, STEFAN_BOLTZMANN},
    dmath::DMath,
};
use serde::{Deserialize, Serialize};

pub const MODEL: &str = "cosmogon-habitat-budget/1";
pub const SOURCE: &str =
    "https://ntrs.nasa.gov/api/citations/20210024855/downloads/BVAD_2.15.22-final.pdf";
pub const PERIOD: f64 = 60.0;
pub(crate) const TASK: usize = 4;
const R: f64 = 8.31446261815324;
const O2: f64 = 0.0319988;
const N2: f64 = 0.0280134;
const CO2: f64 = 0.0440095;

/// All equipment/environment settings are experimental design inputs in SI. NASA's crew
/// rates are fixed by MODEL; they do not certify these illustrative equipment ratings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<crate::crop_process::Config>,
    pub crew: u32,
    pub volume_m3: f64,
    pub firm_power_w: f64,
    pub solar_area_m2: f64,
    pub solar_efficiency: f64,
    pub atmospheric_transmission: f64,
    pub battery_capacity_j: f64,
    pub charge_limit_w: f64,
    pub discharge_limit_w: f64,
    pub charge_efficiency: f64,
    pub discharge_efficiency: f64,
    pub base_load_w: f64,
    pub scrubber_power_w: f64,
    pub scrubber_rate_kg_s: f64,
    pub capture_capacity_kg: f64,
    pub recycler_power_w: f64,
    pub recycler_rate_kg_s: f64,
    pub water_recovery_fraction: f64,
    pub oxygen_makeup_rate_kg_s: f64,
    pub oxygen_setpoint_pa: f64,
    pub heater_limit_w: f64,
    pub radiator_pump_w: f64,
    pub radiator_area_m2: f64,
    pub radiator_emissivity: f64,
    pub heat_capacity_j_k: f64,
    pub conductance_w_k: f64,
    pub temperature_setpoint_k: f64,
    pub oxygen_alert_pa: f64,
    pub co2_alert_pa: f64,
    pub cold_alert_k: f64,
    pub hot_alert_k: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electrolysis: Option<crate::electrolysis::Config>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sabatier: Option<crate::sabatier::Config>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moisture: Option<crate::moisture::Config>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breach: Option<crate::breach::Config>,
    /// Passive signed wall condensation/evaporation; absent means no wall exchange.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_condensation: Option<crate::moisture::WallConfig>,
    /// Nominal slow structural air leakage; absent means no leak (breach is separate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leak: Option<LeakConfig>,
    /// Optional scheduled equipment wear/maintenance and authored capacity calibration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<crate::habitat_maintenance::MaintenanceConfig>,
    /// Optional continuous cabin construction from held finite material stock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub construction: Option<crate::habitat_construction::Config>,
    /// Optional authored life-support calibration factors with retained sensitivity
    /// bands. Absent legacy definitions stay byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration: Option<crate::habitat_calibration::Config>,
    /// Optional exterior radiation environment and authored shielding dose model.
    /// Absent definitions record no dose and stay byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiation: Option<crate::habitat_radiation::Config>,
    /// Finite general storage for the four transferable supply stocks. Absent legacy
    /// definitions keep the historical 1e7 kg general bound byte-exactly.
    #[serde(default = "default_storage_capacity", skip_serializing_if = "is_default_storage_capacity")]
    pub oxygen_reserve_capacity_kg: f64,
    #[serde(default = "default_storage_capacity", skip_serializing_if = "is_default_storage_capacity")]
    pub potable_water_capacity_kg: f64,
    #[serde(default = "default_storage_capacity", skip_serializing_if = "is_default_storage_capacity")]
    pub food_dry_capacity_kg: f64,
    #[serde(default = "default_storage_capacity", skip_serializing_if = "is_default_storage_capacity")]
    pub food_water_capacity_kg: f64,
}
fn default_storage_capacity() -> f64 {
    1.0e7
}
fn is_default_storage_capacity(v: &f64) -> bool {
    *v == 1.0e7
}
impl Default for Config {
    fn default() -> Self {
        Self {
            crew: 4,
            volume_m3: 500.0,
            firm_power_w: 4000.0,
            solar_area_m2: 40.0,
            solar_efficiency: 0.25,
            atmospheric_transmission: 0.7,
            battery_capacity_j: 72_000_000.0,
            charge_limit_w: 5000.0,
            discharge_limit_w: 5000.0,
            charge_efficiency: 0.95,
            discharge_efficiency: 0.95,
            base_load_w: 500.0,
            scrubber_power_w: 300.0,
            scrubber_rate_kg_s: 6.0 / SECONDS_PER_DAY,
            capture_capacity_kg: 100.0,
            recycler_power_w: 300.0,
            recycler_rate_kg_s: 25.0 / SECONDS_PER_DAY,
            water_recovery_fraction: 0.9,
            oxygen_makeup_rate_kg_s: 10.0 / SECONDS_PER_DAY,
            oxygen_setpoint_pa: 21_200.0,
            heater_limit_w: 2000.0,
            radiator_pump_w: 200.0,
            radiator_area_m2: 25.0,
            radiator_emissivity: 0.85,
            heat_capacity_j_k: 50_000_000.0,
            conductance_w_k: 10.0,
            temperature_setpoint_k: 293.15,
            oxygen_alert_pa: 18_000.0,
            co2_alert_pa: 400.0,
            cold_alert_k: 283.15,
            hot_alert_k: 303.15,
            crop: None,
            electrolysis: None,
            sabatier: None,
            moisture: None,
            breach: None,
            wall_condensation: None,
            leak: None,
            maintenance: None,
            construction: None,
            calibration: None,
            radiation: None,
            oxygen_reserve_capacity_kg: default_storage_capacity(),
            potable_water_capacity_kg: default_storage_capacity(),
            food_dry_capacity_kg: default_storage_capacity(),
            food_water_capacity_kg: default_storage_capacity(),
        }
    }
}
pub const LEAK_MODEL: &str = "cosmogon-cabin-structural-leak/1";
/// Nominal slow cabin gas leakage: proportional species escape with assumed constant
/// cabin-gas specific heat. Independent of the pressure-driven breach orifice model.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LeakConfig {
    pub model: String,
    pub rate_kg_s: f64,
    pub gas_specific_heat_j_kg: f64,
}
impl Default for LeakConfig {
    fn default() -> Self {
        Self {
            model: LEAK_MODEL.into(),
            rate_kg_s: 1.0e-6,
            gas_specific_heat_j_kg: 1000.0,
        }
    }
}
impl LeakConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.model != LEAK_MODEL {
            return Err(format!("Unsupported cabin leak model {}", self.model));
        }
        bounds(self.rate_kg_s, 0.0, 100.0, "Structural leak rate (kg/s)")?;
        bounds(
            self.gas_specific_heat_j_kg,
            1.0,
            1e5,
            "Assumed cabin-gas specific heat (J/kg)",
        )
    }
}
/// Last completed interval's structural leak; escaped masses join the cumulative ledger.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LeakBudget {
    pub initial_gas: crate::breach::Gas,
    pub leaked: crate::breach::Gas,
    pub temperature_k: f64,
    pub cooling_j: f64,
}
impl LeakBudget {
    pub fn validate(&self, c: &LeakConfig, seconds: f64) -> Result<(), String> {
        self.initial_gas.validate()?;
        self.leaked.validate()?;
        let total = self.leaked.mass_kg();
        for v in [self.temperature_k, self.cooling_j] {
            if !v.is_finite() || v < 0.0 {
                return Err("Invalid leak interval quantity".into());
            }
        }
        if total > c.rate_kg_s * seconds + 1e-9
            || (self.cooling_j - total * c.gas_specific_heat_j_kg * self.temperature_k).abs()
                > 1e-8 * self.cooling_j.abs().max(1.0)
        {
            return Err("Leak interval disagrees with retained leak configuration".into());
        }
        Ok(())
    }
}
fn bounds(value: f64, min: f64, max: f64, name: &str) -> Result<(), String> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(format!("{name} must be finite and between {min} and {max}"))
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.crew > 1000 {
            return Err("Crew demand supports 0–1000 reference people".into());
        }
        bounds(self.volume_m3, 1.0, 1.0e6, "Cabin volume (m³)")?;
        for (v, n) in [
            (self.firm_power_w, "Firm power"),
            (self.charge_limit_w, "Charge limit"),
            (self.discharge_limit_w, "Discharge limit"),
            (self.base_load_w, "Base load"),
            (self.scrubber_power_w, "Scrubber power"),
            (self.recycler_power_w, "Recycler power"),
            (self.heater_limit_w, "Heater power"),
            (self.radiator_pump_w, "Radiator pump power"),
        ] {
            bounds(v, 0.0, 1.0e6, n)?;
        }
        for (v, n) in [
            (self.solar_area_m2, "Solar area"),
            (self.radiator_area_m2, "Radiator area"),
        ] {
            bounds(v, 0.0, 100_000.0, n)?;
        }
        for (v, n) in [
            (self.solar_efficiency, "PV efficiency"),
            (self.atmospheric_transmission, "Atmospheric transmission"),
            (self.water_recovery_fraction, "Water recovery"),
            (self.radiator_emissivity, "Radiator emissivity"),
        ] {
            bounds(v, 0.0, 1.0, n)?;
        }
        for (v, n) in [
            (self.charge_efficiency, "Charge efficiency"),
            (self.discharge_efficiency, "Discharge efficiency"),
        ] {
            bounds(v, 0.01, 1.0, n)?;
        }
        bounds(self.battery_capacity_j, 0.0, 1.0e12, "Battery capacity (J)")?;
        bounds(
            self.capture_capacity_kg,
            0.0,
            1.0e7,
            "CO₂ capture capacity (kg)",
        )?;
        for (v, n) in [
            (self.oxygen_reserve_capacity_kg, "O₂ reserve storage capacity"),
            (self.potable_water_capacity_kg, "Potable water storage capacity"),
            (self.food_dry_capacity_kg, "Dry food storage capacity"),
            (self.food_water_capacity_kg, "Food water storage capacity"),
        ] {
            bounds(v, 0.0, 1.0e7, n)?;
        }
        for (v, n) in [
            (self.scrubber_rate_kg_s, "CO₂ capture rate"),
            (self.recycler_rate_kg_s, "Water processing rate"),
            (self.oxygen_makeup_rate_kg_s, "O₂ makeup rate"),
        ] {
            bounds(v, 0.0, 100.0, n)?;
        }
        bounds(self.heat_capacity_j_k, 1.0e6, 1.0e12, "Heat capacity (J/K)")?;
        bounds(self.conductance_w_k, 0.0, 1000.0, "Conductance (W/K)")?;
        for (v, n) in [
            (self.temperature_setpoint_k, "Temperature setpoint"),
            (self.cold_alert_k, "Cold alert"),
            (self.hot_alert_k, "Hot alert"),
        ] {
            bounds(v, 100.0, 500.0, n)?;
        }
        if self.cold_alert_k >= self.hot_alert_k {
            return Err("Cold alert must be below hot alert".into());
        }
        for (v, n) in [
            (self.oxygen_setpoint_pa, "O₂ setpoint"),
            (self.oxygen_alert_pa, "O₂ alert"),
            (self.co2_alert_pa, "CO₂ alert"),
        ] {
            bounds(v, 0.0, 200_000.0, n)?;
        }
        if let Some(c) = &self.crop {c.validate()?;if self.moisture.is_none(){return Err("Finite crop vapor requires enabled cabin moisture accounting".into());}}
        if let Some(c) = &self.electrolysis {c.validate()?;}
        if let Some(c) = &self.sabatier {c.validate()?;}
        if let Some(c) = &self.moisture {c.validate()?;}
        if let Some(c) = &self.breach {c.validate()?;}
        if let Some(c) = &self.wall_condensation {c.validate()?;}
        if let Some(c) = &self.leak {c.validate()?;}
        if let Some(m) = &self.maintenance {
            m.validate()?;
            if m.moisture.is_some() && self.moisture.is_none() {
                return Err("Moisture maintenance requires enabled cabin moisture equipment".into());
            }
            if m.electrolysis.is_some() && self.electrolysis.is_none() {
                return Err("Electrolysis maintenance requires enabled water electrolysis".into());
            }
            if m.sabatier.is_some() && self.sabatier.is_none() {
                return Err("Sabatier maintenance requires enabled CO₂ reduction".into());
            }
        }
        if let Some(k) = &self.construction {k.validate()?;}
        if let Some(k) = &self.calibration {k.validate()?;}
        if let Some(k) = &self.radiation {k.validate()?;}
        // Internal oxygen production is gated only by equipment product capacities,
        // so the reserve tank must at least cover every installed oxygen ceiling.
        if let Some(c) = &self.crop {
            if c.oxygen_capacity_kg > self.oxygen_reserve_capacity_kg {
                return Err("O₂ reserve storage capacity is below installed crop oxygen capacity".into());
            }
        }
        if let Some(e) = &self.electrolysis {
            if e.oxygen_capacity_kg > self.oxygen_reserve_capacity_kg {
                return Err("O₂ reserve storage capacity is below installed electrolysis oxygen capacity".into());
            }
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Initial {
    #[serde(default, skip_serializing_if = "zero")]
    pub captured_co2_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub crop_nutrient_equivalent_kg: f64,
    pub temperature_k: f64,
    pub oxygen_pa: f64,
    pub nitrogen_pa: f64,
    pub co2_pa: f64,
    pub battery_j: f64,
    pub oxygen_reserve_kg: f64,
    pub potable_water_kg: f64,
    pub food_dry_kg: f64,
    pub food_water_kg: f64,
}
impl Default for Initial {
    fn default() -> Self {
        Self {
            captured_co2_kg: 0.0,
            crop_nutrient_equivalent_kg: 0.0,
            temperature_k: 293.15,
            oxygen_pa: 21_200.0,
            nitrogen_pa: 79_900.0,
            co2_pa: 40.0,
            battery_j: 36_000_000.0,
            oxygen_reserve_kg: 100.0,
            potable_water_kg: 200.0,
            food_dry_kg: 100.0,
            food_water_kg: 95.0,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub model: String,
    pub name: String,
    pub body: BodyRef,
    pub latitude_deg: f64,
    pub longitude_east_deg: f64,
    pub config: Config,
    pub initial: Initial,
    /// Passive full-interval heat accounting. Absent legacy definitions stay disabled.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub record_heat: bool,
}
impl Definition {
    pub fn at(body: BodyRef, latitude_deg: f64, longitude_east_deg: f64) -> Self {
        Self {
            model: MODEL.into(),
            name: "Surface habitat experiment".into(),
            body,
            latitude_deg,
            longitude_east_deg,
            config: Config::default(),
            initial: Initial::default(),
            record_heat: true,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.model != MODEL {
            return Err(format!("Unsupported habitat model {}", self.model));
        }
        if self.name.trim().is_empty() || self.name.len() > 120 {
            return Err("Habitat name must contain 1–120 bytes".into());
        }
        bounds(self.latitude_deg, -90.0, 90.0, "Latitude")?;
        bounds(self.longitude_east_deg, 0.0, 360.0, "East longitude")?;
        self.config.validate()?;
        let i = &self.initial;
        bounds(i.temperature_k, 100.0, 500.0, "Initial temperature")?;
        for (v, n) in [
            (i.oxygen_pa, "Initial O₂ pressure"),
            (i.nitrogen_pa, "Initial N₂ pressure"),
            (i.co2_pa, "Initial CO₂ pressure"),
        ] {
            bounds(v, 0.0, 200_000.0, n)?;
        }
        bounds(
            i.battery_j,
            0.0,
            self.config.battery_capacity_j,
            "Initial battery energy",
        )?;
        for (v, n) in [
            (i.oxygen_reserve_kg, "O₂ reserve"),
            (i.potable_water_kg, "Potable water"),
            (i.food_dry_kg, "Dry food"),
            (i.food_water_kg, "Food water"),
        ] {
            bounds(v, 0.0, 1.0e7, n)?;
        }
        for (stock, capacity, n) in [
            (i.oxygen_reserve_kg, self.config.oxygen_reserve_capacity_kg, "Initial O₂ reserve"),
            (
                i.potable_water_kg,
                self.config.potable_water_capacity_kg,
                "Initial potable water",
            ),
            (i.food_dry_kg, self.config.food_dry_capacity_kg, "Initial dry food"),
            (
                i.food_water_kg,
                self.config.food_water_capacity_kg,
                "Initial food water",
            ),
        ] {
            if stock > capacity {
                return Err(format!("{n} exceeds its storage capacity"));
            }
        }
        bounds(i.captured_co2_kg,0.,self.config.capture_capacity_kg,"Initial captured CO2")?;
        bounds(i.crop_nutrient_equivalent_kg,0.,1e7,"Initial crop nutrient-equivalent")?;
        if self.config.crop.is_none() && i.crop_nutrient_equivalent_kg!=0. {return Err("Initial crop nutrients require an explicit installed crop".into());}
        if let Some(c)=&self.config.crop {if i.oxygen_reserve_kg>c.oxygen_capacity_kg {return Err("Initial oxygen reserve exceeds crop product capacity".into());}}
        if let Some(c) = &self.config.electrolysis {
            if i.oxygen_reserve_kg > c.oxygen_capacity_kg {return Err("Initial O₂ reserve exceeds electrolysis product capacity".into());}
        }
        Ok(())
    }
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        use std::io::Read;
        let metadata=std::fs::metadata(path).map_err(|e|e.to_string())?;
        if !metadata.is_file() || metadata.len()>65_536 {return Err("Habitat definition must be a regular file at most 64 KiB".into());}
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(65_537)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 65_536 {
            return Err("Habitat definition exceeds 64 KiB".into());
        }
        let value: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
}
fn zero(v: &f64) -> bool {*v == 0.0}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<crate::crop_process::State>,
    pub temperature_k: f64,
    pub oxygen_kg: f64,
    pub nitrogen_kg: f64,
    pub co2_kg: f64,
    pub oxygen_reserve_kg: f64,
    pub potable_water_kg: f64,
    pub wastewater_kg: f64,
    pub brine_kg: f64,
    pub food_dry_kg: f64,
    pub food_water_kg: f64,
    pub dry_waste_kg: f64,
    pub captured_co2_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub hydrogen_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub methane_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub sabatier_water_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub water_vapor_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub condensate_kg: f64,
    /// Passive wall film liquid; held with no recycling path in this model.
    #[serde(default, skip_serializing_if = "zero")]
    pub wall_liquid_kg: f64,
    /// Cumulative gas outside the cabin, never available to cabin consumers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escaped_gas: Option<crate::breach::Gas>,
    /// Held per-component equipment condition/cycle; absent when maintenance is disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<crate::habitat_maintenance::MaintenanceState>,
    /// Held finite processed construction material; zero when construction is unused.
    #[serde(default, skip_serializing_if = "zero")]
    pub construction_material_kg: f64,
    /// Cumulative material converted into cabin structure mass (volume comes from config).
    #[serde(default, skip_serializing_if = "zero")]
    pub construction_structure_kg: f64,
    pub battery_j: f64,
}
impl State {
    pub fn mass_kg(&self) -> f64 {
        self.oxygen_kg
            + self.nitrogen_kg
            + self.co2_kg
            + self.oxygen_reserve_kg
            + self.potable_water_kg
            + self.wastewater_kg
            + self.brine_kg
            + self.food_dry_kg
            + self.food_water_kg
            + self.dry_waste_kg
            + self.captured_co2_kg
            + self.hydrogen_kg
            + self.methane_kg
            + self.sabatier_water_kg
            + self.water_vapor_kg + self.condensate_kg + self.wall_liquid_kg
            + self.construction_material_kg + self.construction_structure_kg
            + self.crop.as_ref().map_or(0.,|c|c.mass_kg())
    }
    pub fn accounted_mass_kg(&self)->f64 {self.mass_kg()+self.escaped_gas.map(|g|g.mass_kg()).unwrap_or(0.0)}
    pub fn cabin_gas(&self)->crate::breach::Gas {crate::breach::Gas {oxygen_kg:self.oxygen_kg,nitrogen_kg:self.nitrogen_kg,co2_kg:self.co2_kg,vapor_kg:self.water_vapor_kg}}
    pub fn oxygen_pa(&self, c: &Config) -> f64 {
        self.oxygen_kg / O2 * R * self.temperature_k / c.volume_m3
    }
    pub fn co2_pa(&self, c: &Config) -> f64 {
        self.co2_kg / CO2 * R * self.temperature_k / c.volume_m3
    }
    pub fn vapor_pressure_pa(&self,c:&Config)->f64 {crate::moisture::vapor_pressure_pa(self.water_vapor_kg,self.temperature_k,c.volume_m3)}
    pub fn relative_humidity(&self,c:&Config)->Option<f64> {crate::moisture::relative_humidity(self.water_vapor_kg,self.temperature_k,c.volume_m3)}
    pub fn phase_energy_j(&self)->f64 {self.water_vapor_kg*crate::moisture::REFERENCE_PHASE_J_KG}
    pub fn pressure_pa(&self, c: &Config) -> f64 {
        (self.oxygen_kg / O2 + self.nitrogen_kg / N2 + self.co2_kg / CO2) * R * self.temperature_k
            / c.volume_m3 + self.vapor_pressure_pa(c)
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Totals {
    pub generated_j: f64,
    pub delivered_j: f64,
    pub losses_j: f64,
    pub curtailed_j: f64,
    pub unserved_j: f64,
    pub unserved_person_seconds: f64,
    pub metabolic_person_seconds: f64,
    /// Cumulative reference enthalpy transferred, not current recoverable chemical stock.
    #[serde(default, skip_serializing_if = "zero")]
    pub reaction_enthalpy_j: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub sabatier_co2_consumed_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub sabatier_heat_j: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub sabatier_water_processed_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub condensate_processed_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub crew_vapor_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub condensed_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub breach_enthalpy_j: f64,
    /// Cumulative signed net wall condensation (positive) or evaporation (negative).
    #[serde(default, skip_serializing_if = "zero")]
    pub wall_condensed_kg: f64,
    /// Cumulative nominal structural leak by species (breach escapes stay separate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaked_gas: Option<crate::breach::Gas>,
    /// Cumulative opt-in inward breach mass by species (dry-air receiver assumption).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inflow_gas: Option<crate::breach::Gas>,
    /// Cumulative imported inward-flow enthalpy (positive; reported as negative
    /// outflow enthalpy on inward breach budgets).
    #[serde(default, skip_serializing_if = "zero")]
    pub inflow_enthalpy_j: f64,
    /// Cumulative component maintenance window time across all components (hours).
    #[serde(default, skip_serializing_if = "zero")]
    pub maintenance_hours: f64,
    /// Cumulative cabin volume added through construction.
    #[serde(default, skip_serializing_if = "zero")]
    pub constructed_volume_m3: f64,
    /// Cumulative construction material converted into structure.
    #[serde(default, skip_serializing_if = "zero")]
    pub construction_material_used_kg: f64,
    /// Cumulative whole-body crew dose from recorded interior dose rates (mSv per
    /// reference crewmember); zero while radiation is unused.
    #[serde(default, skip_serializing_if = "zero")]
    pub cumulative_dose_msv: f64,
}
/// Last completed interval; instantaneous estimates are not silently mixed with these.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Budget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<crate::crop_process::Budget>,
    pub seconds: f64,
    pub configuration: Option<Config>,
    pub environment_available: bool,
    pub temperature_domain_supported: bool,
    pub horizontal_irradiance_w_m2: Option<f64>,
    pub exterior_temperature_k: Option<f64>,
    pub generated_w: f64,
    pub requested_w: f64,
    pub delivered_w: f64,
    pub unserved_w: f64,
    pub battery_losses_w: f64,
    pub curtailed_w: f64,
    pub oxygen_makeup_kg: f64,
    pub co2_captured_kg: f64,
    pub water_processed_kg: f64,
    pub water_recovered_kg: f64,
    pub served_cycle_fraction: f64,
    pub metabolic_heat_w: f64,
    pub heater_w: f64,
    pub radiator_rejection_w: f64,
    pub conduction_w: f64,
    pub thermal_storage_w: f64,
    pub mass_residual_kg: f64,
    pub electric_residual_j: f64,
    pub thermal_residual_j: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electrolysis: Option<crate::electrolysis::Budget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sabatier: Option<crate::sabatier::Budget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moisture: Option<crate::moisture::Budget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breach: Option<crate::breach::Budget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall: Option<crate::moisture::WallBudget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leak: Option<LeakBudget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<crate::habitat_maintenance::MaintenanceBudget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub construction: Option<crate::habitat_construction::Budget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration: Option<crate::habitat_calibration::Budget>,
    /// Last interval's modelled exterior/interior/shelter dose rates and hazard band;
    /// absent when radiation is unused or the environment is unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiation: Option<crate::habitat_radiation::Budget>,
    #[serde(default, skip_serializing_if = "zero")]
    pub condensate_processed_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub condensate_recovered_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub sabatier_water_processed_kg: f64,
    #[serde(default, skip_serializing_if = "zero")]
    pub sabatier_water_recovered_kg: f64,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Environment {
    pub irradiance_w_m2: Option<f64>,
    pub exterior_temperature_k: Option<f64>,
    /// Exterior whole-body dose rate at the habitat site (mSv/day); None when the
    /// radiation environment is unavailable or not evaluated for this site.
    pub exterior_dose_rate_msv_day: Option<f64>,
}
pub const STOCK_TRANSFER_MODEL: &str = "cosmogon-habitat-stock-transfer/1";
/// Retained intra-settlement stock movement evidence on one side of a transfer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StockTransfer {
    pub model: String,
    pub counterparty: u64,
    pub world_time_s: f64,
    pub moved: crate::supply_vehicle::Cargo,
    pub outbound: bool,
}
/// Paired outbound/inbound record agreement across every habitat pair.
pub(crate) fn validate_stock_transfers(u: &Universe) -> Result<(), String> {
    // Pairing keys are time and masses; each side's counterparty is fixed by the filter.
    let key = |r: &StockTransfer| {
        (
            r.world_time_s,
            r.moved.oxygen_reserve_kg,
            r.moved.potable_water_kg,
            r.moved.food_dry_kg,
            r.moved.food_water_kg,
        )
    };
    for a in &u.habitats {
        for r in &a.stock_transfers {
            if !u.habitats.iter().any(|h| h.id == r.counterparty) {
                return Err("Habitat stock transfer counterparty no longer exists".into());
            }
        }
        for b in &u.habitats {
            if a.id == b.id {
                continue;
            }
            let mut outs: Vec<_> = a
                .stock_transfers
                .iter()
                .filter(|r| r.outbound && r.counterparty == b.id)
                .map(key)
                .collect();
            let mut ins: Vec<_> = b
                .stock_transfers
                .iter()
                .filter(|r| !r.outbound && r.counterparty == a.id)
                .map(key)
                .collect();
            outs.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
            ins.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
            if outs != ins {
                return Err("Habitat stock transfer records disagree with their counterparty".into());
            }
        }
    }
    Ok(())
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Habitat {
    pub id: u64,
    pub definition: Definition,
    pub config: Config,
    pub state: State,
    pub totals: Totals,
    pub budget: Budget,
    pub last_time: f64,
    pub alert_bits: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heat: Option<crate::habitat_heat::Ledger>,
    /// Applied atomic cargo receipts from co-located supported craft transfers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<crate::vehicle_transfer::Receipt>,
    /// Retained intra-settlement stock transfer records (both directions).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stock_transfers: Vec<StockTransfer>,
}
impl Habitat {
    pub fn new(id: u64, definition: Definition, time: f64) -> Result<Self, String> {
        bounds(time, -1.0e16, 1.0e16, "Habitat clock")?;
        if ((time + PERIOD) - time - PERIOD).abs() > 0.001 {
            return Err("Habitat model requires a clock resolving fixed minute intervals".into());
        }
        definition.validate()?;
        let c = &definition.config;
        let i = &definition.initial;
        let mass = |p: f64, m: f64| p * c.volume_m3 * m / (R * i.temperature_k);
        let state = State {
            crop: c.crop.as_ref().map(|crop|crate::crop_process::State::new(crop,i.crop_nutrient_equivalent_kg,time)).transpose()?,
            temperature_k: i.temperature_k,
            oxygen_kg: mass(i.oxygen_pa, O2),
            nitrogen_kg: mass(i.nitrogen_pa, N2),
            co2_kg: mass(i.co2_pa, CO2),
            oxygen_reserve_kg: i.oxygen_reserve_kg,
            potable_water_kg: i.potable_water_kg,
            wastewater_kg: 0.0,
            brine_kg: 0.0,
            food_dry_kg: i.food_dry_kg,
            food_water_kg: i.food_water_kg,
            dry_waste_kg: 0.0,
            captured_co2_kg: i.captured_co2_kg,
            hydrogen_kg: 0.0,
            methane_kg: 0.0,
            sabatier_water_kg: 0.0,
            water_vapor_kg:0.0,condensate_kg:0.0,wall_liquid_kg:0.0,escaped_gas:None,
            maintenance: c.maintenance.as_ref().map(|_| Default::default()),
            construction_material_kg: 0.0,
            construction_structure_kg: 0.0,
            battery_j: i.battery_j,
        };
        let mut heat=definition.record_heat.then(||crate::habitat_heat::Ledger::new(time,c.heat_capacity_j_k));
        if c.crop.is_some(){if let Some(l)=&mut heat {l.crop_storage_debits_j=Some([0.;2]);}}
        Ok(Self {
            id,
            config: definition.config.clone(),
            definition,
            state,
            totals: Totals::default(),
            budget: Budget::default(),
            last_time: time,
            alert_bits: 0,
            heat,
            receipts: vec![],
            stock_transfers: vec![],
        })
    }
    pub(crate) fn validate(&self, time: f64) -> Result<(), String> {
        self.definition.validate()?;
        crate::vehicle_transfer::validate_receipts(self, time)?;
        for r in &self.stock_transfers {
            if r.model != STOCK_TRANSFER_MODEL {
                return Err("Unsupported retained habitat stock transfer model".into());
            }
            if r.counterparty == self.id {
                return Err("Habitat stock transfer counterparty must differ".into());
            }
            bounds(r.world_time_s, -1.0e16, time, "Habitat stock transfer time")?;
            for v in [
                r.moved.oxygen_reserve_kg,
                r.moved.potable_water_kg,
                r.moved.food_dry_kg,
                r.moved.food_water_kg,
            ] {
                bounds(v, 0.0, 1.0e7, "Habitat stock transfer mass")?;
            }
        }
        if self.definition.record_heat != self.heat.is_some() {return Err("Heat ledger and original definition disagree".into());}
        if let Some(heat)=&self.heat {heat.validate(self.last_time)?;}
        self.check_config(&self.config)?;
        bounds(
            self.last_time,
            time - PERIOD - 0.001,
            time,
            "Habitat state time",
        )?;
        bounds(
            self.state.temperature_k,
            1.0,
            1.0e8,
            "Cabin model temperature",
        )?;
        let s = &self.state;
        if let Some(crop)=&s.crop {
            crop.validate(self.last_time)?;
            if let Some(original)=self.definition.config.crop.as_ref() {
                if original.crop!=crop.source_crop || original.dry_chemical_energy_j_kg!=crop.dry_chemical_energy_j_kg || self.definition.initial.crop_nutrient_equivalent_kg!=crop.initial_nutrient_equivalent_kg{return Err("Installed crop source/initial stock differs from original setup".into());}
            } else {
                if let Some(live)=self.config.crop.as_ref() {
                    if live.crop!=crop.source_crop || live.dry_chemical_energy_j_kg!=crop.dry_chemical_energy_j_kg{return Err("Runtime-installed crop differs from its retained configuration".into());}
                }
                bounds(crop.initial_nutrient_equivalent_kg,0.,1e7,"Runtime crop initial nutrient-equivalent")?;
            }
        }
        if let Some(l)=&self.heat {
            match (&s.crop,l.crop_storage_debits_j) {
                (Some(c),Some(v)) => {for (actual,wanted) in v.into_iter().zip([-c.chemical_energy_j(),-c.vapor_generated_kg*crate::moisture::REFERENCE_PHASE_J_KG]) {if (actual-wanted).abs()>1e-8*wanted.abs().max(1.) {return Err("Crop heat counters disagree with actual cumulative storage".into());}}}
                (None,None)=>{},_=>return Err("Crop heat supplement and installed state disagree".into())
            }
        }
        if self.definition.config.crop.is_some() && s.crop.is_none(){return Err("Original installed crop state is missing".into());}
        if let Some(b)=&self.budget.crop {let c=self.budget.configuration.as_ref().and_then(|c|c.crop.as_ref()).ok_or("Crop interval has no retained configuration")?;s.crop.as_ref().ok_or("Crop interval lacks installed state")?.validate_budget(b,c,self.budget.seconds,self.last_time)?;}
        for value in [
            s.oxygen_kg,
            s.nitrogen_kg,
            s.co2_kg,
            s.oxygen_reserve_kg,
            s.potable_water_kg,
            s.wastewater_kg,
            s.brine_kg,
            s.food_dry_kg,
            s.food_water_kg,
            s.dry_waste_kg,
            s.captured_co2_kg,
            s.hydrogen_kg,
            s.methane_kg,
            s.sabatier_water_kg, s.water_vapor_kg, s.condensate_kg, s.wall_liquid_kg,
            s.construction_material_kg,
            s.construction_structure_kg,
            s.battery_j,
        ] {
            bounds(value, 0.0, 1.0e12, "Stored habitat quantity")?;
        }
        for (stock, capacity, n) in [
            (
                s.oxygen_reserve_kg,
                self.config.oxygen_reserve_capacity_kg,
                "O₂ reserve",
            ),
            (
                s.potable_water_kg,
                self.config.potable_water_capacity_kg,
                "Potable water",
            ),
            (s.food_dry_kg, self.config.food_dry_capacity_kg, "Dry food"),
            (
                s.food_water_kg,
                self.config.food_water_capacity_kg,
                "Food water",
            ),
        ] {
            if stock > capacity {
                return Err(format!("{n} exceeds its storage capacity"));
            }
        }
        if let Some(g)=s.escaped_gas {g.validate()?;}
        if s.escaped_gas.is_some() || self.totals.inflow_gas.is_some() {
            let mut original=Self::new(self.id,self.definition.clone(),self.last_time)?.state.mass_kg();
            if self.definition.config.crop.is_none() {if let Some(c)=&s.crop {original+=c.initial_nutrient_equivalent_kg;}}
            original += self.totals.inflow_gas.map(|g|g.mass_kg()).unwrap_or(0.0);
            if (s.accounted_mass_kg()-original).abs()>1e-8*original.max(1.0) {return Err("Accounted cabin + escaped mass disagrees with original finite inventory".into());}
        }
        let t = &self.totals;
        for value in [
            t.generated_j,
            t.delivered_j,
            t.losses_j,
            t.curtailed_j,
            t.unserved_j,
            t.metabolic_person_seconds,
            t.unserved_person_seconds,
            t.reaction_enthalpy_j,
            t.sabatier_co2_consumed_kg,
            t.sabatier_heat_j,
            t.sabatier_water_processed_kg, t.condensate_processed_kg,t.crew_vapor_kg,t.condensed_kg,t.breach_enthalpy_j,
            t.inflow_enthalpy_j,
            t.maintenance_hours,
        ] {
            bounds(value, 0.0, 1.0e300, "Cumulative habitat budget")?;
        }
        bounds(t.wall_condensed_kg, -1.0e12, 1.0e12, "Cumulative wall exchange")?;
        if let Some(g)=t.leaked_gas {g.validate()?;}
        if let Some(g)=t.inflow_gas {g.validate()?;}
        if self.alert_bits >= 128 {
            return Err("Unsupported habitat alert bits".into());
        }
        if let Some(b) = &self.budget.electrolysis {
            b.validate(self.budget.seconds)?;
            if self.budget.configuration.as_ref().and_then(|c|c.electrolysis.as_ref()).is_none() {
                return Err("Electrolysis interval lacks its equipment configuration".into());
            }
        }
        if let Some(b) = &self.budget.sabatier {
            b.validate(self.budget.seconds)?;
            let c=self.budget.configuration.as_ref().and_then(|c|c.sabatier.as_ref()).ok_or("Sabatier interval lacks its equipment configuration")?;
            if b.requested_w>c.power_limit_w+1e-6 || b.co2_consumed_kg>c.co2_rate_kg_s*self.budget.seconds+1e-9
                || b.delivered_w>self.budget.delivered_w+1e-6
                || (b.electricity_j-b.co2_consumed_kg*c.electricity_j_kg_co2).abs()>1e-8*b.electricity_j.max(1.0) {
                return Err("Sabatier interval disagrees with retained equipment or bus budget".into());
            }
        }
        if let Some(b)=&self.budget.moisture {
            let c=self.budget.configuration.as_ref().and_then(|c|c.moisture.as_ref()).ok_or("Moisture interval lacks its configuration")?;
            b.validate(c,self.budget.seconds)?;
            let cabin=self.budget.configuration.as_ref().unwrap();
            let expected=cabin.crew as f64*self.budget.seconds/SECONDS_PER_DAY*self.budget.served_cycle_fraction*2.946*c.airborne_fraction;
            if (expected-b.crew_vapor_kg).abs()>1e-8*expected.max(1.0) {return Err("Moisture crew partition disagrees with retained interval".into());}
            if b.delivered_w>self.budget.delivered_w+1e-6 {return Err("Moisture interval exceeds bus budget".into());}
        }
        if let Some(b)=&self.budget.breach {
            let cabin=self.budget.configuration.as_ref().ok_or("Breach interval lacks cabin configuration")?;
            let c=cabin.breach.as_ref().ok_or("Breach interval lacks breach configuration")?;
            b.validate(c,self.budget.seconds,cabin.volume_m3,cabin.heat_capacity_j_k)?;
            let escaped=s.escaped_gas.unwrap_or_default();
            for (total,interval) in [(escaped.oxygen_kg,b.escaped.oxygen_kg),(escaped.nitrogen_kg,b.escaped.nitrogen_kg),(escaped.co2_kg,b.escaped.co2_kg),(escaped.vapor_kg,b.escaped.vapor_kg),(t.breach_enthalpy_j,b.outflow_enthalpy_j.max(0.0))] {if total+1e-8*total.max(1.0)<interval {return Err("Breach cumulative history below completed transfer".into());}}
            let inflow=t.inflow_gas.unwrap_or_default();
            for (total,interval) in [(inflow.oxygen_kg,b.inflow.oxygen_kg),(inflow.nitrogen_kg,b.inflow.nitrogen_kg),(inflow.co2_kg,b.inflow.co2_kg),(inflow.vapor_kg,b.inflow.vapor_kg),(t.inflow_enthalpy_j,(-b.outflow_enthalpy_j).max(0.0))] {if total+1e-8*total.max(1.0)<interval {return Err("Breach cumulative inflow history below completed transfer".into());}}

            for (now,start,out,inn) in [(s.oxygen_kg,b.initial_gas.oxygen_kg,b.escaped.oxygen_kg,b.inflow.oxygen_kg),(s.nitrogen_kg,b.initial_gas.nitrogen_kg,b.escaped.nitrogen_kg,b.inflow.nitrogen_kg),(s.co2_kg,b.initial_gas.co2_kg,b.escaped.co2_kg,b.inflow.co2_kg),(s.water_vapor_kg,b.initial_gas.vapor_kg,b.escaped.vapor_kg,b.inflow.vapor_kg)] {
                if (now-start+out-inn).abs()>1e-8*start.max(1.0) {return Err("Breach final cabin stocks disagree with retained transfer".into());}
            }
            if s.temperature_k!=b.final_temperature_k {return Err("Breach final temperature disagrees with cabin".into());}
        }
        for (actual,expected) in [(s.water_vapor_kg,t.crew_vapor_kg+s.crop.as_ref().map_or(0.,|c|c.vapor_generated_kg)-t.condensed_kg-t.wall_condensed_kg-s.escaped_gas.map(|g|g.vapor_kg).unwrap_or(0.0)),(s.condensate_kg,t.condensed_kg-t.condensate_processed_kg),(s.wall_liquid_kg,t.wall_condensed_kg)] {
            if (actual-expected).abs()>1e-8*actual.abs().max(expected.abs()).max(t.crew_vapor_kg).max(t.condensed_kg).max(1.0) {return Err("Moisture stores disagree with cumulative transfer history".into());}
        }
        if let Some(b)=&self.budget.wall {
            let c=self.budget.configuration.as_ref().and_then(|c|c.wall_condensation.as_ref()).ok_or("Wall interval lacks its configuration")?;
            b.validate(c,self.budget.seconds)?;
        }
        if let Some(b)=&self.budget.leak {
            let c=self.budget.configuration.as_ref().and_then(|c|c.leak.as_ref()).ok_or("Leak interval lacks its configuration")?;
            b.validate(c,self.budget.seconds)?;
            if let Some(g)=t.leaked_gas {
                for (total,interval) in [(g.oxygen_kg,b.leaked.oxygen_kg),(g.nitrogen_kg,b.leaked.nitrogen_kg),(g.co2_kg,b.leaked.co2_kg),(g.vapor_kg,b.leaked.vapor_kg)] {if total+1e-8*total.max(1.0)<interval {return Err("Leak cumulative history below completed transfer".into());}}
            } else {return Err("Leak interval lacks cumulative leaked ledger".into());}
        }
        if let Some(b)=&self.budget.maintenance {
            let c=self.budget.configuration.as_ref().and_then(|c|c.maintenance.as_ref()).ok_or("Maintenance interval lacks its configuration")?;
            b.validate(c,self.budget.seconds)?;
        }
        if let Some(b)=&self.budget.construction {
            let c=self.budget.configuration.as_ref().and_then(|c|c.construction.as_ref()).ok_or("Construction interval lacks its configuration")?;
            b.validate(c,b.material_after_kg)?;
        }
        if let Some(b)=&self.budget.calibration {
            let c=self.budget.configuration.as_ref().and_then(|c|c.calibration.as_ref()).ok_or("Calibration interval lacks its configuration")?;
            b.validate(c)?;
        }
        bounds(t.cumulative_dose_msv,0.0,1.0e9,"Cumulative crew dose")?;
        if let Some(b)=&self.budget.radiation {
            let c=self.budget.configuration.as_ref().and_then(|c|c.radiation.as_ref()).ok_or("Radiation interval lacks its configuration")?;
            b.validate(c,self.budget.seconds)?;
            if t.cumulative_dose_msv + 1.0e-9 < b.interval_dose_msv {return Err("Cumulative crew dose below completed interval transfer".into());}
        }
        if let Some(k)=self.config.construction.as_ref().or(self.definition.config.construction.as_ref()) {
            bounds(s.construction_material_kg,0.0,k.material_capacity_kg,"Construction material")?;
        }
        if let Some(ms)=&s.maintenance {
            if let Some(m)=self.config.maintenance.as_ref().or(self.definition.config.maintenance.as_ref()) {
                for (slot,(config,name)) in ms.slots().into_iter().zip(m.components()) {
                    if let (Some(slot),Some(c))=(slot,config) {slot.validate(c).map_err(|e|format!("{name}: {e}"))?;}
                }
            }
        }
        bounds(self.budget.condensate_processed_kg,0.0,self.budget.water_processed_kg,"Condensate processed")?;
        bounds(self.budget.condensate_recovered_kg,0.0,self.budget.condensate_processed_kg,"Condensate recovered")?;
        let reacted = t.sabatier_co2_consumed_kg;
        for (actual,expected,name) in [
            (s.methane_kg,reacted*crate::sabatier::METHANE_PER_CO2,"methane"),
            (s.sabatier_water_kg,reacted*crate::sabatier::WATER_PER_CO2-t.sabatier_water_processed_kg,"raw water"),
            (t.sabatier_heat_j,reacted*crate::sabatier::REFERENCE_HEAT_J_KG_CO2,"reaction heat"),
        ] {
            if (actual-expected).abs()>1e-8*actual.abs().max(expected.abs()).max(reacted).max(1.0) {return Err(format!("Sabatier {name} disagrees with cumulative reaction history"));}
        }
        bounds(self.budget.sabatier_water_processed_kg,0.0,self.budget.water_processed_kg,"Raw product water processed")?;
        bounds(self.budget.sabatier_water_recovered_kg,0.0,self.budget.sabatier_water_processed_kg,"Raw product water recovered")?;
        let expected_hydrogen = self.totals.reaction_enthalpy_j
            / crate::electrolysis::REFERENCE_ENTHALPY_J_KG_OXYGEN
            * crate::electrolysis::HYDROGEN_PER_OXYGEN - reacted*crate::sabatier::HYDROGEN_PER_CO2;
        if (s.hydrogen_kg - expected_hydrogen).abs() > 1e-8 * expected_hydrogen.abs().max(reacted).max(1.0) {
            return Err("Held hydrogen disagrees with cumulative reaction transfer".into());
        }
        bounds(self.budget.seconds, 0.0, PERIOD + 0.001, "Budget interval")?;
        bounds(
            self.budget.served_cycle_fraction,
            0.0,
            1.0,
            "Served reference-cycle fraction",
        )?;
        if let Some(c) = &self.budget.configuration {
            c.validate()?;
        }
        for value in [
            self.budget.generated_w,
            self.budget.requested_w,
            self.budget.delivered_w,
            self.budget.unserved_w,
            self.budget.battery_losses_w,
            self.budget.curtailed_w,
            self.budget.oxygen_makeup_kg,
            self.budget.co2_captured_kg,
            self.budget.water_processed_kg,
            self.budget.water_recovered_kg,
            self.budget.metabolic_heat_w,
            self.budget.heater_w,
            self.budget.radiator_rejection_w,
        ] {
            bounds(value, 0.0, 1.0e30, "Interval habitat budget")?;
        }
        for value in [
            self.budget.conduction_w,
            self.budget.thermal_storage_w,
            self.budget.mass_residual_kg,
            self.budget.electric_residual_j,
            self.budget.thermal_residual_j,
        ] {
            bounds(value, -1.0e30, 1.0e30, "Signed interval habitat budget")?;
        }
        if let Some(v) = self.budget.horizontal_irradiance_w_m2 {
            bounds(v, 0.0, 1.0e8, "Recorded irradiation")?;
        }
        if let Some(v) = self.budget.exterior_temperature_k {
            bounds(v, 1.0, 5000.0, "Recorded exterior temperature")?;
        }
        if self.budget.environment_available
            && (self.budget.horizontal_irradiance_w_m2.is_none()
                || self.budget.exterior_temperature_k.is_none())
        {
            return Err("Available habitat environment lacks inputs".into());
        }
        Ok(())
    }
    pub fn check_config(&self, c: &Config) -> Result<(), String> {
        c.validate()?;
        if let Some(crop)=&c.crop {self.state.crop.as_ref().ok_or("Install crops through the greenhouse installation edit; configuration does not grant initial nutrients")?.check_config(crop)?;if self.state.oxygen_reserve_kg>crop.oxygen_capacity_kg{return Err("New crop product capacity is below held oxygen; no gas discarded".into());}} else if self.state.crop.is_some() && self.definition.config.crop.is_none() {return Err("A runtime-installed greenhouse cannot be disabled; its operational configuration stays with the installation".into());}
        if c.battery_capacity_j < self.state.battery_j {
            return Err(
                "New battery capacity is below stored energy; no energy was discarded".into(),
            );
        }
        if c.capture_capacity_kg < self.state.captured_co2_kg {
            return Err(
                "New capture capacity is below already captured CO₂; no mass was discarded".into(),
            );
        }
        if let Some(e) = &c.electrolysis {
            if e.oxygen_capacity_kg < self.state.oxygen_reserve_kg || e.hydrogen_capacity_kg < self.state.hydrogen_kg {
                return Err("New electrolysis product capacity is below held O₂ or H₂; no mass was discarded".into());
            }
        }
        if let Some(e) = &c.sabatier {
            if e.methane_capacity_kg < self.state.methane_kg || e.water_capacity_kg < self.state.sabatier_water_kg {
                return Err("New Sabatier capacity is below held methane or raw water; no mass was discarded".into());
            }
        }
        if let Some(m)=&c.moisture {if m.condensate_capacity_kg<self.state.condensate_kg {return Err("New condensate capacity is below held liquid; no water discarded".into());}}
        if let Some(k)=&c.construction {if k.material_capacity_kg<self.state.construction_material_kg {return Err("New construction material capacity is below held stock; no material discarded".into());}}
        if let Some(w)=&c.wall_condensation {if w.liquid_capacity_kg<self.state.wall_liquid_kg {return Err("New wall liquid capacity is below held film; no water discarded".into());}}
        for (held, capacity, n) in [
            (
                self.state.oxygen_reserve_kg,
                c.oxygen_reserve_capacity_kg,
                "O₂ reserve storage",
            ),
            (
                self.state.potable_water_kg,
                c.potable_water_capacity_kg,
                "Potable water storage",
            ),
            (
                self.state.food_dry_kg,
                c.food_dry_capacity_kg,
                "Dry food storage",
            ),
            (
                self.state.food_water_kg,
                c.food_water_capacity_kg,
                "Food water storage",
            ),
        ] {
            if capacity < held {
                return Err(format!(
                    "New {n} capacity is below held stock; no mass was discarded"
                ));
            }
        }
        Ok(())
    }
    /// Current held supply stocks in the transferable species mapping.
    pub fn held_cargo(&self) -> crate::supply_vehicle::Cargo {
        crate::supply_vehicle::Cargo {
            oxygen_reserve_kg: self.state.oxygen_reserve_kg,
            potable_water_kg: self.state.potable_water_kg,
            food_dry_kg: self.state.food_dry_kg,
            food_water_kg: self.state.food_water_kg,
        }
    }
    /// Finite general storage capacities for the same species mapping.
    pub fn storage_capacity_cargo(&self) -> crate::supply_vehicle::Cargo {
        crate::supply_vehicle::Cargo {
            oxygen_reserve_kg: self.config.oxygen_reserve_capacity_kg,
            potable_water_kg: self.config.potable_water_capacity_kg,
            food_dry_kg: self.config.food_dry_capacity_kg,
            food_water_kg: self.config.food_water_capacity_kg,
        }
    }
    /// Free space per species: capacity minus held stock, never negative.
    pub fn available_cargo(&self) -> crate::supply_vehicle::Cargo {
        let capacity = self.storage_capacity_cargo();
        let held = self.held_cargo();
        crate::supply_vehicle::Cargo {
            oxygen_reserve_kg: (capacity.oxygen_reserve_kg - held.oxygen_reserve_kg).max(0.0),
            potable_water_kg: (capacity.potable_water_kg - held.potable_water_kg).max(0.0),
            food_dry_kg: (capacity.food_dry_kg - held.food_dry_kg).max(0.0),
            food_water_kg: (capacity.food_water_kg - held.food_water_kg).max(0.0),
        }
    }
    pub fn uses_crop(&self)->bool {self.config.crop.is_some() || self.definition.config.crop.is_some() || self.state.crop.is_some() || self.budget.crop.is_some()}
    pub fn uses_breach(&self)->bool {self.config.breach.is_some() || self.definition.config.breach.is_some() || self.budget.breach.is_some() || self.state.escaped_gas.is_some() || self.totals.breach_enthalpy_j>0.0}
    pub fn uses_moisture(&self)->bool {self.config.moisture.is_some() || self.definition.config.moisture.is_some() || self.budget.moisture.is_some() || self.totals.crew_vapor_kg>0.0 || self.state.water_vapor_kg>0.0 || self.state.condensate_kg>0.0 || self.uses_wall()}
    pub fn uses_wall(&self)->bool {self.config.wall_condensation.is_some() || self.definition.config.wall_condensation.is_some() || self.budget.wall.is_some() || self.state.wall_liquid_kg!=0.0 || self.totals.wall_condensed_kg!=0.0}
    pub fn uses_leak(&self)->bool {self.config.leak.is_some() || self.definition.config.leak.is_some() || self.budget.leak.is_some() || self.totals.leaked_gas.is_some()}
    pub fn uses_maintenance(&self)->bool {self.config.maintenance.is_some() || self.definition.config.maintenance.is_some() || self.budget.maintenance.is_some() || self.state.maintenance.is_some() || self.totals.maintenance_hours>0.0}
    pub fn uses_construction(&self)->bool {self.config.construction.is_some() || self.definition.config.construction.is_some() || self.budget.construction.is_some() || self.state.construction_material_kg>0.0 || self.state.construction_structure_kg>0.0 || self.totals.constructed_volume_m3>0.0 || self.totals.construction_material_used_kg>0.0}
    pub fn uses_calibration(&self)->bool {self.config.calibration.is_some() || self.definition.config.calibration.is_some() || self.budget.calibration.is_some()}
    pub fn uses_radiation(&self)->bool {self.config.radiation.is_some() || self.definition.config.radiation.is_some() || self.budget.radiation.is_some() || self.totals.cumulative_dose_msv>0.0}
    pub fn uses_sabatier(&self) -> bool {
        self.config.sabatier.is_some() || self.definition.config.sabatier.is_some() || self.budget.sabatier.is_some()
            || self.state.methane_kg>0.0 || self.state.sabatier_water_kg>0.0 || self.totals.sabatier_co2_consumed_kg>0.0
    }
    pub fn uses_electrolysis(&self) -> bool {
        self.config.electrolysis.is_some() || self.definition.config.electrolysis.is_some()
            || self.budget.electrolysis.is_some() || self.state.hydrogen_kg > 0.0 || self.totals.reaction_enthalpy_j > 0.0
    }
    /// Only fixed scheduler intervals or explicit edit-time remainders enter here.
    pub fn step(&mut self, time: f64, env: Environment) -> Result<(), String> {
        if let Some(c)=&self.config.crop {self.state.crop.as_ref().ok_or("Crop state is not installed")?.check_config(c)?;}
        let dt = time - self.last_time;
        if !dt.is_finite() || !(0.0..=PERIOD + 0.001).contains(&dt) {
            return Err("Habitat interval must be forward and at most 60 seconds".into());
        }
        if dt == 0.0 {
            return Ok(());
        }
        let (Some(flux), Some(exterior)) = (env.irradiance_w_m2, env.exterior_temperature_k) else {
            self.budget = Budget {
                seconds: dt,
                temperature_domain_supported: (100.0..=500.0).contains(&self.state.temperature_k),
                ..Default::default()
            };
            if let Some(heat)=&mut self.heat {heat.exclude(dt,false);}
            self.last_time = time;
            return Ok(());
        };
        bounds(flux, 0.0, 1.0e8, "Local irradiance")?;
        bounds(exterior, 1.0, 5000.0, "Exterior model temperature")?;
        if let Some(dose) = env.exterior_dose_rate_msv_day {
            bounds(dose, 0.0, 1.0e4, "Exterior dose rate")?;
        }
        if !(100.0..=500.0).contains(&self.state.temperature_k) {
            self.budget = Budget {
                seconds: dt,
                environment_available: true,
                horizontal_irradiance_w_m2: Some(flux),
                exterior_temperature_k: Some(exterior),
                configuration: Some(self.config.clone()),
                ..Default::default()
            };
            if let Some(heat)=&mut self.heat {heat.exclude(dt,true);}
            self.last_time = time;
            return Ok(());
        }
        let c = &self.config;
        let cal = c.calibration.as_ref();
        let crew_factor = cal.map_or(1.0, |k| k.crew_demand.central);
        let scrubber_factor = cal.map_or(1.0, |k| k.scrubber.central);
        let recycler_factor = cal.map_or(1.0, |k| k.recycler.central);
        let makeup_factor = cal.map_or(1.0, |k| k.oxygen_makeup.central);
        let s = &mut self.state;
        let old_mass = s.accounted_mass_kg();
        let old_battery = s.battery_j;
        let old_temperature = s.temperature_k;
        let crop_start=(s.potable_water_kg,s.captured_co2_kg,s.oxygen_reserve_kg);
        // Scheduled equipment maintenance plans from start-of-interval condition/cycle;
        // wear/restore settle after power allocation once served fractions are known.
        if c.maintenance.is_some() && s.maintenance.is_none() {s.maintenance=Some(Default::default());}
        let maintenance_effects=c.maintenance.as_ref().map(|m|crate::habitat_maintenance::plan(m,s.maintenance.as_ref(),dt));
        let fx=|i:usize| maintenance_effects.map(|a|a[i]).unwrap_or_default();
        // Nominal structural leak samples start-of-interval gas; mass leaves before the
        // breach substep so breach retained identities hold on the post-leak cabin.
        let leak = c.leak.as_ref().map(|l| {
            let entry = s.cabin_gas();
            let total = entry.mass_kg();
            let leaked_mass = (l.rate_kg_s * dt).min(total).max(0.0);
            let share = if total > 0.0 { leaked_mass / total } else { 0.0 };
            LeakBudget {
                initial_gas: entry,
                leaked: entry.fraction(share),
                temperature_k: s.temperature_k,
                cooling_j: leaked_mass * l.gas_specific_heat_j_kg * s.temperature_k,
            }
        });
        let crop_request=c.crop.as_ref().zip(s.crop.as_ref()).map_or(0.,|(c,state)|state.request_w(c,self.last_time,dt));
        let construction_request=c.construction.as_ref().map_or(0.,|k|k.power_w);
        let generated = c.firm_power_w
            + flux * c.solar_area_m2 * c.solar_efficiency * c.atmospheric_transmission;
        let heater_request =
            ((c.temperature_setpoint_k - s.temperature_k).max(0.0) * c.heat_capacity_j_k / dt)
                .min(c.heater_limit_w * fx(2).scale);
        let pump_request = if s.temperature_k > c.temperature_setpoint_k {
            c.radiator_pump_w
        } else {
            0.0
        };
        // Reserve nominal crew water before planning optional production. New recycle
        // water and freed product headroom wait one interval, so requests stay causal.
        let reactor_start_stocks = (s.potable_water_kg-c.crew as f64*crew_factor*3.217*dt/SECONDS_PER_DAY,s.oxygen_reserve_kg,s.hydrogen_kg);
        let planned_oxygen = c.electrolysis.as_ref().map(|e|e.planned_oxygen_kg(
            s.potable_water_kg-c.crew as f64*crew_factor*3.217*dt/SECONDS_PER_DAY,
            s.oxygen_reserve_kg,s.hydrogen_kg,dt)*fx(5).scale).unwrap_or(0.0);
        let reactor_request = c.electrolysis.as_ref().map(|e|
            (planned_oxygen*e.electricity_j_kg_oxygen/dt).min(e.power_limit_w)).unwrap_or(0.0);
        // Order is explicit: base services, CO2 capture, water recovery, heater, radiator,
        // then optional electrolysis and CO₂ reduction. Neither displaces existing life-support power.
        let sabatier_start = crate::sabatier::Stocks {co2_kg:s.captured_co2_kg,hydrogen_kg:s.hydrogen_kg,methane_kg:s.methane_kg,water_kg:s.sabatier_water_kg};
        let planned_co2 = c.sabatier.as_ref().map(|e|e.planned_co2_kg(sabatier_start,dt)*fx(6).scale).unwrap_or(0.0);
        let sabatier_request = c.sabatier.as_ref().map(|e|(planned_co2*e.electricity_j_kg_co2/dt).min(e.power_limit_w)).unwrap_or(0.0);
        // Condensation uses start vapor/headroom and comes after existing services/reactors.
        let condensate_start=s.condensate_kg;
        let moisture_plan=c.moisture.as_ref().map(|m|{let mut p=m.plan(s.water_vapor_kg,s.condensate_kg,s.temperature_k,c.volume_m3,dt);p.planned_kg*=fx(4).scale;p.requested_w=(p.planned_kg*m.electricity_j_kg/dt).min(m.power_limit_w);p});
        let moisture_request=moisture_plan.as_ref().map(|p|p.requested_w).unwrap_or(0.0);
        // Maintenance prorates each process slot by its operating share and adds only the
        // window's maintenance power; priority order and legacy no-maintenance path are unchanged.
        let scrubber_process_w=c.scrubber_power_w*fx(0).op_fraction;
        let recycler_process_w=c.recycler_power_w*fx(1).op_fraction;
        let heater_slot_w=heater_request*fx(2).op_fraction;
        let pump_slot_w=pump_request*fx(3).op_fraction;
        let reactor_slot_w=reactor_request*fx(5).op_fraction;
        let sabatier_slot_w=sabatier_request*fx(6).op_fraction;
        let moisture_slot_w=moisture_request*fx(4).op_fraction;
        let requests = [
            c.base_load_w,
            scrubber_process_w + fx(0).maintenance_w,
            recycler_process_w + fx(1).maintenance_w,
            heater_slot_w + fx(2).maintenance_w,
            pump_slot_w + fx(3).maintenance_w,
            reactor_slot_w + fx(5).maintenance_w,
            sabatier_slot_w + fx(6).maintenance_w,
            moisture_slot_w + fx(4).maintenance_w,
            crop_request,
            construction_request,
        ];
        let requested: f64 = requests.iter().sum();
        let discharge = (requested - generated)
            .max(0.0)
            .min(c.discharge_limit_w)
            .min(s.battery_j * c.discharge_efficiency / dt);
        let charge = (generated - requested)
            .max(0.0)
            .min(c.charge_limit_w)
            .min((c.battery_capacity_j - s.battery_j).max(0.0) / (c.charge_efficiency * dt));
        s.battery_j = (s.battery_j + charge * c.charge_efficiency * dt
            - discharge / c.discharge_efficiency * dt)
            .max(0.0);
        let mut available = generated + discharge;
        let mut supplied = [0.0; 10];
        for (i, request) in requests.iter().enumerate() {
            supplied[i] = available.min(*request);
            available -= supplied[i];
        }
        let delivered: f64 = supplied.iter().sum();
        let losses =
            charge * (1.0 - c.charge_efficiency) + discharge * (1.0 / c.discharge_efficiency - 1.0);
        let curtailed = (generated - delivered - charge).max(0.0);
        let unserved = (requested - delivered).max(0.0);
        let fraction = |i: usize| {
            if requests[i] > 0.0 {
                supplied[i] / requests[i]
            } else {
                0.0
            }
        };
        // Process work uses only the operating share of each prorated slot; maintenance
        // power is real delivered energy but performs no mass throughput.
        let proc_frac = |i: usize, proc_w: f64| if proc_w > 0.0 { fraction(i) } else { 0.0 };
        let proc_delivered = |i: usize, proc_w: f64| if requests[i] > 0.0 { supplied[i] * proc_w / requests[i] } else { 0.0 };
        let maintenance = c.maintenance.as_ref().map(|m| {
            let served=[fraction(1),fraction(2),fraction(3),fraction(4),fraction(7),fraction(5),fraction(6)];
            let (b,hours)=crate::habitat_maintenance::settle(m,s.maintenance.as_mut(),dt,served);
            self.totals.maintenance_hours+=hours.iter().sum::<f64>();
            b
        });
        // Construction consumes held material at the served power fraction; consumed mass
        // becomes structure mass here and the live volume grows only after this interval's
        // configuration has been retained in the budget.
        let mut construction_volume_add = 0.0;
        let construction = c.construction.as_ref().map(|k| {
            let served = fraction(9);
            let before = s.construction_material_kg;
            let (consumed, volume_added, volume_constraint) =
                crate::habitat_construction::draw(k, before, c.volume_m3, dt, served);
            s.construction_material_kg = (before - consumed).max(0.0);
            s.construction_structure_kg += consumed;
            self.totals.constructed_volume_m3 += volume_added;
            self.totals.construction_material_used_kg += consumed;
            construction_volume_add = volume_added;
            let mut constraints: Vec<String> = volume_constraint.into_iter().collect();
            if served < 1.0 {
                constraints.push("construction power curtailed".into());
            }
            crate::habitat_construction::Budget {
                material_before_kg: before,
                material_after_kg: s.construction_material_kg,
                consumed_kg: consumed,
                volume_added_m3: volume_added,
                requested_w: k.power_w,
                delivered_w: supplied[9],
                served_fraction: served,
                constraints,
            }
        });
        // A mechanical regulator transfers stored oxygen; it creates no new oxygen.
        let oxygen_target = c.oxygen_setpoint_pa * c.volume_m3 * O2 / (R * s.temperature_k);
        let makeup = (oxygen_target - s.oxygen_kg)
            .max(0.0)
            .min(c.oxygen_makeup_rate_kg_s * makeup_factor * dt)
            .min(s.oxygen_reserve_kg);
        s.oxygen_kg += makeup;
        s.oxygen_reserve_kg -= makeup;
        let demand = c.crew as f64 * crew_factor * dt / SECONDS_PER_DAY;
        let cycle = if demand > 0.0 {
            demand
                .min(s.oxygen_kg / 0.895)
                .min(s.potable_water_kg / 3.217)
                .min(s.food_dry_kg / 0.800)
                .min(s.food_water_kg / 0.760)
        } else {
            0.0
        };
        s.oxygen_kg = (s.oxygen_kg - cycle * 0.895).max(0.0);
        s.potable_water_kg = (s.potable_water_kg - cycle * 3.217).max(0.0);
        s.food_dry_kg = (s.food_dry_kg - cycle * 0.800).max(0.0);
        s.food_water_kg = (s.food_water_kg - cycle * 0.760).max(0.0);
        s.co2_kg += cycle * 1.085;
        let crew_vapor=c.moisture.as_ref().map(|m|cycle*2.946*m.airborne_fraction).unwrap_or(0.0);
        s.water_vapor_kg+=crew_vapor;
        s.wastewater_kg += cycle * 4.467-crew_vapor;
        s.dry_waste_kg += cycle * 0.120;
        let captured = (c.scrubber_rate_kg_s * scrubber_factor * dt * proc_frac(1,scrubber_process_w) * fx(0).op_fraction * fx(0).scale)
            .min(s.co2_kg)
            .min((c.capture_capacity_kg - s.captured_co2_kg).max(0.0));
        s.co2_kg -= captured;
        s.captured_co2_kg += captured;
        let processed = (c.recycler_rate_kg_s * recycler_factor * dt * proc_frac(2,recycler_process_w) * fx(1).op_fraction * fx(1).scale).min(s.wastewater_kg);
        // Recovery enters the finite potable tank; overflow stays in the processed
        // stream as brine, never discarded. Default 1e7 kg tanks never bind.
        let recovered = (processed * c.water_recovery_fraction)
            .min((c.potable_water_capacity_kg - s.potable_water_kg).max(0.0));
        s.wastewater_kg -= processed;
        s.potable_water_kg = (s.potable_water_kg + recovered).min(c.potable_water_capacity_kg);
        s.brine_kg += processed - recovered;
        // Raw product needs the existing powered processing assumption, after wastewater.
        // Products generated below are unavailable here until a later interval.
        let product_processed = (c.recycler_rate_kg_s*recycler_factor*dt*proc_frac(2,recycler_process_w)*fx(1).op_fraction*fx(1).scale-processed).max(0.0).min(sabatier_start.water_kg);
        let product_recovered = (product_processed*c.water_recovery_fraction)
            .min((c.potable_water_capacity_kg - s.potable_water_kg).max(0.0));
        s.sabatier_water_kg = (s.sabatier_water_kg-product_processed).max(0.0);
        s.potable_water_kg = (s.potable_water_kg + product_recovered).min(c.potable_water_capacity_kg);
        s.brine_kg += product_processed-product_recovered;
        let condensate_processed=(c.recycler_rate_kg_s*recycler_factor*dt*proc_frac(2,recycler_process_w)*fx(1).op_fraction*fx(1).scale-processed-product_processed).max(0.0).min(condensate_start);
        let condensate_recovered=(condensate_processed*c.water_recovery_fraction)
            .min((c.potable_water_capacity_kg - s.potable_water_kg).max(0.0));
        s.condensate_kg=(s.condensate_kg-condensate_processed).max(0.0);
        s.potable_water_kg=(s.potable_water_kg+condensate_recovered).min(c.potable_water_capacity_kg);
        s.brine_kg+=condensate_processed-condensate_recovered;
        let calibration = cal.map(|k| crate::habitat_calibration::Budget::new(
            k,
            cycle,
            makeup,
            captured,
            processed + product_processed + condensate_processed,
        ));
        let moisture=c.moisture.as_ref().zip(moisture_plan).map(|(m,p)|crate::moisture::Budget::from_power(m,p,proc_delivered(7,moisture_slot_w),crew_vapor,dt));
        let crew_phase=crew_vapor*crate::moisture::REFERENCE_PHASE_J_KG;
        let condensation_heat=moisture.as_ref().map(|m|m.condensation_heat_j).unwrap_or(0.0);
        if let Some(m)=&moisture {s.water_vapor_kg=(s.water_vapor_kg-m.condensed_kg).max(0.0);s.condensate_kg+=m.condensed_kg;self.totals.condensed_kg+=m.condensed_kg;}
        self.totals.crew_vapor_kg+=crew_vapor;
        self.totals.condensate_processed_kg+=condensate_processed;
        let metabolic = cycle * 12_426_000.0 / dt;
        let rejection_capacity = c.radiator_emissivity
            * c.radiator_area_m2
            * STEFAN_BOLTZMANN
            * (s.temperature_k.powi(4) - exterior.powi(4)).max(0.0)
            * proc_frac(4,pump_slot_w) * fx(3).op_fraction * fx(3).scale;
        let reactor = c.electrolysis.as_ref().map(|e| {
            let mut b = crate::electrolysis::Budget::from_power(e,planned_oxygen,reactor_request,proc_delivered(5,reactor_slot_w),dt);
            b.constraints = e.constraints(reactor_start_stocks.0,reactor_start_stocks.1,reactor_start_stocks.2,dt,reactor_request,supplied[5]);
            b
        });
        let reaction_enthalpy = reactor.as_ref().map(|b|b.reaction_enthalpy_j).unwrap_or(0.0);
        if let Some(b) = &reactor {
            s.potable_water_kg = (s.potable_water_kg-b.water_consumed_kg).max(0.0);
            s.oxygen_reserve_kg += b.oxygen_generated_kg;
            s.hydrogen_kg += b.hydrogen_generated_kg;
        }
        let reduction = c.sabatier.as_ref().map(|e| {
            let mut b=crate::sabatier::Budget::from_power(e,planned_co2,sabatier_request,proc_delivered(6,sabatier_slot_w),dt);
            b.constraints=e.constraints(sabatier_start,dt,sabatier_request,supplied[6]); b
        });
        let reaction_heat = reduction.as_ref().map(|b|b.reaction_heat_j).unwrap_or(0.0);
        if let Some(b)=&reduction {
            s.captured_co2_kg=(s.captured_co2_kg-b.co2_consumed_kg).max(0.0);
            s.hydrogen_kg=(s.hydrogen_kg-b.hydrogen_consumed_kg).max(0.0);
            s.methane_kg+=b.methane_generated_kg;
            s.sabatier_water_kg+=b.water_generated_kg;
            self.totals.sabatier_co2_consumed_kg+=b.co2_consumed_kg;
        }
        // Crop transfer uses only feeds/headroom present at interval start after prior
        // claims. Newly captured CO2/recycled water/freed reserve space wait one minute.
        let crop=c.crop.as_ref().zip(s.crop.as_mut()).map(|(crop,state)| {
            let capacity=crop.oxygen_capacity_kg.min(c.electrolysis.as_ref().map_or(crop.oxygen_capacity_kg,|e|e.oxygen_capacity_kg));
            let stocks=crate::crop_process::Stocks {water_kg:s.potable_water_kg.min((crop_start.0-c.crew as f64*crew_factor*3.217*dt/SECONDS_PER_DAY-reactor.as_ref().map_or(0.,|b|b.water_consumed_kg)).max(0.)),co2_kg:s.captured_co2_kg.min((crop_start.1-reduction.as_ref().map_or(0.,|b|b.co2_consumed_kg)).max(0.)),oxygen_headroom_kg:(capacity-s.oxygen_reserve_kg).max(0.).min((capacity-crop_start.2-reactor.as_ref().map_or(0.,|b|b.oxygen_generated_kg)).max(0.))};
            state.execute(crop,self.last_time,dt,supplied[8],stocks)
        }).transpose()?;
        let crop_debit=crop.as_ref().map_or(0.,|b|b.thermal_debit_j());
        if let Some(b)=&crop {s.potable_water_kg=(s.potable_water_kg-b.water_consumed_kg()).max(0.);s.captured_co2_kg=(s.captured_co2_kg-b.co2_consumed_kg).max(0.);s.oxygen_reserve_kg+=b.oxygen_generated_kg;s.water_vapor_kg+=b.vapor_generated_kg;}
        // Passive wall condensation/evaporation runs on end-of-interval vapor after
        // equipment/crop transfers; its phase heat enters the thermal balance below.
        let wall = c.wall_condensation.as_ref().map(|w|w.exchange(s.water_vapor_kg,s.wall_liquid_kg,s.temperature_k,exterior,c.volume_m3,dt));
        if let Some((net,_))=&wall {s.water_vapor_kg=(s.water_vapor_kg-net).max(0.0);s.wall_liquid_kg=(s.wall_liquid_kg+net).max(0.0);self.totals.wall_condensed_kg+=net;}
        let wall_heat=wall.as_ref().map(|x| x.1.condensation_heat_j).unwrap_or(0.0);
        let leak_cooling=leak.as_ref().map(|b|b.cooling_j).unwrap_or(0.0);
        // Reaction enthalpy leaves the thermal balance; the remaining reactor electricity
        // is waste heat. Never count all bus electricity as heat AND as reaction energy.
        let gross = delivered + losses + metabolic - reaction_enthalpy/dt + reaction_heat/dt - crew_phase/dt + condensation_heat/dt - crop_debit/dt + wall_heat/dt - leak_cooling/dt;
        let rejection = rejection_capacity.min(
            ((s.temperature_k - c.temperature_setpoint_k).max(0.0) * c.heat_capacity_j_k / dt
                + gross)
                .max(0.0),
        );
        let net = gross - rejection;
        // Exact lumped linear conduction for constant interval heat load. Unlike Euler,
        // this remains stable for all accepted conductance/capacity combinations.
        let x = c.conductance_w_k * dt / c.heat_capacity_j_k;
        // Stable (1-exp(-x))/x near zero; the cubic series' omitted term is <1e-22.
        let phi = if x < 1.0e-5 {
            1.0 - x / 2.0 + x * x / 6.0 - x * x * x / 24.0
        } else {
            (1.0 - (-x).dexp()) / x
        };
        s.temperature_k = old_temperature
            + (exterior - old_temperature) * x * phi
            + net * dt / c.heat_capacity_j_k * phi;
        // Structural leak transfers join the cumulative outside ledger before the breach
        // substep so both identities compose within one interval.
        if let Some(b)=&leak {
            s.oxygen_kg=(s.oxygen_kg-b.leaked.oxygen_kg).max(0.0);
            s.nitrogen_kg=(s.nitrogen_kg-b.leaked.nitrogen_kg).max(0.0);
            s.co2_kg=(s.co2_kg-b.leaked.co2_kg).max(0.0);
            s.water_vapor_kg=(s.water_vapor_kg-b.leaked.vapor_kg).max(0.0);
            s.escaped_gas.get_or_insert_with(Default::default).add(b.leaked);
            self.totals.leaked_gas.get_or_insert_with(Default::default).add(b.leaked);
        }
        // The existing thermal/process minute precedes the bounded finite breach substep.
        let breach=c.breach.as_ref().map(|e|crate::breach::integrate(e,s.cabin_gas(),s.temperature_k,c.volume_m3,c.heat_capacity_j_k,dt));
        let cooling=breach.as_ref().map(|b|b.lumped_cooling_j).unwrap_or(0.0);
        if let Some(b)=&breach {
            s.oxygen_kg=(s.oxygen_kg-b.escaped.oxygen_kg+b.inflow.oxygen_kg).max(0.0);
            s.nitrogen_kg=(s.nitrogen_kg-b.escaped.nitrogen_kg+b.inflow.nitrogen_kg).max(0.0);
            s.co2_kg=(s.co2_kg-b.escaped.co2_kg+b.inflow.co2_kg).max(0.0);
            s.water_vapor_kg=(s.water_vapor_kg-b.escaped.vapor_kg+b.inflow.vapor_kg).max(0.0);
            s.escaped_gas.get_or_insert_with(Default::default).add(b.escaped);
            if b.inflow.mass_kg()>0.0 {self.totals.inflow_gas.get_or_insert_with(Default::default).add(b.inflow);}
            s.temperature_k=b.final_temperature_k;
            self.totals.breach_enthalpy_j+=b.outflow_enthalpy_j.max(0.0);
            self.totals.inflow_enthalpy_j+=(-b.outflow_enthalpy_j).max(0.0);
        }
        // Exterior radiation dose is derived bookkeeping after the existing thermal,
        // process and breach steps; it changes no mass, power or heat.
        let radiation=c.radiation.as_ref().and_then(|r|env.exterior_dose_rate_msv_day.map(|d|crate::habitat_radiation::Budget::new(r,d,dt)));
        if let Some(rb)=&radiation {self.totals.cumulative_dose_msv+=rb.interval_dose_msv;}
        let storage = c.heat_capacity_j_k * (s.temperature_k - old_temperature) / dt;
        let conduction = c.conductance_w_k * (exterior - old_temperature) * phi - net * (1.0 - phi);
        let electrical_residual =
            s.battery_j - old_battery - (generated - delivered - losses - curtailed) * dt;
        self.budget = Budget {
            seconds: dt,
            configuration: Some(c.clone()),
            environment_available: true,
            temperature_domain_supported: (100.0..=500.0).contains(&s.temperature_k),
            horizontal_irradiance_w_m2: Some(flux),
            exterior_temperature_k: Some(exterior),
            generated_w: generated,
            requested_w: requested,
            delivered_w: delivered,
            unserved_w: unserved,
            battery_losses_w: losses,
            curtailed_w: curtailed,
            oxygen_makeup_kg: makeup,
            co2_captured_kg: captured,
            water_processed_kg: processed+product_processed+condensate_processed,
            water_recovered_kg: recovered+product_recovered+condensate_recovered,
            served_cycle_fraction: if demand > 0.0 { cycle / demand } else { 1.0 },
            metabolic_heat_w: metabolic,
            heater_w: supplied[3],
            radiator_rejection_w: rejection,
            conduction_w: conduction,
            thermal_storage_w: storage,
            mass_residual_kg: s.accounted_mass_kg() - old_mass,
            electric_residual_j: electrical_residual,
            thermal_residual_j: (storage - net - conduction) * dt + cooling,
            crop,
            electrolysis: reactor,
            sabatier: reduction,
            breach,
            moisture,condensate_processed_kg:condensate_processed,condensate_recovered_kg:condensate_recovered,
            wall: wall.map(|x| x.1),
            leak,
            maintenance,
            construction,
            calibration,
            radiation,
            sabatier_water_processed_kg: product_processed,
            sabatier_water_recovered_kg: product_recovered,
        };
        if construction_volume_add > 0.0 {
            self.config.volume_m3 += construction_volume_add;
        }
        if let Some(heat)=&mut self.heat {heat.record(&self.budget);}
        self.totals.reaction_enthalpy_j += reaction_enthalpy;
        self.totals.sabatier_heat_j += reaction_heat;
        self.totals.sabatier_water_processed_kg += product_processed;
        self.totals.generated_j += generated * dt;
        self.totals.delivered_j += delivered * dt;
        self.totals.losses_j += losses * dt;
        self.totals.curtailed_j += curtailed * dt;
        self.totals.unserved_j += unserved * dt;
        self.totals.metabolic_person_seconds += cycle * SECONDS_PER_DAY;
        self.totals.unserved_person_seconds += (demand - cycle) * SECONDS_PER_DAY;
        self.last_time = time;
        Ok(())
    }
    pub fn alerts(&self) -> u16 {
        let s = &self.state;
        let c = &self.config;
        let mut bits = 0;
        for (i, condition) in [
            !self.budget.environment_available,
            self.budget.unserved_w > 1.0,
            s.oxygen_pa(c) < c.oxygen_alert_pa,
            s.co2_pa(c) > c.co2_alert_pa,
            s.temperature_k < c.cold_alert_k || s.temperature_k > c.hot_alert_k,
            self.budget.served_cycle_fraction < 0.999999,
            !(100.0..=500.0).contains(&s.temperature_k),
            c.radiation.as_ref().is_some_and(|r| self.budget.radiation.as_ref().is_some_and(|b| b.interior_dose_rate_msv_day > r.alert_dose_rate_msv_day)),
        ]
        .into_iter()
        .enumerate()
        {
            if condition {
                bits |= 1 << i;
            }
        }
        bits
    }
    pub fn report(&self) -> serde_json::Value {
        let mut report=serde_json::json!({"format":"cosmogon-habitat-report","schema_version":1,"habitat":self,"diagnostics":crate::habitat_diagnostics::explain(self),"oxygen_partial_pressure_pa":self.state.oxygen_pa(&self.config),"co2_partial_pressure_pa":self.state.co2_pa(&self.config),"total_pressure_pa":self.state.pressure_pa(&self.config),"data_states":{"crew_rates":"Published nominal reference model (NASA BVAD), not individual measurements","equipment_and_initial_stocks":"User-assumed design inputs","current_stores_and_budgets":"Modelled","exterior_temperature":"Global reduced climate model; local measured weather unavailable","horizontal_irradiance":"Derived from primary-star state and approximate body frame"},"electrolysis_source":self.uses_electrolysis().then(crate::electrolysis::source),"sabatier_source":self.uses_sabatier().then(crate::sabatier::source),"moisture_source":self.uses_moisture().then(crate::moisture::source),"breach_source":self.uses_breach().then(crate::breach::source),"moisture_observation":self.uses_moisture().then(||serde_json::json!({"vapor_pressure_pa":self.state.vapor_pressure_pa(&self.config),"relative_humidity":self.state.relative_humidity(&self.config),"reference_phase_energy_j":self.state.phase_energy_j(),"supersaturated":self.state.relative_humidity(&self.config).map(|v|v>1.0)})),"wall_observation":self.uses_wall().then(||serde_json::json!({"wall_liquid_kg":self.state.wall_liquid_kg,"cumulative_net_condensed_kg":self.totals.wall_condensed_kg,"model":crate::moisture::WALL_MODEL})),"leak_observation":self.uses_leak().then(||serde_json::json!({"leaked_gas":self.totals.leaked_gas,"model":LEAK_MODEL})),"leak_source":self.uses_leak().then(crate::moisture::leak_source),"maintenance_observation":self.uses_maintenance().then(||serde_json::json!({"components":self.state.maintenance,"cumulative_maintenance_hours":self.totals.maintenance_hours,"model":crate::habitat_maintenance::MAINTENANCE_MODEL})),"maintenance_source":self.uses_maintenance().then(crate::habitat_maintenance::source),"construction_observation":self.uses_construction().then(||serde_json::json!({"held_material_kg":self.state.construction_material_kg,"structure_mass_kg":self.state.construction_structure_kg,"cumulative_volume_added_m3":self.totals.constructed_volume_m3,"cumulative_material_used_kg":self.totals.construction_material_used_kg,"live_volume_m3":self.config.volume_m3,"model":crate::habitat_construction::CONSTRUCTION_MODEL})),"construction_source":self.uses_construction().then(crate::habitat_construction::source),"uncertainty":null,"uncertainty_scope":"Input and predictive model uncertainty are not quantified","source":{"url":SOURCE,"version":"NASA BVAD Rev2 February 2022, Table 3-31","equipment":"User-assumed capacities, powers, efficiencies and alert thresholds"},"limits":"Reduced nominal demand and lumped thermal model, not validated mission hardware, local weather, purification quality, medical response or safety certification. Optional reduced humidity budget, passive lumped wall condensation (no ice), nominal proportional structural leak and scheduled equipment wear/maintenance with authored capacity calibration; no greenhouse exchange, seal/rarefied leakage, EVA or resupply. Exterior radiation environment, shielding and interior dose are optional and separately sourced; no solar-particle-event forecast, particle transport or activation. Continuous cabin construction from held material with an authored bulk ratio is optional and separately sourced; no structural strength, assembly physics, dust, crew labour or vehicle-mediated material delivery."});
        if self.heat.is_some() {report["heat_accounting"]=crate::habitat_heat::habitat_span(self);}
        if self.uses_crop() {report["crop_source"]=crate::crop_process::source();report["data_states"]["crop_operation"] = serde_json::json!("Finite modelled transfers under user-assumed ideal maintained source conditions; not calibrated biology/chamber hardware");report["limits"]=serde_json::json!("Reduced nominal demand and lumped thermal model with optional conserved maintained-reference crop transfers. No independently controlled chamber, calibrated crop physiology, complete diet credit, ice condensation, seal/rarefied leakage, EVA or resupply. Passive lumped wall condensation and nominal structural leak are optional and separately sourced; scheduled equipment wear/maintenance with authored capacity calibration is optional and separately sourced; crop-specific source/progress/energy assumptions are retained separately in crop_source.");}
        if self.uses_calibration() {
            report["calibration_observation"]=serde_json::json!({"factors":self.config.calibration.as_ref().or(self.definition.config.calibration.as_ref()),"last_interval":self.budget.calibration,"model":crate::habitat_calibration::CALIBRATION_MODEL});
            report["calibration_source"]=crate::habitat_calibration::source();
            report["source"]["calibration"]=serde_json::json!("Authored calibration multipliers and ± sensitivity bands on the published reference rates (ESTIMATED design inputs); not fitted to measurements");
            report["data_states"]["life_support_calibration"]=serde_json::json!("Authored calibration factors and ± sensitivity bands on the published reference rates (ESTIMATED design inputs), not measured hardware, fitted parameters or predictive statistics");
            report["uncertainty_scope"]=serde_json::json!("Life-support calibration bands are authored sensitivity bounds around applied calibrated exchanges, not measured variance or confidence intervals; remaining input and predictive model uncertainty is not quantified");
        }
        if self.uses_radiation() {
            let band=crate::habitat_radiation::HazardBand::of(self.budget.radiation.map(|b|b.interior_dose_rate_msv_day));
            report["radiation_observation"]=serde_json::json!({"exterior_dose_rate_msv_day":self.budget.radiation.map(|b|b.exterior_dose_rate_msv_day),"interior_dose_rate_msv_day":self.budget.radiation.map(|b|b.interior_dose_rate_msv_day),"storm_shelter_dose_rate_msv_day":self.budget.radiation.map(|b|b.storm_shelter_dose_rate_msv_day),"interval_dose_msv":self.budget.radiation.map(|b|b.interval_dose_msv),"hazard_band":band.label(),"cumulative_dose_msv":self.totals.cumulative_dose_msv,"model":crate::habitat_radiation::MODEL});
            report["radiation_source"]=crate::habitat_radiation::source();
            report["source"]["radiation"]=serde_json::json!("Authored exterior radiation environment and authored shielding transmission fractions (ESTIMATED design inputs); not particle transport, fitted measurements or dosimetry");
            report["data_states"]["radiation"]=serde_json::json!("Modelled exterior/interior dose rates and cumulative dose from authored body/star inputs and authored shielding fractions (ESTIMATED design inputs), not measured dose");
        }
        report
    }
}
pub const ALERTS: [&str; 8] = [
    "environment unavailable",
    "electric demand unserved",
    "oxygen below design threshold",
    "CO₂ above design threshold",
    "temperature outside design band",
    "nominal crew cycle unserved",
    "temperature outside supported model domain; budgets suspended",
    "radiation dose rate above design threshold",
];
/// Bit index of the radiation alert; appended after the original seven so existing
/// alert bit positions never shift.
pub const RADIATION_ALERT: usize = 7;

impl Universe {
    pub fn habitat_environment(&self, d: &Definition, time: f64) -> Environment {
        let Some(sys) = self.systems.get(d.body.system as usize) else {
            return Environment::default();
        };
        let Some(body) = sys
            .bodies
            .get(d.body.body as usize)
            .filter(|b| b.exists() && b.kind.has_surface())
        else {
            return Environment::default();
        };
        let sun =
            sys.star_local_position(time) - sys.body_local_position(d.body.body as usize, time);
        let distance = sun.length();
        if distance <= body.radius || !distance.is_finite() {
            return Environment::default();
        }
        let local = sys.inertial_to_body_fixed(d.body.body as usize, sun / distance, time);
        let radial = terrain::dir_from_lat_lon(
            d.latitude_deg.to_radians(),
            d.longitude_east_deg.to_radians(),
        );
        let cosine = local
            .iter()
            .zip(radial)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .max(0.0);
        let flux = sys.star.luminosity(time) * SOLAR_LUMINOSITY
            / (4.0 * std::f64::consts::PI * distance * distance)
            * cosine;
        let dose = crate::habitat_radiation::exterior_dose_rate_msv_day(
            sys.star.flare_activity_at(time),
            body.magnetic_field,
            body.atmosphere.pressure_bar,
        );
        if !flux.is_finite()
            || flux > 1.0e8
            || !body.temperature.is_finite()
            || !(1.0..=5000.0).contains(&body.temperature)
            || !dose.is_finite()
            || dose > 1.0e4
        {
            return Environment::default();
        }
        Environment {
            irradiance_w_m2: Some(flux),
            exterior_temperature_k: Some(body.temperature),
            exterior_dose_rate_msv_day: Some(dose),
        }
    }
    pub(crate) fn ensure_habitat_task(&mut self) {
        if self.habitats.is_empty() {
            if self.scheduler.tasks.get(TASK).is_some_and(|t| t.name == "habitats") {
                if self.scheduler.tasks.len() == TASK + 1 {
                    self.scheduler.tasks.remove(TASK);
                } else {
                    // Later task indices must never shift when a habitat is removed.
                    self.scheduler.tasks[TASK] = crate::scheduler::Task {
                        name: "habitats".into(), origin: self.time,
                        period: crate::scheduler::NEVER, steps: 0,
                    };
                }
            }
            return;
        }
        if self.scheduler.tasks.len() == TASK {
            self.scheduler.add("habitats", self.time, PERIOD);
        } else if self.scheduler.tasks.get(TASK).is_some_and(|t| t.name == "habitats" && t.period == crate::scheduler::NEVER) {
            self.scheduler.tasks[TASK] = crate::scheduler::Task {
                name: "habitats".into(), origin: self.time, period: PERIOD, steps: 0,
            };
        }
    }
    pub(crate) fn step_habitats(&mut self, time: f64) {
        for i in 0..self.habitats.len() {
            let env = self.habitat_environment(&self.habitats[i].definition, time);
            // Invalid external domains become explicitly unavailable, never fake local data.
            if self.habitats[i].step(time, env).is_err() {
                self.habitats[i].budget = Budget::default();
                self.habitats[i].last_time = time;
            }
            self.habitat_events(i, time);
        }
    }
    pub(crate) fn habitat_events(&mut self, i: usize, time: f64) {
        let h = &mut self.habitats[i];
        let now = h.alerts();
        let changed = now ^ h.alert_bits;
        for (bit, message) in ALERTS.iter().enumerate() {
            if changed & (1 << bit) != 0 {
                let active = now & (1 << bit) != 0;
                let detail = if bit == RADIATION_ALERT {
                    let band = crate::habitat_radiation::HazardBand::of(h.budget.radiation.map(|b|b.interior_dose_rate_msv_day));
                    let threshold = h.config.radiation.as_ref().map_or(0.0,|r|r.alert_dose_rate_msv_day);
                    format!("Experimental design alert. Interior dose rate {:.3} mSv/day (band {}), cumulative {:.3} mSv; design threshold {:.3} mSv/day. Authored model, not medical or safety certification.",h.budget.radiation.map_or(0.0,|b|b.interior_dose_rate_msv_day),band.label(),h.totals.cumulative_dose_msv,threshold)
                } else {
                    format!("Experimental design alert. O₂ {:.1} kPa; CO₂ {:.1} Pa; {:.2} K; unserved {:.1} W. Not medical or safety certification.",h.state.oxygen_pa(&h.config)/1000.0,h.state.co2_pa(&h.config),h.state.temperature_k,h.budget.unserved_w)
                };
                self.history.push(Event {time,category:if active {Category::Disaster} else {Category::Space},importance:3,title:format!("{}: {}{message}",h.definition.name,if active {""} else {"recovered from "}),detail,system:Some(h.definition.body.system),body:Some(h.definition.body.body),civ: None, cause: None });
            }
        }
        h.alert_bits = now;
    }
}

/// Observer rows retain both the requested world time and the completed habitat interval.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub simulation_time_s: f64,
    pub habitat: Habitat,
}
pub fn sample(u: &Universe, id: u64) -> Result<Sample, String> {
    Ok(Sample {
        simulation_time_s: u.time,
        habitat: u
            .habitats
            .iter()
            .find(|h| h.id == id)
            .ok_or("Habitat no longer exists")?
            .clone(),
    })
}
/// Shared actual observation formatting for export and recorded consistency checks.
pub(crate) fn samples_csv(rows: &[Sample]) -> String {
    let wall_present = rows.iter().any(|r| r.habitat.uses_wall());
    let leak_present = rows.iter().any(|r| r.habitat.uses_leak());
    let maintenance_present = rows.iter().any(|r| r.habitat.uses_maintenance());
    let construction_present = rows.iter().any(|r| r.habitat.uses_construction());
    let calibration_present = rows.iter().any(|r| r.habitat.uses_calibration());
    let radiation_present = rows.iter().any(|r| r.habitat.uses_radiation());
    let mut csv=String::from("simulation_time_s,completed_habitat_time_s,interval_s,temperature_k,total_pressure_pa,oxygen_partial_pressure_pa,co2_partial_pressure_pa,battery_j,oxygen_kg,oxygen_reserve_kg,potable_water_kg,wastewater_kg,brine_kg,food_dry_kg,food_water_kg,dry_waste_kg,captured_co2_kg,environment_available,temperature_domain_supported,horizontal_irradiance_w_m2,exterior_temperature_k,generated_w,requested_w,delivered_w,unserved_w,battery_losses_w,curtailed_w,served_cycle_fraction,metabolic_heat_w,heater_w,radiator_rejection_w,conduction_w,thermal_storage_w,mass_residual_kg,electric_residual_j,thermal_residual_j,total_unserved_j,total_unserved_person_seconds,hydrogen_kg,electrolysis_requested_w,electrolysis_delivered_w,electrolysis_oxygen_generated_kg,electrolysis_hydrogen_generated_kg,electrolysis_water_consumed_kg,electrolysis_electricity_j,electrolysis_reaction_enthalpy_j,electrolysis_waste_heat_j,total_reaction_enthalpy_j,electrolysis_constraints,methane_kg,sabatier_water_kg,sabatier_requested_w,sabatier_delivered_w,sabatier_co2_consumed_kg,sabatier_hydrogen_consumed_kg,sabatier_methane_generated_kg,sabatier_water_generated_kg,sabatier_electricity_j,sabatier_reaction_heat_j,sabatier_total_heat_j,sabatier_water_processed_kg,sabatier_water_recovered_kg,total_sabatier_co2_consumed_kg,total_sabatier_heat_j,total_sabatier_water_processed_kg,sabatier_constraints,water_vapor_kg,condensate_kg,vapor_pressure_pa,relative_humidity,saturation_pressure_pa,reference_phase_energy_j,moisture_requested_w,moisture_delivered_w,crew_vapor_kg,crew_phase_energy_j,condensed_kg,condensation_heat_j,moisture_electricity_j,moisture_constraints,condensate_processed_kg,condensate_recovered_kg,total_crew_vapor_kg,total_condensed_kg,total_condensate_processed_kg,escaped_oxygen_kg,escaped_nitrogen_kg,escaped_co2_kg,escaped_vapor_kg,accounted_mass_kg,breach_initial_pressure_pa,breach_final_pressure_pa,breach_initial_temperature_k,breach_final_temperature_k,breach_initial_rate_kg_s,breach_final_rate_kg_s,breach_integrated_seconds,breach_steps,breach_oxygen_kg,breach_nitrogen_kg,breach_co2_kg,breach_vapor_kg,breach_lumped_cooling_j,breach_structure_energy_change_j,breach_gas_energy_change_j,breach_phase_energy_change_j,breach_outflow_enthalpy_j,breach_energy_residual_j,breach_status,total_breach_enthalpy_j");
    if wall_present {csv.push_str(",wall_liquid_kg,total_wall_condensed_kg");}
    if leak_present {csv.push_str(",leaked_oxygen_kg,leaked_nitrogen_kg,leaked_co2_kg,leaked_vapor_kg,total_leaked_gas_kg");}
    if maintenance_present {csv.push_str(",maintenance_operating_s,maintenance_active_s,worst_equipment_condition,total_maintenance_hours");}
    if construction_present {csv.push_str(",construction_material_kg,construction_requested_w,construction_delivered_w,construction_volume_added_m3,total_constructed_volume_m3");}
    if calibration_present {csv.push_str(",calibration_crew_cycle_applied,calibration_crew_cycle_low,calibration_crew_cycle_high,calibration_oxygen_makeup_applied,calibration_oxygen_makeup_low,calibration_oxygen_makeup_high,calibration_co2_captured_applied,calibration_co2_captured_low,calibration_co2_captured_high,calibration_water_processed_applied,calibration_water_processed_low,calibration_water_processed_high");}
    if radiation_present {csv.push_str(",radiation_exterior_dose_rate_msv_day,radiation_interior_dose_rate_msv_day,radiation_shelter_dose_rate_msv_day,radiation_interval_dose_msv,radiation_cumulative_dose_msv,radiation_hazard_band");}
    csv.push('\n');
    for row in rows {
        let h = &row.habitat;
        let s = &h.state;
        let b = &h.budget;
        let c = &h.config;
        let values = [
            row.simulation_time_s,
            h.last_time,
            b.seconds,
            s.temperature_k,
            s.pressure_pa(c),
            s.oxygen_pa(c),
            s.co2_pa(c),
            s.battery_j,
            s.oxygen_kg,
            s.oxygen_reserve_kg,
            s.potable_water_kg,
            s.wastewater_kg,
            s.brine_kg,
            s.food_dry_kg,
            s.food_water_kg,
            s.dry_waste_kg,
            s.captured_co2_kg,
        ];
        csv.push_str(
            &values
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push_str(&format!(
            ",{},{},{},{}",
            b.environment_available,
            b.temperature_domain_supported,
            b.horizontal_irradiance_w_m2
                .map(|v| v.to_string())
                .unwrap_or_default(),
            b.exterior_temperature_k
                .map(|v| v.to_string())
                .unwrap_or_default()
        ));
        for value in [
            b.generated_w,
            b.requested_w,
            b.delivered_w,
            b.unserved_w,
            b.battery_losses_w,
            b.curtailed_w,
            b.served_cycle_fraction,
            b.metabolic_heat_w,
            b.heater_w,
            b.radiator_rejection_w,
            b.conduction_w,
            b.thermal_storage_w,
            b.mass_residual_kg,
            b.electric_residual_j,
            b.thermal_residual_j,
            h.totals.unserved_j,
            h.totals.unserved_person_seconds,
        ] {
            csv.push(',');
            csv.push_str(&value.to_string());
        }
        csv.push(',');csv.push_str(&s.hydrogen_kg.to_string());
        let reaction_values = b.electrolysis.as_ref().map(|e|[
            e.requested_w,e.delivered_w,e.oxygen_generated_kg,e.hydrogen_generated_kg,
            e.water_consumed_kg,e.electricity_j,e.reaction_enthalpy_j,e.waste_heat_j]);
        for i in 0..8 {csv.push(',');if let Some(v)=reaction_values {csv.push_str(&v[i].to_string());}}
        csv.push(',');csv.push_str(&h.totals.reaction_enthalpy_j.to_string());
        csv.push(',');if let Some(e)=&b.electrolysis {csv.push_str(&e.constraints.iter().map(|c|c.label()).collect::<Vec<_>>().join("; "));}
        for v in [s.methane_kg,s.sabatier_water_kg] {csv.push(',');csv.push_str(&v.to_string());}
        let reduction_values=b.sabatier.as_ref().map(|e|[e.requested_w,e.delivered_w,e.co2_consumed_kg,e.hydrogen_consumed_kg,e.methane_generated_kg,e.water_generated_kg,e.electricity_j,e.reaction_heat_j,e.total_heat_j]);
        for i in 0..9 {csv.push(',');if let Some(v)=reduction_values {csv.push_str(&v[i].to_string());}}
        for v in [b.sabatier_water_processed_kg,b.sabatier_water_recovered_kg,h.totals.sabatier_co2_consumed_kg,h.totals.sabatier_heat_j,h.totals.sabatier_water_processed_kg] {csv.push(',');csv.push_str(&v.to_string());}
        csv.push(',');if let Some(e)=&b.sabatier {csv.push_str(&e.constraints.iter().map(|c|c.label()).collect::<Vec<_>>().join("; "));}
        for v in [Some(s.water_vapor_kg),Some(s.condensate_kg),h.uses_moisture().then(||s.vapor_pressure_pa(c)),h.uses_moisture().then_some(()).and_then(|_|s.relative_humidity(c)),h.uses_moisture().then_some(()).and_then(|_|crate::moisture::saturation_pa(s.temperature_k)),Some(s.phase_energy_j())] {csv.push(',');if let Some(v)=v {csv.push_str(&v.to_string());}}
        let mv=b.moisture.as_ref().map(|m|[m.requested_w,m.delivered_w,m.crew_vapor_kg,m.crew_phase_energy_j,m.condensed_kg,m.condensation_heat_j,m.electricity_j]);
        for i in 0..7 {csv.push(',');if let Some(v)=mv {csv.push_str(&v[i].to_string());}}
        csv.push(',');if let Some(m)=&b.moisture {csv.push_str(&m.constraints.join("; "));}
        for v in [b.condensate_processed_kg,b.condensate_recovered_kg,h.totals.crew_vapor_kg,h.totals.condensed_kg,h.totals.condensate_processed_kg] {csv.push(',');csv.push_str(&v.to_string());}
        let escaped=s.escaped_gas.unwrap_or_default();
        for v in [escaped.oxygen_kg,escaped.nitrogen_kg,escaped.co2_kg,escaped.vapor_kg,s.accounted_mass_kg()] {csv.push(',');csv.push_str(&v.to_string());}
        let bv=b.breach.as_ref().map(|e|[e.initial_pressure_pa,e.final_pressure_pa,e.initial_temperature_k,e.final_temperature_k,e.initial_rate_kg_s,e.final_rate_kg_s,e.integrated_seconds,e.integration_steps as f64,e.escaped.oxygen_kg,e.escaped.nitrogen_kg,e.escaped.co2_kg,e.escaped.vapor_kg,e.lumped_cooling_j,e.structure_energy_change_j,e.gas_energy_change_j,e.phase_energy_change_j,e.outflow_enthalpy_j,e.energy_residual_j]);
        for i in 0..18 {csv.push(',');if let Some(v)=bv {csv.push_str(&v[i].to_string());}}
        csv.push(',');if let Some(e)=&b.breach {csv.push_str(e.status.label());}
        csv.push(',');csv.push_str(&h.totals.breach_enthalpy_j.to_string());
        if wall_present {csv.push(',');csv.push_str(&s.wall_liquid_kg.to_string());csv.push(',');csv.push_str(&h.totals.wall_condensed_kg.to_string());}
        if leak_present {let leaked=h.totals.leaked_gas.unwrap_or_default();for v in [leaked.oxygen_kg,leaked.nitrogen_kg,leaked.co2_kg,leaked.vapor_kg,leaked.mass_kg()] {csv.push(',');csv.push_str(&v.to_string());}}
        if maintenance_present {
            let mut operating_s=0.0;let mut active_s=0.0;let mut worst=1.0_f64;
            if let Some(mb)=&b.maintenance {for x in mb.slots().into_iter().flatten() {operating_s+=x.operating_s;active_s+=x.maintenance_s;worst=worst.min(x.condition_after);}}
            else if let Some(ms)=&s.maintenance {for x in ms.slots().into_iter().flatten() {worst=worst.min(x.condition);}}
            for v in [operating_s,active_s,worst,h.totals.maintenance_hours] {csv.push(',');csv.push_str(&v.to_string());}
        }
        if construction_present {
            for v in [s.construction_material_kg,b.construction.as_ref().map_or(0.0,|k|k.requested_w),b.construction.as_ref().map_or(0.0,|k|k.delivered_w),b.construction.as_ref().map_or(0.0,|k|k.volume_added_m3),h.totals.constructed_volume_m3] {csv.push(',');csv.push_str(&v.to_string());}
        }
        if calibration_present {
            let cb = b.calibration.as_ref();
            for v in [
                cb.map_or(0.0,|k|k.crew_cycle_person_days.applied), cb.map_or(0.0,|k|k.crew_cycle_person_days.low), cb.map_or(0.0,|k|k.crew_cycle_person_days.high),
                cb.map_or(0.0,|k|k.oxygen_makeup_kg.applied), cb.map_or(0.0,|k|k.oxygen_makeup_kg.low), cb.map_or(0.0,|k|k.oxygen_makeup_kg.high),
                cb.map_or(0.0,|k|k.co2_captured_kg.applied), cb.map_or(0.0,|k|k.co2_captured_kg.low), cb.map_or(0.0,|k|k.co2_captured_kg.high),
                cb.map_or(0.0,|k|k.water_processed_kg.applied), cb.map_or(0.0,|k|k.water_processed_kg.low), cb.map_or(0.0,|k|k.water_processed_kg.high),
            ] {csv.push(',');csv.push_str(&v.to_string());}
        }
        if radiation_present {
            let rb = b.radiation.as_ref();
            // Missing interval dose rate leaves empty cells (unavailable, never zero).
            for v in [rb.map(|r|r.exterior_dose_rate_msv_day),rb.map(|r|r.interior_dose_rate_msv_day),rb.map(|r|r.storm_shelter_dose_rate_msv_day),rb.map(|r|r.interval_dose_msv)] {
                csv.push(',');
                if let Some(v) = v {csv.push_str(&v.to_string());}
            }
            csv.push(',');csv.push_str(&h.totals.cumulative_dose_msv.to_string());
            csv.push(',');
            csv.push_str(crate::habitat_radiation::HazardBand::of(rb.map(|r|r.interior_dose_rate_msv_day)).label());
        }
        csv.push('\n');
    }
    csv
}
/// The caller supplies actual completed states; no advancement or fabricated samples.
/// Exclusive directory/file creation protects existing studies and saves.
pub fn export(
    initial: &Universe,
    final_world: &Universe,
    id: u64,
    rows: &[Sample],
    path: &std::path::Path,
) -> Result<(), String> {
    use std::io::Write;
    if rows.is_empty() || rows.len() > 1024 {
        return Err("Export requires 1–1024 observer samples".into());
    }
    let h = final_world
        .habitats
        .iter()
        .find(|h| h.id == id)
        .ok_or("No such habitat")?;
    if rows
        .iter()
        .any(|r| r.habitat.id != id || !r.simulation_time_s.is_finite())
    {
        return Err("Observer samples must identify this habitat".into());
    }
    let csv=samples_csv(rows);
    let mut manifest=serde_json::json!({"format":"cosmogon-habitat-study","schema_version":1,"model":MODEL,"habitat_id":id,"sample_count":rows.len(),"initial_state_fingerprint":crate::research::state_fingerprint(initial)?,"final_state_fingerprint":crate::research::state_fingerprint(final_world)?,"fingerprint_scope":"FNV-1a state identity, not cryptographic integrity","app_version":env!("CARGO_PKG_VERSION"),"simulation_time_coordinate":"seconds since J2000, same coordinate as world","habitat_intervals":"fixed 60 s plus explicit edit-time remainders; samples may follow the last completed interval","source":SOURCE,"model_descriptor":crate::research::PhysicsModelRegistry::get("habitat"),"electrolysis_model_descriptor":h.uses_electrolysis().then(||crate::research::PhysicsModelRegistry::get("electrolysis")),"electrolysis_source":h.report()["electrolysis_source"],"sabatier_model_descriptor":h.uses_sabatier().then(||crate::research::PhysicsModelRegistry::get("sabatier")),"sabatier_source":h.report()["sabatier_source"],"breach_source":h.uses_breach().then(crate::breach::source),"breach_model_descriptor":h.uses_breach().then(||crate::research::PhysicsModelRegistry::get("breach")),"moisture_source":h.uses_moisture().then(crate::moisture::source),"moisture_model_descriptor":h.uses_moisture().then(||crate::research::PhysicsModelRegistry::get("moisture")),"radiation_source":h.uses_radiation().then(crate::habitat_radiation::source),"radiation_model_descriptor":h.uses_radiation().then(||crate::research::PhysicsModelRegistry::get("habitat-radiation")),"limits":h.report()["limits"]});
    if h.heat.is_some() {manifest["heat_accounting_source"]=crate::habitat_heat::source();manifest["heat_accounting_model_descriptor"]=serde_json::to_value(crate::research::PhysicsModelRegistry::get("habitat-heat")).map_err(|e|e.to_string())?;}
    if h.uses_crop() {manifest["crop_source"]=crate::crop_process::source();manifest["crop_model_descriptor"]=serde_json::to_value(crate::research::PhysicsModelRegistry::get("finite-crop")).map_err(|e|e.to_string())?;}
    let mut data=vec![("definition.json",serde_json::to_vec_pretty(&h.definition).map_err(|e|e.to_string())?),("report.json",serde_json::to_vec_pretty(&h.report()).map_err(|e|e.to_string())?),("samples.json",serde_json::to_vec_pretty(rows).map_err(|e|e.to_string())?),("samples.csv",csv.into_bytes()),("initial.cosmo",crate::save::to_json(initial,"Habitat study initial world").map_err(|e|e.to_string())?.into_bytes()),("final.cosmo",crate::save::to_json(final_world,"Habitat study final world").map_err(|e|e.to_string())?.into_bytes()),("manifest.json",serde_json::to_vec_pretty(&manifest).map_err(|e|e.to_string())?)];
    if h.uses_crop() {data.insert(data.len()-1,("crop.csv",crate::crop_process::samples_csv(rows).into_bytes()));}
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir(path)
        .map_err(|e| format!("Existing exports are protected; choose a new folder: {e}"))?;
    for (name, data) in data {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.join(name))
            .map_err(|e| format!("Incomplete habitat export: {e}"))?;
        file.write_all(&data)
            .map_err(|e| format!("Incomplete habitat export: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn habitat() -> Habitat {
        Habitat::new(
            0,
            Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0),
            0.0,
        )
        .unwrap()
    }
    fn environment() -> Environment {
        Environment {
            irradiance_w_m2: Some(0.0),
            exterior_temperature_k: Some(210.0),
            exterior_dose_rate_msv_day: None,
        }
    }
    #[test]
    fn balanced_crew_and_finite_processors_conserve_mass_and_electric_energy() {
        let mut h = habitat();
        let mass = h.state.mass_kg();
        let energy = h.state.battery_j;
        for k in 1..=1440 {
            h.step(k as f64 * PERIOD, environment()).unwrap();
            assert!(h.budget.mass_residual_kg.abs() < 1.0e-9);
            assert!(h.budget.electric_residual_j.abs() < 1.0e-6);
        }
        assert!((h.state.mass_kg() - mass).abs() < 1.0e-8);
        assert!((h.totals.metabolic_person_seconds - 4.0 * SECONDS_PER_DAY).abs() < 1.0e-6);
        assert!((h.state.food_dry_kg - 96.8).abs() < 1.0e-9);
        assert!((h.state.food_water_kg - 91.96).abs() < 1.0e-9);
        assert!(h.state.brine_kg > 0.0);
        assert!(
            (h.state.captured_co2_kg
                - h.definition.initial.co2_pa * h.config.volume_m3 * CO2
                    / (R * h.definition.initial.temperature_k)
                - 4.34)
                .abs()
                < 1.0e-8
        );
        assert!(
            (h.state.battery_j - energy - h.totals.generated_j
                + h.totals.delivered_j
                + h.totals.losses_j
                + h.totals.curtailed_j)
                .abs()
                < 0.01
        );
    }
    #[test]
    fn shortages_never_create_mass_power_or_negative_stores() {
        let mut h = habitat();
        h.config.firm_power_w = 0.0;
        h.config.solar_area_m2 = 0.0;
        h.state.battery_j = 0.0;
        h.state.potable_water_kg = 0.0;
        let mass = h.state.mass_kg();
        h.step(PERIOD, environment()).unwrap();
        assert_eq!(h.budget.delivered_w, 0.0);
        assert_eq!(h.budget.co2_captured_kg, 0.0);
        assert_eq!(h.budget.served_cycle_fraction, 0.0);
        assert_eq!(h.totals.unserved_person_seconds, 240.0);
        assert_eq!(h.state.mass_kg(), mass);
        assert!(h.alerts() & (1 << 1) != 0 && h.alerts() & (1 << 5) != 0);
        let prior = h.clone();
        assert!(h.step(1000.0, environment()).is_err());
        assert_eq!(h, prior);
        h.step(120.0, Environment::default()).unwrap();
        assert!(!h.budget.environment_available);
        assert!(h.budget.horizontal_irradiance_w_m2.is_none());
        assert_eq!(h.state, prior.state);
        assert!(h.budget.temperature_domain_supported);
        h.state.temperature_k = 600.0;
        let state = h.state.clone();
        h.step(180.0, environment()).unwrap();
        assert_eq!(h.state, state);
        assert!(!h.budget.temperature_domain_supported);
        assert!(h.alerts() & (1 << 6) != 0);
    }
    #[test]
    fn validation_refuses_future_model_nonfinite_and_destructive_capacity_changes() {
        let h = habitat();
        let mut d = h.definition.clone();
        d.model = "future/2".into();
        assert!(d.validate().is_err());
        d = h.definition.clone();
        d.config.solar_efficiency = f64::NAN;
        assert!(d.validate().is_err());
        let c = Config {
            battery_capacity_j: 0.0,
            ..h.config.clone()
        };
        assert!(h.check_config(&c).is_err());
    }
    fn moisture_world(wall: bool, leak: bool) -> Habitat {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.crew = 1;
        d.config.moisture = Some(crate::moisture::Config::default());
        d.config.firm_power_w = 10_000.0;
        d.config.base_load_w = 0.0;
        d.config.scrubber_power_w = 0.0;
        d.config.recycler_power_w = 0.0;
        d.config.heater_limit_w = 0.0;
        d.config.radiator_pump_w = 0.0;
        d.config.radiator_area_m2 = 0.0;
        d.config.conductance_w_k = 0.0;
        d.config.heat_capacity_j_k = 1e9;
        if wall {
            d.config.wall_condensation = Some(crate::moisture::WallConfig {
                wall_coupling: 0.0,
                ..Default::default()
            });
        }
        if leak {
            d.config.leak = Some(LeakConfig {
                rate_kg_s: 1.0e-4,
                ..Default::default()
            });
        }
        Habitat::new(0, d, 0.0).unwrap()
    }
    fn cold_env() -> Environment {
        Environment {
            irradiance_w_m2: Some(0.0),
            exterior_temperature_k: Some(275.15),
            exterior_dose_rate_msv_day: None,
        }
    }
    #[test]
    fn passive_wall_condensation_moves_vapor_to_film_and_keeps_identities() {
        let mut h = moisture_world(true, false);
        let mut dry = moisture_world(false, false);
        for n in 1..=5760 {
            h.step(n as f64 * PERIOD, cold_env()).unwrap();
            dry.step(n as f64 * PERIOD, cold_env()).unwrap();
            h.validate(h.last_time).unwrap();
            assert!(h.budget.mass_residual_kg.abs() < 1e-9);
            assert!(h.budget.electric_residual_j.abs() < 1e-6);
        }
        assert!(h.state.wall_liquid_kg > 0.0);
        assert!(h.totals.wall_condensed_kg > 0.0);
        assert_eq!(h.state.wall_liquid_kg, h.totals.wall_condensed_kg);
        assert!(h.state.water_vapor_kg < dry.state.water_vapor_kg);
        assert!(h.uses_wall() && h.uses_moisture());
        let b = h.budget.wall.as_ref().unwrap();
        assert!(b.net_condensed_kg.is_finite());
        if b.net_condensed_kg != 0.0 {
            assert!((b.condensation_heat_j - b.net_condensed_kg * crate::moisture::REFERENCE_PHASE_J_KG).abs() < 1e-6);
        }
    }
    #[test]
    fn structural_leak_removes_proportional_gas_cools_and_keeps_accounted_mass() {
        let mut h = moisture_world(false, true);
        let accounted = h.state.accounted_mass_kg();
        for n in 1..=60 {
            h.step(n as f64 * PERIOD, cold_env()).unwrap();
            h.validate(h.last_time).unwrap();
            assert!(h.budget.mass_residual_kg.abs() < 1e-9);
        }
        let leaked = h.totals.leaked_gas.unwrap();
        assert!(leaked.mass_kg() > 0.0);
        assert!((h.state.accounted_mass_kg() - accounted).abs() < 1e-8);
        let escaped = h.state.escaped_gas.unwrap();
        assert!((escaped.mass_kg() - leaked.mass_kg()).abs() < 1e-12);
        let lb = h.budget.leak.as_ref().unwrap();
        let share = lb.leaked.mass_kg() / lb.initial_gas.mass_kg();
        for (a, b) in [
            (lb.leaked.oxygen_kg, lb.initial_gas.oxygen_kg * share),
            (lb.leaked.nitrogen_kg, lb.initial_gas.nitrogen_kg * share),
            (lb.leaked.co2_kg, lb.initial_gas.co2_kg * share),
            (lb.leaked.vapor_kg, lb.initial_gas.vapor_kg * share),
        ] {
            assert!((a - b).abs() < 1e-9 * b.abs().max(1.0));
        }
        assert!(lb.cooling_j > 0.0);
        let cp = h.config.leak.as_ref().unwrap().gas_specific_heat_j_kg;
        assert!((lb.cooling_j - lb.leaked.mass_kg() * cp * lb.temperature_k).abs() < 1e-6);
    }
    #[test]
    fn leak_and_breach_compose_within_one_interval() {
        let mut h = moisture_world(false, true);
        h.config.breach = Some(crate::breach::Config::default());
        for n in 1..=120 {
            h.step(n as f64 * PERIOD, cold_env()).unwrap();
            h.validate(h.last_time).unwrap();
            assert!(h.budget.mass_residual_kg.abs() < 1e-9);
        }
        assert!(h.budget.leak.is_some() && h.budget.breach.is_some());
        let escaped = h.state.escaped_gas.unwrap();
        let leaked = h.totals.leaked_gas.unwrap();
        assert!(escaped.vapor_kg >= leaked.vapor_kg - 1e-12);
        assert!(h.uses_leak() && h.uses_breach());
    }
    #[test]
    fn wall_and_leak_refusals_leave_configuration_and_state_unchanged() {
        let mut h = moisture_world(true, true);
        // Held film above a new capacity is refused by config gating.
        let mut grown = h.clone();
        grown.state.wall_liquid_kg = 10.0;
        let mut small = h.config.clone();
        small.wall_condensation.as_mut().unwrap().liquid_capacity_kg = 1.0;
        assert!(grown.check_config(&small).is_err());
        // Tampered cumulative counters fail the retained identities.
        let mut bad = h.clone();
        bad.totals.wall_condensed_kg += 1.0;
        assert!(bad.validate(bad.last_time).is_err());
        h.step(PERIOD, cold_env()).unwrap();
        h.validate(h.last_time).unwrap();
        let mut bad = h.clone();
        bad.totals.leaked_gas = Some(Default::default());
        assert!(bad.validate(bad.last_time).is_err());
        // Absent optional sections never serialize.
        let plain = moisture_world(false, false);
        let json = serde_json::to_string(&plain).unwrap();
        for key in ["wall_condensation", "wall_liquid_kg", "wall_condensed_kg", "\"leak\"", "leaked_gas", "\"wall\""] {
            assert!(!json.contains(key), "legacy serialization leaked {key}");
        }
    }
    #[test]
    fn heat_ledger_wall_and_leak_supplement_closes_span_residual() {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.record_heat = true;
        d.config.crew = 1;
        d.config.moisture = Some(crate::moisture::Config::default());
        d.config.wall_condensation = Some(crate::moisture::WallConfig::default());
        d.config.leak = Some(LeakConfig {
            rate_kg_s: 1.0e-5,
            ..Default::default()
        });
        d.config.firm_power_w = 10_000.0;
        d.config.base_load_w = 0.0;
        d.config.scrubber_power_w = 0.0;
        d.config.recycler_power_w = 0.0;
        d.config.heater_limit_w = 0.0;
        d.config.radiator_pump_w = 0.0;
        d.config.radiator_area_m2 = 0.0;
        d.config.conductance_w_k = 0.0;
        d.config.heat_capacity_j_k = 1e9;
        let mut h = Habitat::new(0, d, 0.0).unwrap();
        let mut rows = Vec::new();
        for n in 1..=240 {
            h.step(n as f64 * PERIOD, cold_env()).unwrap();
            h.validate(h.last_time).unwrap();
            rows.push(Sample {
                simulation_time_s: n as f64 * PERIOD,
                habitat: h.clone(),
            });
        }
        let l = h.heat.as_ref().unwrap();
        l.validate(h.last_time).unwrap();
        assert!(l.wall_leak_storage_j.is_some());
        let s = crate::habitat_heat::span(&rows).unwrap();
        assert!((s.heat_path_sum_j - s.storage_sum_j).abs() < 1.0);
        assert!(s.terms.iter().any(|t| t.id == "wall_condensation_heat"));
        assert!(s.terms.iter().any(|t| t.id == "leak_cooling"));
    }
    fn maintenance_world(scrubber: crate::habitat_maintenance::ComponentMaintenance) -> Habitat {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.crew = 1;
        d.config.firm_power_w = 10_000.0;
        d.config.base_load_w = 0.0;
        d.config.recycler_power_w = 0.0;
        d.config.heater_limit_w = 0.0;
        d.config.radiator_pump_w = 0.0;
        d.config.radiator_area_m2 = 0.0;
        d.config.conductance_w_k = 0.0;
        d.config.heat_capacity_j_k = 1e9;
        d.config.maintenance = Some(crate::habitat_maintenance::MaintenanceConfig {
            scrubber: Some(scrubber),
            ..Default::default()
        });
        Habitat::new(0, d, 0.0).unwrap()
    }
    #[test]
    fn scheduled_maintenance_wears_scrubber_then_restores_in_window() {
        let mut h = maintenance_world(crate::habitat_maintenance::ComponentMaintenance {
            wear_per_hour: 60.0,
            repair_per_hour: 60.0,
            interval_hours: 60.0 / 3600.0,
            duration_hours: 60.0 / 3600.0,
            maintenance_power_w: 100.0,
            capacity_calibration: 1.0,
        });
        assert!(h.uses_maintenance());
        // Minute one: full operating wear of 1.0 condition, capture at rated scale.
        h.step(60.0, cold_env()).unwrap();
        h.validate(60.0).unwrap();
        assert!(h.budget.mass_residual_kg.abs() < 1e-9);
        assert!(h.budget.electric_residual_j.abs() < 1e-6);
        let b = h.budget.maintenance.as_ref().unwrap().scrubber.unwrap();
        assert_eq!(b.operating_s, 60.0);
        assert_eq!(b.maintenance_s, 0.0);
        assert!((b.wear - 1.0).abs() < 1e-12);
        assert_eq!(b.condition_after, 0.0);
        assert!(h.budget.co2_captured_kg > 0.0);
        // Minute two: maintenance window, zero capture, full restoration, real power draw.
        h.step(120.0, cold_env()).unwrap();
        h.validate(120.0).unwrap();
        assert!(h.budget.mass_residual_kg.abs() < 1e-9);
        let b = h.budget.maintenance.as_ref().unwrap().scrubber.unwrap();
        assert_eq!(b.operating_s, 0.0);
        assert_eq!(b.maintenance_s, 60.0);
        assert!((b.restored - 1.0).abs() < 1e-12);
        assert_eq!(b.condition_after, 1.0);
        assert_eq!(h.budget.co2_captured_kg, 0.0);
        assert!((h.totals.maintenance_hours - 60.0 / 3600.0).abs() < 1e-12);
        assert!(h.state.maintenance.as_ref().unwrap().scrubber.unwrap().condition == 1.0);
    }
    #[test]
    fn authored_calibration_halves_effective_scrubber_capacity() {
        let mut derated = maintenance_world(crate::habitat_maintenance::ComponentMaintenance {
            wear_per_hour: 0.0,
            repair_per_hour: 1.0,
            interval_hours: 720.0,
            duration_hours: 4.0,
            maintenance_power_w: 0.0,
            capacity_calibration: 0.5,
        });
        let mut rated = maintenance_world(crate::habitat_maintenance::ComponentMaintenance {
            wear_per_hour: 0.0,
            repair_per_hour: 1.0,
            interval_hours: 720.0,
            duration_hours: 4.0,
            maintenance_power_w: 0.0,
            capacity_calibration: 1.0,
        });
        derated.step(60.0, cold_env()).unwrap();
        rated.step(60.0, cold_env()).unwrap();
        assert!((derated.budget.co2_captured_kg - 0.5 * rated.budget.co2_captured_kg).abs() < 1e-12);
        assert!(derated.budget.co2_captured_kg > 0.0);
    }
    #[test]
    fn maintenance_refusals_leave_configuration_and_state_unchanged() {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.electrolysis = None;
        d.config.maintenance = Some(crate::habitat_maintenance::MaintenanceConfig {
            electrolysis: Some(Default::default()),
            ..Default::default()
        });
        assert!(Habitat::new(0, d, 0.0).is_err());
        let bad = crate::habitat_maintenance::ComponentMaintenance {
            wear_per_hour: -1.0,
            ..Default::default()
        };
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.maintenance = Some(crate::habitat_maintenance::MaintenanceConfig {
            scrubber: Some(bad),
            ..Default::default()
        });
        assert!(Habitat::new(0, d, 0.0).is_err());
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.maintenance = Some(crate::habitat_maintenance::MaintenanceConfig::default());
        assert!(Habitat::new(0, d, 0.0).is_err());
    }
    #[test]
    fn legacy_world_serializes_without_maintenance_state() {
        let mut h = habitat();
        for n in 1..=10 {
            h.step(n as f64 * PERIOD, environment()).unwrap();
        }
        assert!(!h.uses_maintenance());
        assert!(h.state.maintenance.is_none());
        assert!(h.budget.maintenance.is_none());
        assert_eq!(h.totals.maintenance_hours, 0.0);
        let value = serde_json::to_value(&h.state).unwrap();
        assert!(value.get("maintenance").is_none());
        let value = serde_json::to_value(&h.totals).unwrap();
        assert!(value.get("maintenance_hours").is_none());
    }
    #[test]
    fn maintenance_world_is_deterministic_and_csv_tail_is_additive() {
        let mut a = maintenance_world(crate::habitat_maintenance::ComponentMaintenance::default());
        let mut b = a.clone();
        let mut rows = Vec::new();
        for n in 1..=30 {
            a.step(n as f64 * PERIOD, cold_env()).unwrap();
            b.step(n as f64 * PERIOD, cold_env()).unwrap();
            rows.push(Sample {
                simulation_time_s: n as f64 * PERIOD,
                habitat: a.clone(),
            });
        }
        assert_eq!(a, b);
        let csv = samples_csv(&rows);
        assert_eq!(
            csv.lines().next().unwrap().split(',').count(),
            csv.lines().last().unwrap().split(',').count()
        );
        assert_eq!(csv.lines().next().unwrap().split(',').count(), 114);
        assert!(csv.lines().next().unwrap().ends_with("total_maintenance_hours"));
    }
    fn construction_world() -> Habitat {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.crew = 0;
        d.config.firm_power_w = 10_000.0;
        d.config.base_load_w = 0.0;
        d.config.recycler_power_w = 0.0;
        d.config.heater_limit_w = 0.0;
        d.config.radiator_pump_w = 0.0;
        d.config.radiator_area_m2 = 0.0;
        d.config.conductance_w_k = 0.0;
        d.config.heat_capacity_j_k = 1e9;
        d.config.construction = Some(crate::habitat_construction::Config {
            material_capacity_kg: 1000.0,
            build_rate_kg_s: 1.0,
            power_w: 200.0,
            volume_per_kg_m3: 0.01,
            ..Default::default()
        });
        let mut h = Habitat::new(0, d, 0.0).unwrap();
        h.state.construction_material_kg = 500.0;
        h
    }
    #[test]
    fn construction_consumes_material_and_grows_live_volume() {
        let mut h = construction_world();
        assert!(h.uses_construction());
        h.step(60.0, cold_env()).unwrap();
        h.validate(60.0).unwrap();
        let b = h.budget.construction.as_ref().unwrap();
        // Fully served minute at 1 kg/s: 60 kg consumed, 0.6 m³ built.
        assert_eq!(b.consumed_kg, 60.0);
        assert_eq!(b.volume_added_m3, 0.6);
        assert_eq!(b.served_fraction, 1.0);
        assert_eq!(b.material_before_kg, 500.0);
        assert_eq!(b.material_after_kg, 440.0);
        assert!(b.constraints.is_empty());
        // The retained interval configuration ran at the pre-add volume.
        assert_eq!(b.material_after_kg, h.state.construction_material_kg);
        assert!(h.budget.configuration.as_ref().unwrap().volume_m3 < h.config.volume_m3);
        assert!((h.config.volume_m3 - 500.6).abs() < 1e-9);
        assert!((h.totals.constructed_volume_m3 - 0.6).abs() < 1e-12);
        assert!((h.totals.construction_material_used_kg - 60.0).abs() < 1e-12);
        assert!((h.state.construction_structure_kg - 60.0).abs() < 1e-12);
        // Material became structure: accounted mass is conserved, not created.
        assert!(h.budget.mass_residual_kg.abs() < 1e-9);
        assert!(h.budget.electric_residual_j.abs() < 1e-6);
        h.step(120.0, cold_env()).unwrap();
        h.validate(120.0).unwrap();
        assert!((h.state.construction_material_kg - 380.0).abs() < 1e-9);
        assert!((h.config.volume_m3 - 501.2).abs() < 1e-9);
        assert!(h.budget.mass_residual_kg.abs() < 1e-9);
    }
    #[test]
    fn construction_stops_without_served_power_and_at_volume_bound() {
        // Starved bus: the base load consumes every available watt before construction.
        let mut h = construction_world();
        h.config.base_load_w = 1.0e6;
        h.step(60.0, cold_env()).unwrap();
        h.validate(60.0).unwrap();
        let b = h.budget.construction.as_ref().unwrap();
        assert_eq!(b.served_fraction, 0.0);
        assert_eq!(b.consumed_kg, 0.0);
        assert_eq!(h.state.construction_material_kg, 500.0);
        assert!(!b.constraints.is_empty());
        // A held stock that already sits at the authored volume bound never builds.
        let mut h = construction_world();
        h.config.volume_m3 = 1.0e6;
        h.step(60.0, cold_env()).unwrap();
        h.validate(60.0).unwrap();
        let b = h.budget.construction.as_ref().unwrap();
        assert_eq!(b.consumed_kg, 0.0);
        assert!(b.constraints.iter().any(|c| c.contains("upper bound")));
        assert_eq!(h.state.construction_material_kg, 500.0);
    }
    #[test]
    fn legacy_world_serializes_without_construction_state() {
        let mut h = habitat();
        for n in 1..=10 {
            h.step(n as f64 * PERIOD, environment()).unwrap();
        }
        assert!(!h.uses_construction());
        assert_eq!(h.state.construction_material_kg, 0.0);
        assert_eq!(h.state.construction_structure_kg, 0.0);
        assert!(h.budget.construction.is_none());
        assert_eq!(h.totals.constructed_volume_m3, 0.0);
        let value = serde_json::to_value(&h.state).unwrap();
        assert!(value.get("construction_material_kg").is_none());
        assert!(value.get("construction_structure_kg").is_none());
        let value = serde_json::to_value(&h.totals).unwrap();
        assert!(value.get("constructed_volume_m3").is_none());
        let value = serde_json::to_value(&h.config).unwrap();
        assert!(value.get("construction").is_none());
    }
    #[test]
    fn construction_world_is_deterministic_and_csv_tail_is_additive() {
        let mut a = construction_world();
        let mut b = a.clone();
        let mut rows = Vec::new();
        for n in 1..=10 {
            a.step(n as f64 * PERIOD, cold_env()).unwrap();
            b.step(n as f64 * PERIOD, cold_env()).unwrap();
            rows.push(Sample {
                simulation_time_s: n as f64 * PERIOD,
                habitat: a.clone(),
            });
        }
        assert_eq!(a, b);
        let csv = samples_csv(&rows);
        assert_eq!(
            csv.lines().next().unwrap().split(',').count(),
            csv.lines().last().unwrap().split(',').count()
        );
        assert_eq!(csv.lines().next().unwrap().split(',').count(), 115);
        assert!(csv
            .lines()
            .next()
            .unwrap()
            .ends_with("total_constructed_volume_m3"));
    }
    #[test]
    fn disabling_construction_freezes_held_material_and_reenable_resumes() {
        let mut h = construction_world();
        h.step(60.0, cold_env()).unwrap();
        let held = h.state.construction_material_kg;
        let mut off = h.config.clone();
        off.construction = None;
        h.check_config(&off).unwrap();
        h.config = off;
        h.step(120.0, cold_env()).unwrap();
        h.validate(120.0).unwrap();
        assert!(h.budget.construction.is_none());
        assert_eq!(h.state.construction_material_kg, held);
        let mut on = h.config.clone();
        on.construction = Some(crate::habitat_construction::Config::default());
        h.check_config(&on).unwrap();
        h.config = on;
        h.step(180.0, cold_env()).unwrap();
        h.validate(180.0).unwrap();
        assert!(h.budget.construction.is_some());
        assert!(h.state.construction_material_kg < held);
    }
    #[test]
    fn construction_capacity_cannot_drop_below_held_material() {
        let h = construction_world();
        let mut small = h.config.clone();
        small.construction.as_mut().unwrap().material_capacity_kg = 100.0;
        let error = h.check_config(&small).unwrap_err();
        assert!(error.contains("below held stock"), "{error}");
    }
    #[test]
    fn identity_calibration_preserves_physics_and_records_bounds() {
        let mut control = habitat();
        let mut calibrated = habitat();
        calibrated.config.calibration = Some(crate::habitat_calibration::Config::default());
        assert!(calibrated.uses_calibration());
        assert!(!control.uses_calibration());
        for n in 1..=120 {
            control.step(n as f64 * PERIOD, environment()).unwrap();
            calibrated.step(n as f64 * PERIOD, environment()).unwrap();
        }
        // Identity factors leave every state value and total bit-identical.
        assert_eq!(control.state, calibrated.state);
        assert_eq!(control.totals, calibrated.totals);
        assert!(control.budget.calibration.is_none());
        assert!(serde_json::to_value(&control.config)
            .unwrap()
            .get("calibration")
            .is_none());
        assert!(serde_json::to_value(&calibrated.config)
            .unwrap()
            .get("calibration")
            .is_some());
        let b = calibrated.budget.calibration.as_ref().unwrap();
        // Identity bands collapse to the applied exchange.
        assert_eq!(b.crew_cycle_person_days.applied, b.crew_cycle_person_days.low);
        assert_eq!(b.crew_cycle_person_days.applied, b.crew_cycle_person_days.high);
        assert!((b.crew_cycle_person_days.applied - 4.0 * PERIOD / SECONDS_PER_DAY).abs() < 1e-12);
        assert_eq!(b.oxygen_makeup_kg.applied, calibrated.budget.oxygen_makeup_kg);
        assert_eq!(b.co2_captured_kg.applied, calibrated.budget.co2_captured_kg);
        assert_eq!(b.water_processed_kg.applied, calibrated.budget.water_processed_kg);
        calibrated.validate(calibrated.last_time).unwrap();
        assert!(serde_json::to_value(&calibrated.budget)
            .unwrap()
            .get("calibration")
            .is_some());
    }
    #[test]
    fn raised_crew_demand_draws_faster_and_bands_wrap_applied() {
        let mut control = habitat();
        let mut raised = habitat();
        raised.config.calibration = Some(crate::habitat_calibration::Config {
            crew_demand: crate::habitat_calibration::Factor {
                central: 1.5,
                relative_band: 0.2,
            },
            ..Default::default()
        });
        for n in 1..=2880 {
            control.step(n as f64 * PERIOD, environment()).unwrap();
            raised.step(n as f64 * PERIOD, environment()).unwrap();
        }
        // Calibrated crew demand draws proportionally more of every reference store.
        assert!(raised.state.oxygen_kg < control.state.oxygen_kg);
        assert!(raised.state.potable_water_kg < control.state.potable_water_kg);
        assert!(raised.state.food_dry_kg < control.state.food_dry_kg);
        assert!(raised.state.food_water_kg < control.state.food_water_kg);
        assert!(raised.budget.metabolic_heat_w > control.budget.metabolic_heat_w);
        let b = raised.budget.calibration.as_ref().unwrap();
        assert!((b.crew_cycle_person_days.low - b.crew_cycle_person_days.applied * 0.8).abs() < 1e-12);
        assert!((b.crew_cycle_person_days.high - b.crew_cycle_person_days.applied * 1.2).abs() < 1e-12);
        raised.validate(raised.last_time).unwrap();
        // An out-of-bounds factor is refused before it can be applied.
        let mut bad = raised.config.clone();
        bad.calibration.as_mut().unwrap().scrubber.relative_band = 2.0;
        let error = raised.check_config(&bad).unwrap_err();
        assert!(error.contains("relative band"), "{error}");
    }
    #[test]
    fn calibrated_processors_scale_capture_and_processing_exchanges() {
        let mut control = habitat();
        // Nominal rates outpace nominal crew waste, so bind both processors below the
        // crew's production to make the authored rate the active constraint.
        control.config.scrubber_rate_kg_s = 1.0 / SECONDS_PER_DAY;
        control.config.recycler_rate_kg_s = 5.0 / SECONDS_PER_DAY;
        let mut boosted = control.clone();
        boosted.config.calibration = Some(crate::habitat_calibration::Config {
            scrubber: crate::habitat_calibration::Factor {
                central: 2.0,
                relative_band: 0.0,
            },
            recycler: crate::habitat_calibration::Factor {
                central: 2.0,
                relative_band: 0.0,
            },
            ..Default::default()
        });
        for n in 1..=2880 {
            control.step(n as f64 * PERIOD, environment()).unwrap();
            boosted.step(n as f64 * PERIOD, environment()).unwrap();
        }
        assert!(boosted.budget.co2_captured_kg > control.budget.co2_captured_kg);
        assert!(boosted.budget.water_processed_kg > control.budget.water_processed_kg);
        let b = boosted.budget.calibration.as_ref().unwrap();
        // Zero bands keep low = high = applied, and applied equals the budget terms.
        assert_eq!(b.co2_captured_kg.applied, b.co2_captured_kg.low);
        assert_eq!(b.co2_captured_kg.applied, boosted.budget.co2_captured_kg);
        assert_eq!(b.water_processed_kg.applied, boosted.budget.water_processed_kg);
        boosted.validate(boosted.last_time).unwrap();
    }
    #[test]
    fn calibration_world_is_deterministic_and_csv_tail_is_additive() {
        let mut a = habitat();
        a.config.calibration = Some(crate::habitat_calibration::Config::default());
        let mut b = a.clone();
        let mut rows = Vec::new();
        for n in 1..=10 {
            a.step(n as f64 * PERIOD, environment()).unwrap();
            b.step(n as f64 * PERIOD, environment()).unwrap();
            rows.push(Sample {
                simulation_time_s: n as f64 * PERIOD,
                habitat: a.clone(),
            });
        }
        assert_eq!(a, b);
        let csv = samples_csv(&rows);
        assert_eq!(
            csv.lines().next().unwrap().split(',').count(),
            csv.lines().last().unwrap().split(',').count()
        );
        assert_eq!(csv.lines().next().unwrap().split(',').count(), 122);
        assert!(csv
            .lines()
            .next()
            .unwrap()
            .ends_with("calibration_water_processed_high"));
    }
    #[test]
    fn radiation_budget_applies_authored_law_alerts_and_extends_csv() {
        let mut on = habitat();
        on.config.radiation = Some(crate::habitat_radiation::Config::default());
        let mut off = on.clone();
        off.config.radiation = None;
        let env = Environment {
            irradiance_w_m2: Some(0.0),
            exterior_temperature_k: Some(210.0),
            exterior_dose_rate_msv_day: Some(3.0),
        };
        on.step(PERIOD, env).unwrap();
        off.step(PERIOD, env).unwrap();
        let rb = on.budget.radiation.as_ref().unwrap();
        assert!((rb.exterior_dose_rate_msv_day - 3.0).abs() < 1e-12);
        assert!((rb.interior_dose_rate_msv_day - 1.5).abs() < 1e-12);
        assert!((rb.storm_shelter_dose_rate_msv_day - 0.375).abs() < 1e-12);
        assert!((rb.interval_dose_msv - 1.5 * PERIOD / 86_400.0).abs() < 1e-12);
        assert_eq!(
            rb.hazard_band,
            crate::habitat_radiation::HazardBand::AboveDesign
        );
        let first = on.totals.cumulative_dose_msv;
        assert!((first - rb.interval_dose_msv).abs() < 1e-12);
        on.step(2.0 * PERIOD, env).unwrap();
        assert!((on.totals.cumulative_dose_msv - 2.0 * first).abs() < 1e-12);
        assert!(on.alerts() & (1u16 << RADIATION_ALERT) != 0);
        assert!(off.budget.radiation.is_none());
        assert!(off.alerts() & (1u16 << RADIATION_ALERT) == 0);
        let report = on.report();
        assert_eq!(report["radiation_source"], crate::habitat_radiation::source());
        assert!(!report["radiation_observation"]["cumulative_dose_msv"]
            .as_f64()
            .unwrap()
            .is_sign_negative());
        assert!(off.report().get("radiation_source").is_none());
        let rows = |h: &Habitat| {
            vec![Sample {
                simulation_time_s: h.last_time,
                habitat: h.clone(),
            }]
        };
        let csv_on = samples_csv(&rows(&on));
        let csv_off = samples_csv(&rows(&off));
        assert!(csv_on
            .lines()
            .next()
            .unwrap()
            .ends_with("radiation_hazard_band"));
        assert_eq!(
            csv_on.lines().next().unwrap().split(',').count(),
            csv_off.lines().next().unwrap().split(',').count() + 6
        );
        assert_eq!(
            csv_on.lines().next().unwrap().split(',').count(),
            csv_on.lines().last().unwrap().split(',').count()
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{sandbox::Edit, save, Scenario, UniverseSettings};
    fn assert_same_json(a: &Universe, b: &Universe) {
        fn differences(
            a: &serde_json::Value,
            b: &serde_json::Value,
            path: &str,
            out: &mut Vec<String>,
        ) {
            if out.len() >= 10 || a == b {
                return;
            }
            match (a, b) {
                (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                    for (k, v) in a {
                        if let Some(w) = b.get(k) {
                            differences(v, w, &format!("{path}/{k}"), out)
                        } else {
                            out.push(format!("missing {path}/{k}"));
                        }
                    }
                }
                (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                    if a.len() != b.len() {
                        out.push(format!("length {path}: {} vs {}", a.len(), b.len()));
                    }
                    for (i, (v, w)) in a.iter().zip(b).enumerate() {
                        differences(v, w, &format!("{path}/{i}"), out);
                    }
                }
                _ => out.push(format!("{path}: {a} vs {b}")),
            }
        }
        let mut out = Vec::new();
        differences(
            &serde_json::to_value(a).unwrap(),
            &serde_json::to_value(b).unwrap(),
            "",
            &mut out,
        );
        assert!(out.is_empty(), "{}", out.join("\n"));
    }
    fn world() -> (Universe, BodyRef) {
        let u = Universe::new(UniverseSettings {
            scenario: Scenario::SolarSystemLab,
            ..Default::default()
        });
        let r = BodyRef {
            system: 0,
            body: u.systems[0].find_body("Mars").unwrap() as u32,
        };
        (u, r)
    }
    #[test]
    fn scheduler_chunking_save_resume_and_edits_preserve_world_physics() {
        let (mut a, r) = world();
        let systems = a.systems.clone();
        let epoch = a.time;
        a.apply_edit(Edit::CreateHabitat {
            definition: Box::new(Definition::at(r, -4.59, 137.44)),
        })
        .unwrap();
        assert_eq!(a.systems, systems);
        let mut b = a.clone();
        a.advance_to(epoch + 7200.0, None, None);
        for i in 1..=240 {
            b.advance_to(epoch + i as f64 * 30.0, None, None);
        }
        assert_eq!(a, b);
        let mut resumed = save::from_json(&save::to_json(&a, "habitat").unwrap()).unwrap();
        assert_eq!(resumed.habitats, a.habitats);
        a.advance_by(1800.0);
        resumed.advance_by(1800.0);
        assert_same_json(&a, &resumed);
        let mut broken: serde_json::Value =
            serde_json::from_str(&save::to_json(&a, "habitat").unwrap()).unwrap();
        broken["header"]["data_versions"]["cosmogon-habitat-budgets"] =
            serde_json::json!("future/2");
        assert!(matches!(
            save::from_json(&broken.to_string()),
            Err(save::SaveError::Model(_))
        ));
        let unchanged = a.clone();
        let config = Config {
            battery_capacity_j: 0.0,
            ..a.habitats[0].config.clone()
        };
        assert!(a
            .apply_edit(Edit::ConfigureHabitat {
                system: 0,
                id: 0,
                config: Box::new(config)
            })
            .is_err());
        assert_eq!(a, unchanged);
        a.advance_by(17.0);
        let old_time = a.habitats[0].last_time;
        let old_cycle = a.habitats[0].totals.metabolic_person_seconds;
        let config = Config {
            crew: 0,
            ..a.habitats[0].config.clone()
        };
        a.apply_edit(Edit::ConfigureHabitat {
            system: 0,
            id: 0,
            config: Box::new(config),
        })
        .unwrap();
        assert_eq!(a.habitats[0].last_time, a.time);
        assert!(
            (a.habitats[0].totals.metabolic_person_seconds - old_cycle - 4.0 * (a.time - old_time))
                .abs()
                < 1.0e-5
        );
        assert_eq!(a.habitats[0].budget.configuration.as_ref().unwrap().crew, 4);
        assert_eq!(a.habitats[0].config.crew, 0);
        assert!(save::from_json(&save::to_json(&a, "configured").unwrap()).is_ok());
        a.apply_edit(Edit::RemoveHabitat { system: 0, id: 0 })
            .unwrap();
        assert!(a.habitats.is_empty());
        assert_eq!(a.scheduler.tasks.len(), 4);
    }
    #[test]
    fn alert_transitions_are_meaningful_and_removed_body_suspends_budgets() {
        let (mut u, r) = world();
        let mut d = Definition::at(r, 0.0, 0.0);
        d.config.firm_power_w = 0.0;
        d.config.solar_area_m2 = 0.0;
        d.initial.battery_j = 0.0;
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(d),
        })
        .unwrap();
        let start = u.time;
        u.advance_to(start + 60.0, None, None);
        let count = u
            .history
            .events
            .iter()
            .filter(|e| e.title.contains("electric demand unserved"))
            .count();
        assert_eq!(count, 1);
        u.advance_by(600.0);
        assert_eq!(
            u.history
                .events
                .iter()
                .filter(|e| e.title.contains("electric demand unserved"))
                .count(),
            count
        );
        let c = Config {
            firm_power_w: 10_000.0,
            ..u.habitats[0].config.clone()
        };
        u.apply_edit(Edit::ConfigureHabitat {
            system: 0,
            id: 0,
            config: Box::new(c),
        })
        .unwrap();
        u.advance_by(60.0);
        assert_eq!(
            u.history
                .events
                .iter()
                .filter(|e| e.title.contains("recovered from electric demand unserved"))
                .count(),
            1
        );
        u.apply_edit(Edit::RemoveBody { body: r }).unwrap();
        let state = u.habitats[0].state.clone();
        u.advance_by(120.0);
        assert_eq!(u.habitats[0].state, state);
        assert!(!u.habitats[0].budget.environment_available);
        assert!(u.habitats[0].budget.horizontal_irradiance_w_m2.is_none());
        assert!(save::from_json(&save::to_json(&u, "destroyed host").unwrap()).is_ok());
    }
    #[test]
    fn thermal_storage_matches_analytic_constant_power_and_passive_cooling() {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config = Config {
            crew: 0,
            firm_power_w: 1000.0,
            solar_area_m2: 0.0,
            base_load_w: 100.0,
            scrubber_power_w: 0.0,
            recycler_power_w: 0.0,
            heater_limit_w: 0.0,
            radiator_pump_w: 0.0,
            charge_limit_w: 0.0,
            conductance_w_k: 0.0,
            ..Default::default()
        };
        let mut h = Habitat::new(0, d, 0.0).unwrap();
        let initial = h.state.temperature_k;
        let env = Environment {
            irradiance_w_m2: Some(0.0),
            exterior_temperature_k: Some(210.0),
            exterior_dose_rate_msv_day: None,
        };
        for k in 1..=60 {
            h.step(k as f64 * 60.0, env).unwrap();
        }
        assert!(
            (h.state.temperature_k - initial - 100.0 * 3600.0 / h.config.heat_capacity_j_k).abs()
                < 1.0e-10
        );
        assert!(h.budget.thermal_residual_j.abs() < 1.0e-4);
        h.config.base_load_w = 0.0;
        h.config.conductance_w_k = 100.0;
        let initial = h.state.temperature_k;
        for k in 61..=120 {
            h.step(k as f64 * 60.0, env).unwrap();
        }
        let exact =
            210.0 + (initial - 210.0) * (-100.0 * 3600.0 / h.config.heat_capacity_j_k).dexp();
        assert!((h.state.temperature_k - exact).abs() < 1.0e-10);
        assert!(h.budget.thermal_residual_j.abs() < 1.0e-4);
    }
    #[test]
    fn numeric_exports_and_save_boundaries_keep_actual_context_and_protect_existing_files() {
        let (mut u, r) = world();
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(Definition::at(r, 0.0, 0.0)),
        })
        .unwrap();
        let initial = u.clone();
        let first = sample(&u, 0).unwrap();
        u.advance_by(120.0);
        let last = sample(&u, 0).unwrap();
        let root =
            std::env::temp_dir().join(format!("cosmogon-habitat-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        export(&initial, &u, 0, &[first, last], &root).unwrap();
        let csv = std::fs::read_to_string(root.join("samples.csv")).unwrap();
        assert_eq!(csv.lines().count(), 3);
        assert_eq!(
            csv.lines().next().unwrap().split(',').count(),
            csv.lines().last().unwrap().split(',').count()
        );
        let final_world = save::read_file(&root.join("final.cosmo")).unwrap();
        assert_same_json(&u, &final_world);
        assert!(export(&initial, &u, 0, &[sample(&u, 0).unwrap()], &root).is_err());
        assert_eq!(
            csv,
            std::fs::read_to_string(root.join("samples.csv")).unwrap()
        );
        let mut v: serde_json::Value =
            serde_json::from_str(&save::to_json(&u, "boundary").unwrap()).unwrap();
        v["universe"]["habitats"][0]["state"]["potable_water_kg"] = serde_json::json!(-1.0);
        assert!(matches!(
            save::from_json(&v.to_string()),
            Err(save::SaveError::Model(_))
        ));
        let mut v: serde_json::Value =
            serde_json::from_str(&save::to_json(&u, "boundary").unwrap()).unwrap();
        let duplicate = v["universe"]["habitats"][0].clone();
        v["universe"]["habitats"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(save::from_json(&v.to_string()).is_err());
        std::fs::write(root.join("bad.json"), vec![b'x'; 65_537]).unwrap();
        assert!(Definition::read(&root.join("bad.json")).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn storage_capacities_are_skipped_when_default_and_authored_when_present() {
        let json = serde_json::to_string(&Config::default()).unwrap();
        assert!(!json.contains("oxygen_reserve_capacity_kg"));
        assert!(!json.contains("potable_water_capacity_kg"));
        assert!(!json.contains("food_dry_capacity_kg"));
        assert!(!json.contains("food_water_capacity_kg"));
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Config::default());
        assert_eq!(parsed.oxygen_reserve_capacity_kg, 1.0e7);
        let authored = Config {
            potable_water_capacity_kg: 250.,
            ..Config::default()
        };
        let json = serde_json::to_string(&authored).unwrap();
        assert!(json.contains("potable_water_capacity_kg"));
        assert!(!json.contains("oxygen_reserve_capacity_kg"));
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.potable_water_capacity_kg, 250.);
    }

    #[test]
    fn storage_capacities_bound_initial_state_and_reconfiguration() {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.potable_water_capacity_kg = 100.;
        let error = d.validate().unwrap_err();
        assert!(error.contains("exceeds its storage capacity"), "{error}");
        d.initial.potable_water_kg = 100.;
        d.validate().unwrap();
        let mut h = Habitat::new(0, d, 0.).unwrap();
        h.validate(0.).unwrap();
        h.state.potable_water_kg = 101.;
        let error = h.validate(0.).unwrap_err();
        assert!(error.contains("below held stock"), "{error}");
        h.state.potable_water_kg = 100.;
        let mut shrink = h.config.clone();
        shrink.potable_water_capacity_kg = 99.;
        let error = h.check_config(&shrink).unwrap_err();
        assert!(error.contains("no mass was discarded"), "{error}");
    }

    #[test]
    fn available_cargo_reports_capacity_minus_held_stock() {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.oxygen_reserve_capacity_kg = 120.;
        d.config.food_dry_capacity_kg = 50.;
        d.initial.food_dry_kg = 60.;
        assert!(d.validate().is_err());
        d.initial.food_dry_kg = 45.;
        let h = Habitat::new(0, d, 0.).unwrap();
        assert_eq!(h.held_cargo().oxygen_reserve_kg, 100.);
        let free = h.available_cargo();
        assert_eq!(free.oxygen_reserve_kg, 20.);
        assert_eq!(free.food_dry_kg, 5.);
        assert_eq!(free.potable_water_kg, 1.0e7 - 200.);
        assert_eq!(free.food_water_kg, 1.0e7 - 95.);
    }

    #[test]
    fn oxygen_storage_must_cover_installed_equipment_capacity() {
        let mut d = Definition::at(BodyRef { system: 0, body: 0 }, 0.0, 0.0);
        d.config.electrolysis = Some(crate::electrolysis::Config::default());
        d.config.oxygen_reserve_capacity_kg = 150.;
        let error = d.validate().unwrap_err();
        assert!(error.contains("below installed electrolysis"), "{error}");
        d.config.oxygen_reserve_capacity_kg = 200.;
        d.validate().unwrap();
    }

    #[test]
    fn maintenance_save_pin_is_conditional_and_enforced() {
        let (mut u, r) = world();
        assert!(!save::data_versions(&u).contains_key("cosmogon-cabin-maintenance"));
        let mut d = Definition::at(r, 0.0, 0.0);
        d.config.maintenance = Some(crate::habitat_maintenance::MaintenanceConfig {
            scrubber: Some(Default::default()),
            ..Default::default()
        });
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(d),
        })
        .unwrap();
        assert_eq!(
            save::data_versions(&u)["cosmogon-cabin-maintenance"],
            crate::habitat_maintenance::MAINTENANCE_MODEL
        );
        u.advance_by(600.0);
        assert!(u.habitats[0].uses_maintenance());
        let serialized = save::to_json(&u, "Maintenance fixture").unwrap();
        let resumed = save::from_json(&serialized).unwrap();
        assert_eq!(resumed.habitats, u.habitats);
        let mut bad: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        bad["header"]["data_versions"]["cosmogon-cabin-maintenance"] = "future/2".into();
        assert!(save::from_json(&bad.to_string()).is_err());
        bad["header"]["data_versions"]
            .as_object_mut()
            .unwrap()
            .remove("cosmogon-cabin-maintenance");
        assert!(save::from_json(&bad.to_string()).is_err());
    }

    #[test]
    fn construction_supply_edit_is_bounded_pinned_and_builds_volume() {
        let (mut u, r) = world();
        assert!(!save::data_versions(&u).contains_key("cosmogon-cabin-construction"));
        let mut d = Definition::at(r, 0.0, 0.0);
        d.config.crew = 0;
        d.config.construction = Some(crate::habitat_construction::Config::default());
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(d),
        })
        .unwrap();
        let id = u.habitats[0].id;
        // Declared supply may never exceed the installed capacity.
        let error = u
            .apply_edit(Edit::SupplyHabitatConstructionMaterial {
                system: r.system,
                id,
                requested_kg: 2000.0,
            })
            .unwrap_err();
        assert!(error.contains("exceeds held capacity"), "{error}");
        assert_eq!(u.habitats[0].state.construction_material_kg, 0.0);
        u.apply_edit(Edit::SupplyHabitatConstructionMaterial {
            system: r.system,
            id,
            requested_kg: 600.0,
        })
        .unwrap();
        assert_eq!(u.habitats[0].state.construction_material_kg, 600.0);
        let before_volume = u.habitats[0].config.volume_m3;
        u.advance_by(600.0);
        let h = &u.habitats[0];
        assert!(h.uses_construction());
        assert!(h.config.volume_m3 > before_volume);
        assert!(h.state.construction_material_kg < 600.0);
        assert!(h.state.construction_structure_kg > 0.0);
        assert!(h.budget.mass_residual_kg.abs() < 1e-6);
        assert_eq!(
            save::data_versions(&u)["cosmogon-cabin-construction"],
            crate::habitat_construction::CONSTRUCTION_MODEL
        );
        let serialized = save::to_json(&u, "Construction fixture").unwrap();
        let resumed = save::from_json(&serialized).unwrap();
        assert_eq!(resumed.habitats, u.habitats);
        let mut bad: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        bad["header"]["data_versions"]["cosmogon-cabin-construction"] = "future/2".into();
        assert!(save::from_json(&bad.to_string()).is_err());
        bad["header"]["data_versions"]
            .as_object_mut()
            .unwrap()
            .remove("cosmogon-cabin-construction");
        assert!(save::from_json(&bad.to_string()).is_err());
    }

    #[test]
    fn calibration_pin_lifecycle_and_invalid_configuration_refusal() {
        let (mut u, r) = world();
        assert!(!save::data_versions(&u).contains_key("cosmogon-life-support-calibration"));
        let mut d = Definition::at(r, 0.0, 0.0);
        d.config.crew = 0;
        d.config.calibration = Some(crate::habitat_calibration::Config {
            crew_demand: crate::habitat_calibration::Factor {
                central: 1.5,
                relative_band: 0.1,
            },
            ..Default::default()
        });
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(d),
        })
        .unwrap();
        assert_eq!(
            save::data_versions(&u)["cosmogon-life-support-calibration"],
            crate::habitat_calibration::CALIBRATION_MODEL
        );
        u.advance_by(600.0);
        let h = &u.habitats[0];
        assert!(h.uses_calibration());
        let b = h.budget.calibration.as_ref().unwrap();
        // Zero crew leaves the applied cycle at zero inside its authored band.
        assert_eq!(b.crew_cycle_person_days.applied, 0.0);
        assert_eq!(b.crew_cycle_person_days.low, 0.0);
        assert_eq!(b.crew_cycle_person_days.high, 0.0);
        h.validate(u.time).unwrap();
        let serialized = save::to_json(&u, "Calibration fixture").unwrap();
        let resumed = save::from_json(&serialized).unwrap();
        assert_eq!(resumed.habitats, u.habitats);
        // A mismatched model pin refuses the reopen.
        let mut bad: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        bad["header"]["data_versions"]["cosmogon-life-support-calibration"] = "future/2".into();
        assert!(save::from_json(&bad.to_string()).is_err());
        // A removed pin refuses the reopen; a tampered band is caught by validation.
        let mut bad: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        bad["header"]["data_versions"]
            .as_object_mut()
            .unwrap()
            .remove("cosmogon-life-support-calibration");
        assert!(save::from_json(&bad.to_string()).is_err());
        let mut bad: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        bad["universe"]["habitats"][0]["budget"]["calibration"]["co2_captured_kg"]["high"] =
            999.0.into();
        assert!(save::from_json(&bad.to_string()).is_err());
        // An out-of-bounds calibration factor is refused at authoring time.
        let mut d = Definition::at(r, 0.0, 0.0);
        d.config.calibration = Some(crate::habitat_calibration::Config {
            scrubber: crate::habitat_calibration::Factor {
                central: 20.0,
                relative_band: 0.0,
            },
            ..Default::default()
        });
        let error = u
            .apply_edit(Edit::CreateHabitat {
                definition: Box::new(d),
            })
            .unwrap_err();
        assert!(error.contains("CO₂ capture"), "{error}");
    }

    #[test]
    fn habitat_stock_transfer_is_conserved_paired_and_pinned() {
        let (mut u, r) = world();
        assert!(!save::data_versions(&u).contains_key("cosmogon-habitat-stock-transfer"));
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(Definition::at(r, 0.0, 0.0)),
        })
        .unwrap();
        u.apply_edit(Edit::CreateHabitat {
            definition: Box::new(Definition::at(r, 1.0, 1.0)),
        })
        .unwrap();
        let (a, b) = (u.habitats[0].id, u.habitats[1].id);
        u.advance_by(120.0);
        let before_a = u.habitats[0].held_cargo();
        let before_b = u.habitats[1].held_cargo();
        let requested = crate::supply_vehicle::Cargo {
            oxygen_reserve_kg: 50.0,
            potable_water_kg: 80.0,
            food_dry_kg: 30.0,
            food_water_kg: 20.0,
        };
        u.apply_edit(Edit::TransferHabitatStock {
            system: r.system,
            from: a,
            to: b,
            requested: requested.clone(),
        })
        .unwrap();
        let after_a = u.habitats[0].held_cargo();
        let after_b = u.habitats[1].held_cargo();
        // Species-by-species conservation: sender decrease equals receiver increase.
        for (sent, got) in [
            (
                before_a.oxygen_reserve_kg - after_a.oxygen_reserve_kg,
                after_b.oxygen_reserve_kg - before_b.oxygen_reserve_kg,
            ),
            (
                before_a.potable_water_kg - after_a.potable_water_kg,
                after_b.potable_water_kg - before_b.potable_water_kg,
            ),
            (
                before_a.food_dry_kg - after_a.food_dry_kg,
                after_b.food_dry_kg - before_b.food_dry_kg,
            ),
            (
                before_a.food_water_kg - after_a.food_water_kg,
                after_b.food_water_kg - before_b.food_water_kg,
            ),
        ] {
            assert!((sent - got).abs() < 1e-9, "{sent} vs {got}");
            assert!(sent > 0.0);
        }
        assert!(u.habitats[0]
            .stock_transfers
            .iter()
            .any(|t| t.outbound && t.counterparty == b));
        assert!(u.habitats[1]
            .stock_transfers
            .iter()
            .any(|t| !t.outbound && t.counterparty == a));
        crate::habitat::validate_stock_transfers(&u).unwrap();
        assert_eq!(
            save::data_versions(&u)["cosmogon-habitat-stock-transfer"],
            crate::habitat::STOCK_TRANSFER_MODEL
        );
        let serialized = save::to_json(&u, "Transfer fixture").unwrap();
        let resumed = save::from_json(&serialized).unwrap();
        assert_eq!(resumed.habitats, u.habitats);
        // Dropping one side's paired evidence must refuse the save.
        let mut bad: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        bad["universe"]["habitats"][1]["stock_transfers"] = serde_json::json!([]);
        assert!(save::from_json(&bad.to_string()).is_err());
        // A request that accepts no material refuses and changes neither side.
        let error = u
            .apply_edit(Edit::TransferHabitatStock {
                system: r.system,
                from: a,
                to: b,
                requested: crate::supply_vehicle::Cargo {
                    oxygen_reserve_kg: 0.0,
                    potable_water_kg: 0.0,
                    food_dry_kg: 0.0,
                    food_water_kg: 0.0,
                },
            })
            .unwrap_err();
        assert!(error.contains("accepts no material"), "{error}");
        assert_eq!(u.habitats[0].held_cargo(), after_a);
        assert_eq!(u.habitats[1].held_cargo(), after_b);
        // A transfer across bodies is refused before anything moves.
        if let Some(earth) = u.systems[0].find_body("Earth") {
            u.apply_edit(Edit::CreateHabitat {
                definition: Box::new(Definition::at(
                    BodyRef {
                        system: r.system,
                        body: earth as u32,
                    },
                    0.0,
                    0.0,
                )),
            })
            .unwrap();
            let far = u.habitats[2].id;
            let error = u
                .apply_edit(Edit::TransferHabitatStock {
                    system: r.system,
                    from: a,
                    to: far,
                    requested,
                })
                .unwrap_err();
            assert!(error.contains("same body"), "{error}");
        }
    }
}
