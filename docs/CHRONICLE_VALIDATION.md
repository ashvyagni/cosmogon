# Validation — filters, Legends, protected export and the O(1) push counter

Build 62. Everything below was run against the release binaries built from this commit. The
golden fingerprint stays `0xf2c4140a45ae509b` and no save byte changed.

## What is covered

Chronicle *usability* and *export integrity* — not new history content. No event producer was
added or moved, so the recorded history of any world is exactly what it was before this build.

### Unit and integration tests

| Test | Checks |
|---|---|
| `history::tests::chronicle_filters_by_importance_category_text_and_scope` | `Category::index` slots (Astronomy=0 … Contact=7); importance clamp both sides (0 → 1, 9 → 5); category toggles; case-insensitive title **and** detail matching; empty search matches everything; exact AND scope for system/body/civ; an unknown system matches nothing |
| `history::tests::chronicle_is_deterministically_ordered_on_time_ties` | same-time events keep push order; repeated queries agree; `chronicle_indices` ordering is the basis of both views |
| `history::tests::history_minor_counter_prunes_exactly_like_the_legacy_recount` | 7,000 seeded pushes of mixed importance 1–5: `events` equal to an inline legacy-recount implementation after **every** push |
| `history::tests::history_serialisation_is_byte_identical_with_the_counter_cached` | serialize → push → serialize equal; deserialize → serialize byte-identical (the cache never reaches the JSON) |
| `chronicle::tests::chronicle_export_creates_manifest_json_and_csv` | `format`/`schema_version`/counts/first-last/filters/limitations/files; no `exported_at` wall-clock field; event JSON keys are exactly the serde names; chronological order; CSV header and row count; second export into the same folder refused and the original bytes survive |
| `chronicle::tests::chronicle_export_refuses_existing_dir_and_empty_result` | empty match refuses *before* creating the directory; an existing folder is protected, its contents untouched and nothing added |
| `chronicle::tests::chronicle_csv_escapes_free_text` | RFC-4180 quoting/doubling for commas, quotes and line breaks; CRLF records; `None` ids → empty cells; a minimal parser round-trips both tricky and plain rows exactly |
| `argument_tests::chronicle_arguments_require_output_and_refuse_ignored_flags` | `--output` required; unknown category, importance outside 1–5, `--body` without `--system`, `--save`/`--samples`/`--life`, `--initial` combined with `--seed`/`--years` all refuse; Chronicle flags refuse on other commands; defaults are `min_importance = 1` and all categories |

Test totals: **702 passed, 0 failed, 1 existing ignored** (build-61 baseline 694; +8).

### Golden-neutrality evidence

- `cargo test --release -p cosmogon_sim golden_universe_fingerprint` → `1 passed`, and
  `GOLDEN = 0xf2c4140a45ae509b` in `crates/cosmogon_sim/src/save.rs` is untouched.
- `round_trip_then_continue_matches_uninterrupted_run` and `version_1_saves_still_load` pass
  unchanged (they compare live and deserialized universes with the manual `History::PartialEq`).
- The legacy-save check below loads and exports a save whose `History` has no counter fields at
  all — the same situation as any pre-build-62 file.

### Legacy save (pre-existing history)

```sh
./target/release/cosmogon-cli run --seed 7 --years 3000 --save /tmp/b62/legacy.cosmo
./target/release/cosmogon-cli chronicle --initial /tmp/b62/legacy.cosmo --output /tmp/b62/legacy-export
```

Both exit 0. The save holds **33** events under `universe.history.events`; `manifest.event_count`,
`chronicle.json` rows and CSV data rows are all **33** (34 lines including the header), and the
run is not advanced by `--initial`.

### CLI round-trip

```sh
./target/release/cosmogon-cli chronicle --seed 2026 --scenario sol --years 5000 --output /tmp/b62/full
```

Exit 0. `format == "cosmogon-chronicle"`, `schema_version == 1`, `event_count == 45`,
`len(chronicle.json) == 45`, CSV data rows `== 45`; first/last times match the newest/oldest
events; importance mix `{5:1, 4:25, 3:10, 2:9}` and category mix
`{Life:29, War:10, Technology:3, Disaster:1, Astronomy:1, Civilization:1}`.

```sh
./target/release/cosmogon-cli chronicle --seed 2026 --scenario sol --years 5000 \
  --min-importance 4 --category war --category disaster --output /tmp/b62/filtered
```

Exit 0; its `event_count` (1) equals the count of importance ≥4 War/Disaster rows in the full
export, and every exported row satisfies the filter. The manifest records
`categories: ["Disaster","War"]`, `min_importance: 4`.

A text search (`--search war`, 10 rows) returns only rows whose title or detail contains "war"
case-insensitively.

### Refusals (each exits 1 with the named message)

| Invocation | Message |
|---|---|
| export into `/tmp/b62/full` again | `/tmp/b62/full: File exists (os error 17)` |
| `--category nonsense` | `Unknown Chronicle category nonsense; use astronomy, …` |
| `--min-importance 9` (and `0`) | `--min-importance must be between 1 and 5` |
| `--civ 999` | `Unknown civilization id 999` |
| `--system 99` | `Unknown system 99; this world has 24 systems` |
| `--search zzzznomatch` | `No events match the Chronicle filter` |
| missing `--output` | `chronicle requires --output NEW_DIR` |
| `--body 1` without `--system` | `--body requires --system` |
| `--save FILE` | `--save is not a chronicle option` |

Scope ids are validated against the universe before any directory is created, so a refused
invocation leaves no partial bundle.

### Captures

```sh
./target/release/cosmogon --new sol --advance-years 20000 --panel chronicle --focus Earth \
  --capture output/qa/b62-chronicle.png --capture-metadata --capture-after 240 --quiet --quit
./target/release/cosmogon --new sol --advance-years 20000 --panel chronicle-legends --focus Earth \
  --capture output/qa/b62-legends.png --capture-metadata --capture-after 240 --quiet --quit
```

Both exit 0 and were inspected:

- `output/qa/b62-chronicle.png` — Chronicle tab in Timeline mode: View (Timeline/Legends), Show
  (milestones/major/all), all eight category toggles, the search field ("title or detail"), an
  enabled "Scope to selection" checkbox (Earth is focused and therefore selected), and the label
  `20 events (showing 20)` above a newest-first list (Pandemic in red, Humanity and microbial
  milestones in the Life colour). Rows continue below the 480-px pane, as with every previous
  long list; the pane scrolls and the count label makes the total explicit.
- `output/qa/b62-legends.png` — Legends mode: `2 events (showing 2)` (scoped to Earth), era
  headers **The first year** (Humanity, 198,001 BCE) and **Years 1,000–10,000** (Pandemic,
  196,920 BCE), and the collapsed **Export** section. The 20-event universe-wide Timeline list
  versus the 2-event scoped Legends list is the expected difference: Legends always scopes to the
  selection, Timeline only on the checkbox.

## Limits (honest)

- The Chronicle records what the simulation writes: minor events are capped at 4,000 and older
  ones are pruned, so an export can only be as complete as the surviving vector. The manifest
  says so; this page repeats it.
- Events are narrative records, not measurements or uncertainty-bearing observations.
- The native export writes the single current filter; cross-location civilization rows that the
  Legends *view* unions in are not part of that bundle (documented in [CHRONICLE](CHRONICLE.md)).
- The 400-row render cap remains; filtering reduces the visible set but does not render more.
- No interactive transcription of typing/toggling is automated — those paths are exercised by
  the captures above plus the shared unit tests; the search/toggle behaviour itself lives in the
  tested filter API.
