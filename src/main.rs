//! Elite — a cross-platform, single-player remake of the 1984 space
//! trading/combat game, built in Rust with the Bevy engine.
//!
//! `main` is intentionally thin: it boots the Bevy engine with our window
//! configuration and then registers one plugin per game system. All gameplay
//! behavior lives inside those plugins (see `src/plugins/`), keeping the app
//! decoupled and the entry point readable as a table of contents.

mod plugins;

use bevy::prelude::*;

use plugins::{camera::CameraPlugin, core::CorePlugin, world::WorldPlugin};

fn main() {
    App::new()
        // Bevy's batteries-included plugins (windowing, rendering, input,
        // audio, asset loading, ...), with our primary window customized.
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Elite".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        // One plugin per game system. As the game grows (flight, combat,
        // galaxy, economy, ...), each new system is added as a line here.
        .add_plugins((CorePlugin, CameraPlugin, WorldPlugin))
        .run();
}
