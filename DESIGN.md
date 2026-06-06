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

| Module            | Plugin         | Responsibility                                  |
| ----------------- | -------------- | ----------------------------------------------- |
| `plugins/core`    | `CorePlugin`   | App-wide config (clear color; later: states)    |
| `plugins/camera`  | `CameraPlugin` | The 3D camera (fixed now; ship-follow in P1)    |
| `plugins/world`   | `WorldPlugin`  | Placeholder lit scene (replaced from P1)        |

## Decision log

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
- **Phase 0 visible scene:** a single lit cube, deliberately, to prove the 3D
  render pipeline (mesh + material + light + camera) works end-to-end. It is a
  placeholder, not gameplay.

### Decisions still open (raise before committing)

- **Flight model (Phase 1):** arcade vs. Newtonian (or a damped hybrid).
  *Recommendation:* start with a damped/arcade model with a capped top speed and
  responsive rotation — closest to the original's feel and easy to fly — and
  revisit Newtonian later if desired.
- **Camera (Phase 1):** cockpit vs. chase. *Recommendation:* chase cam first
  (easier to make motion legible in 3D), with cockpit as a later option.
- **Procedural seed scheme (Phase 3):** how galaxy generation is seeded and made
  deterministic.
- **Save format (Phase 6):** serialization approach for player + universe state.

## Roadmap

Ordered milestones (may be reprioritized).

- **Phase 0 — Foundations** ✅ *(current)*
  Cargo project, pinned Bevy, plugin skeleton, a window that opens and renders,
  cross-platform CI with per-platform artifacts, README/DESIGN/CLAUDE docs.
- **Phase 1 — Flight vertical slice**
  Controllable ship in 3D (thrust/pitch/yaw/roll), follow camera, starfield +
  reference objects (asteroid, station placeholder), minimal HUD (speed,
  orientation).
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

- macOS code-signing + notarization for distributable builds (CI produces an
  unsigned macOS binary today).
- Decide whether to invest in a WebAssembly build target.
- Add unit tests as systems with real logic appear (none meaningful in Phase 0).
