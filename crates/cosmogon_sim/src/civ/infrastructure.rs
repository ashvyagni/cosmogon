//! Derived infrastructure: what a civilization physically is on its world.
//!
//! A civilization's settlements already carry the material facts: where they are, how many
//! people live there, whether they sit on an ore body, whether they are coastal, how they are
//! linked, how urbanised the society is and how much energy it uses per person. This module
//! turns those facts into *infrastructure* — a dominant economic role per settlement and an
//! aggregated footprint — without adding a second simulation.
//!
//! Every function here is pure: no RNG, no mutation, no serialized state. That is deliberate.
//! The footprint is a view of authoritative state, so it cannot drift from the simulation and
//! cannot change save bytes or the golden fingerprint. If the civ step changes a settlement's
//! population or links, the footprint changes with it.
//!
//! Quantities that the model does not actually store are labelled as assumptions:
//! * built-up densities are representative values per settlement tier (people/km²);
//! * cropland uses 0.15 ha per person, the real global average order of magnitude;
//! * fossil depletion uses the simulation's own documented conversion
//!   ([`super::FOSSIL_JOULES_PER_UNIT`]); metals have no such conversion, so this module reports
//!   the *workforce* living at mines and the remaining endowment, never an invented tonnage.

use super::settlements::{LinkKind, Tier};
use super::Civilization;
use crate::astro::{Body, ResourceKind};
use crate::planet::terrain::dir_from_lat_lon;
use crate::time::SECONDS_PER_YEAR;

/// Representative built-up population densities by settlement tier (people per km²).
const URBAN_DENSITY: [f64; 6] = [50.0, 400.0, 1_500.0, 3_000.0, 4_500.0, 6_000.0];
/// Cropland per person (km²): 0.15 ha. Earth's ~8 billion people give ~12 million km², the right
/// order of magnitude for global cropland, so the constant is defensible rather than arbitrary.
const CROPLAND_KM2_PER_PERSON: f64 = 0.0015;
/// Energy use per person at which a society counts as fully electrified (W).
const ELECTRIFIED_W_PER_PERSON: f64 = 2_000.0;

/// The economic function a settlement performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    Farm,
    Mine,
    Port,
    Industry,
    Spaceport,
}

impl Role {
    pub const ALL: [Role; 5] = [Role::Farm, Role::Mine, Role::Port, Role::Industry, Role::Spaceport];

    pub fn label(self) -> &'static str {
        match self {
            Role::Farm => "Farm",
            Role::Mine => "Mine",
            Role::Port => "Port",
            Role::Industry => "Industry",
            Role::Spaceport => "Spaceport",
        }
    }

    /// Stable filter/display slot.
    pub fn index(self) -> usize {
        match self {
            Role::Farm => 0,
            Role::Mine => 1,
            Role::Port => 2,
            Role::Industry => 3,
            Role::Spaceport => 4,
        }
    }

    /// Plural noun for counts ("3 ports", "12 industry sites").
    pub fn plural(self) -> &'static str {
        match self {
            Role::Farm => "farms",
            Role::Mine => "mines",
            Role::Port => "ports",
            Role::Industry => "industry sites",
            Role::Spaceport => "spaceports",
        }
    }

    /// A count with a correctly inflected noun ("1 port", "3 ports").
    pub fn count(self, n: usize) -> String {
        let name = if n == 1 {
            match self {
                Role::Farm => "farm",
                Role::Mine => "mine",
                Role::Port => "port",
                Role::Industry => "industry site",
                Role::Spaceport => "spaceport",
            }
        } else {
            self.plural()
        };
        format!("{n} {name}")
    }
}

/// How strongly a settlement performs each function, each intensity in `0..=1`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Works {
    pub farm: f64,
    pub mine: f64,
    pub port: f64,
    pub industry: f64,
    pub spaceport: f64,
}

impl Works {
    /// The strongest function. Ties resolve in [`Role::ALL`] order, later winning, so the result
    /// is fully deterministic.
    pub fn dominant(self) -> Role {
        if self.farm <= 0.0 && self.mine <= 0.0 && self.port <= 0.0 && self.industry <= 0.0 && self.spaceport <= 0.0 {
            return Role::Farm;
        }
        let mut best = Role::Farm;
        let mut value = self.farm;
        for (role, v) in [
            (Role::Mine, self.mine),
            (Role::Port, self.port),
            (Role::Industry, self.industry),
            (Role::Spaceport, self.spaceport),
        ] {
            if v >= value {
                best = role;
                value = v;
            }
        }
        best
    }

    pub fn intensity(self, role: Role) -> f64 {
        match role {
            Role::Farm => self.farm,
            Role::Mine => self.mine,
            Role::Port => self.port,
            Role::Industry => self.industry,
            Role::Spaceport => self.spaceport,
        }
    }
}

/// One settlement with its dominant role, for the inspector and the marker layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiteRole {
    pub site: usize,
    pub role: Role,
    pub intensity: f64,
}

/// Aggregated, derived physical footprint of one civilization on one world.
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    pub settlements: usize,
    pub farms: usize,
    pub mines: usize,
    pub ports: usize,
    pub industries: usize,
    pub spaceports: usize,
    pub urban_km2: f64,
    pub farmland_km2: f64,
    pub road_km: f64,
    pub rail_km: f64,
    pub sea_km: f64,
    /// People living in settlements whose dominant role is mining.
    pub mining_population: f64,
    /// Society-wide electrification, 0..=1 (`electric_light` × energy per person).
    pub electrified: f64,
    pub energy_w_per_person: f64,
    /// Years of fossil endowment left at the current burn rate, when the civ burns fossils.
    pub fossil_years_left: Option<f64>,
}

impl Footprint {
    pub fn roles(&self) -> [usize; 5] {
        [self.farms, self.mines, self.ports, self.industries, self.spaceports]
    }

    pub fn total_links_km(&self) -> f64 {
        self.road_km + self.rail_km + self.sea_km
    }
}

fn density(tier: Tier) -> f64 {
    let i = match tier {
        Tier::Camp => 0,
        Tier::Village => 1,
        Tier::Town => 2,
        Tier::City => 3,
        Tier::Metropolis => 4,
        Tier::Megacity => 5,
    };
    URBAN_DENSITY[i]
}

fn tier_weight(tier: Tier) -> f64 {
    match tier {
        Tier::Camp => 0.25,
        Tier::Village => 0.4,
        Tier::Town => 0.6,
        Tier::City => 0.8,
        Tier::Metropolis => 1.0,
        Tier::Megacity => 1.1,
    }
}

/// Society-wide electrification, 0..=1. Requires the `electric_light` flag and saturates at
/// 2 kW per person.
pub fn electrification(c: &Civilization) -> f64 {
    if !c.flags.contains("electric_light") {
        return 0.0;
    }
    (c.energy_per_capita / ELECTRIFIED_W_PER_PERSON).clamp(0.0, 1.0)
}

/// The active settlement that hosts the spaceport, if the civilization has one: the largest
/// settlement, ties broken by the lowest index.
pub fn spaceport_site(c: &Civilization) -> Option<usize> {
    let flight = c.flags.contains("satellites")
        || c.missions_launched > 0
        || c.satellites > 0
        || c.flags.contains("moon_landing")
        || c.flags.contains("stations")
        || c.flags.contains("colonies");
    if !flight {
        return None;
    }
    (0..c.sites.len())
        .filter(|&i| c.sites[i].active())
        .max_by(|&a, &b| c.sites[a].population.total_cmp(&c.sites[b].population).then(b.cmp(&a)))
}

/// Score every settlement function from authoritative state.
///
/// * `farm` — rural share of the society × the world's fertile-land endowment; small settlements
///   also carry a smallholding bonus.
/// * `mine` — only at a site with an ore deposit; scaled by the body's endowment of that ore and
///   by how much labour the settlement can supply.
/// * `port` — only on a coast; full weight with a sea link, partial with seafaring technology.
/// * `industry` — urban share × electrification × settlement size; requires electric light.
/// * `spaceport` — the largest settlement, only once the civilization flies.
pub fn works(index: usize, c: &Civilization, body: &Body) -> Works {
    let Some(site) = c.sites.get(index) else {
        return Works::default();
    };
    if !site.active() {
        return Works::default();
    }
    let tier = site.tier();
    let rural = (1.0 - c.urbanisation).clamp(0.0, 1.0);
    let fertile = (body.resources.fertile_land / 1.5).clamp(0.0, 1.0);
    let mut farm = if body.kind.has_surface() {
        rural * fertile.max(0.15)
    } else {
        0.0
    };
    if tier <= Tier::Village {
        farm = (farm + 0.25).min(1.0);
    }
    let labour = (site.population / 5_000.0).clamp(0.0, 1.0);
    let mine = site
        .near_deposit
        .map_or(0.0, |kind| (body.resources.get(kind) / 2.0).clamp(0.15, 1.0) * labour);
    let sea_link = c
        .links
        .iter()
        .any(|l| l.kind == LinkKind::Sea && (l.a as usize == index || l.b as usize == index));
    let sea = if sea_link {
        1.0
    } else if c.flags.contains("seafaring") {
        0.35
    } else {
        0.0
    };
    let port = if site.coastal { sea } else { 0.0 };
    let electrified = electrification(c);
    let industry = if electrified > 0.0 {
        c.urbanisation.clamp(0.0, 1.0) * electrified * tier_weight(tier)
    } else {
        0.0
    };
    let spaceport = match spaceport_site(c) {
        Some(i) if i == index => {
            if c.flags.contains("satellites") || c.missions_launched > 0 || c.satellites > 0 {
                1.0
            } else {
                0.5
            }
        }
        _ => 0.0,
    };
    Works { farm, mine, port, industry, spaceport }
}

/// Active settlements ranked by population (largest first), with their dominant role. Ties keep
/// the lower site index first.
pub fn site_roles(c: &Civilization, body: &Body, min_population: f64) -> Vec<SiteRole> {
    let mut out: Vec<SiteRole> = (0..c.sites.len())
        .filter(|&i| c.sites[i].active() && c.sites[i].population >= min_population)
        .map(|i| {
            let w = works(i, c, body);
            SiteRole { site: i, role: w.dominant(), intensity: w.intensity(w.dominant()) }
        })
        .collect();
    out.sort_by(|a, b| {
        c.sites[b.site]
            .population
            .total_cmp(&c.sites[a.site].population)
            .then(a.site.cmp(&b.site))
    });
    out
}

fn link_km(a: &super::settlements::Site, b: &super::settlements::Site, radius_m: f64) -> f64 {
    let (pa, pb) = (a.dir(), b.dir());
    let dx = pa[0] - pb[0];
    let dy = pa[1] - pb[1];
    let dz = pa[2] - pb[2];
    (dx * dx + dy * dy + dz * dz).sqrt() * radius_m / 1_000.0
}

/// Aggregate the derived footprint of `c` on `body`.
pub fn footprint(c: &Civilization, body: &Body) -> Footprint {
    let mut fp = Footprint {
        settlements: 0,
        farms: 0,
        mines: 0,
        ports: 0,
        industries: 0,
        spaceports: 0,
        urban_km2: 0.0,
        farmland_km2: 0.0,
        road_km: 0.0,
        rail_km: 0.0,
        sea_km: 0.0,
        mining_population: 0.0,
        electrified: electrification(c),
        energy_w_per_person: c.energy_per_capita,
        fossil_years_left: None,
    };
    for (i, site) in c.sites.iter().enumerate() {
        if !site.active() {
            continue;
        }
        fp.settlements += 1;
        fp.urban_km2 += site.population / density(site.tier());
        let w = works(i, c, body);
        match w.dominant() {
            Role::Farm => fp.farms += 1,
            Role::Mine => {
                fp.mines += 1;
                fp.mining_population += site.population;
            }
            Role::Port => fp.ports += 1,
            Role::Industry => fp.industries += 1,
            Role::Spaceport => fp.spaceports += 1,
        }
    }
    fp.farmland_km2 =
        c.population.max(0.0) * CROPLAND_KM2_PER_PERSON * body.resources.fertile_land.clamp(0.0, 1.0);
    for link in &c.links {
        let Some(a) = c.sites.get(link.a as usize) else { continue };
        let Some(b) = c.sites.get(link.b as usize) else { continue };
        let km = link_km(a, b, body.radius);
        match link.kind {
            LinkKind::Road => fp.road_km += km,
            LinkKind::Rail => fp.rail_km += km,
            LinkKind::Sea => fp.sea_km += km,
        }
    }
    let use_per_year = c.population * c.energy_per_capita * c.fossil_share * SECONDS_PER_YEAR
        / super::FOSSIL_JOULES_PER_UNIT;
    let reserves = body.resources.coal + body.resources.oil;
    if use_per_year > 0.0 && reserves > 0.0 && use_per_year.is_finite() {
        fp.fossil_years_left = Some(reserves / use_per_year);
    }
    fp
}

/// Remaining endowment of every ore kind, in the simulation's relative units (Earth = 1).
pub fn endowment(body: &Body) -> Vec<(ResourceKind, f64)> {
    ResourceKind::ALL
        .into_iter()
        .map(|k| (k, body.resources.get(k)))
        .filter(|(_, v)| *v > 0.0)
        .collect()
}

/// Direction helper used by the marker layer for glyph placement on the sphere.
pub fn site_dir(site: &super::settlements::Site) -> [f64; 3] {
    dir_from_lat_lon(site.lat, site.lon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universe::{Scenario, Universe, UniverseSettings};
    use crate::BodyRef;

    fn sol() -> Universe {
        Universe::new(UniverseSettings { seed: 7, scenario: Scenario::SolarSystemLab, ..Default::default() })
    }

    fn humanity(u: &Universe) -> usize {
        u.civs.iter().position(|c| c.is_alive()).expect("present-day Earth has humanity")
    }

    fn body_of<'a>(u: &'a Universe, c: &Civilization) -> &'a Body {
        u.body(BodyRef { system: c.system, body: c.body })
    }

    #[test]
    fn works_are_zero_without_the_material_basis() {
        let u = sol();
        let ci = humanity(&u);
        let mut c = u.civs[ci].clone();
        let body = body_of(&u, &c).clone();
        c.flags.clear();
        c.missions_launched = 0;
        c.satellites = 0;
        c.energy_per_capita = 0.0;
        c.urbanisation = 0.0;
        for i in 0..c.sites.len() {
            c.sites[i].near_deposit = None;
            c.sites[i].coastal = false;
        }
        c.links.clear();
        let w = works(0, &c, &body);
        assert_eq!(w.mine, 0.0, "no deposit -> no mine");
        assert_eq!(w.port, 0.0, "inland -> no port");
        assert_eq!(w.industry, 0.0, "no electric light -> no industry");
        assert_eq!(w.spaceport, 0.0, "no flight -> no spaceport");
        assert!(spaceport_site(&c).is_none());
    }

    #[test]
    fn works_track_authoritative_state() {
        let u = sol();
        let ci = humanity(&u);
        let mut c = u.civs[ci].clone();
        let body = body_of(&u, &c).clone();

        // Electrification and industry rise with energy per person.
        c.flags.insert("electric_light".into());
        c.urbanisation = 0.8;
        c.energy_per_capita = 500.0;
        let low = works(0, &c, &body).industry;
        c.energy_per_capita = 2_500.0;
        let high = works(0, &c, &body).industry;
        assert!(high > low, "more power per person must mean more industry: {low} -> {high}");
        assert_eq!(electrification(&c), 1.0);

        // A deposit site mines more than a bare site at equal population.
        let deposit_kind = ResourceKind::Iron;
        c.sites[0].near_deposit = Some(deposit_kind);
        c.sites[1].near_deposit = None;
        c.sites[1].population = c.sites[0].population;
        assert!(works(0, &c, &body).mine > works(1, &c, &body).mine);

        // A sea-linked coast is a stronger port than a merely coastal one.
        c.sites[0].coastal = true;
        c.sites[1].coastal = true;
        c.links.retain(|l| l.kind != LinkKind::Sea);
        let bare = works(0, &c, &body).port;
        c.links.push(super::super::settlements::Link { a: 0, b: 1, kind: LinkKind::Sea });
        let linked = works(0, &c, &body).port;
        assert!(linked > bare, "sea link must strengthen a port: {bare} -> {linked}");
    }

    #[test]
    fn footprint_is_deterministic_and_additive() {
        let u = sol();
        let ci = humanity(&u);
        let c = &u.civs[ci];
        let body = body_of(&u, c);
        let a = footprint(c, body);
        let b = footprint(c, body);
        assert_eq!(a, b, "derivation must be pure");
        let sum: usize = a.roles().iter().sum();
        assert_eq!(sum, a.settlements, "every settlement has exactly one dominant role");
        assert!(a.settlements > 0);
        assert!(a.urban_km2 > 0.0);
        // Link lengths are the analytic sums of the stored links.
        let expected_rail: f64 = c
            .links
            .iter()
            .filter(|l| l.kind == LinkKind::Rail)
            .map(|l| link_km(&c.sites[l.a as usize], &c.sites[l.b as usize], body.radius))
            .sum();
        assert!((a.rail_km - expected_rail).abs() < 1e-9);
    }

    #[test]
    fn farmland_matches_the_stated_cropland_assumption() {
        let u = sol();
        let ci = humanity(&u);
        let mut c = u.civs[ci].clone();
        let mut body = body_of(&u, &c).clone();
        body.resources.fertile_land = 1.0;
        c.population = 8.0e9;
        let fp = footprint(&c, &body);
        assert!(
            (10.0e6..=15.0e6).contains(&fp.farmland_km2),
            "8 billion people should need ~12 million km², got {}",
            fp.farmland_km2
        );
    }

    #[test]
    fn dominant_role_has_a_fixed_tie_break() {
        let tie = Works { farm: 0.5, mine: 0.5, port: 0.0, industry: 0.0, spaceport: 0.0 };
        assert_eq!(tie.dominant(), Role::Mine, "later role wins an exact tie");
        let all = Works { farm: 1.0, mine: 1.0, port: 1.0, industry: 1.0, spaceport: 1.0 };
        assert_eq!(all.dominant(), Role::Spaceport);
        assert_eq!(Works::default().dominant(), Role::Farm);
        for (i, role) in Role::ALL.into_iter().enumerate() {
            assert_eq!(role.index(), i);
        }
    }

    #[test]
    fn roles_rank_largest_settlements_first() {
        let u = sol();
        let ci = humanity(&u);
        let c = &u.civs[ci];
        let body = body_of(&u, c);
        let roles = site_roles(c, body, 0.0);
        assert_eq!(roles.len(), c.sites.iter().filter(|s| s.active()).count());
        assert!(roles.windows(2).all(|w| c.sites[w[0].site].population >= c.sites[w[1].site].population));
    }
}
