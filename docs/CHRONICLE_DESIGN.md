# Design — Chronicle queries, Legends eras, export schema and the O(1) push counter

Build 62. Everything here is read/derive-only with respect to saved state: no `Edit`, no new
serialized field, no new event producer. The golden fingerprint `0xf2c4140a45ae509b` is
unchanged by construction (see the table in `docs/SPEC_BUILD_62.md` §4).

## Query API (`crates/cosmogon_sim/src/history.rs`)

```rust
pub struct ChronicleFilter {
    pub min_importance: u8,          // clamped to 1..=5 when matching; 1 keeps everything
    pub categories: [bool; 8],       // index = Category::index(); default all true
    pub text: Option<String>,        // case-insensitive substring over title + detail
    pub system: Option<u32>, pub body: Option<u32>, pub civ: Option<u32>,
}
```

`ChronicleFilter::matches` is a pure predicate: importance floor, category toggle, then each
`Some` scope field must match the event **exactly** (logical AND); `None` means unrestricted.
Text matching lowercases both sides — an empty search string is ignored. `Default` is
importance 1, all categories on, no text, no scope.

`History::chronicle_indices(&f) -> Vec<usize>` filters in push order and stable-sorts by
`time` with the vector index as tie-break, so the result is chronological and fully
deterministic (same-time events keep push order). `History::chronicle(&f) -> Vec<&Event>` maps
those indices. `Category::index()` is the stable filter slot (Astronomy=0 … Contact=7);
`Category::from_label` is the case-insensitive lookup the CLI uses.

**Why the UI scopes with a union.** A single `ChronicleFilter` with `civ: Some(c)` plus
`system`/`body` would hide every event that is *not* attributed to that civilization — including
the world's own climate and impact records. Instead the app builds an ordered filter set
(`chronicle_scope_filters`): the primary filter (importance/categories/search, plus
`system`/`body` when scoped) and, for a body that hosts a civilization, one extra filter per
civ (`civ: Some(id)`, no body scope). The union is merged by `chronicle_union` and ordered by
`(time, push index)` — identical to `chronicle_indices` ordering, deduplicated by vector index.
Export deliberately writes the **single primary filter** (the T1 filter the spec names); the
cross-location civ rows are a view-only extra and are documented as such.

## Legends eras

`era_label(years_since_start)` buckets by decade of magnitude: `years < 1` → "The first year",
otherwise `decade = 10^floor(log10(years))` → "Years {decade}–{decade·10}" (grouped digits).
Bounds are powers of ten, so a 14-By run yields at most ~11 era headers plus the first year.
The view walks the union in chronological order and emits a header whenever the label changes
(labels are non-decreasing in time, so a single pass suffices). Legends always scopes to the
selection and renders the earliest 400 rows; Timeline renders the newest 400. Both are capped
by the historical render cap, and the `N events (showing M)` label makes the cap explicit.

## Export schema (`crates/cosmogon_sim/src/chronicle.rs`)

`export(&Universe, &ChronicleFilter, dir)` is the only writer and takes `&Universe`. Order of
operations:

1. Match the events (`rows`) — empty result refuses with `No events match the Chronicle filter`
   *before* anything is created.
2. `create_dir_all(parent)`, then `create_dir(dir)` — the destination must not exist. On any
   write failure the partial directory is removed, so a failed export leaves nothing behind.
3. Write `manifest.json`, `chronicle.json`, `chronicle.csv` with `create_new(true)` + `sync_all`.

`manifest.json` has no wall-clock field, so exporting the same universe twice is byte-identical.
`filters.categories` lists the enabled category **labels** in canonical order. `event_count` is
the row count; first/last times are `null` for an empty set (which cannot be exported).
`limitations` is fixed: the 4,000-minor-event cap/pruning statement and "events are
simulation-authored narrative records, not measurements".

CSV quoting is RFC 4180: a cell is quoted when it contains a comma, quote, CR or LF, embedded
quotes are doubled, records end CRLF, and `None` ids are empty cells. `date` is rendered with
`time::format_date` (so it may itself contain commas and get quoted). The CSV is written with a
UTF-8 BOM-free byte order.

## O(1) minor-event counter (`History`)

`push` used to recount every event on every call to enforce `MINOR_CAP = 4000`. `History` now
carries two cached fields:

```rust
#[serde(skip)] minor: Option<usize>,       // count of importance-<3 events
#[serde(skip)] first_minor: Option<usize>, // cursor of the oldest minor event
```

`push` lazily initializes the cache (`minor.get_or_insert_with(recount)`, which also derives
`first_minor` by one forward scan), increments on minor pushes, and when the count exceeds the
cap removes the event at `first_minor` and advances the cursor forward. Each index is scanned at
most once, so pruning is amortized O(1). A stale cursor is impossible: both fields are absent
from serialization (`#[serde(skip)]`), so a loaded `History` starts at `None` and recounts on
the first push.

`History` therefore drops `#[derive(PartialEq)]` for a manual impl comparing **only** `events`.
The cache is an optimization, not semantic state — this matters because `save.rs` compares a
live universe against a freshly deserialized one with `assert_eq!`.

Byte-identity chain (see `docs/CHRONICLE_VALIDATION.md` for results):

1. The golden input is `serde_json::to_string(universe)`; `serde(skip)` omits both fields in
   both directions, so the string is unchanged.
2. `history_minor_counter_prunes_exactly_like_the_legacy_recount` compares the events vector
   after every push against an inline copy of the legacy recount algorithm.
3. `history_serialisation_is_byte_identical_with_the_counter_cached` serializes before/after a
   push and after a deserialize round-trip.

## UI wiring

`UiState` gains `chronicle_categories: [bool; 8]`, `chronicle_search`, `chronicle_scope`,
`chronicle_mode`, `chronicle_export_path` and `chronicle_export_status` (all app-only; never
saved into the universe). `--panel chronicle` opens the Chronicle tab in Timeline mode and
`--panel chronicle-legends` in Legends mode; unknown panel values keep the previous behaviour.

## Cost

`chronicle()` is O(events) with substring scans; the universe holds at most 4,000 minor plus
milestones, and the timeline view already walks every event per frame, so the filter costs the
same order as existing rendering. No cache is added until a measured need appears (spec §6).
