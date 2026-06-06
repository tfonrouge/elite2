//! Core, app-wide configuration that isn't owned by any single gameplay
//! system: the render clear color today, and the natural home for global
//! settings and the top-level game state machine (menu / flight / docked) as
//! the project grows.

use bevy::prelude::*;

/// Deep-space background color the renderer clears to each frame.
const SPACE_BLACK: Color = Color::srgb(0.01, 0.01, 0.02);

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SPACE_BLACK));
    }
}
