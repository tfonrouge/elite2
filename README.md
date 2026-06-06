# Elite (Rust + Bevy remake)

A cross-platform, single-player remake of the classic 1984 space trading and
combat game **Elite**, built in [Rust](https://www.rust-lang.org/) with the
[Bevy](https://bevyengine.org/) game engine.

Open-ended sandbox: pilot a ship through a procedurally generated universe and
trade, fight, mine, or explore. Modern 3D rendering, faithful to the original's
systems — procedural galaxy, supply/demand economy, real-time combat, ship
progression, and combat/legal ranks. No forced story.

> **Status:** Phase 0 (foundations) complete — the app opens a window and
> renders a placeholder 3D scene. See [`DESIGN.md`](DESIGN.md) for the roadmap
> and current phase.

## Prerequisites

- **Rust** (latest stable) via [rustup](https://rustup.rs/). The toolchain is
  pinned to `stable` in [`rust-toolchain.toml`](rust-toolchain.toml), so rustup
  installs the right components automatically.
- A C toolchain for the linker:
  - **macOS** — Xcode Command Line Tools: `xcode-select --install`
  - **Windows** — [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC).
  - **Linux** — a C compiler plus Bevy's system dependencies (below).

### Linux system dependencies

Bevy needs ALSA and udev development headers to build:

```sh
# Debian / Ubuntu
sudo apt-get install --no-install-recommends libasound2-dev libudev-dev
```

(Equivalent packages exist for Fedora/Arch — `alsa-lib-devel`, `systemd-devel`,
etc.) See the [Bevy Linux dependencies guide](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

## Build & run

```sh
cargo run            # build + launch the game (debug)
cargo build          # build only
cargo build --release
```

A window titled **"Elite"** opens showing a lit reference cube against a
deep-space background. Close the window to exit.

### Faster iteration during development

Bevy compiles a large dependency tree. The `dev` feature enables Bevy's dynamic
linking for much faster incremental rebuilds (not for distribution):

```sh
cargo run --features dev
```

The first build is slow (it compiles all of Bevy); subsequent builds are fast.

## Quality checks

The tree is kept warning-free and formatted:

```sh
cargo fmt --all                       # format
cargo fmt --all -- --check            # verify formatting (CI)
cargo clippy --all-targets -- -D warnings   # lints, warnings as errors (CI)
cargo test
```

## Project layout

```
src/
├── main.rs            # thin entry point: window config + the list of plugins
└── plugins/           # one Bevy Plugin per game system
    ├── mod.rs
    ├── core.rs        # CorePlugin   — app-wide config (clear color, ...)
    ├── camera.rs      # CameraPlugin — the 3D camera
    └── world.rs       # WorldPlugin  — placeholder scene (replaced in Phase 1)
assets/                # runtime assets loaded by Bevy's AssetServer
.github/workflows/     # cross-platform CI (Windows / macOS / Linux)
```

Architecture: components hold data, systems hold behavior, resources hold global
state. Each major system is its own plugin; `main` is just a list of plugins.

## Platforms

Windows, macOS, and Linux are first-class desktop targets, built natively per OS
in CI. A WebAssembly path is kept in mind but not actively developed.

> macOS release binaries from CI are **unsigned**. Code-signing and notarization
> for distribution are tracked as a later TODO.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option.

This is an independent, fan-made reimplementation inspired by the original
*Elite* (David Braben & Ian Bell, 1984). It contains no original Elite assets or
code and is not affiliated with or endorsed by the rights holders.
