//! The reference environment: lighting plus a few placeholder objects (a ring
//! station and some asteroids) so the player's motion through 3D space is
//! legible. These are stand-ins — real stations, asteroids, and ships arrive in
//! later phases.

use bevy::prelude::*;

/// Slowly rotates an entity about a world axis, to give static objects life and
/// make motion easier to read.
#[derive(Component)]
struct Spin {
    /// Radians per second.
    speed: f32,
    axis: Dir3,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_lights, spawn_reference_objects))
            .add_systems(Update, spin_objects);
    }
}

fn spawn_lights(mut commands: Commands) {
    // Key light — the local "sun".
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(6.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Dim fill from the opposite side so shadowed faces aren't pure black.
    // (Two directional lights instead of an ambient term — simple and cheap.)
    commands.spawn((
        DirectionalLight {
            illuminance: 1800.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-8.0, -3.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_reference_objects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // --- Ring station, dead ahead of the player's start ---------------------
    let hull = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.57, 0.62),
        perceptual_roughness: 0.6,
        metallic: 0.4,
        ..default()
    });
    commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, -300.0),
            Visibility::default(),
            Spin {
                speed: 0.15,
                axis: Dir3::Z,
            },
        ))
        .with_children(|station| {
            // Outer ring.
            station.spawn((
                Mesh3d(meshes.add(Torus {
                    minor_radius: 6.0,
                    major_radius: 34.0,
                })),
                MeshMaterial3d(hull.clone()),
                Transform::default(),
            ));
            // Central hub.
            station.spawn((
                Mesh3d(meshes.add(Cuboid::new(10.0, 10.0, 18.0))),
                MeshMaterial3d(hull.clone()),
                Transform::default(),
            ));
        });

    // --- A scattering of asteroids ------------------------------------------
    let rock = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.28, 0.24),
        perceptual_roughness: 0.95,
        ..default()
    });
    let asteroids = [
        (Vec3::new(-60.0, 18.0, -120.0), 14.0, 0.20),
        (Vec3::new(85.0, -28.0, -210.0), 22.0, -0.12),
        (Vec3::new(36.0, 40.0, -260.0), 9.0, 0.30),
    ];
    for (position, radius, spin) in asteroids {
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(radius).mesh().ico(2).unwrap())),
            MeshMaterial3d(rock.clone()),
            Transform::from_translation(position),
            Spin {
                speed: spin,
                axis: Dir3::Y,
            },
        ));
    }
}

fn spin_objects(time: Res<Time>, mut query: Query<(&mut Transform, &Spin)>) {
    let dt = time.delta_secs();
    for (mut transform, spin) in &mut query {
        transform.rotate_axis(spin.axis, spin.speed * dt);
    }
}
