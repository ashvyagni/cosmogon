# Build 63 — Visible civilization infrastructure and economic/spatial presence (implementation spec)

Status: ready. **Golden-neutral build**: no save-schema change; `GOLDEN = 0xf2c4140a45ae509b`
(`crates/cosmogon_sim/src/save.rs:645`) must remain unchanged. Build 62 (`d4acdf7`) is committed;
this spec assumes `civ/`, `ui/markers.rs`, `ui/inspect.rs`, `ui/mod.rs` and
`cosmogon_cli/src/main.rs` are otherwise as shipped. All file:line refs verified against `main`.

## 1. Objective

Phase 12's gap (`docs/MASTER_GAP_ANALYSIS.md:19`) reads: *"Lights, overlays, procedural land-use
tint | Infrastructure geometry, visible roads/industry/ports/traffic driven by authoritative
state."* Today a civilization has real settlements, polities, links, population, energy,
urbanisation and a body endowment, but the only visual expression is a dot per settlement, a line
per link and a baked night-light texture. Nothing says *what a place is*: a mining town at an ore
body, a port, a farming district, an industrial belt, a spaceport.

Build 63 closes that gap with **derived infrastructure**:

1. a new pure module `cosmogon_sim::civ::infrastructure` that classifies every active settlement
   into a dominant economic role and aggregates a per-civilization footprint (areas, counts, link
   lengths, extraction share, electrification);
2. role glyphs and animated traffic on the planet sphere, driven by that derivation and by the
   authoritative settlement/link state;
3. an Infrastructure section in the Civilization inspector and a deterministic
   `cosmogon-cli footprint` report (JSON or table) so the derivation is testable from outside the
   renderer;
4. an optional land-use tint in the night-light bake that biases built-up glow by the role mix.

Everything is **read/derive-only**: no `Edit`, no RNG, no new serialized field, no mutation of
`Universe`. The derivation is a function of state that already exists, so the fingerprint cannot
move.

### Out of scope
- Mining *depleting* ore deposits, new resource stocks, trade flows, prices, or a separate
  economic simulation. The build reports intensities and shares of the body's *existing* modelled
  endowment; it does not add a second economy that could disagree with the civ step.
- New settlement sites, link geometry, or civ colonisation rules.
- Terrain-mesh changes, surface (walk-around) rendering, or roads drawn as 3-D geometry. Glyphs
  are screen-space markers over the sphere, consistent with the existing marker layer.
- Save-schema additions (would need a deliberate golden re-pin; not warranted for visuals).

## 2. Context (verified)

- `Civilization` (`civ/mod.rs:133-222`) holds `population`, `urbanisation`, `energy_per_capita`,
  `energy_shortfall`, `knowledge`, `flags: BTreeSet<String>`, `sites: Vec<Site>`,
  `links: Vec<Link>`, `polities`, `satellites`, `missions`, `missions_launched`, `colonies`.
- `Site` (`civ/settlements.rs:53-67`): `name, lat, lon, score, coastal, near_deposit:
  Option<ResourceKind>, founded, population, polity, share`; `tier()` from population
  (`settlements.rs:30-38`); `active()` = founded.
- `Link { a: u16, b: u16, kind: Road|Rail|Sea }` (`settlements.rs:88-97`), built between
  significant (≥ Town) sites, ≤ 2 land + 1 sea per site, ≤ 120 linked sites
  (`settlements.rs:255-300`).
- `Body` resources are relative endowments ~1.0 Earth (`planet/resources.rs:15-35`); `coal`+`oil`
  already have a documented conversion `FOSSIL_JOULES_PER_UNIT = 2.0e22` (`civ/mod.rs:32`).
  `fertile_land = land.min(2.5) * temperate` (`universe.rs:208`). Deposits are geographic
  `{kind, lat, lon, richness}` points (`astro/body.rs:203-210`).
- Flags used: `electric_light`, `roads`, `rail`, `seafaring`, `short_crossings`, `satellites`,
  `crewed_orbit`, `moon_landing`, `stations`, `colonies`, `fusion`, `computing`, `networks`
  (`data/technologies.toml`, `unlocks = [...]`).
- Render layer: `ui/markers.rs` — settlements at `px >= 40` (`:251-284`), links (`:262-277`),
  satellites (`:300+`); marker colour helpers `rgb`, palette constants `CIV`, `MUTED`.
- Night lights: `render/bake.rs:200-260` — `splat` splats outposts and `civ.sites` with sprawl and
  lit corridors along non-sea links when `electric`.
- Inspector civ tab: `ui/inspect.rs:1121+` (`civilization()`), `kv`/`heading`/`bar` helpers,
  `space_programme`, `interfere`.
- CLI: dispatch `main.rs:670+`, `initial_universe(a)`, `chronicle_export` as the current template
  for a read-only, save-or-simulate command (`main.rs:1068-1120`), help/unknown list `:1918`.

## 3. Tasks

### T1 — `civ::infrastructure` derived model (new module)

`crates/cosmogon_sim/src/civ/infrastructure.rs`, re-exported from `civ/mod.rs`
(`pub mod infrastructure;`). Pure functions, no `serde`, no `rand`.

```rust
pub enum Role { Farm, Mine, Port, Industry, Spaceport }
pub struct Works { pub farm: f64, pub mine: f64, pub port: f64, pub industry: f64,
                   pub spaceport: f64 }   // each 0..=1
pub struct SiteRole { pub site: usize, pub role: Role, pub intensity: f64 }
pub struct Footprint { /* counts, areas, lengths, shares */ }
```

Rules (all documented in the module header and `INFRASTRUCTURE.md`; every threshold is a stated
modelling assumption, never a physical measurement):

- `works(site, civ, body) -> Works`:
  - `farm = rural * fertile`, where `rural = 1 - urbanisation.clamp(0,1)` and
    `fertile = (body.resources.fertile_land / 1.5).clamp(0.15, 1.0)` for active land sites; a site
    at or below Village scale gets a `+0.25` smallholding bonus (clamped to 1). Zero on airless
    bodies.
  - `mine = deposit_kind.map(|k| (body.resources.get(k) / 2.0).clamp(0.15, 1.0) * labour)` where
    `labour = (site.population / 5_000).clamp(0, 1)`; zero without a `near_deposit`.
  - `port = coastal * sea * open`, where `sea` is 1 if any `Link` of kind `Sea` touches the site,
    else 0.35 if the civ has `seafaring`. Non-coastal sites are zero.
  - `industry = urban_share * electrified * tier_weight`, `urban_share =
    urbanisation.clamp(0,1)`, `electrified = (energy_per_capita / 2_000).clamp(0,1)`
    (2 kW/person ≈ fully electrified), `tier_weight = 0.25, 0.4, 0.6, 0.8, 1.0, 1.1` for
    Camp…Megacity. Zero without `electric_light` or any energy.
  - `spaceport = 1.0` for the largest active site when the civ has `satellites` **and**
    (`missions_launched > 0` or `satellites > 0` or `crewed_orbit`), else 0; `0.5` at the largest
    site with `spaceflight`-scale flags (`moon_landing`/`stations`/`colonies`).
  - `Works::dominant()` returns the highest intensity with a fixed tie-break order
    `Farm < Mine < Port < Industry < Spaceport` (later wins ties) so results are deterministic.
- `site_roles(civ, body, min_pop) -> Vec<SiteRole>` — active sites with `population >= min_pop`,
  sorted by `(population desc, index asc)`.
- `footprint(civ, body) -> Footprint`, aggregating:
  - `settlements`, `farms`/`mines`/`ports`/`industries`/`spaceports` (dominant-role counts),
  - `urban_km2` = Σ `population / density(tier)`, densities `{50, 400, 1_500, 3_000, 4_500,
    6_000}` people/km² for Camp…Megacity (representative built-up densities, stated as such),
  - `farmland_km2` = `population * 0.0015` (0.15 ha of cropland per person — the real global
    average order; Earth's 8e9 people → ~12 million km²) scaled by `fertile_land.clamp(0,1)`,
  - `road_km`/`rail_km`/`sea_km` = Σ chord lengths of links × body radius (true spherical chord,
    no artificial detour factor),
  - `electrified` (civ-level, 0..1), `energy_w_per_person`,
  - `extraction_share_per_year` = mining labour-weighted population × 1e-4 divided by the body's
    total metal endowment (a share of the modelled endowment per year; **not** a tonnage),
  - `fossil_years_left` = `(coal + oil) / annual_fossil_use` using the documented 2e22 J/unit, or
    `None` when the civ burns no fossils.
- `report(...) -> serde_json::Value`-friendly struct is built in the CLI, not the sim, to keep the
  sim free of presentation code. The sim exposes `Footprint` plus `resources_summary(body)`.

Tests (`civ/infrastructure.rs`):
1. `works_are_zero_without_the_material_basis` — no deposit → `mine == 0`; inland → `port == 0`;
   no `electric_light` → `industry == 0`; no satellites → `spaceport == 0`.
2. `works_track_authoritative_state` — electrification and urbanisation monotonic; a deposit site
   mines more than a bare site at equal population; a sea-linked coastal site ports more than a
   merely coastal one.
3. `footprint_is_deterministic_and_additive` — two calls equal; role counts sum to settlements;
   link lengths equal the analytic chord sum; `farmland_km2` for 8e9 people on `fertile_land = 1`
   is within 10–15 million km².
4. `dominant_role_has_a_fixed_tie_break` — ties resolve in the documented order.

### T2 — CLI `footprint` report

`cosmogon_cli/src/main.rs`:

```sh
cosmogon-cli footprint [--initial SAVE | --seed/--scenario/--years] [--civ ID] [--json]
```

- Reuses `initial_universe(a)`; advances only when `--initial` is absent (same semantics as
  `chronicle`). Validates `--civ` against `u.civs` before printing.
- Default output: one line per living civilization —
  `name | system/body | settlements N | farms F mines M ports P industry I spaceports S |
   built X km² | farmland Y km² | road/rail/sea km | electrified Z% | extraction W%/yr`.
  `--json` emits `{"format":"cosmogon-footprint","schema_version":1,"world":{...},
  "civilizations":[...], "limitations":[...]}` with the same numbers and an explicit limitation
  string ("derived from settlement, population, energy and endowment state; not a second
  simulation; areas use representative densities").
- Refusals: `--civ` unknown, `--json` with a non-footprint command, `--output` (not supported),
  `--initial` together with `--seed`/`--years`.
- Help text and the unknown-command list gain `footprint`.

Test (`main.rs` argument tests): `footprint_arguments_refuse_unknown_civ_and_ignored_flags`.

### T3 — Sphere markers: role glyphs and traffic

`crates/cosmogon/src/ui/markers.rs` (inside the civ loop, `px >= 40`):

- Compute `let fp = infrastructure::works(s, c, b)` for the largest ≤ 40 active sites and draw a
  small **role glyph** next to each settlement dot when `px >= 60`:
  - Mine — triangle (ore colour `200, 170, 120`),
  - Port — a short anchor-like arc (`120, 190, 255`),
  - Industry — square (`200, 130, 90`),
  - Spaceport — small ring (`230, 230, 255`),
  - Farm — no glyph (it would carpet the map); farms are represented only by the land-use tint and
    the inspector counts.
- Glyph alpha follows the existing `alpha` fade so zoomed-out views do not clutter.
- **Traffic**: for each road/rail/sea link, draw `1 + (population/2e9).clamp(0,2)` dots at
  `phase = frac(t / period + hash(link))`, period `rail 2 h`, `road 6 h`, `sea 12 h` of simulated
  time; colour follows the link; rail dots slightly larger. Cap total dots at 600 to keep the
  marker pass cheap. Traffic is visual only and derived from time + link identity.
- Keep draw order: links → traffic → settlements → glyphs → labels, so labels stay on top.

No new resources or systems; the existing `draw_markers` signature is unchanged.

### T4 — Inspector infrastructure section and land-use tint

- `ui/inspect.rs::civilization()` gains a `heading("Infrastructure")` block after the nation list:
  dominant-role counts, `urban_km2`, `farmland_km2`, link lengths, electrification, and the three
  largest sites with their role label. A `MUTED` caveat line states the derivation and that areas
  are representative densities, not measured footprints.
- `--panel civilization` sets `ui.body_tab = BodyTab::Civilization` (capture support, mirrors
  `--panel chronicle`).
- `render/bake.rs`: multiply each settlement's splat `core` by a role bias —
  `1.0 + 0.25 * industry + 0.15 * port - 0.2 * farm` (clamped 0.4–1.5) — so industrial belts read
  brighter and farmland dimmer in the night lights. The existing `electric` gate and sprawl are
  unchanged; `bake_surface`'s signature already receives the civ and body.

### T5 — Docs, validation, tracker

- `docs/INFRASTRUCTURE.md` (guide), `docs/INFRASTRUCTURE_DESIGN.md` (rules + constants + why
  derived, not simulated), `docs/INFRASTRUCTURE_VALIDATION.md` (what was run).
- `docs/SAVE_FORMAT.md`: one line that infrastructure is derived and never saved.
- `CHANGELOG.md`, `docs/BUILD_TRACKER.md` row 63, `docs/MASTER_GAP_ANALYSIS.md` entry 61,
  `HANDOFF.md`, `docs/CONTINUATION_HANDOFF.md` verification record.
- Captures:
  - `--focus Earth --distance 1.6 --advance-years 20000 --panel civilization` (inspector section),
  - the same view without the panel at `--distance 1.3` (glyphs + traffic on the sphere),
  - a `--panel chronicle`-style regression capture is not needed; markers are the change.

## 4. Golden-neutrality argument

| Change | Saved field? | RNG draw? | Mutates Universe? | Golden risk |
|---|---|---|---|---|
| `infrastructure.rs` | none | none | no (`&Civilization`, `&Body`) | none |
| CLI `footprint` | none | none | no (reads a cloned universe) | none |
| Marker glyphs/traffic | none | none | no (reads `Sim`) | none |
| Inspector section | none | none | no | none |
| Night-light role bias | none | reuses the same `mix` seeds in the same order; only multiplies an already-computed splat | no | none |

The bake runs only when a texture is baked (render-time), never inside `advance`; the golden test
advances the universe headless and serializes it. `cargo test -p cosmogon_sim
golden_universe_fingerprint` must stay green with the constant unchanged.

## 5. Validation plan

1. `cargo test --release --workspace` — expect the build-62 baseline 702 + ≥5 new = ≥707 passed,
   0 failed, 1 existing ignored.
2. `cargo clippy --release --workspace --all-targets` — must stay at the 43-warning / 0-error
   baseline, with no warnings in touched files.
3. `cargo test --release -p cosmogon_sim golden_universe_fingerprint` — `0xf2c4140a45ae509b`.
4. CLI: `footprint --seed 2026 --scenario sol --years 5000` (table) and `--json`; assert form/schema
   and that counts are consistent (`farms+mines+ports+industries+spaceports == settlements`); a
   filtered `--civ` run; refusal cases.
5. Save round-trip: run with `--save`, then `footprint --initial SAVE` must print the same numbers
   as the live run (derivation uses only saved state).
6. Captures inspected at the two viewpoints above.

## 6. Risks

- **Clutter.** Glyphs are capped (≤40 sites, `px >= 60`, ≤600 traffic dots) and alpha-faded;
  the inspector is the authoritative place to read the full picture.
- **False precision.** Areas use representative densities and farmland per capita; the docs, the
  CLI limitation string and the inspector caveat say so. No tonnage is invented.
- **Derivation drift.** If the civ step later changes a settlement's fields, the footprint changes
  with it — that is the intended coupling ("driven by authoritative state"), and the unit tests
  pin the mapping rather than raw numbers.
