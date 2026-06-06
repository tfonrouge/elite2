//! The player's view into the world.
//!
//! For now this is a single fixed 3D camera pointed at the origin so the
//! placeholder scene is visible. In the flight slice (Phase 1) this plugin
//! grows into a proper cockpit/chase camera that follows the player's ship.

use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
