//! Shield and energy regeneration.
//!
//! Faithful to `GAMEPLAY.md` §11: the energy banks recharge steadily, and the
//! shields top back up *from* energy only while energy is above half. The
//! original ties this to a per-tick cadence; we express it as a time-based rate
//! scaled by `dt` for frame-rate independence (`GAMEPLAY.md` §4).

use bevy::prelude::*;

use super::components::{Energy, Shields};

/// Energy regained per second.
const ENERGY_RECHARGE: f32 = 4.0;
/// Shield strength regained per second, per bank, while energy is over half.
const SHIELD_RECHARGE: f32 = 8.0;
/// Shields only recharge while energy is above this fraction of its maximum.
const SHIELD_RECHARGE_ENERGY_FRACTION: f32 = 0.5;

/// Recharge every ship's energy banks, and its shields when energy is high enough.
pub(super) fn recharge(time: Res<Time>, mut ships: Query<(&mut Shields, &mut Energy)>) {
    let dt = time.delta_secs();
    for (mut shields, mut energy) in &mut ships {
        energy.current = (energy.current + ENERGY_RECHARGE * dt).min(energy.max);

        if energy.current > energy.max * SHIELD_RECHARGE_ENERGY_FRACTION {
            let step = SHIELD_RECHARGE * dt;
            shields.fore = (shields.fore + step).min(shields.max);
            shields.aft = (shields.aft + step).min(shields.max);
        }
    }
}
