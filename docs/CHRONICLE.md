# The Chronicle — recorded history, filtering, Legends and export

The Chronicle is the simulation's own record of what happened: stellar events, life and
civilization milestones, disasters, wars, discoveries, spaceflight and contact. Every entry is
written by the simulation with a time, category, importance (1 minor … 5 milestone), title,
detail and optional system/body/civ identifiers. It is saved inside the universe
(`Universe.history`), so loading a world restores exactly the events that world recorded.

Nothing in this build invents events. It makes the existing record usable.

## Filtering and search (Universe browser → Chronicle)

- **View** — `Timeline` lists the newest events first; `Legends` is the era-grouped story (below).
- **Show** — importance floor: milestones (5), major (4) or all (≥2).
- **Categories** — eight toggles (Astronomy, Life, Civilization, Technology, Disaster, War,
  Space, Contact). Clicking one turns that category off or on.
- **Search** — case-insensitive substring over title *and* detail.
- **Scope to selection** — restrict the list to the selected world (or its system when a star is
  selected). When a civilization sits on the selected world, its own records are included too —
  including events it recorded elsewhere. The checkbox is disabled until something is selected.
- **Count label** — `N events (showing M)`. `N` is everything the current filter matches; `M`
  is what is rendered (the list renders at most 400 rows). The cap is no longer silent.

Clicking an event title focuses its world, as before; hovering shows the full detail.

## Legends

Legends is one chronological narrative grouped into **eras** by time since the world started:
"The first year", then decade-of-magnitude buckets ("Years 1–10", "Years 10–100",
"Years 100–1,000", …). With nothing selected it is the universe-wide story; select a world and
it becomes that world's story (plus its civilization's records). Legends always scopes to the
selection — the scope checkbox applies to Timeline.

Legends renders the earliest 400 matching events (a chronicle reads from the beginning); the
count label still reports the full filtered total.

## Export (Chronicle tab → Export, or `cosmogon-cli chronicle`)

Both paths write the same protected bundle:

| File | Contents |
|---|---|
| `manifest.json` | `format: "cosmogon-chronicle"`, `schema_version: 1`, app version, the exact filter, `event_count`, first/last event times, `files`, and the limitations below. No wall-clock fields, so re-exporting the same history is byte-identical. |
| `chronicle.json` | Flat array of events with the exact stored field names (`time, category, importance, title, detail, system, body, civ`). |
| `chronicle.csv` | Header `time_s,date,category,importance,title,detail,system,body,civ`; free text RFC-4180 quoted; missing ids are empty cells; CRLF records. |

**Existing folders are never overwritten** — the destination is created exclusively. An empty
match refuses with `No events match the Chronicle filter`.

The native button exports the current filter (importance, categories, search and the scoped
system/body). Cross-location civilization rows that Legends adds to the *view* are not part of
that single-filter bundle.

### CLI

```sh
./target/release/cosmogon-cli chronicle --output /tmp/chronicle                 # default world, all distances
./target/release/cosmogon-cli chronicle --seed 2026 --scenario sol --years 5000 --output /tmp/chronicle-sol
./target/release/cosmogon-cli chronicle --initial saved.cosmo --output /tmp/chronicle-saved
./target/release/cosmogon-cli chronicle --seed 2026 --scenario sol --years 5000 \
  --min-importance 4 --category war --category disaster --search "nuclear" \
  --system 0 --body 3 --output /tmp/chronicle-war
```

`--initial SAVE` exports a saved world exactly as-is (it is not advanced). Without `--initial`
a fresh world is advanced by `--years` (default 200,000) with `--seed`/`--scenario`. The manifest
is printed on success. Scope ids are validated before anything is written: an unknown system,
body or civilization id refuses.

## Honesty about completeness

The Chronicle is not a complete history of the universe — it is a complete history of what the
simulation *chose to record*. Minor events (importance < 3) are capped at 4,000 and the oldest
are pruned during a run; pruned events cannot be exported. Milestones are never pruned. Both
the manifest's `limitations` array and this page say so rather than implying a full record.
Events are simulation-authored narrative records, not measurements.

See [CHRONICLE_DESIGN](CHRONICLE_DESIGN.md) for the filter API, export schema and counter
design, and [CHRONICLE_VALIDATION](CHRONICLE_VALIDATION.md) for what was actually verified.
