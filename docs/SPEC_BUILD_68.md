# Build 68 — Real nebulae/galaxy/deep-sky datasets

Status: ready. **Golden-neutral build**: `GOLDEN = 0xf2c4140a45ae509b` unchanged.

## Objective
Add real deep-sky objects (nebulae, galaxies, star clusters) as visitable locations with
provenance tracking. Each object is a `Nebula` with position, type, size, and source
citation. CLI lists deep-sky objects.

## Scope
- `crates/cosmogon_sim/src/astro/deepsky.rs`: Deep-sky object catalogue (Orion Nebula,
  Andromeda Galaxy, Pleiades, etc.) with positions, types, magnitudes, provenance
- CLI `cosmogon-cli deepsky`: list deep-sky objects
- Test: objects have valid positions, types, provenance
- Docs: `docs/DEEPSKY{,_DESIGN,_VALIDATION}.md`, tracker row 68

## Out of scope
- Full deep-sky catalogues (millions of objects) — notable subset only
- Physical modelling of nebulae/galaxies (visual locations only)
- Interstellar travel to deep-sky objects (build 65 covers probe infrastructure)

## Validation
- `cargo test --release --workspace` green
- `cargo clippy` at 43/0 baseline
- Golden unchanged
