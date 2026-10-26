# Research controls and experiment recordings

View → Control Panel & Research opens five native tabs. The command palette also exposes
Controls, Models, Sources and Plots. The window follows the selected object; a running recording
keeps its starting target even when selection changes.

## Controls and scientific context

The shared simulation research module describes 36 parameters and seven implemented models.
Search matches names, canonical unit keys, sections and model IDs. Physical, rotation, environment,
star, solver and state-vector groups expose supported values for the selected object. Derived values
are locked; editable inputs use the existing sandbox Edit journal and undo snapshots. Reference
universes stay read-only. Reset restores the snapshot taken when this target/generation was selected;
it is not a catalogue reset. State components preserve the other five components and activate dynamic
gravity. Solver steps are available only for a dynamic system.

Info explains dependencies, affected outputs, equations and input limits. Models documents the
implemented method, assumptions, validity, limitations, cost, regression references and primary
sources. The Models tab can also calculate the analytic/cached-ephemeris checks described below; registry references alone are not live test results. Numerical input guards prevent obviously
unsafe values but do not establish a validated physical regime for every possible experiment.

Data distinguishes measured, derived, estimated, modelled, procedural and user-modified quantities.
ML inference and unknown are reserved states; the current models produce no ML inference. Missing
uncertainty remains null. Legacy stars have no stored per-field provenance: known Solar/Trapezium
loader identities are identified as estimates with age/composition assumptions, not measurements;
other origins are procedural. An edit marks only the exact changed star input. Model outputs remain
modelled. No bodies are invented around observed stars and no dataset is fetched at runtime.

The registry links [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/manual.html),
[Yoshida's symplectic integrator](https://doi.org/10.1016/0375-9601(90)90092-3),
[Will's relativity review](https://arxiv.org/abs/1403.7377),
[Leinhardt & Stewart's collision framework](https://arxiv.org/abs/1106.6084) and
[the Trapezium stellar study](https://arxiv.org/abs/astro-ph/0510288). Climate and stellar fits cite
local implementations and their calibration limits. Source descriptions do not grant distribution
rights; see the separate SOURCE_REGISTRY inventory.

## Native recordings

Plots & Export starts/stops a bounded observer recording (maximum 4096 states). Cadence is requested
in real seconds; actual simulation times are recorded, without interpolation. Charts share elapsed
simulation seconds and use canonical SI units. Unchanged paused states are deduplicated; edits at
paused time produce another row distinguished by edit_count. Rewind, universe replacement, removal
of the target or loss of a recorded quantity stops the recording. The most recent 1000 observed
Chronicle events are retained. Charts expose zoom buttons, horizontal drag after zooming, nearest-state inspection, event markers,
positive-value logarithmic scale and fractional change from the first reading. A fractional change
is not an accuracy estimate. Uncertainty propagation remains unimplemented.

Export creates a fresh local directory under Cosmogon/experiments. It never replaces an existing
path. A bundle contains:

| File | Meaning |
|---|---|
| manifest.json | Format cosmogon-experiment, schema 1; target, units, source/model states, versions, seed, actual interval/context and limitations |
| samples.csv | simulation_time_s, edit_count and canonical unit keys; finite observed values |
| initial.cosmo | Full initial save accepted by the normal save loader, including existing edits |
| edits.json | Edits observed after that initial snapshot |
| events.json | Retained observed Chronicle events |
| models.json | Model and scientific source registry |

The initial serialized save is limited to 32 MiB; allocation temporarily includes serialization/export
buffers as well as the live universe. A stopped recording remains exportable. A bundle is evidence
and a reusable initial state, not a general accuracy guarantee. Definition-based experiments below have a separate replay runner;
the live observer journal is not automatically replayed.

## Headless and capture workflows

```sh
./target/release/cosmogon-cli models > /tmp/models.json
./target/release/cosmogon-cli experiment --scenario lab --seed 1 --target Earth \
  --samples 32 --interval-days 1 --output /tmp/earth-experiment-1
./target/release/cosmogon --new lab --focus Earth --speed 0 --quiet --panel plots \
  --research-export /tmp/native-earth-1 --research-samples 60 --research-every 20 \
  --capture /tmp/research.png --capture-metadata --capture-after 1300 --quit
```

The CLI samples at uniform requested simulation times (one day is 86400 seconds). Samples clamp
between 2 and 4096; interval-days must be finite and between 0.000001 and 3652.5. Target names refer
to existing stars/bodies, case-insensitively. Native automation instead requests a sample every
research-every rendered app frames (1–600); actual simulation times depend on clock/frame timing.
Native research-samples clamps to 2–4096. Automated research defaults to the Solar System Lab when
no scene/load is specified; CLI experiment/sweep creation also defaults to Lab. Capture, performance and research exports coordinate --quit so all
requested outputs complete. Export/load failure returns a nonzero process status.

The research registries and recordings are outside Universe serialization. The save schema and
golden fingerprint remain unchanged. These tools support exploratory studies with the implemented
approximations; broader independent model validation and control coverage remain open work.

## Orbital measurements and diagnostics

Orbit exposes eccentricity, semi-major axis, inclination, osculating period, specific mechanical
energy and specific angular momentum. These are instantaneous two-body quantities derived from the
body's state relative to its actual parent, with μ = G(M_parent + M_body), even for moons. They do
not predict a full future orbit in a perturbed system. Inclination refers to the simulation XY plane
(ecliptic J2000 in Solar Lab). Bound-only period and parabolic singularities are unavailable rather
than filled with fake numbers. A recording stops when one of its captured quantities disappears.

Diagnostics adds total Newtonian energy and angular-momentum magnitude for dynamic particles.
These exclude moons on rails, thermal/internal energy and the 1PN energy correction. The angular
momentum is about the simulation system origin. Observer cost is bounded to systems with at most
256 bodies/particles. Edits, contact outcomes, stellar mass changes and incomplete rails coupling
can change these budgets; drift is only a useful numerical diagnostic for the appropriate closed,
fixed-mass regime. No universal conservation tolerance is implied.

The measurement design was informed by [REBOUND diagnostics](https://rebound.hanno-rein.de/simulation/diagnostics/)
and [orbital-element conventions](https://rebound.hanno-rein.de/particles/orbitalelements/),
[JPL's frame and osculating-element cautions](https://ssd.jpl.nasa.gov/horizons/manual.html),
and [Astropy's units](https://docs.astropy.org/en/stable/units/) and
[uncertainty/distribution treatment](https://docs.astropy.org/en/stable/uncertainty/).
Cosmogon uses its own existing deterministic engine; these references do not make its models
scientifically equivalent to those tools.

## Frozen experiments, reusable definitions and sweeps

Experiments guides the user through freezing a starting state, changing one input and comparing
computed outcomes. Three introductory questions explore radius/gravity, albedo/global-mean climate,
and solver convergence. Advanced controls expose the same authoritative model and sampling plan. The changed object and measured object can differ: changing Jupiter’s mass can record Earth’s eccentricity. A primary measurement is saved in the definition, while each run still exports all supported readings.
The copied starting state stays fixed while the live universe continues. Reference data can be
copied for an experiment without changing the read-only reference universe.

A background worker validates the definition, applies fixed overrides and then one ordered sweep
value per independent branch. Each branch begins from the same frozen state. Maximum 64 runs,
64 fixed overrides, 4096 states per run; definition input is capped at 1 MiB. Simulation integration
checks cancellation between bounded work slices; validation/snapshot/export operations may take
longer than a slice. The worker has a bounded progress queue. It never mutates the live Sim. A run
exports only after all requested sample times were reached. Completed runs survive cancellation;
batch.json explicitly says incomplete and records the reason. The Results view opens when a worker finishes, including failures with completed branches; partial results do not count as successful completion. Thread/application termination is not a resumable batch service yet.

The batch root contains base.cosmo, definition.json, batch.json and run-0001 etc. Each completed
run has the six-file observer bundle. batch.json provides ordered input values and final outputs
in canonical units, including orbital readings and diagnostics when available. Loaded definitions
preserve explicit non-uniform sweep grids until the user edits the grid controls. Definitions can
also be saved with their frozen base before integration, with status prepared.

Generated/prepared definitions pin an initial-state FNV-1a fingerprint. This is a deterministic
identity check, not a cryptographic authenticity claim. Modified bases are refused. Hand-authored
unbound definitions may omit the fingerprint; they still validate IDs, parameters and bounds.
Reproducing results requires compatible simulation algorithms as well as identical state; the app
version/physics-engine/dataset context is recorded but current version strings alone do not prove
compatibility across unreleased code changes.

```sh
./target/release/cosmogon-cli sweep --scenario lab --seed 1 --target Earth \
  --parameter albedo --from 0.1 --to 0.6 --runs 6 --samples 8 --interval-days 1 \
  --output /tmp/albedo-study-1
./target/release/cosmogon-cli replay --initial /tmp/albedo-study-1/base.cosmo \
  --definition /tmp/albedo-study-1/definition.json --output /tmp/albedo-replay-1
./target/release/cosmogon --load /tmp/albedo-study-1/base.cosmo --focus Earth --quiet \
  --panel experiments --research-plan /tmp/albedo-study-1/definition.json \
  --research-batch /tmp/native-albedo-1 --capture /tmp/batch.png --capture-after 360 --quit
```

Sweep inputs use canonical descriptor units: mass kg, radius m, stellar age seconds, pressure bar,
CO2/albedo fractions, angles rad. This preserves the existing atmosphere convention explicitly;
not every canonical quantity is SI. `models` lists parameter IDs/keys/units. CLI experiment also
accepts --initial to observe a saved state. CLI replay executes the definition against its base;
it does not replay arbitrary live-user edits at their timestamps. Native automated batching freezes
before live simulation advancement. Its --quit coordinates with screenshots, performance and live
research exports, and returns failure status for invalid plans/output/load errors.

Comparisons and CSV expose real calculated data. Broader observed-data coverage, uncertainty
propagation, stable event identities, research catalogue ingestion, independently benchmarked
model regimes, screen-reader/manual keyboard checks and full graph-layout persistence remain open.


## Measured orbit validation

Models → Check orbit models against reference cases calculates a fresh report on an app worker;
`cosmogon-cli validate --output /tmp/orbit-check-1.json` runs the same fixtures headlessly. The report
contains two analytic circular test-particle orbits (1000/2000 steps per period) and the Accurate
Solar Lab at 365 days against nine locally cached 2027 JPL endpoints. Every metric has its actual
value, unit and historical regression guard. The report exports initial vectors, rounded endpoint
coordinates, epoch, frame, source and solver conditions. These are fixed fixtures, independent of
the currently selected/edited universe. Existing report files are protected.

The JPL endpoint residual is hundreds of kilometres for Earth/Moon despite tiny numerical error in
the simple analytic case. Numerical precision and physical-model completeness are different. Passing
historical regression guards does not certify a sandbox or validate collision, stellar or
civilization models. Climate now has its own analytic fixtures and a regression guard
(`climate-analytic-greenhouse`) in the same report, but those pin the 0-D model's
self-consistency rather than observational accuracy (see [CLIMATE_VALIDATION](CLIMATE_VALIDATION.md)). No covariance, interval-wide trajectory bound or independent integrator comparison
is supplied. See [SCIENTIFIC_VALIDATION](SCIENTIFIC_VALIDATION.md) for measured evidence.

```sh
./target/release/cosmogon-cli --help
./target/release/cosmogon-cli validate --output /tmp/orbit-check-1.json
./target/release/cosmogon-cli sweep --target Jupiter --parameter mass_kg \
  --from 9.49e26 --to 3.796e27 --runs 3 --measure-target Earth --measure eccentricity \
  --samples 4 --interval-days 10 --output /tmp/jupiter-earth-study-1
```

`--target` selects the changed object for sweeps; `--measure-target` selects the observed object.
`--measure` accepts a canonical descriptor key or parameter ID and persists the comparison choice.
The typed definition/native sweep interval permits up to one million years between states; long
N-body runs can require substantial computation. This input bound is not an accuracy claim. The
single CLI `experiment` observer retains its 3652.5-day maximum. Malformed command-line requests
return a concise error and failure status rather than panicking.


## Reopening data, trajectory comparison and saved workspace choices

Plots & Export → Open previously recorded data opens a six-file observer bundle's numeric table.
It reads the manifest, CSV and events on a worker, without replacing or evolving the live universe.
`cosmogon-cli inspect --recording DIR` exposes the same verified numeric rows, parameter keys/units
and initial source context as JSON. This accepts Cosmogon's numeric CSV dialect, not arbitrary CSV.
Format/schema, registered units/keys, row counts, finite values and chronological edit ordering are
checked; limits are 1 MiB manifest, 8 MiB CSV, 2 MiB events, 4096 states and 1000 events. Corrupt data
is refused. This checks format consistency, not independent provenance, file authenticity or model
accuracy. Source labels and optional uncertainty refer to the recording start; they are not
per-state uncertainty propagation. The complete original bundle remains on disk.

Completed studies load up to eight saved trajectories on a background worker. The selected output
is plotted for those branches in the same canonical unit, while all up to 64 runs remain in the
numeric result table and exported bundles. Lines connect actual samples for display; nearest-state
inspection does not interpolate results. Charts share elapsed-time zoom/pan and cursor across the
selected graphs. Each recording/branch uses elapsed time since its own start; this does not align
absolute epochs from unrelated studies. Difference from first completed run requires identical elapsed sample times and reveals small branch effects without interpolation. The first branch is not necessarily an unmodified control, and the difference is not a physical error estimate. CLI `inspect --recording DIR --reference BASELINE_DIR --measure KEY` exposes the same calculated differences.
Log requires positive values; fractional change uses each
series' own first value and is not an error estimate. Unsupported transforms fall back visibly.
Click a chart and use left/right arrows for recorded-state inspection; direct keyboard/pointer and
assistive-technology behavior still require manual verification.

Guidance offers Explore, Learn and Research within the same workspace. Explore initially hides
solver/diagnostic/state-vector groups unless searched; Learn explains current inputs behind Info's
formula and the study path; Research keeps the full controls. Guidance changes presentation, not
physical accuracy. Parameter stars and Favorites only are user choices. Choose graphs and time
units selects up to four quantities, with seconds, 86400-second days or 365.25-day Julian years.
Unsupported graph choices stay saved for other objects rather than silently discarding them.

Save workspace persists guidance, favorites/filter, selected graphs/scales, comparison scale/difference choice,
time unit and normalized time-range fractions in Cosmogon/research-workspace.json, separate from
Universe. It has its own format/schema 1, 64 KiB read limit and bounded/validated choices. Older
partial choices receive defaults. Invalid/newer files are preserved and disabled for writes;
failed saves are visible. Changes are marked unsaved until the explicit Save action. The cursor
is transient; full window positions/docking and per-sandbox layouts remain future work.

Developer `--research-recording DIR --panel plots` opens data at startup; `--research-workspace FILE`
loads a separate choices file for capture QA. Opening saved data and automatic live export are
mutually exclusive. Capture waits for requested imported data or completed automated studies, and
coordinated quit includes the new data-loading task. No new dependency, Universe field, physical
algorithm or golden-fingerprint change is introduced.
