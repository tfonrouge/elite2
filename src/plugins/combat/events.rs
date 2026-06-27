//! Combat events as Bevy 0.18 **Messages** (`#[derive(Message)]` +
//! `MessageWriter`/`MessageReader`). They decouple the combat pipeline now that
//! it has more than one producer/consumer: `fire_weapons` (player *and* enemy)
//! emits [`ProjectileHit`]; `apply_damage` resolves it and emits
//! [`ShipDestroyed`]; `handle_death` reacts.

use bevy::prelude::*;

/// Which side of a ship a hit landed on, selecting the fore or aft shield
/// (`GAMEPLAY.md` §11). Determined from where the shot came from relative to the
/// target's facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hemisphere {
    Fore,
    Aft,
}

/// A shot struck `target` for `damage`, on the given `hemisphere`. Emitted by the
/// weapon system, consumed by the damage system.
#[derive(Message, Debug, Clone)]
pub struct ProjectileHit {
    pub target: Entity,
    pub damage: f32,
    pub hemisphere: Hemisphere,
}

/// A ship's energy reached zero. Carries its `bounty` for the credit/rank award.
/// The death system despawns it — *except the player*, whose loss is an escape
/// pod / game-over handled in a later phase.
#[derive(Message, Debug, Clone)]
pub struct ShipDestroyed {
    pub ship: Entity,
    pub bounty: f32,
}
