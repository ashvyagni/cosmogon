# Build 65 — Interstellar consequence and contact modeling (implementation spec)

Status: ready. **Golden-neutral build**: `GOLDEN = 0xf2c4140a45ae509b` unchanged. Build 64
(`5047773`) is committed; this spec assumes `universe.rs`, `intervene.rs`, `civ/space.rs`,
`civ/tech.rs`, `history.rs`, `ui/inspect.rs`, `crates/cosmogon_cli/src/main.rs` are as shipped.

## 1. Objective

Build 64 added the cause graph foundation (`Event.cause`, `why_trace`, `cause_children`).
Build 65 makes interstellar events consequential: probe arrivals trigger discovery/first-contact
events, contact detection between civilizations produces downstream effects (technology exchange,
cultural pressure, stability shifts), and every interstellar event links back through the cause
graph to its originating probe launch or radio signal.

### In scope
- Probe arrival consequences: when a probe reaches another star system, push discovery events
  (first contact if a civilization exists, biosphere survey, resource scan) with `cause` linking
  to the probe launch event
- Contact detection consequences: when civilization A's radio reaches civilization B, downstream
  events (tech sharing, cultural pressure, stability shift, astronomy research boost) with
  `cause` linking to the detection event
- Cause graph links: probe arrival → probe launch, contact → radio signal, discovery → probe arrival
- CLI `cosmogon-cli probe <index>` / `cosmogon-cli contact <index>` — print causal chain for a
  probe arrival or contact event
- CLI `cosmogon-cli interstellar` — list all interstellar events (probe launches/arrivals,
  contact detections) with their causal chains
- Tests, docs, tracker update

### Out of scope
- Two-way communication protocols (deferred — requires sustained contact state)
- Technology transfer mechanics (deferred — requires tech graph changes)
- Multi-parent cause graph (single `cause` parent retained from build 64)
- Physical interstellar travel simulation (probes use light-speed straight-line travel, already
  implemented in `step_probes`)

## 2. Context (verified)

- `step_probes` (`universe.rs:789-817`) — launches probes to other star systems at
  `civilization.probe_speed_c` fraction of light speed, tracks arrival, pushes
  "Probe launched towards X" and "Probe arrives at X" events with `cause: None`
- `step_contact` (`universe.rs:757-787`) — radio signals propagate at light speed; when
  civilization A's signals reach civilization B (which has `radio_astronomy`), pushes a
  Contact event with `cause: None` and sets `pressures.contact = 1.0`
- `Probe { civ, from, to, launched, speed, arrived }` (`universe.rs`) — probe state
- `Civilization.detected: Vec<u32>` — set of detected civilization IDs
- `Civilization.flags` — includes `radio_astronomy`, `probes`, `spaceflight`, etc.
- `Event.cause: Option<usize>` — build 64 addition, `#[serde(skip_serializing_if=None)]`
- `History::why_trace` / `cause_children` — build 64 addition
- Golden test: Sol seed 2026, 5 kyr, no interstellar civilizations → no interstellar events
  → golden unchanged

## 3. Tasks

### T1 — Probe arrival consequences

`crates/cosmogon_sim/src/universe.rs` — `step_probes`:

When a probe arrives (`p.arrived = true`), after the existing "Probe arrives at X" event:
- If the target body has a civilization: push a first-contact Contact event
  (`cause = Some(probe_arrival_event_index)`)
- If the target body has life but no civ: push a Life discovery event
- If the target body is barren: push an Astronomy survey event
- All arrival consequences link `cause` to the probe arrival event index

### T2 — Contact detection consequences

`crates/cosmogon_sim/src/universe.rs` — `step_contact`:

When civilization A's radio reaches civilization B, after the existing Contact detection event:
- Push a downstream event: technology awareness boost (B's astronomy knowledge increases)
- Push a cultural pressure event (B's stability shifts, contact pressure rises)
- All downstream events link `cause` to the detection event index

### T3 — Cause graph links for interstellar events

- Probe launch event: `cause = None` (root/user action)
- Probe arrival event: `cause = Some(launch_event_index)`
- First contact event: `cause = Some(arrival_event_index)`
- Contact detection event: `cause = None` (root — radio propagation is natural)
- Contact consequence events: `cause = Some(detection_event_index)`

Thread event indices through `step_probes` and `step_contact` to set causes.

### T4 — CLI commands

`crates/cosmogon_cli/src/main.rs`:

```sh
cosmogon-cli probe <event_index> [--initial SAVE | --seed/--scenario/--years]
  # Print causal chain for a probe event (launch → arrival → discovery)
cosmogon-cli contact <event_index> [--initial SAVE | --seed/--scenario/--years]
  # Print causal chain for a contact event (detection → consequences)
cosmogon-cli interstellar [--initial SAVE | --seed/--scenario/--years]
  # List all interstellar events with chains
```

- Reuses `initial_universe(a)`; advances unless `--initial`
- `probe`/`contact`: validate event index, print `why_trace` as indented chain
- `interstellar`: scan history for Space/Contact events, print each with its chain
- Refusals: unknown event index, `--initial`+`--seed/--years`

Test (`main.rs`): `interstellar_arguments_refuse_unknown_index_and_ignored_flags`

### T5 — Docs, validation, tracker

- `docs/INTERSTELLAR.md` (guide), `docs/INTERSTELLAR_DESIGN.md` (design),
  `docs/INTERSTELLAR_VALIDATION.md` (evidence)
- `CHANGELOG.md`, `docs/BUILD_TRACKER.md` row 65, `docs/MASTER_GAP_ANALYSIS.md` entry 63
- `HANDOFF.md`, `docs/CONTINUATION_HANDOFF.md` verification record
- Captures: `output/qa/b65-interstellar.png` (CLI interstellar output)

## 4. Golden-neutrality argument

| Change | Saved field? | RNG draw? | Mutates Universe? | Golden risk |
|---|---|---|---|---|
| Probe arrival consequences | new Event rows only | none | yes (new events) | none (golden test has no interstellar civs) |
| Contact consequences | new Event rows only | none | yes (new events) | none (golden test has no radio civs) |
| CLI `probe`/`contact`/`interstellar` | none | none | no | none |

The golden test advances Sol seed 2026 for 5 kyr with a single civilization (Humanity on Earth).
No other civilizations exist, no probes are launched to other stars, no radio signals cross
systems. No new events are pushed → golden unchanged.

## 5. Validation plan

1. `cargo test --release --workspace` — expect build-64 baseline 714 + ≥6 new = ≥720 passed
2. `cargo clippy --release --workspace --all-targets` — 43-warning / 0-error baseline
3. `cargo test --release -p cosmogon_sim golden_universe_fingerprint` — unchanged
4. CLI: `interstellar` on a seeded run with civilizations; `probe`/`contact` chains
5. Save round-trip: interstellar events survive save/load (cause=None or Some preserved)
6. Capture inspected

## 6. Risks

- **Event index invalidation**: pruning removes minor events; cause indices to pruned events
  become dangling. `why_trace` stops at missing indices. Document this.
- **New events in non-golden scenarios**: interstellar events only fire when multiple
  civilizations exist across star systems. The golden test has one civilization → no impact.
- **Probe arrival without target body**: probe `to` is a system ID, not a body; need to check
  if any body in the target system has a civilization.
