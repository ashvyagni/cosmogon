# Design — greenhouse forcing, blast winters and environment scheduling

## Radiative core

`equilibrium_temperature`, `greenhouse_tau` and `update_climate` are pure functions of
`(body, star, t, distance)` (`crates/cosmogon_sim/src/planet/environment.rs`). The four inner
passes are a fixed-point relaxation of albedo ↔ water phase ↔ temperature; convergence is not
iterated to tolerance — four passes is the calibrated working point.

Albedo feedback runs **only for non-real bodies** (`!body.real`): real bodies carry a measured
albedo and the `climate_bias` calibration, so letting the ice/albedo loop loose on them would
double-count. The snowball threshold therefore lives entirely in the non-real branch.

## Blast winter — the forcing law

```text
severity  = clamp( log10(fluence / FLUENCE_HARM) / log10(FLUENCE_STERILE / FLUENCE_HARM), 0, 1 )
peak_k    = severity * BLAST_WINTER_MAX_K          # 30.0 K, the documented impact-winter cap
```

- **Units:** kelvin of surface-air cooling, identical sign and units to `ImpactWinter.peak_k`
  (temperature is reduced by `cooling_at(t)` in `update_climate`).
- **Arithmetic:** `clamp` and `*` on an already-computed severity — pure IEEE-754, bit-exact,
  no RNG, no trajectory-coupled transcendentals; DMath is not required.
- **Why 30 K:** `winter_peak_cooling` already caps impact winters at 26–30 K as the physical
  surface-air bound; the blast path reuses that cap rather than inventing a second scale.

### Application point

In `propagate_blasts` (`stellar.rs`), at arrival, **before the biosphere gate**: every body
with `exists() && kind.has_surface()` takes the winter and an immediate
`refresh_body_environment`, so ΔT and habitability land in the same step as arrival. The
existing biosphere/civ damage and the `Supernova radiation reaches …` Chronicle event stay
gated on an active biosphere — lifeless worlds cool silently.

### Stacking (multiple simultaneous blasts)

```text
peak_now = max( existing.cooling_at(t), new_peak )   # restart the decay clock at arrival
```

Restarting the clock with the larger instantaneous cooling keeps the total ≤30 K, deterministic,
and never additive beyond the cap.

### Recovery and the static-system wiring

Decay is 3-year e-fold (`ImpactWinter::EFOLD_YEARS`), expiry below 0.05 K. Expiry runs in
`step_environment` — which historically only did work for **dynamic** systems, and was not
scheduled at all when no system is dynamic. Build 61 closes both legs:

1. `ensure_environment_task` also schedules when any body carries an `impact_winter`.
2. `step_environment`'s non-dynamic branch iterates only bodies with an active winter: clear
   expired ones, refresh active ones, skip all flux bookkeeping (nothing moves).

Only universes that receive a blast winter change their schedule, so the golden universe's
scheduler is untouched.

## Chronicle causal detail

The environment task's >2 % event and the blast arrival event both carry the chain:
sunlight/severity → `T a → b (Δ K)` → `habitability a % → b %` → `capacity …`. The capacity
term is the civ's **cached** value, recomputed at its next civ step — at most one step stale at
the moment the event is written. This is accepted and stated here rather than forcing a civ-step
inside the environment task.

## Persistence

No new serialized field. Blast-in-transit is the existing `Blast` (`Universe.blasts`); the
transient is the existing `Body.impact_winter` with an absolute `start` time. The golden
fingerprint (`GOLDEN = 0xf2c4140a45ae509b`) is therefore unchanged by this build.
