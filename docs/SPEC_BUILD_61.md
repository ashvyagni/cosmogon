# SPEC — Build 61: cross-system blast→climate leg + analytic greenhouse validation suite

Status: spec only (no code written). Owner decisions (binding): both legs in ONE build, medium
scope, no phase split; spec lands before build 62; coding starts only after review.

## 1. Problem / context / objective

Phase 11 (`docs/MASTER_DEVELOPMENT_DIRECTIVE.md:3265`: orbit → climate → environment →
civilization → Chronicle → WHY) works *inside* a system — `refresh_climate`
(`crates/cosmogon_sim/src/universe.rs:212`) via `TASK_ENV` (`universe.rs:158`, period 10 d
`sandbox.rs:1379`) refreshes bodies whose insolation moves >0.2 % (`sandbox.rs:1318`) and
Chronicle-reports >2 % (`sandbox.rs:1329-1345`) — but the cross-system blast leg
(`propagate_blasts`, `stellar.rs:380-433`) applies biosphere/civ damage and **zero** temperature
forcing; and `docs/RESEARCH.md:176-177` + `docs/SCIENTIFIC_VALIDATION.md:36` still say climate is
unvalidated. Build 61 closes both: (a) a 0-D analytic greenhouse validation suite (hand-computed
fixtures, monotonicity, ice-albedo runaway, Earth ≈288 K) wired into the replayable `validate`
report plus a `CLIMATE{,_DESIGN,_VALIDATION}.md` triple; (b) a bounded, deterministic, save-safe
blast→climate transient flowing through habitability → civ capacity → Chronicle detail, with
end-to-end chain tests. `BUILD_TRACKER.md:115` gap 61 goes ⬜ → ✅.

**Out of scope (deferred, with reasons):**
- *Regional/latitude effects* — model is explicitly 0-D grey greenhouse (`environment.rs:1-8`);
  needs GCM-class work. Documented deferral in `CLIMATE.md`.
- *WHY causal panel* — builds 62/64/65 own it; this build only enriches Chronicle `Event.detail`.
- *Interstellar contact/communication modeling* — separate directive phase (62+).
- *New CLI subcommand* — the existing `validate` arm (`cosmogon_cli/src/main.rs:834-839`) already
  exports any extended `validation_report()` and exits 1 on regression; extend the report, not
  the CLI surface.

**Inputs used:** research handoff (re-verified; corrections §7), owner decisions, `HANDOFF.md`
capture template (:213-216), `docs/CONTINUATION_HANDOFF.md` build-57/59 golden records,
`BUILD_TRACKER.md`, directive §109. **Assumptions:** coding after review; no UI file changes
beyond Chronicle text produced in `cosmogon_sim`; golden re-pin forbidden unless §3 fallback.

## 2. Golden fingerprint decision (checked before tasks)

**`GOLDEN` stays `0xf2c4140a45ae509b` (`save.rs:645`); no re-pin expected.**
- Blast-in-transit **already persists**: `Blast { system, time, energy, reached }` is a serialized
  `Universe` field (`universe.rs:160` derive, `:178`); arrival times recompute deterministically
  from state (`stellar.rs:134-139`). No new field for transit.
- The post-arrival transient reuses `Body.impact_winter: Option<ImpactWinter>`
  (`astro/body.rs:293`), already `Serialize/Deserialize` (`impact.rs:91-96`) with absolute
  `start` ⇒ survives save/load. **T2 introduces no new serialized field.**
- Golden universe (`Scenario::Sol`, 5 systems, 5 kyr, `save.rs:636-642`) contains no blast.
- **Fallback if a new field proves unavoidable:** it must carry `#[serde(default)]` (build-57
  pattern, `docs/CONTINUATION_HANDOFF.md`: default-absent keeps golden byte-identical); if bytes
  still change, re-pin = run `golden_universe_fingerprint`, put printed value into `GOLDEN`
  (`save.rs:645`), note in `CHANGELOG.md` + `CONTINUATION_HANDOFF.md`. **Ask owner before
  re-pinning.**

## 3. Tasks

### T1 — Analytic greenhouse validation suite + CLIMATE docs triple
- **Files:** `planet/environment.rs` (tests mod :146-179), `research/validation.rs` (:79 report,
  :261 limitations, :314 case test), `research/registry.rs:103` (`climate` descriptor), docs.
- **Changes:**
  1. Unit tests in `environment.rs` (pure functions):
     - `equilibrium_temperature_matches_hand_computed_fixtures`: `(1,1,0)` = 278.6 K ±1e-9;
       `(1,1,0.3)` = 254.83 K ±0.05 (hand: 278.6·0.7^¼); identity `T_eq(4L,d,A)=√2·T_eq(L,d,A)`
       ±1e-9. Pure IEEE ops — no DMath needed; say why in the test comment.
     - `greenhouse_tau_is_monotone_in_co2`: 9-point CO₂ sweep 1e-5→0.5 bar (fixed 1-bar air) ⇒
       `greenhouse_tau` (:37) and `update_climate` temperature both non-decreasing.
     - `ice_albedo_runaway_has_a_threshold`: Earth-like body `real=false`, `climate_bias=0.0`,
       albedo 0.3, water 1.0; luminosity λ: 1.00→0.70 step 0.01; largest adjacent jump ⇒ warm
       side ≥265 K & albedo ≤0.42, frozen side ≤235 K & albedo ≥0.45, jump ≥20 K (feedback path
       :71-107; real bodies bypass it at :100, hence `real=false`).
     - Keep `model_reproduces_terrestrial_planets_without_calibration` (Earth 288±8, :151-161) as
       the named Earth regression case.
  2. Extend `validation_report()` (:79) with case `climate-analytic-greenhouse`, 4 metrics in the
     existing `value ≤ regression_limit` convention (:23-28), limits ≈10× observed (build-59):
     `equilibrium_fixture_error_K` (0.01), `earth_surface_temperature_error_K` (observed×10),
     `co2_doubling_out_of_band_K` (deficit form: 0 when ΔT ∈ [1.5,6] K per :171; limit 0.01),
     `ice_albedo_missing_jump_K` (max(0, 20−jump); limit 0.01). Update top-level `limitations`
     (:261) and `assert_eq!(a.cases.len(), 6)` (:314 → 7) plus new assertions.
  3. Registry: append new test paths to `climate.validation_cases` (`registry.rs:103`).
  4. Docs: create `docs/CLIMATE.md` (model scope, blast transient, "no regional effects"
     deferral), `CLIMATE_DESIGN.md` (forcing law, units, stacking, expiry),
     `CLIMATE_VALIDATION.md` (tests + CLI replay + limits). Reword `RESEARCH.md:176-177` and
     `SCIENTIFIC_VALIDATION.md:36`: analytic climate fixtures now covered; stellar/collision/civ
     models and regional/GCM behaviour remain unvalidated.
- **AC:** 4 new/kept tests green under `cargo test --release --workspace`; `cosmogon-cli validate`
  JSON contains `"climate-analytic-greenhouse"` and exits 0; both disclaimers reworded (grep);
  three docs exist.

### T2 — Blast→climate transient
- **Files:** `stellar.rs` (`propagate_blasts` :380-433), `impact.rs`, `sandbox.rs`
  (`step_environment` :1292, `refresh_body_environment` :1018, `ensure_environment_task`).
- **Forcing law (exact):** new pure `pub fn blast_winter_peak_k(severity: f64) -> f64` in
  `impact.rs` beside `winter_peak_cooling` (:153-155, the documented 30 K physical cap):
  `peak_k = severity.clamp(0.0,1.0) * BLAST_WINTER_MAX_K`, `pub const BLAST_WINTER_MAX_K = 30.0`.
  Units: kelvin of surface-air cooling (same sign/units as `ImpactWinter.peak_k`). `severity` is
  the existing bounded log-ratio at `stellar.rs:400` from `fluence` (:396). Only `+ − * / clamp`
  ⇒ IEEE bit-exact, no RNG, no transcendentals (DMath not required — documented).
- **Application (in `propagate_blasts`, at arrival, *before* the biosphere gate at :405):** each
  body with `exists() && kind.has_surface()` and `peak > 0.05` ⇒ stack onto any active winter as
  `ImpactWinter { start: t, peak_k: max(existing.cooling_at(t), peak), efold_years:
  ImpactWinter::EFOLD_YEARS }` (restart clock, keep larger instantaneous cooling ⇒ simultaneous
  blasts stay ≤30 K), then `self.refresh_body_environment(s, j, t)` (:1018) so ΔT + habitability
  land at arrival. **Not gated on biosphere presence/stage** (today :405 `continue`s lifeless and
  sterile worlds). Existing biosphere/civ damage and event (:419-428) stay biosphere-gated.
  **Do not touch `climate_bias`** — reserved for real-body calibration (`environment.rs:135-143`).
- **Recovery (verified):** 3-yr e-fold decay (`ImpactWinter::EFOLD_YEARS`, `impact.rs:99`;
  `cooling_at` :101-106) applied in `update_climate` (`environment.rs:108-110`), auto-expiry
  below 0.05 K (`impact.rs:108-110`). **Gap found:** expiry runs only in `step_environment`
  (`sandbox.rs:1310-1312`), which skips non-dynamic systems (:1295-1297) and is unscheduled in
  fully static universes (`ensure_environment_task` early-returns when no system is dynamic).
  Fix, event-driven: (a) on first blast-winter application call `self.ensure_environment_task()`
  after relaxing its guard to `any(is_dynamic) || any body has impact_winter`; (b) in
  `step_environment`, for non-dynamic systems, iterate only bodies with
  `impact_winter.is_some()` — clear expired, `refresh_body_environment` active; skip all flux
  bookkeeping (no `dynamics`). Scheduler contents change only for universes receiving a blast
  winter ⇒ golden unaffected.
- **Persistence:** covered by existing state (§2); any new field discovered anyway ⇒
  `#[serde(default)]` + §2 fallback.
- **AC (tests in `stellar.rs` tests mod — it can see private `RADIATION_FRACTION` :34-35):**
  `blast_winter_is_bounded_and_decays` (clamp 30 K; 3 e-folds <5 % of peak);
  `blast_forcing_reaches_worlds_without_biospheres` (lifeless surface body cools);
  `blast_winter_expires_on_static_system` (≈25 yr ⇒ `impact_winter == None`, temperature within
  0.1 K of pre-blast); `blast_transit_and_winter_survive_save_roundtrip` (save mid-transit via
  `save::to_json`, reload, advance ⇒ arrival time and drop identical to uninterrupted run).

### T3 — End-to-end chain tests
- **Files:** `stellar.rs` tests, `universe.rs` tests (:915), `sandbox.rs` tests (:1603).
- **Changes (3 tests):**
  1. `supernova_blast_drives_climate_habitability_capacity_and_chronicle` (`stellar.rs`):
     `Universe::new { scenario: Scenario::Sol, seed: <fixed>, system_count: 6 }` (Sol has Earth's
     biosphere + early humans; assert the precondition — neighbour system + Earth civ — and pick
     the seed at implementation time; never skip). Pick nearest system n≥1, inject
     `Blast { system: n, time: t0, energy: E, reached: vec![n] }` with `E` solved backwards from
     the real distance so severity ≈0.5 (no invented constants); `ensure_star_task()`; advance
     past arrival. Assert: event "Supernova radiation reaches Earth" with
     `t0 + d/c ≤ t_event ≤ t0 + d/c + 1 yr` (stellar grid `STAR_PERIOD` = 1 yr, `stellar.rs:102`);
     Earth ΔT at assert time = `peak·exp(−(t_assert−t_event)/3 yr)` within 1 %, `0 < drop ≤ 30`;
     habitability `score` fell >0.01 (`life.rs:107` cache via `assess`, `habitability.rs:54`);
     civ `capacity` fell after ≥1 civ step (`civ/mod.rs:367-374`, :501-503); event `detail`
     contains the T4 causal terms; a lifeless surface body in the same system cooled with no event.
  2. `blast_climate_chain_is_independent_of_chunking` (`universe.rs`, pattern of
     `outcome_is_independent_of_frame_chunking` :930): blast-injected universe, `advance_by(total)`
     vs 4-phase chunked `advance_to` ⇒ `serde_json::to_string` equal.
  3. `edit_drives_climate_and_chronicle_reports_the_chain` (`sandbox.rs`): SolarSystemLab +
     `apply_edit(Edit::SetStar { system: 0, property: StarProperty::Mass(1.1×) })` (`sandbox.rs:131`,
     `StarProperty::Mass` :71) → advance ≥2 env periods ⇒ temperature changed and a Chronicle event
     whose `detail` contains insolation %, ΔT in K, and the T4 habitability/capacity terms.
- **AC:** all three pass in `cargo test --release --workspace`; names referenced from
  `CLIMATE_VALIDATION.md` and the registry.

### T4 — Minimal surfacing + closeout
- **Files:** `sandbox.rs` (:1329-1345 event), `stellar.rs` (:419-428 event), trackers/docs, capture.
- **Changes:**
  1. Insolation Chronicle detail keeps "sunlight +x %" + "T a → b (Δ K)", appends
     `habitability {a:.0} % → {b:.0} %` (cached score read immediately before/after
     `refresh_body_environment`) and, when `civ_on(...) == Some`, `capacity {..}` (civ's cached
     capacity — recomputed at its next step; one-step staleness documented in `CLIMATE_DESIGN.md`).
  2. Blast Chronicle detail gains severity %, `−{peak} K` winter, resulting ΔT, and the same
     habitability/capacity terms (capacity term omitted when no civ).
  3. Closeout: `BUILD_TRACKER.md:115` row 61 → ✅ with evidence; `CHANGELOG.md` build-61 entry
     (golden: unchanged); `MASTER_GAP_ANALYSIS.md` gap-11 row; `HANDOFF.md` +
     `CONTINUATION_HANDOFF.md` record; registry `validation_cases` (from T1).
  4. Capture QA (one command, §4).
- **AC:** substring assertions (`habitability`, `capacity`, `K`) inside T3 tests 1 and 3 pin the
  surfacing machine-side; trackers greppably contain "61" with ✅/evidence; capture exits 0 with a
  non-empty inspected PNG.

## 4. Validation plan (exact commands, expected outcomes)

```sh
cargo build --release --workspace                                              # exit 0
cargo test  --release --workspace                                              # exit 0; 684 baseline + new tests (record exact count)
cargo clippy --release --workspace --all-targets                               # exit 0; 43-warning baseline, 0 errors, no new warnings in touched files
cargo test --release --workspace golden_universe_fingerprint                   # PASS; GOLDEN still 0xf2c4140a45ae509b
./target/release/cosmogon-cli validate --output /tmp/b61-climate-validate.json # exit 0; JSON has "climate-analytic-greenhouse"; rerun to 2nd path → byte-identical (diff)
./target/release/cosmogon --new lab --what-if sun_supernova --advance-years 400 \
  --capture output/qa/b61-blast-climate.png --capture-after 240 --quiet --quit # exit 0, non-empty PNG, inspected, no panic
```
Flags verified in `crates/cosmogon/src/args.rs`: `--new` :138 (accepts `lab`), `--what-if` :163
(any `WHAT_IFS` id, `sandbox.rs:1436`; `sun_supernova` :1447), `--advance-years` :150 (applies
*after* the what-if, `cosmogon/src/sim.rs:374-381`), `--capture` :156, `--capture-after` :157,
`--quit` :158, `--quiet` :229. **Capture limitation (documented, not worked around):** no CLI flag
opens the Chronicle tab (left panel defaults open but to Systems — `ui/mod.rs:165`,
`ui/inspect.rs:35`), so the capture is a render/robustness smoke of the post-supernova state;
Chronicle detail text is machine-verified by T3 tests. Adding `--panel chronicle` is out of scope.

## 5. Risks & edge cases

- **Determinism:** ordered index loops, pure IEEE arithmetic, no RNG/iteration-order dependence;
  guarded by T3 test 2 (chunk independence).
- **Perf:** forcing only at blast arrival (once per blast×system); static-winter branch is
  O(bodies-with-active-winters) per 10-day env step; no per-tick additions.
- **Old saves:** no schema change (impact_winter/Blast exist) ⇒ load unchanged; any new field
  that slips in ⇒ `#[serde(default)]` (§2 fallback).
- **Already-dead worlds:** forcing biosphere-independent; sterile worlds cool with zero
  biodiversity/stage (damage math untouched); one event per blast×system — no spam.
- **Multiple simultaneous blasts:** `max(remaining, new)` + clock restart ⇒ deterministic ≤30 K.
- **Beyond-harm distances:** fluence < `FLUENCE_HARM` (1e7 J/m² ≈ 9.4 ly for a canonical
  supernova) ⇒ no transient; documented as the existing harm-threshold semantics, not a new cutoff.
- **Static universes:** without the T2 guard/branch fix, winters decay in `cooling_at` but never
  clear and temperature is never re-sampled — hence the fix is mandatory.
- **Stale cached capacity** in Chronicle detail until the next civ step: documented honestly.
- **Capture cannot show Chronicle text:** accepted; unit tests carry that AC.

## 6. Corrections to the research handoff (all re-verified)

1. `TASK_ENV` is `universe.rs:158` (not 159); `ENVIRONMENT_PERIOD` is `sandbox.rs:1379` (not
   1378); `step_environment` is `sandbox.rs:1292`, gates at :1318 (0.002) / :1329 (0.02).
2. `propagate_blasts` spans `stellar.rs:380-433` (handoff :379-415); the biosphere gate at :405
   skips lifeless/sterile worlds entirely — climate forcing must be applied before it.
3. **Impact-winter recovery exists and is the right mechanism** (3-yr e-fold, `impact.rs:99-110`),
   but expiry is wired only into the dynamic branch (`sandbox.rs:1310`) and the env task may be
   unscheduled in static universes — spec adds the static path (T2).
4. `climate_bias` must NOT carry blast forcing — it is the real-body calibration offset
   (`environment.rs:135-143`); the brief listed it as an option.
5. No CLI change needed: `validate` (`cosmogon_cli/src/main.rs:834-839`) already exports the
   extended report and gates on `passes_regression`.
6. Report strings/tests needing edits: `validation.rs:261` (limitations), `:314` (6 → 7 cases).
7. Chronicle headless capture is impossible without a new flag (§4).

## 7. Forecast & Task Contract

- `estimated_scope`: **medium** (≈100–400 changed lines incl. tests/docs)
- `affected_files`: `planet/environment.rs`, `impact.rs`, `stellar.rs`, `sandbox.rs`,
  `universe.rs`, `research/validation.rs`, `research/registry.rs`, `cosmogon_cli/main.rs`
  (verify only, no change expected), `docs/CLIMATE{,_DESIGN,_VALIDATION}.md`, `docs/RESEARCH.md`,
  `docs/SCIENTIFIC_VALIDATION.md`, `BUILD_TRACKER.md`, `CHANGELOG.md`,
  `docs/MASTER_GAP_ANALYSIS.md`, `HANDOFF.md`
- `suggested_phases`: **none**

**Task Contract**
- `objective`: A supernova blast reaching another system yields a bounded (≤30 K), decaying,
  save-safe temperature transient that propagates to habitability, civ capacity and Chronicle
  detail, backed by an analytic greenhouse validation suite in the replayable CLI report and the
  CLIMATE docs triple — golden unchanged.
- `success_criteria`: §4 command block all exit 0 with stated expectations; tests named T1–T4
  pass; `validate` JSON contains the climate case; disclaimers reworded; trackers updated;
  capture PNG inspected.
- `non_goals`: regional/GCM effects, WHY panel, interstellar contact, new CLI subcommand,
  `--panel chronicle` flag, `climate_bias` usage, new serialized fields.
- `assumptions`: `ImpactWinter` is acceptable as the blast transient representation;
  severity→cooling is a linear bounded map; a fixed-seed Sol/6-system setup with a living target
  exists (asserted, seed chosen at implementation time).
- `open_questions`: none.
- `accepted_tradeoffs`: Chronicle capacity term is one civ-step stale; lifeless worlds get the
  transient without a Chronicle event; capture cannot display Chronicle text.
- `validation`: §4 command block + the named unit tests.
- `ask_abort_triggers`: golden test fails due to a new serialized field (ask owner before
  re-pin); ice-albedo threshold proves absent for the specified body/λ range; no fixed-seed
  multi-system setup with a living target can be found; workspace test/clippy failure outside the
  touched-file baseline.

## 8. Result Contract (spec closing)

- `status`: pass
- `summary`: Build-61 spec written — 4 tasks (T1 analytic greenhouse validation suite + docs
  triple, T2 blast→climate transient, T3 end-to-end chain tests, T4 surfacing/closeout), golden
  decision (no re-pin; fallback documented), full validation plan and risks; 7 research claims
  corrected after re-verification.
- `artifacts`: `docs/SPEC_BUILD_61.md`
- `next_recommended`: owner/lead review of the spec, then handoff to `developer` for build 61.
- `risks`: static-system winter expiry is a pre-existing wiring gap T2 must fix; multi-system test
  seed availability; capture cannot show Chronicle text (accepted).
- `skill_resolution`: none applicable (no Skill Resolution block in the handoff prompt; no listed
  skill matches specification authoring) — fallback to global instructions.

Markers: `implementation_decisions_count: 9` (forcing law, stacking rule, application point,
recovery wiring, no-new-field/golden decision, report case+metric shapes, doc triple, capture
choice, capacity-term staleness) · `testing_decisions_count: 11` (4 T1 + 4 T2 + 3 T3, with T4
substrings pinned inside T3) · `slices_defined: 4`
