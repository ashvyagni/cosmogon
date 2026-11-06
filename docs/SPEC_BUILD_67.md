# Build 67 — Known exoplanet systems

Status: ready. **Golden-neutral build**: `GOLDEN = 0xf2c4140a45ae509b` unchanged.

## Objective
Add known exoplanet systems (TRAPPIST-1, Proxima Centauri b, etc.) as explorable star
systems. Each system is constructed from published orbital parameters with provenance
tracking. No new serialized fields.

## Scope
- `crates/cosmogon_sim/src/astro/exoplanets.rs`: Known exoplanet systems (TRAPPIST-1,
  Proxima Centauri, etc.) as `StarSystem` constructions with provenance
- CLI `cosmogon-cli exoplanets`: list known exoplanet systems
- CLI `cosmogon-cli exoplanets-load <name> --output NEW_DIR`: export a system as JSON
- Test: exoplanet systems have valid orbits, provenance, finite parameters
- Docs: `docs/EXOPLANETS{,_DESIGN,_VALIDATION}.md`, tracker row 67

## Out of scope
- Full exoplanet archive (thousands of systems) — nearby/notable subset only
- Habitability modelling for exoplanets (uses existing assess() function)
- Interstellar travel to exoplanets (build 65 covers probe infrastructure)

## Validation
- `cargo test --release --workspace` green
- `cargo clippy` at 43/0 baseline
- Golden unchanged
