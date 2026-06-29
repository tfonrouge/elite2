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
| `plugins/skybox`    | *(camera helper)* | Deep-space background: a `Skybox` cubemap (DL-018)       |
| `plugins/hud`       | `HudPlugin`       | Minimal flight HUD (speed, throttle, orientation)       |
| `plugins/combat`    | `CombatPlugin`    | *(Phase 2, in progress)* weapons, damage, AI, spawning  |

Cross-plugin sharing is limited to small marker/data types: `camera`, `hud`, and
`combat` reference `flight::{Player, Ship, FlightConfig}`. The cockpit camera
mounts in `PostStartup` (so the ship, spawned in `Startup`, already exists) and
carries the `Skybox` cubemap built by `skybox`.

## Decision log

> The formal ledger — with alternatives considered, who approved each decision,
> and a falsification condition per decision — lives in
> [`DECISIONS.md`](DECISIONS.md). The faithful gameplay spec (what the original
> game *is*, with verified data tables) lives in [`GAMEPLAY.md`](GAMEPLAY.md). The
> summary below is the quick reference.

- **Canonical version baseline = BBC disc / enhanced superset** (DL-010): the
  remake targets the disc/enhanced BBC original (missions, mining, full laser
  line-up), excluding port-only extras; a `cassette-pure` mode is a later toggle.

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
- **Flight axes = faithful no-yaw default + yaw-assist toggle** (DL-011). The 1984
  original has **no yaw** (pitch + roll only); the default flight model matches it,
  with an optional yaw-assist mode behind a toggle. This reworks the three-axis
  control shipped in the initial Phase 1 slice; the damped-arcade model is intact.
- **Camera = first-person cockpit** (Phase 1, decided). Mounted as a child of
  the ship so transform propagation handles the follow; no per-frame sync.
- **Controls:** keyboard — **W/S pitch, A/D roll, R/F throttle** by default; with
  yaw assist on (toggle `Y`), **Q/E add yaw**. Verified against Bevy's rotation
  math (W = nose down, A = roll left, Q = yaw left). Mouse/gamepad and an
  invert-pitch option are later additions.
- **Sky background = `Skybox` cubemap** (DL-018, supersedes the original
  Fibonacci point-starfield). A cubemap on the HDR + bloom cockpit camera: turning
  the ship reveals the sky, translation shows no parallax. A runtime-generated
  placeholder cubemap (stars + a faint band) stands in for a real astronomical
  image, swapped in later.

### Decisions still open (raise before committing)

- **Procedural seed scheme (Phase 3):** how galaxy generation is seeded and made
  deterministic.
- **Save format (Phase 6):** serialization approach for player + universe state.

## Roadmap

Ordered milestones (may be reprioritized).

- **Phase 0 — Foundations** ✅
  Cargo project, pinned Bevy, plugin skeleton, a window that opens and renders,
  cross-platform CI with per-platform artifacts, README/DESIGN/CLAUDE docs.
- **Phase 1 — Flight vertical slice** ✅
  Controllable ship in 3D (thrust/pitch/roll, damped-arcade model),
  first-person cockpit camera, procedural starfield + reference objects (ring
  station, asteroids), minimal HUD (speed, throttle, orientation). Pure-logic
  unit tests for the flight/starfield math.
- **Phase 1.5 — Gameplay spec & faithful-flight rework** ✅
  Researched and documented the original 1984 *Elite* mechanics in
  [`GAMEPLAY.md`](GAMEPLAY.md) (the design baseline for all later phases);
  resolved the canonical version baseline (DL-010) and the no-yaw flight model
  (DL-011), reworking the Phase-1 controls to the faithful two-axis default with
  an optional yaw-assist toggle.
- **Phase 2 — Combat** ⏳ *(current; spec in [`GAMEPLAY.md`](GAMEPLAY.md) §10–§13,
  decisions DL-012…DL-017)*
  Built as one full faithful slice. **Step 0 (prep):** `FlightConfig`
  Resource→Component, the flight integrator `Single`→`Query` split into
  player/AI movement, a `FlightSet::ReadInput` set before `Integrate`, and the
  HUD reading `FlightConfig` from the player entity. **Combat:** per-view
  instant-hit lasers (analytic ray-vs-sphere, DL-012), homing missiles + ECM
  (DL-013), sphere-sphere collision (DL-014), shields→energy damage (no separate
  hull), the full faithful enemy tactics routine (DL-015) with a hybrid evade
  exit (DL-016), destruction + bounty→rank, and a debug command-spawn plus an
  in-system spawn hook (DL-017). Combat events modelled as 0.18 **Messages**;
  debug gizmos gated so the headless tests stay green.
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
- Sky: **swap the placeholder skybox for a real astronomical image** (DL-018) and
  tune `SKYBOX_BRIGHTNESS`/bloom at the screen; decide fixed-vs-per-galaxy skies
  (Phase 3). HDR + bloom on the cockpit camera is now in place.
- Visual polish: a real cockpit frame overlay; emissive materials on objects.
- macOS code-signing + notarization for distributable builds (CI produces an
  unsigned macOS binary today).
- Decide whether to invest in a WebAssembly build target.
