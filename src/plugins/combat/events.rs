//! Combat events as Bevy 0.18 **Messages** (`#[derive(Message)]` +
//! `MessageWriter`/`MessageReader`). They decouple the combat pipeline now that
//! it has more than one producer/consumer.
//!
//! [`DamageDealt`] is the **single damage event**: every damage source — laser
//! fire and ramming collisions — emits it, and only `apply_damage` consumes it to
//! mutate health and (when a ship dies) emit [`ShipDestroyed`]. Keeping one
//! chokepoint means a target killed by two sources in one frame is reported once.

use bevy::prelude::*;

/// Which side of a ship a hit landed on, selecting the fore or aft shield
/// (`GAMEPLAY.md` §11). Determined from where the hit came from relative to the
/// target's facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hemisphere {
    Fore,
    Aft,
}

impl Hemisphere {
    /// The hemisphere struck, given the target's `forward` and the direction from
    /// the target *toward the source* of the hit: fore if the source is ahead of
    /// the nose, else aft.
    pub(super) fn facing(forward: Vec3, toward_source: Vec3) -> Self {
        if forward.dot(toward_source) > 0.0 {
            Self::Fore
        } else {
            Self::Aft
        }
    }
}

/// `target` took `damage` on `hemisphere`, from any source (laser or collision).
/// Consumed only by `apply_damage` — the single owner of health changes.
#[derive(Message, Debug, Clone)]
pub struct DamageDealt {
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
