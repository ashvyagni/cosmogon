# Design — civilization infrastructure derivation

Build 63. Everything here is read/derive-only: no `Edit`, no RNG, no new serialized field, no mutation of `Universe`.
The golden fingerprint `0xf2c4140a45ae509b` is unchanged.

## Module `civ::infrastructure` (`crates/cosmogon_sim/src/civ/infrastructure.rs`)

Pure functions operating on `&Civilization` and `&Body`. No `serde`, no `rand`, no mutation.

### `Role` enum

Five economic roles with stable indices: Farm=0, Mine=1, Port=2, Industry=3, Spaceport=4.

### `Works` — per-settlement function intensities (each 0..=1)

```rust
pub struct Works { pub farm, mine, port, industry, spaceport: f64 }
```

Computed by `works(index, civ, body)`:

- `farm = rural * fertile` where `rural = (1 - urbanisation).clamp(0,1)`, `fertile = (fertile_land / 1.5).clamp(0.15, 1.0)`; small settlements (≤ Village) get +0.25 smallholding bonus; zero on airless bodies.
- `mine = (resource_get(kind) / 2.0).clamp(0.15, 1.0) * labour` only when `near_deposit == Some(kind)`, where `labour = (population / 5000).clamp(0,1)`; zero without a deposit.
- `port = coastal * sea` where `sea = 1.0` if a sea link touches the site, `0.35` if the civ has `seafaring`, `0` otherwise; zero for non-coastal sites.
- `industry = urbanisation * electrified * tier_weight` where `electrified = (energy_per_capita / 2000).clamp(0,1)` (requires `electric_light`), tier_weight = {0.25, 0.4, 0.6, 0.8, 1.0, 1.1} for Camp…Megacity; zero without `electric_light`.
- `spaceport = 1.0` for the largest active site when the civ has flight flags (`satellites`, `missions_launched > 0`, `satellites > 0`, `moon_landing`, `stations`, `colonies`); `0.5` at the largest site with spaceflight-scale flags only; zero otherwise.

`Works::dominant()` returns the strongest intensity; ties resolve in `Role::ALL` order, later winning, so deterministic.

### `site_roles(civ, body, min_population)`

Active settlements with `population >= min_population`, sorted by `(population desc, index asc)`, each with its dominant role and intensity.

### `footprint(civ, body)` — aggregate

| Field | Derivation |
|-------|-----------|
| `settlements` | count of active sites |
| `farms/mines/ports/industries/spaceports` | dominant-role counts |
| `urban_km2` | Σ population / density(tier), densities {50,400,1500,3000,4500,6000} people/km² |
| `farmland_km2` | population * 0.0015 * fertile_land.clamp(0,1) (0.15 ha per person) |
| `road_km/rail_km/sea_km` | Σ chord length of links × body radius / 1000 |
| `mining_population` | Σ population at mine-dominant sites |
| `electrified` | society-wide electrification (0..=1) |
| `energy_w_per_person` | civ energy per capita |
| `fossil_years_left` | (coal + oil) / annual fossil burn, or None |

`footprint::roles()` returns `[farms, mines, ports, industries, spaceports]`; sum always equals `settlements`.

### `endowment(body)`

Returns `[(ResourceKind, f64)]` for resources with positive endowment, in relative units (Earth = 1).

### `site_dir(site)`

Returns `[lat, lon]` as a unit direction vector, used by the marker layer.

### Why derived, not simulated

The civ step already models settlement founding, population distribution, link building, technology adoption and
resource consumption. A separate economic simulation would need its own state, its own save fields, its own calibration
and its own risk of disagreeing with the civ step. Derivation avoids all of that: the footprint is always consistent
with the authoritative state by construction.

### Constants and assumptions (all stated)

- `ELECTRIFIED_W_PER_PERSON = 2000` W — saturation point for full electrification.
- `URBAN_DENSITY = [50, 400, 1500, 3000, 4500, 6000]` people/km² by tier — representative built-up densities.
- `CROPLAND_KM2_PER_PERSON = 0.0015` km² — 0.15 ha per person, the real global average order of magnitude.
- `FOSSIL_JOULES_PER_UNIT = 2.0e22` J — from the simulation's own documented constant (`civ/mod.rs:32`).
- Mine labour threshold 5000 people; deposit angle threshold 0.12 rad; smallholding bonus +0.25; port seafaring partial 0.35.

### UI wiring

`crates/cosmogon/src/ui/markers.rs`:

- Role glyphs beside the largest ≤40 active settlements when `screen_radius >= 60` px: mine (triangle), port (anchor), industry (square), spaceport (ring). Farms have no glyph.
- Traffic dots along links: `1 + clamp(population / 4e9, 0, 2)` dots per link, period rail 2 h / road 6 h / sea 12 h of simulated time, phase = frac(t / period + hash(link_id)), capped at 600 total dots.
- Alpha follows the existing distance fade.

`crates/cosmogon/src/ui/inspect.rs`:

- Infrastructure section in `civilization()` after Settlements, before Species: role counts grid, areas, transport, electrification, workforce, fossil horizon, largest settlements with roles, role legend, caveat line.

`crates/cosmogon/src/ui/mod.rs`:

- `--panel civilization` sets `ui.body_tab = BodyTab::Civilization`.

`crates/cosmogon/src/render/bake.rs`:

- `bake_lights_with` gains an `Option<&Body>` parameter; each settlement's splat core is multiplied by `1.0 + 0.25*industry + 0.15*port - 0.2*farm` (clamped 0.4–1.5).

### CLI

`crates/cosmogon_cli/src/main.rs`:

- New `footprint` command reusing `initial_universe(a)`, advancing unless `--initial`.
- Validates `--civ` against `u.civs` before printing.
- `--json` emits `{"format":"cosmogon-footprint","schema_version":1,"world":{...},"civilizations":[...],"limitations":[...]}`.
- Table mode prints one line per living civilization.
- Refusals: unknown `--civ`, `--json` on non-footprint commands, `--initial` with `--seed/--scenario/--years`.

## Tests

Six unit tests in `civ/infrastructure.rs`:

1. `works_are_zero_without_the_material_basis` — no deposit → mine=0; inland → port=0; no `electric_light` → industry=0; no flight → spaceport=0.
2. `works_track_authoritative_state` — industry monotonic in energy; deposit site mines more than bare site; sea-linked coast ports stronger than merely coastal.
3. `footprint_is_deterministic_and_additive` — pure (two calls equal); role counts sum to settlements; rail km equals analytic link sum.
4. `farmland_matches_the_stated_cropland_assumption` — 8e9 people on fertile_land=1 → 10–15 million km².
5. `dominant_role_has_a_fixed_tie_break` — later role wins ties; all-zero → Farm.
6. `roles_rank_largest_settlements_first` — sorted by population desc.

One present-day test extension: `seed_present_humanity` now derives coastal/near_deposit for real cities from deposits and terrain.

## Cost

O(settlements + links) per call, dominated by the civ step's existing O(settlements²) adjacency build. No caching; the marker layer calls `works` for ≤40 sites per frame.
