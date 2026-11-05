# Build 62 — Recorded history and Chronicle completeness (implementation spec)

Status: ready for review. **Golden-neutral build**: no save-schema change; `GOLDEN = 0xf2c4140a45ae509b`
(`crates/cosmogon_sim/src/save.rs:645`) must remain unchanged. Causal links / cause graph are
**deferred to builds 64/65** (owner decision). Build 61 is coded first; this spec assumes
`history.rs`, `ui/inspect.rs`, `ui/mod.rs`, `cosmogon_cli/src/main.rs` are untouched by 61 (61 is
stellar/climate) — re-verify at start. All file:line refs verified against current `main`.

## 1. Objective

Make the existing Chronicle (the ~5 dozen event producers already writing `History { events }`,
`crates/cosmogon_sim/src/history.rs:47-49`) fully usable: the Chronicle tab gains category
filtering, text search, selection scoping and an honest "N events (showing M)" count replacing the
silent `.take(400)` cap (`ui/inspect.rs:98`); events can be exported to disk as a protected,
manifest-carrying JSON/CSV bundle from both a new CLI `chronicle` subcommand and a native
"Export Chronicle" button; per-world/per-civ chronicles surface as one era-grouped "Legends" view;
and `History::push` stops doing an O(n) minor recount on every push. All of it is read/derive-only —
the serialized `Universe` never changes.

### Out of scope
- Causal links, cause graph, WHY-graph — builds 64/65 (gap `docs/MASTER_GAP_ANALYSIS.md:18`,
  directive §109 "Chronicle explains it", `docs/MASTER_DEVELOPMENT_DIRECTIVE.md:3265-3275`).
- Story-card / Markdown "Chronicle as a book" export — 1.x (`docs/PRODUCT_PLAN_1_0.md:52`, :103).
- **New §74 event types: none in this build** (§74 Chronicle, `MASTER_DEVELOPMENT_DIRECTIVE.md:2307`).
  Any new event *producer* firing inside the golden test's 5-kyr Sol window (seed 2026,
  `save.rs:637-638`) would change the fingerprint; new producers ride with consequence builds
  64/65 where a repin is deliberate. Build 62 = discoverability and export of existing events;
  zero new push sites.
- No notification-routing changes (inbox stays 500, `notifications.rs:10`).

## 2. Context (verified)

- Store: `Event { time, category, importance 1..=5, title, detail, system/body/civ: Option<u32> }`
  (`history.rs:33-44`), 8 `Category` variants (`history.rs:7-16`), `push` prunes minor (<3) beyond
  `MINOR_CAP = 4000` oldest-first via a full recount each call (`history.rs:52-63`). ~42
  `history.push(...)` call sites (grep; civ events batch as `CivEvent` in `civ/mod.rs` then drain).
- Renderers: Chronicle tab = importance filter + `.rev().take(400)` + click-to-focus
  (`ui/inspect.rs:90-116`); ticker = newest 3 with importance ≥4 (`ui/hud.rs:246`); per-body 120
  (`ui/inspect.rs:584`); per-civ 60 (`ui/inspect.rs:1119`); timeline milestones (`ui/timeline.rs:131`).
  Chronicle lives in `Universe`, is saved (`docs/SAVE_FORMAT.md:117-119`).
- Protected-export conventions: research bundles create the destination exclusively, refusing
  existing dirs, with `manifest.json` carrying `"format"/"schema_version"`
  (`cosmogon_sim/src/research/recording.rs:237-248`); CLI refuses existing outputs
  (`cosmogon_cli/src/main.rs:1000-1002`, `:1250` "Existing exports are protected"); single-file UI
  exports use a path field + `create_new(true)` + status label (`ui/inspect.rs:1213-1231`).
- CLI: hand-rolled `Args` + `parse_args` (`main.rs:94`), unknown-flag refusal (`main.rs:387`),
  per-command validation block (`main.rs:430+`), dispatch match (`main.rs:740-847`), unknown-command
  text (`main.rs:844`). **Correction to research: there is no `crates/cosmogon_cli/src/args.rs`**;
  `crates/cosmogon/src/args.rs` is the *app* parser (`--panel` :166, `--capture/--capture-after/
  --quiet` :156-158/:229, `--capture-metadata` :170, `--advance-years` :150). `--panel` values
  dispatch in `ui/mod.rs:282-306`; launch-requiring panels validate at `args.rs:258/286` — copy that.
- No CSV free-text escaping exists (writers are numeric, `recording.rs:218-236`); no `--format` flag
  in the CLI. Tracker already reserves build 62 under this name (`docs/BUILD_TRACKER.md:116`).

## 3. Tasks

### T1 — Chronicle query API + tab filtering/search/scoping
Files: `cosmogon_sim/src/history.rs`, `cosmogon/src/ui/mod.rs`, `cosmogon/src/ui/inspect.rs`.

`history.rs` (read-only additions, no field changes):
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct ChronicleFilter {
    pub min_importance: u8,        // clamped to 1..=5; default 1
    pub categories: [bool; 8],     // index = Category::index(); default all true
    pub text: Option<String>,      // case-insensitive substring over title + detail
    pub system: Option<u32>, pub body: Option<u32>, pub civ: Option<u32>,
}
impl Default for ChronicleFilter { /* all categories, min_importance 1, no scope */ }
impl Category { pub fn index(self) -> usize } // Astronomy=0 … Contact=7; inherent method only
impl History {
    /// Matches in chronological order; ties broken by original push (vector) index
    /// (stable sort, so output is fully deterministic).
    pub fn chronicle(&self, f: &ChronicleFilter) -> Vec<&Event>;
}
```
UI: `UiState` (`ui/mod.rs`) gains `chronicle_categories: [bool; 8]` (default all true),
`chronicle_search: String`, `chronicle_scope: bool` (scope to `sim.selected`: `Target::Star(s)` →
`system`, `Target::Body(r)` → `system`+`body`, plus `civ` when a civ sits on the body) and
`chronicle_mode: bool` (false = Timeline, true = Legends, used by T3). Chronicle tab
(`ui/inspect.rs:90-116`): a row of 8 category toggle buttons, a search `text_edit_singleline`, a
"Scope to selection" checkbox (greyed "Select a world" when nothing is selected), and a header
label `"{N} events (showing {M})"` where N = `chronicle().len()`, M = `min(N, 400)` — the render
cap stays 400 but is no longer silent. Click-to-focus and hover detail unchanged.

AC: `history.rs` tests `chronicle_filters_by_importance_category_text_and_scope` and
`chronicle_is_deterministically_ordered_on_time_ties`; capture `output/qa/b62-chronicle.png` shows
toggles, search box and count label, with N dropping when a category is toggled off.

### T2 — Protected Chronicle export (CLI + native button)
Files: new `cosmogon_sim/src/chronicle.rs` (+ `pub mod chronicle;` in `lib.rs` near :50),
`cosmogon_cli/src/main.rs`, `cosmogon/src/ui/inspect.rs`, `cosmogon/src/ui/mod.rs`.

New module (pure + one writer; takes `&Universe`, never `&mut`):
- `chronicle::rows(&Universe, &ChronicleFilter) -> Vec<Row>` — Row adds a `date` string via
  `time::format_date(e.time, u.start_time, u.gregorian())` (`ui/inspect.rs:101` precedent).
- `chronicle::export(&Universe, &ChronicleFilter, dir: &Path) -> Result<(), String>` —
  exclusively creates `dir` (refuse existing, mirror `recording.rs:237-243`) and writes:
  - `manifest.json`: `{"format": "cosmogon-chronicle", "schema_version": 1, "app_version": …,
    "filters": …, "event_count": …, "first/last_event_time_s": …, "files": [...] , "limitations":
    ["The Chronicle retains at most 4000 minor events; older minor events were pruned during the
    run and cannot be exported.", "Events are simulation-authored narrative records, not
    measurements."]}` — no wall-clock fields, so re-export is byte-identical.
  - `chronicle.json`: flat array of events with the exact serde field names
    (`time, category, importance, title, detail, system, body, civ`).
  - `chronicle.csv`: header `time_s,date,category,importance,title,detail,system,body,civ`;
    free-text fields RFC-4180 quoted (new convention — none exists); `None` ids = empty cell.
  - Empty result → refuse "No events match the Chronicle filter" (precedent `recording.rs:239-241`).
CLI: subcommand `chronicle` in the dispatch match + usage text (`main.rs:844`), flags
`--initial SAVE | --seed/--scenario/--years` (reuse `initial_universe`, `main.rs:1025`),
`--output NEW_DIR` (required), `--min-importance` (existing field, `main.rs:366`), `--category NAME`
(repeatable, new field), `--search TEXT`, `--system N`, `--body N`, `--civ N` (new fields).
Validation-block entry (pattern `main.rs:430+`): missing `--output`, unknown category name,
importance outside 1..=5, unknown system/body/civ ids → refusal before any I/O. Print the manifest
JSON on success (pattern `main.rs:817`). Update the module doc header (`main.rs:3-8`).
Native: Chronicle tab bottom "Export" collapsing header cloned from the surface-reading export
(`ui/inspect.rs:1213-1231`): path field defaulting to `persistence::data_dir().join("chronicle")`,
label "JSON + CSV with a manifest. Existing folders are protected. Exports the current filter.",
button calls `chronicle::export` with the *current UI filter* (T1), status label reports
"Saved N events to …" or the refusal.

AC: `chronicle` module unit tests (`chronicle_export_creates_manifest_json_and_csv`,
`chronicle_export_refuses_existing_dir_and_empty_result`, `chronicle_csv_escapes_free_text`);
CLI `argument_tests` addition `chronicle_arguments_require_output_and_refuse_ignored_flags`
(pattern `main.rs:1542+`); end-to-end loop in §5.4.

### T3 — Per-world/per-civ "Legends" era-grouped view
Files: `cosmogon/src/ui/inspect.rs`, `cosmogon/src/ui/mod.rs`.

Chronicle tab gains a `Timeline | Legends` `selectable_value` pair (`chronicle_mode`).
Legends mode renders the T1-filtered events scoped to the selection as one chronological,
era-grouped narrative: for `Target::Body(r)` union `chronicle(body=r)` with
`chronicle(civ=c)` for each `u.civs` entry whose `(system, body) == r` (dedup by vector index);
for a selected civ reuse its body + civ queries; no selection → universe-wide. Era headers are
decade-of-magnitude buckets over years since `u.start_time`: "The first year", "Years 1–10",
"Years 10–100", … (bounds 1, 10, 100 … powers of ten; at most ~15 groups for a 14-By universe).
Rows reuse the existing per-body styling (date + colored title + hover detail,
`ui/inspect.rs:584-596`). The compact per-body (120) and per-civ (60) lists stay as they are.

AC: capture `output/qa/b62-legends.png` (`--panel chronicle-legends`) showing era headers and
events for a selected world; no-selection capture shows universe-wide eras; ordering matches T1's
deterministic ordering (inspection + the shared unit tests).

### T4 — O(1) minor-event counter in `History::push` (golden-neutral)
File: `cosmogon_sim/src/history.rs` (and nothing else).

`History` gains `#[serde(skip)] minor: Option<usize>` (cached count of importance-<3 events) plus
`#[serde(skip)] first_minor: Option<usize>` (index cursor of the oldest minor event). `push`:
`minor.get_or_insert_with(recount)` (lazy — covers `Default` and deserialization); increment on
minor pushes; when `minor > MINOR_CAP`, remove at `first_minor` and advance the cursor by a forward
scan (amortized O(1); each index is rescanned at most once); `first_minor` is recomputed lazily
alongside `minor`, so a stale cursor is impossible. Both fields are `serde(skip)` → absent from
`serde_json::to_string(Universe)` in both directions → the golden FNV input (`save.rs:631-632`) is
byte-identical; deserialization yields `None` and recounts on first push. **`History` must drop
`#[derive(PartialEq)]` for a manual impl comparing only `events`** (the cache is not semantic
state) — required by `save.rs:307` and `:329`, which `assert_eq!` a live Universe against a freshly
deserialized one.

Byte-identity proof chain: (a) existing `golden_universe_fingerprint` (`save.rs:635-644`, constant
:645), `round_trip_then_continue_matches_uninterrupted_run` (`save.rs:301`), `version_1_saves_still_load`
(`save.rs:325`) all pass unchanged; (b) new test `history_minor_counter_prunes_exactly_like_the_legacy_recount`
(deterministic seeded mix of importances 1..=5, several thousand pushes, `events` compared after
every push against an inline copy of the legacy recount algorithm); (c) new test
`history_serialisation_is_byte_identical_with_the_counter_cached` (serialize → push → serialize →
equal; deserialize → serialize → byte-identical).

AC: both new tests pass; all three pre-existing save tests pass with the golden constant
untouched; legacy-save check in §5.3 passes.

### T5 — Closeout
Files: docs only (+ `docs/SAVE_FORMAT.md` one-paragraph note that `cosmogon-chronicle/1` is a
read-only export, Chronicle-in-Universe unchanged, `SAVE_FORMAT.md:117-119` section).

Follow the current triple-doc convention (builds 55–60): `docs/CHRONICLE.md` (user guide:
filters, scoping, Legends, CLI + native export, partial-history honesty note),
`docs/CHRONICLE_DESIGN.md` (filter API, export schema, counter design + serde rationale),
`docs/CHRONICLE_VALIDATION.md` (evidence: commands, counts, refusals, captures). Then, in the
same commit: `CHANGELOG.md` new `## Unreleased — recorded history and Chronicle completeness
(build 62)` section (pattern: build 60 entry); `docs/BUILD_TRACKER.md:116` mark ✅ with evidence;
append a numbered entry to `docs/MASTER_GAP_ANALYSIS.md` following the build-60 entry (:149);
update the active-build paragraph in `HANDOFF.md`.

AC: all five docs exist and cross-link; tracker/changelog/gap-analysis updated in one commit;
captures referenced by path (`output/` is untracked — record commands, not binaries).

## 4. Golden-neutral proof

| Task | Why serialized `Universe` bytes cannot change |
|---|---|
| T1 | Adds methods to `History` and app-only `UiState` fields; no `Universe`/`History` fields. Golden hashes `serde_json::to_string(universe)` (`save.rs:631-632`) — inputs untouched. |
| T2 | New `chronicle` module takes `&Universe`; writes only to the destination directory. No `Edit`, no `History::push`. |
| T3 | App-crate rendering only. |
| T4 | The two new counters are `#[serde(skip)]`: serde omits them from both `Serialize` and `Deserialize`, so the JSON string (the golden's sole input) is byte-identical; deserialization yields `None` and lazily recounts on first push. Manual `PartialEq` keeps `assert_eq!(live, loaded)` meaningful without touching bytes. The pruning equivalence test proves the *events vector* — the only serialized part — evolves exactly as before. |
| T5 | Docs only. |

## 5. Validation plan

Order: finish T1–T4, then run (re-baseline test counts against whatever build 61 leaves;
build-60 baseline was 684 passed + 1 ignored, `docs/CONTINUATION_HANDOFF.md:17`).

1. `cargo test --release --workspace` → exit 0; new count = baseline + 5 (3 history, 3
   chronicle-export, 1 CLI argument test; adjust to actual) + 1 ignored; `golden_universe_fingerprint`
   passes with `GOLDEN = 0xf2c4140a45ae509b` — command:
   `cargo test --release -p cosmogon_sim golden_universe_fingerprint`.
2. `cargo clippy --release --workspace --all-targets` → exit 0, baseline 43 warnings / 0 errors,
   no new warnings in touched files.
3. Legacy-save check: *before* editing `history.rs`, generate the fixture with the current binary —
   `./target/release/cosmogon-cli run --seed 7 --years 3000 --save /tmp/b62/legacy.cosmo` — then
   after T4: `./target/release/cosmogon-cli chronicle --initial /tmp/b62/legacy.cosmo --output /tmp/b62/legacy-export`
   succeeds and `manifest.event_count` equals the `"category"` occurrences inside the save's
   `history.events` array (python3 json check).
4. CLI export round trip:
   `./target/release/cosmogon-cli chronicle --seed 2026 --scenario sol --years 5000 --output /tmp/b62/full`
   → then `/tmp/b62/full/manifest.json` has `format == "cosmogon-chronicle"`, `schema_version == 1`;
   `python3 -c` check: `len(json.load(open('chronicle.json'))) == event_count ==
   sum(1 for line in csv.reader(open('chronicle.csv'))) - 1`.
   Filtered rerun `--min-importance 4 --category war --category disaster -o /tmp/b62/filtered`
   → its `event_count` equals the count of importance ≥4 War/Disaster rows inside
   `/tmp/b62/full/chronicle.json`; rerun of any command into an existing dir → refusal;
   `--category nonsense`, `--min-importance 9`, `--civ 999`, missing `--output` → refusals
   (each exits non-zero with the named message).
5. Capture QA (build first: `cargo build --release -p cosmogon`; `output/` is untracked):
   `./target/release/cosmogon --new sol --advance-years 20000 --panel chronicle --focus Earth --capture output/qa/b62-chronicle.png --capture-metadata --capture-after 240 --quiet --quit`
   `./target/release/cosmogon --new sol --advance-years 20000 --panel chronicle-legends --focus Earth --capture output/qa/b62-legends.png --capture-metadata --capture-after 240 --quiet --quit`
   (`--panel` values are new and verified consistent with the dispatcher `ui/mod.rs:282-306`;
   `--advance-years` pre-advances before observing, `sim.rs:379`; `--capture*`/`--quiet`/`--quit`
   at `args.rs:156-158/229/158`). **Inspect both PNGs**: toggles/search/count label legible; era
   headers render; no clipped rows at the 480-px pane height (document if below the fold, as
   build 55 did).
6. Interactive spot check (30–60 s normal run per `HANDOFF.md`): type in search, toggle a
   category, select a moon and confirm scoping + click-to-focus still work.

## 6. Risks & edge cases

- **Partial exports from 4000-minor pruning**: old minor events no longer exist; manifest
  `limitations` and the guide must say so plainly (never imply a complete record); milestones are
  complete forever.
- **Search/filter cost per frame**: `chronicle()` is O(events) with substring scans; bounded in
  practice (≤4000 minor + milestones; the timeline already iterates all events per frame,
  `ui/timeline.rs:131`). If a run exceeds ~50k events, cache keyed on `(events.len(), filter)` —
  defer unless measured.
- **Notification/Chronicle divergence**: notifications follow the tail by identity
  (`notifications.rs:110-113`) and tolerate pruning; T4 preserves push/prune ordering exactly
  (equivalence test), so this build cannot introduce divergence.
- **Tie ordering**: same-time events ordered by push index (stable sort) — deterministic
  cross-platform; stated in the design doc.
- **UI scope creep**: Legends is a Chronicle-tab mode only; per-body (120) and per-civ (60) panels
  untouched; no new windows or dock changes.
- **Build-61 ordering**: if 61 unexpectedly touches `history.rs`/`inspect.rs`/CLI main, rebase
  T1–T4 and re-run §5 before claiming green.

## 7. Forecast

- `estimated_scope`: **medium** (≈300 lines: ~90 sim, ~120 app UI, ~60 CLI, ~50 tests).
- `affected_files`: `cosmogon_sim/src/{history.rs,chronicle.rs(new),lib.rs}`,
  `cosmogon/src/{ui/mod.rs,ui/inspect.rs,args.rs}`, `cosmogon_cli/src/main.rs`,
  `docs/CHRONICLE{,_DESIGN,_VALIDATION}.md`, `docs/SAVE_FORMAT.md`, `CHANGELOG.md`,
  `docs/BUILD_TRACKER.md`, `docs/MASTER_GAP_ANALYSIS.md`, `HANDOFF.md`.
- `suggested_phases`: **none** — land T1→T4 in one build, T5 in the closing commit.

## Task Contract
- objective: Chronicle is filterable, scoped, era-grouped, exportable (CLI + native, protected, manifest-carrying) and O(1)-push, with the golden `0xf2c4140a45ae509b` and all save bytes unchanged.
- success_criteria: §5 steps 1–6 all pass; 2 new history tests + 3 chronicle tests + 1 CLI argument test green; both captures inspected; refusals exercised; legacy save loads and exports.
- non_goals: causal links/cause graph (64/65); book/story-card export (1.x); new event producers or §74 types; notification changes; changes to per-body/per-civ compact lists; any save-schema change.
- assumptions: build 61 leaves the touched files alone; test/clippy baselines re-measured after 61; `--panel chronicle`/`chronicle-legends` are acceptable new dev-flag values.
- open_questions: none.
- accepted_tradeoffs: render cap stays 400 (honest label, not unbounded rendering); native and CLI share one folder-bundle writer (second export to an existing path is refused, user edits the path); RFC-4180 quoting is a new CSV convention (no reusable helper exists).
- validation: §5 commands and outcomes as written.
- ask_abort_triggers: golden test would change; save round-trip tests fail after the counter change; build 61 has already modified `history.rs`/`inspect.rs` conflictingly; owner rejects an accepted tradeoff.

## Result Contract
- status: pass (spec produced; not yet implemented).
- summary: Build-62 spec with 5 tasks (Chronicle query+UI, protected JSON/CSV export CLI+native, Legends view, golden-neutral O(1) push counter, closeout), golden-neutral proof table and an exact validation plan.
- artifacts: `docs/SPEC_BUILD_62.md`.
- next_recommended: owner/spec review, then `developer` starting with T1 (and the pre-T4 legacy save fixture of §5.3).
- risks: partial-export honesty; per-frame filter cost at very large event counts; build-61 file overlap (all in §6).
- skill_resolution: none loaded (no Skill Resolution block); repo conventions verified directly against `HANDOFF.md`, `AGENTS.md`, changelog/tracker/validation-doc precedents.
