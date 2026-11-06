# Build 66 — Real catalogue streaming (Gaia nearby stars) with provenance

Status: ready. **Golden-neutral build**: `GOLDEN = 0xf2c4140a45ae509b` unchanged.

## Objective
Add Gaia DR3 nearby-star catalogue data as a provenance-tracked source. Stars from the
catalogue become `Body` records with `provenance.source = "Gaia DR3"` and per-field quality
annotations. A CLI command lists catalogue entries. No new serialized field beyond the
existing `Provenance` (already `#[serde(skip_serializing_if=None)]`).

## Scope
- `crates/cosmogon_sim/src/astro/gaia.rs`: Gaia DR3 nearby-star catalogue (within 10 pc)
  with positions, parallax, magnitudes; each entry becomes a `Body` with provenance
- CLI `cosmogon-cli gaia [--limit N]`: list nearby stars from the catalogue
- CLI `cosmogon-cli gaia-load --output NEW_DIR`: export catalogue as JSON with provenance
- Test: catalogue entries have provenance, positions are finite, limit is respected
- Docs: `docs/GAIA{,_DESIGN,_VALIDATION}.md`, tracker row 66

## Out of scope
- Full Gaia DR3 (1.7 billion sources) — nearby subset only
- Proper motion propagation over time (positions are epoch J2000)
- Catalogue streaming from network (local embedded data)
- Cross-matching with existing Sol system bodies

## Validation
- `cargo test --release --workspace` green
- `cargo clippy` at 43/0 baseline
- Golden unchanged
