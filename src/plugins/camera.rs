//! The player's view: a first-person **cockpit** camera (chosen in DESIGN.md).
//!
//! The camera is spawned as a *child* of the player's ship, so Bevy's transform
//! propagation keeps it locked to the cockpit automatically — no per-frame sync
//! system needed. It mounts in `PostStartup` so the ship (spawned by
//! [`super::flight`] in `Startup`) is guaranteed to exist.

use bevy::prelude::*;

use crate::plugins::flight::Player;

/// Marker for the cockpit camera.
#[derive(Component)]
pub struct CockpitCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, mount_cockpit_camera);
    }
}

fn mount_cockpit_camera(mut commands: Commands, player: Single<Entity, With<Player>>) {
    commands.spawn((
        CockpitCamera,
        Camera3d::default(),
        // Wider-than-default vertical FOV for a more immersive cockpit feel.
        Projection::Perspective(PerspectiveProjection {
            fov: 75.0_f32.to_radians(),
            ..default()
        }),
        // Eye position, in ship-local space (slightly above the ship origin).
        Transform::from_xyz(0.0, 0.5, 0.0),
        ChildOf(*player),
    ));
}
