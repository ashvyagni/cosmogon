# Planetary climate — 0-D grey greenhouse and blast transients

## What this model is

Surface temperature on a world with an atmosphere is a **zero-dimensional (global-mean) grey
greenhouse** on top of radiative equilibrium (`crates/cosmogon_sim/src/planet/environment.rs`):

1. `T_eq = 278.6 · L^(1/4) · (1−A)^(1/4) / √d` — radiative equilibrium with Earth's constant
   folded in (K, L in L☉, d in AU, A = albedo).
2. `T_s = T_eq · (1 + 0.75·τ)^(1/4) + bias` — a grey atmosphere with optical depth τ. τ is a
   parametric fit: a thick-atmosphere power law in CO₂ calibrated on Venus/Mars, a logarithmic
   trace-gas term (≈3 K per doubling on Earth), an optional water-vapour term calibrated on
   Earth, and pressure-induced H₂ absorption.
3. Four coupled passes then settle water phase (ocean/ice fractions), and for non-real bodies the
   ice/albedo/cloud feedback that produces the snowball threshold.

The calibration offset (`climate_bias`) is fitted per real body (`calibrate_real_body`) so the
model reproduces measured present-day temperatures (Venus ≈737 K, Earth ≈288 K, Mars ≈210 K).
**It is a calibration, not a prediction**, and it is reserved for that purpose — nothing else may
write to it.

## What feeds it

- `refresh_climate` (`universe.rs`) recomputes temperature whenever the environment task runs.
- The environment task (`TASK_ENV`, 10-day period) refreshes a body when its insolation moves
  more than 0.2 %; changes above 2 % are Chronicle-reported with the causal chain
  (sunlight % → ΔT → habitability → civ capacity).
- Stellar evolution, user edits (orbit/star changes), terraforming and collisions all funnel
  through the same refresh.

## Blast transients (build 61)

A supernova radiation front that reaches another system applies a **blast winter**: a bounded,
decaying surface-air cooling of at most **30 K** (`blast_winter_peak_k`,
`crates/cosmogon_sim/src/impact.rs`), driven by the front's severity (the bounded log-ratio of
fluence between the harm and sterility thresholds). It is an `ImpactWinter` — the same 3-year
e-fold decay, stacking rule and 0.05 K expiry that asteroid impacts use — so it stacks safely,
survives save/load, and clears itself. Forcing lands on **every surface world** in the reached
system, including lifeless and sterile ones; only the Chronicle event is gated on an active
biosphere.

## What this model deliberately does not do

- **No regional or latitude structure.** One number per world. A real climate answer needs a GCM;
  latitude/season shading is procedural `terrain` decoration, not model output. *(Deferred:
  GCM-class work is out of scope for this model.)*
- **No weather, cloud dynamics or ocean circulation.**
- **No spectral radiative transfer** — τ is a fit, not line-by-line absorption.
- **No uncertainty quantification** on the climate state itself.
- **No WHY causal panel** — Chronicle details carry the causal terms; the interactive cause graph
  belongs to later builds.

See [CLIMATE_DESIGN](CLIMATE_DESIGN.md) for the forcing law and units, and
[CLIMATE_VALIDATION](CLIMATE_VALIDATION.md) for what has actually been verified.
