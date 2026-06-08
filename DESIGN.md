# DESIGN.md — Elite remake

Living design document. Records the vision, architecture, key decisions, and the
phase roadmap. Update it as decisions are made.

## Vision

A single-player, open-ended space sim: pilot a ship through a procedurally
generated universe, free to trade, fight, mine, or explore. We reimagine the
1984 original with modern 3D rendering while preserving the systems that made it
special — a procedural galaxy, a supply/demand economy, real-time combat, ship
progression, and combat/legal ranking. The sandbox is the game; no forced story.

## Architecture

- **Engine:** Bevy (ECS). Components are data, systems are behavior, resources
  are global state.
- **One plugin per system.** Each major game system is a Bevy `Plugin` in its
  own module under `src/plugins/`. `main` boots the engine + window and then
  registers the plugin list — nothing else. This keeps systems decoupled and the
  entry point readable.
- **Vertical slices.** Build one phase at a time; get it compiling and running
  before moving on. A small thing that runs beats a large thing that doesn't.

Current modules:

| Module              | Plugin            | Responsibility                                          |
| ------------------- | ----------------- | ------------------------------------------------------- |
| `plugins/core`      | `CorePlugin`      | App-wide config (clear color; later: states)            |
| `plugins/flight`    | `FlightPlugin`    | Player ship, `FlightConfig`, damped-arcade flight model |
| `plugins/camera`    | `CameraPlugin`    | First-person cockpit camera (child of the ship)         |
| `plugins/world`     | `WorldPlugin`     | Lighting + reference objects (ring station, asteroids)  |
| `plugins/starfield` | `StarfieldPlugin` | Procedural starfield that tracks the player             |
| `plugins/hud`       | `HudPlugin`       | Minimal flight HUD (speed, throttle, orientation)       |

Cross-plugin sharing is limited to small marker/data types: `camera`, `hud`,
and `starfield` reference `flight::{Player, Ship, FlightConfig}`. The cockpit
camera mounts in `PostStartup` so the ship (spawned in `Startup`) already exists.

## Decision log

> The formal ledger — with alternatives considered, who approved each decision,
> and a falsification condition per decision — lives in
> [`DECISIONS.md`](DECISIONS.md). The summary below is the quick reference.

- **Bevy version: pinned to exactly `=0.18.1`** (latest stable as of project
  start; verified against crates.io). Bevy's API changes between releases, so a
  bump is a deliberate, called-out decision with migration work — never casual.
  `Cargo.lock` is committed for reproducible builds.
- **Rust edition 2024**, toolchain pinned to `stable` via `rust-toolchain.toml`.
- **Build profiles:** dev builds optimize dependencies (`opt-level = 3` for
  `*`) but not our crate (`opt-level = 1`) so iteration stays fast while the game
  runs smoothly. Release uses thin LTO + `codegen-units = 1` + `strip`.
- **`dev` feature** forwards to `bevy/dynamic_linking` for fast incremental dev
  builds; off by default (dynamic linking complicates distribution).
- **Clippy:** `clippy::all` at warn, CI denies warnings. `too_many_arguments`
  and `type_complexity` are allowed — they fight Bevy's ECS idioms.
- **Phase 0 visible scene (superseded by Phase 1):** a single lit cube,
  deliberately, to prove the 3D render pipeline (mesh + material + light +
  camera) works end-to-end. Replaced by Phase 1's ring station + asteroids; kept
  here as historical record.
- **Flight model = damped arcade** (Phase 1, decided). Throttle sets a target
  speed the ship accelerates toward; angular velocity ramps toward input and
  damps to zero on release (auto-stabilize). Integration is `dt`-scaled (speed
  and the exponential angular smoothing are frame-rate independent; applying
  angular velocity as three sequential local rotations per frame is a minor
  arcade approximation, since rotations don't commute). A Newtonian /
  flight-assist-off mode is a possible later toggle.
- **Camera = first-person cockpit** (Phase 1, decided). Mounted as a child of
  the ship so transform propagation handles the follow; no per-frame sync.
- **Controls (Phase 1):** keyboard — W/S pitch, A/D yaw, Q/E roll, R/F throttle.
  Verified against Bevy's rotation math (W = nose down, A = yaw left, Q = roll
  left). Mouse/gamepad and an invert-pitch option are later additions.
- **Starfield approach:** stars on a Fibonacci-lattice sphere shell, parented to
  a root that tracks the player's *translation only*. Result: no translation
  parallax (distant-star feel) but rotation reads clearly. Deterministic, no RNG
  crate.

### Decisions still open (raise before committing)

- **Procedural seed scheme (Phase 3):** how galaxy generation is seeded and made
  deterministic.
- **Save format (Phase 6):** serialization approach for player + universe state.

## Roadmap

Ordered milestones (may be reprioritized).

- **Phase 0 — Foundations** ✅
  Cargo project, pinned Bevy, plugin skeleton, a window that opens and renders,
  cross-platform CI with per-platform artifacts, README/DESIGN/CLAUDE docs.
- **Phase 1 — Flight vertical slice** ✅ *(current)*
  Controllable ship in 3D (thrust/pitch/yaw/roll, damped-arcade model),
  first-person cockpit camera, procedural starfield + reference objects (ring
  station, asteroids), minimal HUD (speed, throttle, orientation). Pure-logic
  unit tests for the flight/starfield math.
- **Phase 2 — Combat**
  Weapons (projectile/hitscan), targeting, basic enemy AI (approach/attack/
  evade), shields + hull, destruction.
- **Phase 3 — The universe**
  Deterministic procedural galaxy from a seed (names, positions, economy/tech/
  government), galaxy & system map UI, hyperspace jumps.
- **Phase 4 — Economy & trading**
  Per-system commodity markets driven by economy type and supply/demand;
  credits, cargo hold, buy/sell, arbitrage.
- **Phase 5 — Stations & docking**
  Space stations, docking mechanics, station services (market, refuel, repair,
  outfitting).
- **Phase 6 — Progression & persistence**
  Ship upgrades and additional hulls, combat rank ladder (Harmless → Elite),
  legal status (clean/offender/fugitive), save/load of player + universe seed.
- **Phase 7 — Polish**
  HUD/UI refinement, audio (engine/weapons/ambient), simple missions, balance.

## Follow-ups / TODO

- **Hands-on flight tuning:** the control *directions* are verified against
  Bevy's rotation math, but the *feel* (rates, damping, top speed in
  `FlightConfig`) needs a human at the keyboard — it can't be tested headlessly.
- Optional flight polish: mouse/gamepad input, an invert-pitch toggle, a
  "full stop" / "match speed" key, and a velocity vector marker on the HUD.
- Visual polish: HDR camera + bloom so emissive stars/objects glow; a real
  cockpit frame overlay; lower-poly or instanced stars if the field ever grows.
- macOS code-signing + notarization for distributable builds (CI produces an
  unsigned macOS binary today).
- Decide whether to invest in a WebAssembly build target.
