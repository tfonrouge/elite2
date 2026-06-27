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

// `ai` and `components` are public so tests (and, from Phase 3, sibling plugins)
// can name their types via `combat::ai::Foo` / `combat::components::Foo`. A
// flatter `combat::Foo` re-export waits until a non-test consumer exists, to stay
// `unused_imports`-clean.
pub mod ai;
pub mod components;
mod spawn;
mod survival;
mod weapons;

use bevy::prelude::*;

use crate::plugins::flight::FlightSet;

/// Deterministic ordering for the combat pipeline, all within `Update`, and as a
/// whole **after** [`FlightSet::Integrate`](crate::plugins::flight::FlightSet) so
/// weapons/AI read the current frame's attitude. Stages run in order:
///
/// `Spawn → Fire → Projectiles → Collide → Damage → Death → Cleanup`
///
/// Several stages have no systems yet — they are the slots the missile, collision,
/// and AI systems attach to in later steps.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatSet {
    /// Introduce ships into the world (debug command-spawn; later in-system).
    Spawn,
    /// Resolve weapon fire — instant-hit laser rays and (later) missile launches.
    Fire,
    /// Advance projectiles — missile homing + lifetime (later).
    Projectiles,
    /// Detect contacts — sphere-sphere ship/projectile collision (later).
    Collide,
    /// Apply damage to shields then energy (currently inline in `Fire`).
    Damage,
    /// Handle destruction — despawn, bounty, cargo drops.
    Death,
    /// Per-frame regeneration and transient-state reset.
    Cleanup,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
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
                weapons::fire_laser.in_set(CombatSet::Fire),
                survival::recharge.in_set(CombatSet::Cleanup),
            ),
        );
    }
}
