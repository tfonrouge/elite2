//! A procedural starfield surrounding the player.
//!
//! Stars are placed on a large sphere shell using a deterministic Fibonacci
//! lattice (even coverage, no external RNG). All stars share one mesh and one
//! unlit material, which is batch-friendly — it lets the renderer draw them in
//! far fewer calls than one-per-star (the exact batching is up to Bevy).
//!
//! The shell is parented to a root that tracks the player's *position but not
//! rotation* every frame. The effect: stars stay infinitely far (no translation
//! parallax, like real distant stars) yet hold a fixed world orientation, so the
//! ship's rotation reads clearly against them.

use bevy::prelude::*;

use crate::plugins::flight::{FlightSet, Player};

/// Root the stars hang off of; follows the player's translation each frame.
/// `pub(crate)` so the wiring smoke test can assert exactly one exists.
#[derive(Component)]
pub(crate) struct StarfieldRoot;

const STAR_COUNT: usize = 1400;
const SHELL_RADIUS: f32 = 700.0;
const STAR_RADIUS: f32 = 1.6;

pub struct StarfieldPlugin;

impl Plugin for StarfieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_starfield)
            // Read the player's translation *after* this frame's flight update,
            // so the field is centered on the ship's current position.
            .add_systems(Update, follow_player.after(FlightSet::Integrate));
    }
}

fn spawn_starfield(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // One shared low-poly star mesh + one shared unlit material (cheap Arc
    // clones below → automatic GPU batching across all stars).
    let star_mesh = meshes.add(Sphere::new(STAR_RADIUS).mesh().ico(1).unwrap());
    let star_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 1.0),
        unlit: true,
        ..default()
    });

    let root = commands
        .spawn((StarfieldRoot, Transform::default(), Visibility::default()))
        .id();

    for i in 0..STAR_COUNT {
        let position = fibonacci_sphere(i, STAR_COUNT) * SHELL_RADIUS;
        // Vary apparent size/brightness a little so the field isn't uniform.
        let scale = 0.55 + 0.9 * hash01(i as u32);
        commands.spawn((
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_material.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(scale)),
            ChildOf(root),
        ));
    }
}

/// Keep the starfield centered on the player (position only — never rotation).
fn follow_player(
    player: Single<&Transform, (With<Player>, Without<StarfieldRoot>)>,
    mut root: Single<&mut Transform, (With<StarfieldRoot>, Without<Player>)>,
) {
    root.translation = player.translation;
}

/// `i`-th of `n` points evenly distributed on the unit sphere (Fibonacci
/// lattice). Deterministic, so the same star pattern appears every run.
///
/// Precondition: `n >= 2` (with `n == 1` the `n - 1` divisor is zero).
fn fibonacci_sphere(i: usize, n: usize) -> Vec3 {
    debug_assert!(n >= 2, "fibonacci_sphere requires n >= 2, got {n}");
    // Golden angle in radians.
    let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - (i as f32 / (n as f32 - 1.0)) * 2.0; // 1.0 → -1.0
    let ring_radius = (1.0 - y * y).max(0.0).sqrt();
    let theta = golden_angle * i as f32;
    Vec3::new(theta.cos() * ring_radius, y, theta.sin() * ring_radius)
}

/// Deterministic integer hash → `[0.0, 1.0)`. Used only for cosmetic variety.
fn hash01(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::{STAR_COUNT, fibonacci_sphere, hash01};

    #[test]
    fn stars_lie_on_the_unit_sphere() {
        for i in [0, 1, STAR_COUNT / 2, STAR_COUNT - 1] {
            let v = fibonacci_sphere(i, STAR_COUNT);
            assert!(
                (v.length() - 1.0).abs() < 1e-3,
                "star {i} off the unit sphere: len = {}",
                v.length()
            );
        }
    }

    #[test]
    fn hash01_stays_in_unit_interval() {
        for i in 0..2000u32 {
            let h = hash01(i);
            assert!((0.0..1.0).contains(&h), "hash01({i}) = {h} out of range");
        }
    }
}
