//! Sphere-sphere ship collision — ramming (DL-014).
//!
//! Physical contact with no physics crate: each frame a linear scan compares
//! every pair of ships by `distance_squared ≤ (ra + rb)²` (no `sqrt`) and, for an
//! overlapping pair, deals **time-scaled** ramming damage to *both* ships. The
//! damage is routed through the shared [`DamageDealt`] event, so `apply_damage`
//! stays the single owner of health changes and destruction (preserving the
//! same-frame double-kill guard). O(n²) is fine at Phase-2 ship counts; a spatial
//! grid would only matter at scale (`GAMEPLAY.md` §11, DL-014).

use bevy::prelude::*;

use super::components::CollisionRadius;
use super::events::{DamageDealt, Hemisphere};

/// Ramming damage per second of contact (time-scaled for frame-rate independence).
const COLLISION_DPS: f32 = 120.0;

/// Damage every pair of overlapping ships this frame.
pub(super) fn detect_collisions(
    time: Res<Time>,
    ships: Query<(Entity, &Transform, &CollisionRadius)>,
    mut damage: MessageWriter<DamageDealt>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    // Snapshot the handful of ships, then compare every unordered pair.
    let ships: Vec<(Entity, Vec3, Vec3, f32)> = ships
        .iter()
        .map(|(entity, tf, radius)| (entity, tf.translation, tf.forward().as_vec3(), radius.0))
        .collect();

    for i in 0..ships.len() {
        for j in (i + 1)..ships.len() {
            let (entity_a, pos_a, forward_a, radius_a) = ships[i];
            let (entity_b, pos_b, forward_b, radius_b) = ships[j];

            let reach = radius_a + radius_b;
            if pos_a.distance_squared(pos_b) > reach * reach {
                continue;
            }

            let hit = COLLISION_DPS * dt;
            damage.write(DamageDealt {
                target: entity_a,
                damage: hit,
                hemisphere: Hemisphere::facing(forward_a, pos_b - pos_a),
            });
            damage.write(DamageDealt {
                target: entity_b,
                damage: hit,
                hemisphere: Hemisphere::facing(forward_b, pos_a - pos_b),
            });
        }
    }
}
