# Infrastructure — visible civilization presence on every world

Build 63 makes civilizations physically legible: what each settlement *is* (farm, mine, port, industry, spaceport),
what share of the world it occupies, and how it connects — all derived from authoritative state the simulation already
tracks. Nothing here is a second simulation; the numbers are a view, not a new model.

## What changed

**Derived role per settlement** (`civ::infrastructure::works`). Each active settlement is scored for five functions:

| Role | Basis |
|------|-------|
| **Farm** | Rural share of society × fertile-land endowment; small settlements (Camp/Village) get a smallholding bonus |
| **Mine** | Only at a site with a nearby ore deposit; scaled by endowment richness and available labour |
| **Port** | Coastal sites; full weight with a sea link, partial with seafaring technology |
| **Industry** | Urban share × electrification (energy per person) × settlement size tier; needs `electric_light` |
| **Spaceport** | The largest settlement, only once the civilization flies (`satellites`, missions, stations, colonies) |

The dominant role wins; ties resolve in a fixed order (Farm < Mine < Port < Industry < Spaceport) so results are
deterministic.

**Aggregated footprint per civilization** (`civ::infrastructure::footprint`):

- Role counts: farms, mines, ports, industries, spaceports
- Built-up area: Σ population / representative density per tier (Camp 50 … Megacity 6,000 people/km²)
- Farmland: 0.15 ha per person × fertile-land fraction (Earth's ~8 billion → ~12 million km², the right order)
- Transport: true spherical chord length of every road / rail / sea link × body radius
- Mining workforce: population at mine-dominant settlements
- Electrification: `electric_light` flag × energy per person, saturating at 2 kW/person
- Fossil horizon: years of coal+oil endowment left at the current burn rate, using the simulation's documented
  2e22 J/unit conversion

**CLI report** (`cosmogon-cli footprint`):

```sh
cosmogon-cli footprint                              # table, all living civilizations
cosmogon-cli footprint --scenario lab --years 5000  # simulate then report
cosmogon-cli footprint --civ 3                      # one civilization only
cosmogon-cli footprint --json                       # full manifest: format cosmogon-footprint/1
```

The table line looks like:

```
Humanity · Sol · Earth: 260 settlements (farms 128 mines 2 ports 0 industry 129 spaceports 1)
  · built 210644 km² · farmland 11123607 km² · road 0 km / rail 0 km / sea 0 km
  · electrified 100% · mining workforce 13500000 · fossil horizon 106 yr
```

**Native view** — Civilization inspector gains an Infrastructure section after Settlements with role counts, areas,
transport, electrification, workforce, fossil horizon, largest settlements with roles, and a role legend
(mine ▲, port ⚓, industry ■, spaceport ○). The sphere markers layer draws role glyphs beside the largest
settlements and animated traffic dots along links; the night-light bake biases industrial belts brighter and
farmland dimmer.

## Honesty about what this is

- **Derived, not simulated.** Infrastructure is a function of settlement/population/energy/endowment state. No ore is
  depleted, no trade is modelled, no prices exist. If the civ step changes a settlement, the footprint changes with
  it.
- **Representative densities.** Built-up km² and farmland km² use stated per-tier densities and per-capita cropland.
  They are defensible orders of magnitude, not measured footprints.
- **Relative units.** Resource endowment is in the simulation's relative units (Earth = 1.0). Only fossil fuels have a
  documented energy conversion and therefore a depletion horizon; metals are reported as workforce share, not tonnage.
- **No new saved state.** `#[serde(skip)]` everywhere relevant; the serialized `Universe` is unchanged and the golden
  fingerprint `0xf2c4140a45ae509b` is unaffected.

## Design, validation

See [INFRASTRUCTURE_DESIGN](INFRASTRUCTURE_DESIGN.md) for the full rules, constants and assumptions, and
[INFRASTRUCTURE_VALIDATION](INFRASTRUCTURE_VALIDATION.md) for what was verified.
