//! Deep-sky object catalogue (nebulae, galaxies, clusters).
//!
//! Notable deep-sky objects with positions, types, magnitudes, and provenance.
//! These are visitable locations, not physically modelled objects.

use serde::{Deserialize, Serialize};

use crate::astro::object::Provenance;

/// Type of deep-sky object.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeepSkyType {
    Nebula,
    Galaxy,
    StarCluster,
    GlobularCluster,
    SupernovaRemnant,
    DarkNebula,
}

/// One deep-sky object.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeepSkyObject {
    /// Common name or catalogue designation.
    pub name: &'static str,
    /// Object type.
    pub kind: DeepSkyType,
    /// Distance (pc).
    pub distance_pc: f64,
    /// Apparent magnitude.
    pub mag_v: f64,
    /// Angular size (arcminutes).
    pub size_arcmin: f64,
    /// Right ascension (hours).
    pub ra_h: f64,
    /// Declination (degrees).
    pub dec_deg: f64,
    /// Source citation.
    pub source: &'static str,
}

/// Notable deep-sky objects (representative subset).
pub const DEEP_SKY_OBJECTS: &[DeepSkyObject] = &[
    DeepSkyObject {
        name: "Orion Nebula (M42)",
        kind: DeepSkyType::Nebula,
        distance_pc: 414.0,
        mag_v: 4.0,
        size_arcmin: 65.0,
        ra_h: 5.58,
        dec_deg: -5.39,
        source: "Messier Catalogue / Gaia DR3",
    },
    DeepSkyObject {
        name: "Andromeda Galaxy (M31)",
        kind: DeepSkyType::Galaxy,
        distance_pc: 780_000.0,
        mag_v: 3.4,
        size_arcmin: 190.0,
        ra_h: 0.69,
        dec_deg: 41.27,
        source: "Messier Catalogue / Hipparcos",
    },
    DeepSkyObject {
        name: "Pleiades (M45)",
        kind: DeepSkyType::StarCluster,
        distance_pc: 136.0,
        mag_v: 1.6,
        size_arcmin: 110.0,
        ra_h: 3.47,
        dec_deg: 24.07,
        source: "Messier Catalogue / Gaia DR3",
    },
    DeepSkyObject {
        name: "Crab Nebula (M1)",
        kind: DeepSkyType::SupernovaRemnant,
        distance_pc: 2000.0,
        mag_v: 8.4,
        size_arcmin: 6.0,
        ra_h: 5.58,
        dec_deg: 22.01,
        source: "Messier Catalogue / Chandra X-ray Observatory",
    },
    DeepSkyObject {
        name: "Omega Centauri (NGC 5139)",
        kind: DeepSkyType::GlobularCluster,
        distance_pc: 5400.0,
        mag_v: 3.9,
        size_arcmin: 36.0,
        ra_h: 13.27,
        dec_deg: -47.29,
        source: "New General Catalogue / Gaia DR3",
    },
    DeepSkyObject {
        name: "Ring Nebula (M57)",
        kind: DeepSkyType::Nebula,
        distance_pc: 700.0,
        mag_v: 9.4,
        size_arcmin: 1.4,
        ra_h: 18.85,
        dec_deg: 33.02,
        source: "Messier Catalogue / Hubble Space Telescope",
    },
    DeepSkyObject {
        name: "Whirlpool Galaxy (M51)",
        kind: DeepSkyType::Galaxy,
        distance_pc: 7600.0,
        mag_v: 8.4,
        size_arcmin: 11.0,
        ra_h: 13.30,
        dec_deg: 47.17,
        source: "Messier Catalogue / Hubble Space Telescope",
    },
];

/// Provenance marker for catalogue-derived deep-sky objects.
pub fn catalogue_provenance(source: &'static str) -> Provenance {
    Provenance {
        source: source.to_string(),
        fields: Default::default(),
    }
}

/// Number of deep-sky objects in the catalogue.
pub fn count() -> usize {
    DEEP_SKY_OBJECTS.len()
}
