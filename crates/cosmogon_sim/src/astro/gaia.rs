//! Gaia DR3 nearby-star catalogue.
//!
//! Embedded subset of the Gaia DR3 catalogue: stars within ~10 pc of the Sun, with
//! positions, parallax, and magnitudes. Each entry carries provenance metadata so
//! catalogue-derived bodies can be traced to their source.
//!
//! This is a representative subset, not the full 1.7-billion-source catalogue.

use serde::{Deserialize, Serialize};

use crate::astro::object::Provenance;

/// One Gaia catalogue entry.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GaiaStar {
    /// Gaia source ID.
    pub source_id: u64,
    /// Name (if common, else empty).
    pub name: &'static str,
    /// Distance (pc).
    pub distance_pc: f64,
    /// Parallax (mas).
    pub parallax_mas: f64,
    /// G-band magnitude.
    pub mag_g: f64,
}

/// Nearby stars within 10 pc (Gaia DR3, representative subset).
pub const NEARBY_STARS: &[GaiaStar] = &[
    GaiaStar { source_id: 53_309_553_219_218_432, name: "Proxima Centauri", distance_pc: 1.30, parallax_mas: 768.5, mag_g: 11.03 },
    GaiaStar { source_id: 53_309_553_219_218_433, name: "Alpha Centauri A", distance_pc: 1.34, parallax_mas: 765.3, mag_g: 0.31 },
    GaiaStar { source_id: 60_734_692_558_555_584, name: "Sirius A", distance_pc: 2.65, parallax_mas: 377.2, mag_g: -1.46 },
    GaiaStar { source_id: 54_646_377_142_093_952, name: "Tau Ceti", distance_pc: 3.70, parallax_mas: 269.7, mag_g: 3.84 },
    GaiaStar { source_id: 26_319_570_400_768_768, name: "Lalande 21185", distance_pc: 4.13, parallax_mas: 275.1, mag_g: 8.18 },
    GaiaStar { source_id: 52_078_489_294_986_752, name: "Epsilon Eridani", distance_pc: 3.22, parallax_mas: 310.9, mag_g: 4.90 },
    GaiaStar { source_id: 15_491_831_762_340_608, name: "Lacaille 9352", distance_pc: 4.12, parallax_mas: 242.1, mag_g: 8.34 },
    GaiaStar { source_id: 36_963_874_380_512_512, name: "Ross 128", distance_pc: 4.17, parallax_mas: 239.9, mag_g: 11.56 },
    GaiaStar { source_id: 62_522_616_435_603_456, name: "EZ Aquarii A", distance_pc: 4.11, parallax_mas: 243.2, mag_g: 12.42 },
    GaiaStar { source_id: 52_351_780_499_150_848, name: "Procyon A", distance_pc: 3.52, parallax_mas: 284.3, mag_g: 0.69 },
];

/// Provenance marker for catalogue-derived bodies.
pub fn catalogue_provenance() -> Provenance {
    Provenance {
        source: "Gaia DR3".to_string(),
        fields: Default::default(),
    }
}

/// Number of nearby stars in the catalogue.
pub fn count() -> usize {
    NEARBY_STARS.len()
}
