//! Weapons — instant-hit lasers (DL-012).
//!
//! Faithful to the original (`GAMEPLAY.md` §10): a laser is an *instant* hit, not
//! a travelling projectile. A shot is an analytic ray-vs-sphere test
//! (`RayCast3d::sphere_intersection_at`) from the muzzle along the ship's forward
//! axis against each target's collision sphere — the 3D analog of the original's
//! squared-radius crosshair test. The nearest enemy struck takes damage, which
//! drains shields (fore then aft) and then the energy banks, with no separate
//! hull (`GAMEPLAY.md` §11).
//!
//! This first cut resolves fire → hit → damage → death inline; it splits into the
//! `Fire`/`Collide`/`Damage`/`Death` stages (passing hits as messages) when
//! missiles and multiple shooters arrive.

use bevy::math::Ray3d;
use bevy::math::bounding::{BoundingSphere, RayCast3d};
use bevy::prelude::*;

use crate::plugins::flight::Player;

use super::components::{Bounty, CollisionRadius, Enemy, Energy, Shields};

/// Key that fires the forward laser.
const FIRE_KEY: KeyCode = KeyCode::Space;
/// How far a laser reaches, in world units.
const LASER_RANGE: f32 = 2000.0;
/// Damage per pulse (placeholder — per-laser-type tuning lands with pulse/beam).
const LASER_DAMAGE: f32 = 50.0;

/// On the fire key, cast an instant-hit ray and damage the nearest enemy struck.
pub(super) fn fire_laser(
    keys: Res<ButtonInput<KeyCode>>,
    player: Single<&Transform, With<Player>>,
    mut commands: Commands,
    mut enemies: Query<
        (
            Entity,
            &Transform,
            &CollisionRadius,
            &mut Shields,
            &mut Energy,
            &Bounty,
        ),
        With<Enemy>,
    >,
) {
    if !keys.just_pressed(FIRE_KEY) {
        return;
    }

    let ray = RayCast3d::from_ray(
        Ray3d::new(player.translation, player.forward()),
        LASER_RANGE,
    );

    // The nearest enemy whose collision sphere the ray crosses.
    let mut nearest: Option<(Entity, f32)> = None;
    for (entity, transform, radius, _, _, _) in &enemies {
        let sphere = BoundingSphere::new(transform.translation, radius.0);
        if let Some(toi) = ray.sphere_intersection_at(&sphere)
            && nearest.is_none_or(|(_, best)| toi < best)
        {
            nearest = Some((entity, toi));
        }
    }

    let Some((hit, _)) = nearest else {
        return;
    };
    if let Ok((_, _, _, mut shields, mut energy, bounty)) = enemies.get_mut(hit)
        && apply_laser_damage(&mut shields, &mut energy, LASER_DAMAGE)
    {
        info!("enemy destroyed — bounty {:.1} Cr", bounty.0);
        commands.entity(hit).despawn();
    }
}

/// Subtract `dmg` from the fore shield, then the aft shield, then the energy
/// banks (`GAMEPLAY.md` §11). Returns `true` if the hit drops energy to zero —
/// the ship is destroyed (there is no separate hull). Hit direction isn't
/// modelled yet — it always drains fore-first; see [`Shields`].
fn apply_laser_damage(shields: &mut Shields, energy: &mut Energy, dmg: f32) -> bool {
    let to_fore = dmg.min(shields.fore);
    shields.fore -= to_fore;
    let to_aft = (dmg - to_fore).min(shields.aft);
    shields.aft -= to_aft;
    energy.current -= dmg - to_fore - to_aft;
    energy.current <= 0.0
}
