# Build 64 — Intervention consequences end-to-end with evidence (implementation spec)

Status: ready. **Golden-neutral build**: no save-schema change; `GOLDEN = 0xf2c4140a45ae509b`
(`crates/cosmogon_sim/src/save.rs:645`) must remain unchanged. Build 63 (`2c4533f`) is committed;
this spec assumes `intervene.rs`, `sandbox.rs`, `history.rs`, `ui/inspect.rs`, `ui/mod.rs`,
`ui/markers.rs`, `crates/cosmogon_cli/src/main.rs` are as shipped by build 63.

## 1. Objective

Make every Chronicle event traceable to its cause, so a user can ask "why did this happen?"
and walk back through the causal chain to the originating edit or intervention. Build 62 made
the Chronicle readable and exportable; build 64 adds the **cause graph**: each Chronicle event
optionally references the event that caused it, and a `why_trace` query walks the chain back to
its root. Intervention edits and their downstream consequences are linked end-to-end.

### In scope
- `Event.cause: Option<usize>` — index into `History.events` of the causing event; `None` = root/user-initiated
- `History::push` accepts an optional cause; call sites pass cause where known
- `History::why_trace(start) -> Vec<usize>` — walks back from an event through its causes to the root
- `History::cause_children(index) -> Vec<usize>` — forward links: events caused by this one
- CLI `cosmogon-cli cause <event_index>` / `cosmogon-cli why <event_index>` — prints the full causal chain
- CLI `cosmogon-cli interventions` — lists all interventions with their event indices and downstream traces
- Native UI panel: cause graph view showing the chain for a selected event
- All existing push call sites updated: edits/interventions pass their cause; impact/climate/biosphere/civ consequence events link to their trigger

### Out of scope
- A full "WHY-graph" with multiple parents per event (single parent `cause` is sufficient for build 64; multi-parent deferred to build 65)
- Automatic intervention suggestions or AI-driven explanations
- Save-schema changes beyond `Option<usize>` with `skip_serializing_if=None` (byte-identical when None)
- New event types (build 62 established: zero new push sites in golden window)

## 2. Context (verified)

- `Event { time, category, importance, title, detail, system, body, civ }` (`history.rs:67-77`) — no `cause` field yet
- `History::push(&mut self, e: Event)` (`history.rs:162-179`) — O(1) amortized minor pruning (build 62)
- `History::chronicle_indices(&f)` / `chronicle(&f)` (`history.rs:232-255`) — build 62 query API
- `History` uses manual `PartialEq` comparing only `events` (`history.rs:152-156`) — cache fields are `#[serde(skip)]`
- `Edit` enum (`sandbox.rs:84-136`) — `Intervene { system, civ, action }` plus many physical edits
- `EditRecord { time, summary, edit }` (`sandbox.rs:151-155`) — journal of all edits
- `Universe::apply_edit(&mut self, edit: Edit)` (`sandbox.rs:648-1006`) — validates, applies, journals, pushes Chronicle events
- `Universe::apply_intervention(&mut self, civ, action)` (`intervene.rs:108-164`) — pushes one Chronicle event per intervention
- Impact/consequence chain: `on_contact` → `apply_impact` → `refresh_body_environment` → Chronicle events (`sandbox.rs:1050-1245`)
- `step_environment` (`sandbox.rs:1292+`) — climate/insolation changes push Chronicle events
- Golden test: `golden_universe_fingerprint` advances Sol seed 2026 for 5 kyr, fingerprints the universe (`save.rs:635-645`)
- CLI dispatch: `main.rs:670-885`, `chronicle_export` at `main.rs:1068` as template for read-only commands

## 3. Tasks

### T1 — `Event.cause` field and `push` with cause

`crates/cosmogon_sim/src/history.rs`:

```rust
pub struct Event {
    pub time: f64,
    pub category: Category,
    pub importance: u8,
    pub title: String,
    pub detail: String,
    pub system: Option<u32>,
    pub body: Option<u32>,
    pub civ: Option<u32>,
    /// Index into `History.events` of the event that caused this one; `None` = root cause
    /// (a user edit/intervention, or a natural event with no traced cause).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<usize>,
}
```

- `#[serde(default, skip_serializing_if = "Option::is_none")]` → old saves deserialize (cause=None),
  new saves with cause=None serialize byte-identically (field omitted), golden unchanged
- `History::push_caused(&mut self, e: Event, cause: Option<usize>)` — like `push` but sets `cause`
- `History::push(&mut self, e: Event)` — kept as `push_caused(e, None)` for backward compatibility
- `History::why_trace(&self, start: usize) -> Vec<usize>` — walks `cause` links back to root, returns
  indices root→start (chronological). Detects cycles (impossible by construction but defensive).
- `History::cause_children(&self, index: usize) -> Vec<usize>` — forward scan for events whose `cause == index`

Tests (`history.rs`):
1. `cause_links_form_a_chain_back_to_root` — push events with causes, verify why_trace returns root→leaf
2. `why_trace_detects_cycle_safely` — manually create a cycle, verify it doesn't loop forever
3. `cause_children_returns_forward_links` — verify forward links match reverse traces
4. `cause_is_omitted_from_serialization_when_none` — serialize an event with cause=None, verify no `cause` key
5. `cause_is_preserved_in_serialization_when_some` — serialize with cause=Some, verify round-trip

### T2 — Cause tracking in edits and interventions

`crates/cosmogon_sim/src/sandbox.rs` and `crates/cosmogon_sim/src/intervene.rs`:

- `apply_edit`: after pushing the Chronicle event for an edit, record its event index; downstream
  consequence events link to it
- `apply_intervention`: the Chronicle event pushed gets cause=None (root/user action)
- `on_contact` → `apply_impact`: the impact event's cause = the event index of the prior trigger
- `step_environment`: climate events caused by a prior impact get cause=impact_event_index

For build 64, thread cause through:
- `apply_impact(s, i, rec, cause: Option<usize>)` 
- `on_contact(s, ev, cause: Option<usize>)`
- `step_environment` passes cause to climate events when known

### T3 — CLI `cause` / `why` / `interventions` commands

`crates/cosmogon_cli/src/main.rs`:

```sh
cosmogon-cli cause <event_index> [--initial SAVE | --seed/--scenario/--years]
  # Print the causal chain root→event for event_index
cosmogon-cli why <event_index> [--initial SAVE | --seed/--scenario/--years]
  # Alias for cause
cosmogon-cli interventions [--initial SAVE | --seed/--scenario/--years]
  # List all interventions with event indices, summaries, and their downstream traces
```

- Reuses `initial_universe(a)`; advances unless `--initial`
- `cause`/`why`: validates event index exists, prints `why_trace` as indented chain with titles/details
- `interventions`: scans `u.edits` for `Edit::Intervene`, prints each with its event index and `why_trace`
- Refusals: unknown event index, `--initial`+`--seed/--years`, ignored flags

Test (`main.rs`): `cause_arguments_refuse_unknown_index_and_ignored_flags`

### T4 — Native UI cause graph panel

`crates/cosmogon/src/ui/inspect.rs`, `crates/cosmogon/src/ui/mod.rs`:

- `--panel cause` opens a new Cause tab in the inspector (or a left-panel view)
- Shows: selected event's title/detail, the full causal chain root→event (indented), each link
  labelled with the relationship ("caused", "led to", "triggered")
- Clicking any event in the chain re-roots the view at that event
- `--panel cause` support in `ui/mod.rs` panel dispatch

### T5 — Docs, validation, tracker

- `docs/CAUSE.md` (guide), `docs/CAUSE_DESIGN.md` (design), `docs/CAUSE_VALIDATION.md` (evidence)
- `docs/SAVE_FORMAT.md`: note that `Event.cause` is `#[serde(skip_serializing_if=None)]`, byte-identical when None
- `CHANGELOG.md`, `docs/BUILD_TRACKER.md` row 64, `docs/MASTER_GAP_ANALYSIS.md` entry 62
- `HANDOFF.md`, `docs/CONTINUATION_HANDOFF.md` verification record
- Captures: `output/qa/b64-cause.png` (UI cause panel), `output/qa/b64-interventions.png` (CLI interventions output)

## 4. Golden-neutrality argument

| Change | Saved field? | RNG draw? | Mutates Universe? | Golden risk |
|---|---|---|---|---|
| `Event.cause` field | `#[serde(skip_serializing_if=None)]` — omitted when None | none | no | none (all normal events have cause=None) |
| `push_caused` | none | none | no | none |
| CLI `cause`/`why`/`interventions` | none | none | no (reads a cloned universe) | none |
| UI cause panel | none | none | no | none |

The golden test advances the universe headless with no interventions; all events have cause=None;
serialization omits the field; fingerprint unchanged. `cargo test -p cosmogon_sim
golden_universe_fingerprint` must stay green with the constant unchanged.

## 5. Validation plan

1. `cargo test --release --workspace` — expect build-63 baseline 709 + ≥6 new = ≥715 passed, 0 failed, 1 ignored
2. `cargo clippy --release --workspace --all-targets` — must stay at 43-warning / 0-error baseline
3. `cargo test --release -p cosmogon_sim golden_universe_fingerprint` — `0xf2c4140a45ae509b`
4. CLI: `cause 0`, `why 0`, `interventions` on a seeded run; refusal cases
5. Save round-trip: save → load → cause trace identical (cause=None events survive round-trip)
6. Captures inspected at the two viewpoints above

## 6. Risks

- **Cause index invalidation**: pruning (minor event cap) removes events, invalidating cause indices
  that point to pruned events. Mitigation: `why_trace` stops at a missing/None cause; pruned events
  are minor (importance < 3) and unlikely to be causes of milestone events. Document this.
- **Serialisation of cause=Some**: only intervention/downstream events have cause=Some; these don't
  appear in the golden test. Round-trip saves preserve them.
- **Complexity**: threading cause through the consequence chain is invasive. Mitigation: build 64
  sets cause=None for root events and Some(prior index) only where the prior event index is
  immediately available (impact → climate chain).
