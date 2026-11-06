//! Dataset registry, licensing and citation audit.
//!
//! Tracks data sources, licences, and citation requirements for all external data used
//! by Cosmogon (Gaia, exoplanet archives, deep-sky catalogues, etc.).

use serde::{Deserialize, Serialize};

/// Licence category for a data source.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LicenceCategory {
    PublicDomain,
    CC0,
    CCBY,
    CCBYSA,
    CCBYNC,
    Proprietary,
    Unknown,
}

/// Registration entry for one data source.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DatasetRegistration {
    /// Short dataset id.
    pub id: &'static str,
    /// Human-readable title.
    pub title: &'static str,
    /// Licence category.
    pub licence: LicenceCategory,
    /// URL for the data source.
    pub url: &'static str,
    /// Citation text.
    pub citation: &'static str,
    /// Whether the data is embedded or streamed at runtime.
    pub embedded: bool,
}

/// Registered datasets.
pub const DATASET_REGISTRY: &[DatasetRegistration] = &[
    DatasetRegistration {
        id: "gaia_dr3",
        title: "Gaia DR3 Catalogue",
        licence: LicenceCategory::CCBY,
        url: "https://gea.esac.esa.int/archive/",
        citation: "Gaia Collaboration (2023). Gaia DR3. VizieR Online Data Catalog: I/355.",
        embedded: true,
    },
    DatasetRegistration {
        id: "nasa_exoplanet_archive",
        title: "NASA Exoplanet Archive",
        licence: LicenceCategory::PublicDomain,
        url: "https://exoplanetarchive.ipac.caltech.edu/",
        citation: "NASA Exoplanet Archive. Retrieved 2026.",
        embedded: false,
    },
    DatasetRegistration {
        id: "messier_catalogue",
        title: "Messier Catalogue",
        licence: LicenceCategory::PublicDomain,
        url: "https://messier.seds.org/",
        citation: "Messier, C. (1781). Catalogue des Nébuleuses & des Amas d'Étoiles.",
        embedded: true,
    },
];

/// Number of registered datasets.
pub fn count() -> usize {
    DATASET_REGISTRY.len()
}

/// Licence summary for all registered datasets.
pub fn licence_summary() -> String {
    let mut counts: std::collections::BTreeMap<LicenceCategory, usize> = std::collections::BTreeMap::new();
    for ds in DATASET_REGISTRY {
        *counts.entry(ds.licence).or_insert(0) += 1;
    }
    let mut lines: Vec<String> = Vec::new();
    for (cat, n) in counts {
        lines.push(format!("{n} dataset(s): {cat:?}"));
    }
    lines.sort();
    lines.join("\n")
}
