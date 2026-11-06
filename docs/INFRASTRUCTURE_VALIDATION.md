# Validation — civilization infrastructure derivation

Build 63. Everything below was run against the release binaries built from this commit. The golden fingerprint stays
`0xf2c4140a45ae509b` and no save byte changed.

## Unit and integration tests

| Test | Checks |
|------|--------|
| `civ::infrastructure::tests::works_are_zero_without_the_material_basis` | No deposit → mine=0; inland → port=0; no `electric_light` → industry=0; no flight → spaceport=0 |
| `civ::infrastructure::tests::works_track_authoritative_state` | Industry monotonic in energy per person; deposit site mines more than bare site; sea-linked coast ports stronger |
| `civ::infrastructure::tests::footprint_is_deterministic_and_additive` | Two calls equal (pure); role counts sum to settlements; rail km equals analytic chord sum |
| `civ::infrastructure::tests::farmland_matches_the_stated_cropland_assumption` | 8 billion people → 10–15 million km² farmland (stated 0.15 ha/person assumption) |
| `civ::infrastructure::tests::dominant_role_has_a_fixed_tie_break` | Later role wins exact ties; all-zero → Farm; index mapping correct |
| `civ::infrastructure::tests::roles_rank_largest_settlements_first` | Site roles sorted by population descending |
| `argument_tests::footprint_arguments_refuse_unknown_ids_and_ignored_flags` | `--output`, `--min-importance`, `--category`, `--search`, `--system`, `--save`, `--initial`+`--seed`, `--chronicle`+`--json`, `run`+`--json` all refuse; `--civ`+`--json` accepted |
| `present_day::tests::present_day_humanity_is_seeded` | Real cities derive coastal/near_deposit from deposits and terrain (unchanged assertions) |
| `present_day::tests::humanity_carries_on_for_a_century` | Civ survives a century (unchanged assertions) |

Test totals: **709 passed, 0 failed, 1 existing ignored** (build-62 baseline 702; +7).

## Golden-neutrality evidence

- `cargo test --release -p cosmogon_sim golden_universe_fingerprint` → `1 passed`, `GOLDEN = 0xf2c4140a45ae509b` untouched.
- No new serialized field in `Civilization`, `Site`, `Body` or `Resources`; all infrastructure fields are derived at query time.
- The `#[serde(skip)]` pattern is used where caching would otherwise appear; no cache is added.

## CLI footprint report

```sh
./target/release/cosmogon-cli footprint --seed 2026 --scenario sol --years 5000 --json
```

Exit 0. `format == "cosmogon-footprint"`, `schema_version == 1`, one civilization (Humanity on Earth). Role counts,
areas, link lengths, electrification, fossil horizon and endowment all present. `limitations` array has 3 entries
stating the derivation is not a second simulation, areas use representative densities, and endowment is in relative units.

Table mode:

```
Humanity · Sol · Earth: 122 settlements (farms 122) · built 366 km² · farmland 603 km² · road 0 km / rail 0 km / sea 0 km · electrified 0% · mining workforce 0 · fossil horizon none
```

A longer run (`--scenario lab --years 5000`) shows mature infrastructure: farms/mines/ports/industry/spaceports,
rail/sea links, electrified 100%, fossil horizon 0 yr (depleted).

## Refusals (each exits 1)

| Invocation | Message |
|---|---|
| `--output` | `--output` is not a footprint option |
| `--min-importance` | `--min-importance` is not a footprint option |
| `--category` | `--category` is not a footprint option |
| `--civ 999` | Unknown civilization id 999 |
| `--initial s --seed 2` | Choose a saved --initial world or new --seed/--scenario/--years, not both |
| `chronicle --json` | `--json` requires footprint |
| `run --json` | `--json` requires footprint |

## Captures

```sh
./target/release/cosmogon --new lab --advance-years 2000 --focus Earth --distance 4.5 --panel civilization --capture output/qa/b63-inspector.png --capture-metadata --capture-after 240 --quiet --quit
./target/release/cosmogon --new lab --advance-years 2000 --focus Earth --distance 2.8 --capture output/qa/b63-sphere.png --capture-metadata --capture-after 240 --quiet --quit
```

Both exit 0 and were inspected:

- `output/qa/b63-inspector.png` — Civilization inspector, Infrastructure section: `260 · 90 ports · 169 industry sites · 1 spaceport`, built-up `1205070 km²`, farmland `15213799 km²`, transport `0 km road · 130154 km rail · 97839 km sea`, electrified `100%`, fossil horizon `0 years`, largest settlements with roles, role legend (mine ▲, port ⚓, industry ■, spaceport ○), and the derivation caveat.
- `output/qa/b63-sphere.png` — Sphere view of Earth: role glyphs beside major settlements (Tokyo spaceport ring, Barcelona port anchor, Athens industry square, etc.), transport links with animated traffic dots, population-sized settlement dots with capital rings, all fading with distance.

## Limits (honest)

- Infrastructure is a view of authoritative state, not a second simulation. No ore depletion, no trade, no prices.
- Areas use representative per-tier densities and 0.15 ha/person cropland — defensible orders of magnitude, not measured footprints.
- Endowment is in relative units (Earth = 1); only fossil fuels have a documented energy conversion and therefore a depletion horizon.
- Glyphs are capped (≤40 sites, `px >= 60`, ≤600 traffic dots) and alpha-faded; the inspector is the authoritative place to read the full picture.
