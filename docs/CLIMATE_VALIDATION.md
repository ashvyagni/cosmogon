# Validation — analytic greenhouse fixtures and the blast→climate chain

Build 61. Everything below is computed from the deterministic engine; no value here is hardcoded
into the interface. Golden fingerprint unchanged (`GOLDEN = 0xf2c4140a45ae509b`).

## What is covered

Analytic/self-consistency checks of the 0-D model and the end-to-end consequence chain —
**not** an observational climate validation and **not** regional/GCM behaviour.

### Unit tests (`crates/cosmogon_sim/src/planet/environment.rs`)

| Test | Checks |
|---|---|
| `equilibrium_temperature_matches_hand_computed_fixtures` | T_eq(1,1,0) = 278.6 K ±1e-9; T_eq(1,1,0.3) = 254.83 K (278.6·0.7^¼); homogeneity T_eq(4L,d,A) = √2·T_eq(L,d,A) ±1e-9 |
| `greenhouse_tau_is_monotone_in_co2` | 9-point CO₂ sweep 1e-5 → 0.5 bar in fixed 1-bar air: τ and surface temperature both non-decreasing |
| `ice_albedo_runaway_has_a_threshold` | 31-step luminosity sweep 1.00 → 0.70 L☉ drives the Earth-like fixture across a sharp warm → frozen jump (warm side ≥265 K / A ≤0.42; frozen side ≤260 K / A ≥0.45; largest jump ≥10 K) |
| `model_reproduces_terrestrial_planets_without_calibration` | Earth 288 ±8 K (plus Venus/Mars) with the calibration offset forced to zero |

Note on the ice-albedo gap: with a 1-bar Earth air the water-vapour greenhouse term stays on
while T_eq > 200 K, so the frozen side keeps most of its greenhouse. The collapse gap is
therefore ~15 K, not the ~20 K a dry model would give; the guard is 10 K and the measured jump
is printed by the test.

### Blast → climate chain (`crates/cosmogon_sim/src/stellar.rs`)

| Test | Checks |
|---|---|
| `blast_winter_is_bounded_and_decays` | forcing law pinned: severity 0/0.5/1 → 0/15/30 K, clamps outside [0,1]; 3 e-folds leave <5 % of peak |
| `blast_forcing_reaches_worlds_without_biospheres` | a lifeless surface world cools >1 K with **no** Chronicle event; living Earth reports one |
| `blast_winter_expires_on_static_system` | fully static universe: winter applied to every surface world at arrival, all cleared and within 0.1 K of pre-blast temperature 25 yr later |
| `blast_transit_and_winter_survive_save_roundtrip` | save mid-transit → reload → advance: identical arrival times, identical temperature drop, byte-identical final state vs the uninterrupted run |
| `supernova_blast_drives_climate_habitability_capacity_and_chronicle` | the full chain: event within 1 yr of d/c arrival; ΔT = peak·exp(−Δt/3 yr) within 1 %, 0 < drop ≤ 30 K; habitability fell >0.01; civ capacity fell after a civ step; detail carries `habitability`/`capacity`/`severity`/`winter`/`K`; lifeless body cooled silently |

### Determinism and report

- `blast_climate_chain_is_independent_of_chunking` (`universe.rs`): one `advance_by` vs four
  phased `advance_to` → byte-identical serialized universe.
- `edit_drives_climate_and_chronicle_reports_the_chain` (`sandbox.rs`): star-mass edit →
  insolation change → ΔT → Chronicle detail containing the insolation %, ΔT and
  habitability/capacity terms.
- `validation_report()` case `climate-analytic-greenhouse` with four regression-guarded metrics:
  `equilibrium_fixture_error_K` (≤0.01), `earth_surface_temperature_error_K` (≤5.41 K ≈ 10×
  observed), `co2_doubling_out_of_band_K` (deficit form, 0 while ΔT ∈ [1.5,6] K),
  `ice_albedo_missing_jump_K` (≤0.01).

## Replay

```sh
cargo test --release --workspace
cargo test --release --workspace golden_universe_fingerprint   # 0xf2c4140a45ae509b unchanged
./target/release/cosmogon-cli validate --output /tmp/b61-climate-validate.json
```

The `validate` JSON must contain `"climate-analytic-greenhouse"` and `"passes_regression": true`;
a second run to another path must be byte-identical (`diff`).

Render smoke (post-supernova state; no CLI flag opens the Chronicle tab, so the detail text is
machine-verified by the tests above):

```sh
./target/release/cosmogon --new lab --what-if sun_supernova --advance-years 400 \
  --capture output/qa/b61-blast-climate.png --capture-after 240 --quiet --quit
```

## Limits (honest)

- The equilibrium identities are true by construction — they pin the implementation against
  regressions, they do not certify the physics.
- The Earth case is one observed global-mean number with no latitude/season structure.
- The CO₂-doubling check only asserts the published 1.5–6 K band.
- Stellar, collision and civilization models are **not** validated by this report.
- Regional/GCM climate response remains unvalidated (deferred — see [CLIMATE](CLIMATE.md)).
- The Chronicle capacity term is at most one civ-step stale (see
  [CLIMATE_DESIGN](CLIMATE_DESIGN.md)).
