# CLAUDE.md — Elite Remake (Rust + Bevy)

Persistent project context. Read at the start of every session. Keep this file short and current.

## What this is

A cross-platform, single-player remake of the 1984 space trading/combat game **Elite**, built in Rust with the Bevy engine. Open-ended sandbox: trade, fight, mine, explore. No forced story. Modern 3D rendering, but faithful to the original's systems (procedural galaxy, supply/demand economy, real-time combat, ship progression, combat/legal ranks).

## Tech stack

- **Rust** — latest stable toolchain (edition 2024; pinned to `stable` in `rust-toolchain.toml`).
- **Bevy** — pinned to **exactly `=0.18.1`** in `Cargo.toml` (the version there is the source of truth). `Cargo.lock` is committed.
- **Targets** — Windows, macOS, Linux as first-class desktop platforms. WebAssembly path kept open but not actively developed.

## Critical rules

- **Verify the Bevy API before writing Bevy code.** Bevy's API changes between versions. Do not write Bevy code from memory — confirm system signatures, app/plugin setup, query syntax, schedules/states, and asset loading against the docs for the version pinned in `Cargo.toml`.
- **Do not bump the Bevy version casually.** A version change is a deliberate decision; expect API migration work and call it out before doing it.
- **Keep the tree warning-free.** Code must pass `cargo clippy` clean and be formatted with `cargo fmt`.
- **One plugin per system.** Each major game system is its own Bevy `Plugin` in its own module. `main` is just a list of plugins. Components hold data, systems hold behavior, resources hold global state.
- **Build in vertical slices, one phase at a time.** Get it compiling and running before moving on. See `DESIGN.md` for the phase roadmap.
- **Ask before big forks.** For decisions with real tradeoffs (flight model, procedural seed scheme, save format), lay out options + a recommendation and confirm before committing.

## Commands

- Build: `cargo build`  (fast dev iteration: `cargo run --features dev`)
- Run: `cargo run`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt`
- Test: `cargo test`

(Update these if they change.)

## Project layout

```
src/
  main.rs          # thin: window config + the list of plugins, nothing else
  plugins/
    mod.rs         # declares the plugin modules
    core.rs        # CorePlugin   — app-wide config (clear color; later: states)
    camera.rs      # CameraPlugin — the 3D camera
    world.rs       # WorldPlugin  — placeholder lit scene (replaced from Phase 1)
assets/            # runtime assets (Bevy AssetServer root)
.github/workflows/ # cross-platform CI (Windows/macOS/Linux), per-OS artifacts
```

New systems get a new `plugins/<system>.rs` and a line in `main`. Future
systems (per the roadmap): `flight`, `combat`, `galaxy`/`universe`, `economy`,
`station`, `progression`, `ui`/`hud`.

## Cross-platform notes

- Per-platform builds run in CI (GitHub Actions: Windows, macOS, Linux runners). Build natively per OS rather than cross-compiling.
- macOS distribution requires code-signing/notarization — tracked as a later TODO, not implemented yet.

## Conventions

- Keep `DESIGN.md` updated with decisions and the current phase.
- Commit in small, logical increments with clear messages.
- Personal/machine-specific settings go in `CLAUDE.local.md` (not committed).
