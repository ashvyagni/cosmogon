//! # cosmogon_sim
//!
//! The authoritative simulation for Cosmogon. It has **no engine or rendering
//! dependencies**: the desktop app, the CLI and the tests all drive the same
//! [`Universe`] through the same API.
//!
//! Layering (lower layers never depend on higher ones):
//!
//! ```text
//!   rng, time, noise, params            deterministic foundations
//!   astro (stars, orbits, generation)   physical universe
//!   planet (environment, terrain, resources), habitability
//!   life (biosphere abstraction)
//!   civ (species, knowledge, technology graph, settlements)
//!   history, scheduler, universe, save
//! ```
//!
//! See `SIMULATION.md`, `CIVILIZATION_MODEL.md` and `TECHNOLOGY_MODEL.md` at the
//! repository root for the models and their documented simplifications.

pub mod astro;
pub mod civ;
pub mod collision;
pub mod expansion;
pub mod electrolysis;
pub mod sabatier;
pub mod moisture;
pub mod breach;
pub mod habitability;
pub mod habitat;
pub mod habitat_maintenance;
pub mod habitat_construction;
pub mod habitat_calibration;
pub mod habitat_radiation;
pub mod habitat_diagnostics;
pub mod habitat_recording;
pub mod habitat_pressure;
pub mod habitat_graph;
pub mod habitat_figure;
pub mod habitat_comparison;
pub mod habitat_sweep;
pub mod habitat_factorial;
pub mod habitat_design;
pub mod habitat_origin;
pub mod habitat_replay;
pub mod greenhouse;
pub mod greenhouse_supplies;
pub mod crop_chemistry;
pub mod crop_process;
pub mod chronicle;
pub mod render;
pub mod history;
pub mod impact;
pub mod intervene;
pub mod life;
pub mod names;
pub mod noise;
pub mod nursery;
pub mod params;
pub mod planet;
pub mod present_day;
pub mod rng;
pub mod research;
pub mod sandbox;
pub mod save;
pub mod scheduler;
pub mod stellar;
pub mod time;
pub mod universe;

pub use cosmogon_core::math::Vec3d;
pub use cosmogon_core::units::kelvin_to_rgb;
pub use universe::{BodyRef, Scenario, Universe, UniverseSettings};

pub mod habitat_balance;
pub mod habitat_heat;
pub mod habitat_heat_trajectory;

pub mod habitat_crop_trajectory;

pub mod supply_vehicle;
pub mod vehicle_cargo_mass;

pub mod world_vehicle;

pub mod world_vehicle_recording;

pub mod world_vehicle_rendezvous;

pub mod vehicle_arrival;

pub mod vehicle_arrival_recording;

pub mod vehicle_contact;
pub mod vehicle_contact_dynamics;
pub mod vehicle_handling;
pub mod vehicle_entry;
pub mod vehicle_launch;
pub mod vehicle_landing;
pub mod vehicle_orbit;
pub mod vehicle_operation;
pub mod vehicle_shear;
pub mod vehicle_support;
pub mod vehicle_support_recording;
pub mod vehicle_transfer;
