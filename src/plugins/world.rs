//! A placeholder scene so the window shows something recognizable while the
//! real game systems come online.
//!
//! The single lit reference cube proves the 3D render pipeline works
//! end-to-end (mesh + material + lighting + camera). It will be replaced by
//! ships, stations, asteroids, and a starfield from Phase 1 onward.

use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_placeholder_scene);
    }
}

fn spawn_placeholder_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Reference cube at the origin.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // A directional "sun" so the cube is shaded rather than flat.
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
