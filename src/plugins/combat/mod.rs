//! `CombatPlugin` — weapons, damage, enemy AI, collision, and spawning (Phase 2).
//!
//! Faithful to the 1984 original (`GAMEPLAY.md` §10–§13): **instant-hit lasers**
//! (DL-012), **two-sided faction-based fire**, **hemisphere → shields → energy**
//! damage with no separate hull, **sphere-sphere ship collision** (DL-014), the
//! full **no-yaw enemy tactics** (DL-015) with a **hybrid evade** (DL-016), and a
//! **debug command-spawn + in-system hook** (DL-017). Still to come this phase:
//! homing **missiles + ECM** (DL-013), the ~10% escape-pod bail, and the combat
//! HUD readout.
//!
//! All damage flows through a single chokepoint: every source emits the shared
//! [`events::DamageDealt`] message, `apply_damage` is the only system that mutates
//! health and emits [`events::ShipDestroyed`], and `handle_death` is the only
//! despawner — so a ship killed by two sources in one frame is reported once.

// `ai`, `components`, and `events` are public so tests (and, from Phase 3,
// sibling plugins) can name their types via `combat::<mod>::Foo`. A flatter
// `combat::Foo` re-export waits until a non-test consumer exists, to stay
// `unused_imports`-clean.
pub mod ai;
mod collision;
pub mod components;
pub mod events;
pub mod readout;
mod spawn;
mod survival;
mod weapons;

use bevy::prelude::*;

use crate::plugins::flight::FlightSet;
use events::{DamageDealt, ShipDestroyed};

/// Deterministic ordering for the combat pipeline, all within `Update`, and as a
/// whole **after** [`FlightSet::Integrate`](crate::plugins::flight::FlightSet) so
/// weapons/AI read the current frame's attitude. Stages run in order:
///
/// `Spawn → Fire → Projectiles → Collide → Damage → Death → Cleanup`
///
/// `Projectiles` has no systems yet — the slot the missile homing/lifetime
/// systems attach to (DL-013).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatSet {
    /// Introduce ships into the world (debug command-spawn; later in-system).
    Spawn,
    /// Resolve weapon fire — instant-hit laser rays (and later missile launches).
    Fire,
    /// Advance projectiles — missile homing + lifetime (later).
    Projectiles,
    /// Detect contacts — sphere-sphere ship collision (the `detect_collisions`
    /// system); projectile collision joins here with missiles.
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
        app.add_message::<DamageDealt>()
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
            .add_systems(Startup, readout::setup_combat_hud)
            // The enemy AI writes each enemy's FlightInput, so it joins the flight
            // ReadInput set (before the integrator), per that set's contract.
            .add_systems(Update, ai::enemy_ai.in_set(FlightSet::ReadInput))
            // The combat readout refreshes after the whole combat pipeline so it
            // reflects this frame's damage.
            .add_systems(Update, readout::update_combat_hud.after(CombatSet::Cleanup))
            .add_systems(
                Update,
                (
                    spawn::debug_spawn_enemy.in_set(CombatSet::Spawn),
                    // Set the player's fire intent, then resolve all shots.
                    (weapons::player_fire_intent, weapons::fire_weapons)
                        .chain()
                        .in_set(CombatSet::Fire),
                    collision::detect_collisions.in_set(CombatSet::Collide),
                    weapons::apply_damage.in_set(CombatSet::Damage),
                    weapons::handle_death.in_set(CombatSet::Death),
                    survival::recharge.in_set(CombatSet::Cleanup),
                ),
            );
    }
}
