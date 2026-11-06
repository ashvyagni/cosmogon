# Save format

Experiments are first-class documents ("sandboxes"). Everything is versioned; nothing
depends on network access.

## On-disk layout

```
<data dir>/Cosmogon/
  profiles/
    local/                         the offline profile (future accounts map onto profiles)
      profile.json                 { id, display_name, created }
      sandboxes/
        <sandbox-id>/
          manifest.json            small: listed without reading any state
          state.cosmo              last explicit save
          autosave.cosmo           written every N minutes and on quit
          thumbnail.png            captured on save (UI hidden)
          checkpoints/
            <unix-time>.cosmo      user checkpoints (label in the manifest)
  saves/                           legacy single-file saves (v0.2–0.3), still listed and loadable
```

`<data dir>` is `~/Library/Application Support` on macOS, `%APPDATA%` on Windows,
`~/.local/share` on Linux. Sandbox ids are `sb-<unix-ms>-<random>` and never reused.

## manifest.json

```json
{
  "format": "cosmogon-sandbox", "version": 1,
  "id": "sb-…", "name": "2× Jupiter experiment", "description": "…",
  "created_unix": 0, "modified_unix": 0,
  "template": "solar_system_lab",
  "origin": { "cloned_from": "dataset:jpl-horizons-sol@1", "parent_sandbox": null, "parent_checkpoint": null },
  "sim_date": "1 Mar 2031", "sim_time": 9.5e8,
  "seed": 2026,
  "app_version": "0.4.0", "physics_engine": "nbody-yoshida4/1",
  "data_versions": { "jpl-horizons-sol": "1", "etopo5-earth": "1" },
  "physics": { "model": "NBody", "preset": "Balanced", "relativity": false },
  "systems_enabled": { "climate": true, "life": true, "civilization": true },
  "checkpoints": [ { "file": "checkpoints/….cosmo", "label": "Before impact", "sim_date": "…", "created_unix": 0 } ],
  "bookmarks": [],
  "owner": "local"
}
```

`origin` makes **branches** possible: a branch is a new sandbox whose `parent_sandbox` and
`parent_checkpoint` point at where it diverged (UI planned; the format already allows it).

## State files (`*.cosmo`)

JSON: `{ "header": {…}, "universe": {…} }`, format `cosmogon-save`.

* **v1** (0.2–0.3): header + universe.
* **v2** (0.4): adds `universe.edits` (the journal of user modifications), per-system
  `dynamics` (N-body state), per-body `provenance`, `class`, `removed`, `impacts`, and
  `settings.systems`. All new fields have defaults, so the v1 → v2 migration is a no-op on
  the JSON; v1 files load unchanged and keep Kepler orbits.

* **v2, 0.5 additions** (no version bump; all default on load): civilizations gain `missions`,
  `explored`, `missions_launched`, `last_intervention`, `static_polities`; settlements gain
  `share`; governments gain `State`; edits gain `Intervene`. 0.4 saves load unchanged.

Rules (unchanged): bump `CURRENT_VERSION` and add a migration step for every change;
writes are atomic (temp file + rename); a file newer than the app is refused with a clear
message.

## Reproducibility

Every state file records the app version, physics engine version, dataset versions, seed,
the initial conditions (inside the state) and the journal of user modifications. Saves pin
their data: refreshing a dataset never changes an existing experiment. Because the
simulation is deterministic across platforms, another installation reproduces the same
experiment from the same file.

## Undo / redo

In memory only: a bounded stack (40 entries) of universe snapshots taken immediately before
each edit. *Undo* returns the sandbox to the moment before the edit, including simulation
time. Snapshots are not saved.

## Sharing (planned)

A single-file export bundling manifest + state + thumbnail. The extension will be chosen
after registering document types on each platform (`.cosmogon` is the candidate; macOS
`UTExportedTypeDeclarations`, Windows ProgID via the installer).

## Performance settings and reports (post-1.0)

`detailed_profiling` is an app user-setting boolean, defaults to false for older settings, and is
applied at the next launch. Reports live outside sandboxes in `Cosmogon/reports` (or at a developer
`--profile` path). They have their own `cosmogon-performance` format/schema version 1. No simulation
save fields, save version, physics fingerprint or dataset initial conditions change in this slice.

## Camera, presentation and viewpoints (post-1.0)

User settings add nested `camera` and `presentation` tables, plus `photo_metadata` (default true).
Older settings receive defaults; nonfinite/out-of-range camera values are sanitized on load. These
preferences are independent of the universe save format.

`Cosmogon/viewpoints.json` has its own version 1 library (maximum 128 views). Each entry records a
universe key, optional stable anchor ID/name, relative render-frame metres, quaternion orientation,
presentation values and simulation time for context. Applying a view does not rewind that time.
Reference keys use dataset identity; sandboxes use manifest ID; developer runs use scenario/seed.
Missing anchors fail visibly. Invalid/newer libraries are preserved and disabled for writes. Library
writes replace a temporary file only after serialization succeeds. This is a per-user library, not yet
a packaged/shareable camera path or a per-sandbox manifest migration.

Photo sidecars use `cosmogon-photo` schema version 1 and `.png.json`, distinct from performance reports.
Metadata labels the capture-request simulation time; asynchronous GPU readback is not claimed to be
an exact synchronized scientific observation.

## Sound and notification preferences

Nested `audio` and `notifications` settings preserve existing defaults on older files. The legacy
`sound_volume` remains master gain. Category routes normalize to the eight supported Chronicle
categories; priority thresholds clamp to 1–5. Nonfinite/out-of-range gains are sanitized. Inbox
entries/read marks are derived app state and reset on load/rewind; Chronicle remains in Universe.
Preference writes use temporary-file replacement and expose failures in the new panels. No simulation
save fields or golden fingerprint change.

## Chronicle export bundles (`cosmogon-chronicle/1`)

The Chronicle **stays inside `Universe.history`**; build 62 adds no save field. Exported bundles
(`cosmogon-cli chronicle`, or the Chronicle tab's Export button) are read-only artefacts that
carry `manifest.json` (`format: "cosmogon-chronicle"`, `schema_version: 1`), `chronicle.json` and
an RFC-4180 `chronicle.csv`. They are never loaded back as a world and existing folders are never
overwritten. The in-memory minor-event counter used to keep `push` O(1) is `#[serde(skip)]`, so
save bytes — and the golden fingerprint — are unchanged in both directions.

## Civilization infrastructure (derived, never saved)

Build 63 adds a derived infrastructure view (`civ::infrastructure`): settlement roles
(farm, mine, port, industry, spaceport) and a per-civilization footprint (areas, link
lengths, electrification, fossil horizon). Everything is computed from existing saved
state (settlements, population, energy, endowment, links, technology flags); nothing is
serialized, no new save field is added, and the golden fingerprint is unchanged.

## Research bundles

Research recordings use a separate cosmogon-experiment schema 1 manifest and six-file directory;
see [RESEARCH](RESEARCH.md). The initial.cosmo file is a normal complete save. New edits and
observed events are separate JSON; CSV includes absolute simulation seconds, edit_count and unit
keys. Existing destination directories are refused. This adds no Universe fields or save migration.

Experiment definitions and batch roots have separate schema 1 formats. Definitions carry typed
canonical-unit overrides, independent input/measurement targets, an optional primary measurement
(default absent for older definitions), ordered sweeps and an optional frozen-state fingerprint; batch.json records
prepared/running/complete/incomplete status and completed-run outcomes. base.cosmo is a normal save.
See RESEARCH for replay compatibility and partial/cancelled export semantics.

Measured orbital reports use cosmogon-model-validation schema 1 outside Universe/saves. They contain fixed fixture inputs and reference coordinates, computed metrics and regression guards, with coverage limitations.

Research workspace choices use a separate cosmogon-research-workspace schema 1 file. Guidance, favorites, up to four graph quantities/scales, comparison scale/difference choice, time unit and normalized zoom range are persisted explicitly. Invalid/newer files remain untouched. Observer bundle reopening reads existing manifest/CSV/events without loading Universe; see RESEARCH for limits and source-label semantics.
