//! The player's view: a first-person **cockpit** camera (chosen in DESIGN.md).
//!
//! The camera is spawned as a *child* of the player's ship, so Bevy's transform
//! propagation keeps it locked to the cockpit automatically — no per-frame sync
//! system needed. It mounts in `PostStartup` so the ship (spawned by
//! [`super::flight`] in `Startup`) is guaranteed to exist.

use bevy::core_pipeline::Skybox;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Hdr;

use crate::plugins::flight::Player;
use crate::plugins::skybox::placeholder_cubemap;

/// Marker for the cockpit camera.
#[derive(Component)]
pub struct CockpitCamera;

/// Skybox sample brightness (cd/m²). The placeholder is a dark image with bright
/// star pixels, so this lifts the stars into bloom while space stays black.
/// Tunable — the right value is a judgement made at the screen.
const SKYBOX_BRIGHTNESS: f32 = 600.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, mount_cockpit_camera);
    }
}

fn mount_cockpit_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    player: Single<Entity, With<Player>>,
) {
    commands.spawn((
        CockpitCamera,
        Camera3d::default(),
        // HDR + bloom so bright stars (and the band) glow against black space.
        Hdr,
        Bloom::NATURAL,
        // The deep-space background (DL-018): a cubemap at infinity, so rotation
        // reveals the sky and translation shows no parallax.
        Skybox {
            image: placeholder_cubemap(&mut images),
            brightness: SKYBOX_BRIGHTNESS,
            rotation: Quat::IDENTITY,
        },
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
