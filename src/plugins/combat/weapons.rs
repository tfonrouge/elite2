//! Weapons, damage, and death — the two-sided combat pipeline.
//!
//! Faithful to the original (`GAMEPLAY.md` §10–§11): a laser is an **instant
//! hit** (analytic ray-vs-sphere, DL-012), not a travelling projectile. A shot
//! drains the struck **hemisphere's** shield, then the **energy banks**, with no
//! separate hull. Firing is **faction-based**: a ship's forward ray hits the
//! nearest ship of a *different* faction — so the player hits enemies and enemies
//! hit the player, from one system.
//!
//! Pipeline (within `Update`, ordered by [`CombatSet`](super::CombatSet)):
//! `fire_weapons` (Fire) → [`DamageDealt`] → `apply_damage` (Damage) →
//! [`ShipDestroyed`] → `handle_death` (Death). Collisions feed the same
//! [`DamageDealt`] → `apply_damage` chokepoint (see `super::collision`).

use std::time::Duration;

use bevy::math::Ray3d;
use bevy::math::bounding::{BoundingSphere, RayCast3d};
use bevy::prelude::*;

use crate::plugins::flight::Player;

use super::components::{Bounty, CollisionRadius, Energy, Faction, Shields};
use super::events::{DamageDealt, Hemisphere, ShipDestroyed};

/// Key that fires the player's forward laser (held to fire repeatedly).
const FIRE_KEY: KeyCode = KeyCode::Space;

/// A forward-firing laser carried by an armed ship (player or enemy).
#[derive(Component, Debug)]
pub(super) struct Weapon {
    /// Refire timer — a shot is only possible once it has elapsed.
    cooldown: Timer,
    /// Damage per shot.
    damage: f32,
    /// Maximum reach in world units.
    range: f32,
    /// Set by the ship's controller each frame: does it want to fire now?
    wants_fire: bool,
}

impl Weapon {
    fn new(damage: f32, range: f32, refire_secs: f32) -> Self {
        let mut cooldown = Timer::from_seconds(refire_secs, TimerMode::Once);
        // Start ready: the first trigger fires immediately (as the original does);
        // only subsequent shots wait the cooldown.
        cooldown.tick(Duration::from_secs_f32(refire_secs));
        Self {
            cooldown,
            damage,
            range,
            wants_fire: false,
        }
    }

    /// The player's starting pulse laser.
    pub(super) fn pulse() -> Self {
        Self::new(50.0, 2000.0, 0.25)
    }

    /// A pirate's weaker laser.
    pub(super) fn pirate() -> Self {
        Self::new(8.0, 1500.0, 0.5)
    }

    /// Set by a controller (AI / player input) to request fire this frame.
    pub(super) fn set_firing(&mut self, firing: bool) {
        self.wants_fire = firing;
    }
}

/// Set the player's fire intent from the keyboard (held = firing).
pub(super) fn player_fire_intent(
    keys: Res<ButtonInput<KeyCode>>,
    mut weapon: Single<&mut Weapon, With<Player>>,
) {
    weapon.wants_fire = keys.pressed(FIRE_KEY);
}

/// Resolve every armed ship's fire intent: cast a forward ray and, if it strikes
/// the nearest ship of a *different* faction within range, emit a hit. One
/// system serves the player and all enemies.
pub(super) fn fire_weapons(
    time: Res<Time>,
    mut shooters: Query<(Entity, &Transform, &Faction, &mut Weapon)>,
    targets: Query<(Entity, &Transform, &Faction, &CollisionRadius)>,
    mut hits: MessageWriter<DamageDealt>,
) {
    let dt = Duration::from_secs_f32(time.delta_secs());
    for (shooter, shooter_tf, shooter_faction, mut weapon) in &mut shooters {
        weapon.cooldown.tick(dt);
        if !weapon.wants_fire || !weapon.cooldown.is_finished() {
            continue;
        }

        let ray = RayCast3d::from_ray(
            Ray3d::new(shooter_tf.translation, shooter_tf.forward()),
            weapon.range,
        );

        // Nearest different-faction ship the ray crosses.
        let mut nearest: Option<(Entity, f32, Vec3, Vec3)> = None;
        for (target, target_tf, target_faction, radius) in &targets {
            if target == shooter || target_faction == shooter_faction {
                continue;
            }
            let sphere = BoundingSphere::new(target_tf.translation, radius.0);
            if let Some(toi) = ray.sphere_intersection_at(&sphere)
                && nearest.is_none_or(|(_, best, ..)| toi < best)
            {
                nearest = Some((
                    target,
                    toi,
                    target_tf.translation,
                    target_tf.forward().as_vec3(),
                ));
            }
        }

        if let Some((target, _, target_pos, target_forward)) = nearest {
            let hemisphere =
                Hemisphere::facing(target_forward, shooter_tf.translation - target_pos);
            hits.write(DamageDealt {
                target,
                damage: weapon.damage,
                hemisphere,
            });
            weapon.cooldown.reset();
        }
    }
}

/// Apply each hit: drain the struck shield, then the energy banks. A ship whose
/// energy reaches zero is reported destroyed.
pub(super) fn apply_damage(
    mut hits: MessageReader<DamageDealt>,
    mut ships: Query<(&mut Shields, &mut Energy, Option<&Bounty>)>,
    mut destroyed: MessageWriter<ShipDestroyed>,
) {
    for hit in hits.read() {
        if let Ok((mut shields, mut energy, bounty)) = ships.get_mut(hit.target)
            && apply_hit(&mut shields, &mut energy, hit.damage, hit.hemisphere)
        {
            destroyed.write(ShipDestroyed {
                ship: hit.target,
                bounty: bounty.map_or(0.0, |b| b.0),
            });
        }
    }
}

/// Subtract `dmg` from the hit hemisphere's shield, then the energy banks
/// (`GAMEPLAY.md` §11). Returns `true` if energy hits zero (no separate hull).
fn apply_hit(shields: &mut Shields, energy: &mut Energy, dmg: f32, hemisphere: Hemisphere) -> bool {
    if energy.current <= 0.0 {
        // Already destroyed (e.g. an earlier hit this frame) — absorb nothing and
        // don't re-report, so a second lethal hit can't double-award the bounty.
        return false;
    }
    let bank = match hemisphere {
        Hemisphere::Fore => &mut shields.fore,
        Hemisphere::Aft => &mut shields.aft,
    };
    let absorbed = dmg.min(*bank);
    *bank -= absorbed;
    energy.current -= dmg - absorbed;
    energy.current <= 0.0
}

/// React to destruction: despawn the ship and log its bounty — **except the
/// player**, whose loss becomes an escape pod / game over in a later phase.
pub(super) fn handle_death(
    mut destroyed: MessageReader<ShipDestroyed>,
    players: Query<(), With<Player>>,
    mut commands: Commands,
) {
    for event in destroyed.read() {
        if players.get(event.ship).is_ok() {
            info!("player ship destroyed — escape pod / game-over deferred to a later phase");
        } else {
            info!("enemy destroyed — bounty {:.1} Cr", event.bounty);
            commands.entity(event.ship).try_despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fore_hit_drains_the_fore_shield_not_the_aft() {
        let mut shields = Shields::full(64.0);
        let mut energy = Energy::full(150.0);
        let destroyed = apply_hit(&mut shields, &mut energy, 10.0, Hemisphere::Fore);
        assert!(!destroyed);
        assert_eq!(shields.fore, 54.0, "fore shield absorbed the hit");
        assert_eq!(shields.aft, 64.0, "aft shield untouched");
        assert_eq!(
            energy.current, 150.0,
            "energy untouched while a shield holds"
        );
    }

    #[test]
    fn overflow_past_a_shield_drains_energy_and_can_destroy() {
        let mut shields = Shields::full(10.0);
        let mut energy = Energy::full(20.0);
        // 10 into the aft shield, 20 past it into energy → energy 0 → destroyed.
        let destroyed = apply_hit(&mut shields, &mut energy, 30.0, Hemisphere::Aft);
        assert_eq!(shields.aft, 0.0);
        assert!(energy.current <= 0.0);
        assert!(destroyed, "zero energy destroys the ship");
    }

    #[test]
    fn a_dead_ship_absorbs_no_further_hits() {
        // Guards against two lethal hits in one frame double-reporting a kill
        // (and, later, double-awarding the bounty).
        let mut shields = Shields::full(0.0);
        let mut energy = Energy::full(10.0);
        assert!(
            apply_hit(&mut shields, &mut energy, 20.0, Hemisphere::Fore),
            "the first lethal hit reports destroyed"
        );
        assert!(
            !apply_hit(&mut shields, &mut energy, 20.0, Hemisphere::Fore),
            "a second hit on a dead ship does not re-report"
        );
    }
}
