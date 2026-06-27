//! Combat data components, shared by the player ship and NPCs.
//!
//! Faithful to the original's defence model (`GAMEPLAY.md` §11): incoming fire
//! drains **shields** first, then the **energy banks**; there is *no separate
//! hull* — reaching zero energy destroys the ship. (The damage/recharge systems
//! that consume these land in a later step; this step defines the data and a
//! debug spawn that constructs it.)

use bevy::prelude::*;

/// Fore and aft shields — the first line of defence, absorbing damage before the
/// [`Energy`] banks and recharging back up to [`Shields::max`].
///
/// Faithful to the original, a frontal hit should drain [`Shields::fore`] and a
/// rear hit [`Shields::aft`]. **Hemisphere selection is not implemented yet:**
/// damage currently drains `fore` then `aft` regardless of direction (a
/// 2×-depth, front-first pool). The hit-hemisphere test lands with enemy weapons
/// (Step 5 / DL-015), when shots arrive from varied directions and the
/// distinction becomes observable.
#[derive(Component, Debug, Clone, Copy)]
pub struct Shields {
    pub fore: f32,
    pub aft: f32,
    /// Per-bank ceiling that shields recharge back up to.
    pub max: f32,
}

impl Shields {
    /// Both shields full at the given per-bank maximum.
    pub fn full(max: f32) -> Self {
        Self {
            fore: max,
            aft: max,
            max,
        }
    }
}

impl Default for Shields {
    fn default() -> Self {
        // Placeholder strength; combat tuning lands with the damage system.
        Self::full(64.0)
    }
}

/// The energy banks — survivability *after* shields. There is no separate hull:
/// when `current` reaches zero the ship is destroyed (`GAMEPLAY.md` §11). Energy
/// also recharges over time and tops the shields back up.
#[derive(Component, Debug, Clone, Copy)]
pub struct Energy {
    pub current: f32,
    pub max: f32,
}

impl Energy {
    /// A full bank at the given maximum.
    pub fn full(max: f32) -> Self {
        Self { current: max, max }
    }
}

impl Default for Energy {
    fn default() -> Self {
        // Cobra Mk III max energy = 150 (GAMEPLAY.md §16).
        Self::full(150.0)
    }
}

/// Collision / targetable radius in world units. Feeds both the instant-hit
/// laser test (DL-012) and sphere-sphere collision (DL-014).
#[derive(Component, Debug, Clone, Copy)]
pub struct CollisionRadius(pub f32);

/// Allegiance — drives AI target selection now and legal status later. AI treats
/// a ship of a *different* faction as a potential target. More factions
/// (`Police`, `Thargoid`) are added when the NPCs that need them arrive.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Faction {
    Player,
    Pirate,
}

/// Credit bounty paid to the player for destroying this ship (`GAMEPLAY.md` §12).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Bounty(pub f32);

/// Marker: a hostile NPC ship (carries the flight + combat component set and,
/// once the AI lands, is driven by the enemy tactics controller).
#[derive(Component, Debug)]
pub struct Enemy;
