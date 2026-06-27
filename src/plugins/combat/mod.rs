//! `CombatPlugin` — weapons, damage, enemy spawning (Phase 2).
//!
//! Faithful to the 1984 original (`GAMEPLAY.md` §10–§13): **instant-hit lasers**
//! (DL-012), shields → energy damage with **no separate hull**, and a **debug
//! command-spawn plus an in-system spawn hook** (DL-017). Still to come in this
//! phase: homing missiles + ECM (DL-013), sphere-sphere ship collision (DL-014),
//! and the full faithful enemy tactics routine (DL-015) with a hybrid evade exit
//! (DL-016).
//!
//! This increment is a thin runnable vertical: debug-spawn a pirate, fire an
//! instant-hit laser at it, watch its shields then energy drain to destruction,
//! with shields/energy recharging over time. The events-driven split of the
//! pipeline (Messages between stages) lands when there's more than one producer
//! or consumer to decouple.

// `ai`, `components`, and `events` are public so tests (and, from Phase 3,
// sibling plugins) can name their types via `combat::<mod>::Foo`. A flatter
// `combat::Foo` re-export waits until a non-test consumer exists, to stay
// `unused_imports`-clean.
pub mod ai;
pub mod components;
pub mod events;
mod spawn;
mod survival;
mod weapons;

use bevy::prelude::*;

use crate::plugins::flight::FlightSet;
use events::{ProjectileHit, ShipDestroyed};

/// Deterministic ordering for the combat pipeline, all within `Update`, and as a
/// whole **after** [`FlightSet::Integrate`](crate::plugins::flight::FlightSet) so
/// weapons/AI read the current frame's attitude. Stages run in order:
///
/// `Spawn → Fire → Projectiles → Collide → Damage → Death → Cleanup`
///
/// `Projectiles` and `Collide` have no systems yet — they are the slots the
/// missile and ship-collision systems attach to in later steps.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatSet {
    /// Introduce ships into the world (debug command-spawn; later in-system).
    Spawn,
    /// Resolve weapon fire — instant-hit laser rays (and later missile launches).
    Fire,
    /// Advance projectiles — missile homing + lifetime (later).
    Projectiles,
    /// Detect contacts — sphere-sphere ship/projectile collision (later).
    Collide,
    /// Apply hits to shields then energy (the `apply_damage` system).
    Damage,
    /// Handle destruction — despawn + bounty (the `handle_death` system).
    Death,
    /// Per-frame regeneration and transient-state reset.
    Cleanup,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ProjectileHit>()
            .add_message::<ShipDestroyed>()
            .configure_sets(
                Update,
                (
                    CombatSet::Spawn,
                    CombatSet::Fire,
                    CombatSet::Projectiles,
                    CombatSet::Collide,
                    CombatSet::Damage,
                    CombatSet::Death,
                    CombatSet::Cleanup,
                )
                    .chain()
                    .after(FlightSet::Integrate),
            )
            .add_systems(PostStartup, spawn::init_player_combat)
            // The enemy AI writes each enemy's FlightInput, so it joins the flight
            // ReadInput set (before the integrator), per that set's contract.
            .add_systems(Update, ai::enemy_ai.in_set(FlightSet::ReadInput))
            .add_systems(
                Update,
                (
                    spawn::debug_spawn_enemy.in_set(CombatSet::Spawn),
                    // Set the player's fire intent, then resolve all shots.
                    (weapons::player_fire_intent, weapons::fire_weapons)
                        .chain()
                        .in_set(CombatSet::Fire),
                    weapons::apply_damage.in_set(CombatSet::Damage),
                    weapons::handle_death.in_set(CombatSet::Death),
                    survival::recharge.in_set(CombatSet::Cleanup),
                ),
            );
    }
}
