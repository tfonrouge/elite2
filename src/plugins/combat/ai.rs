//! Enemy AI — the faithful no-yaw tactics controller (DL-015 flight half, DL-016
//! evade). This step is **flight only**: an enemy approaches, aims at, and evades
//! the player by writing its [`FlightInput`]. Firing, the ~10% escape-pod bail,
//! and ECM hook into the `Attack` state and the evade transition in the next
//! increment.
//!
//! **No-yaw aiming** (DL-011, `GAMEPLAY.md` §9/§12): the Cobra has only pitch and
//! roll, so the controller turns the nose toward a target by *rolling the target
//! into the pitch plane, then pitching onto it* — the original's TACTICS approach.
//! It never commands yaw (the `.y` of [`FlightInput::rotation`] stays zero).
//!
//! The steering sign conventions are derived from — and locked by — the flight
//! tests in `wiring.rs`: `rotate_local_x(+)` = nose up, `rotate_local_z(+)` = roll
//! left, nose = local −Z. (An earlier draft had the roll sign inverted; it is
//! `(-l.x).atan2(l.y)`, pinned by the unit tests below.)

use std::time::Duration;

use bevy::prelude::*;

use crate::plugins::flight::{FlightInput, Player, Ship};

use super::components::{Enemy, Energy};
use super::weapons::Weapon;

// --- Steering gains -------------------------------------------------------
/// Roll command per radian of roll error (roll is the fastest axis, so aggressive).
const ROLL_GAIN: f32 = 2.0;
/// Pitch command per radian of pitch error (slightly higher; pitch is slower).
const PITCH_GAIN: f32 = 2.5;

// --- Engagement ranges (world units) -------------------------------------
/// The enemy fights inside this range; outside it, it closes the gap.
const ATTACK_RANGE: f32 = 350.0;
/// Fractional band above `ATTACK_RANGE` before falling back to Approach
/// (prevents Attack/Approach chatter at the boundary).
const RANGE_HYSTERESIS: f32 = 0.15;
/// Break off before ramming when this close.
const EVADE_TOO_CLOSE: f32 = 60.0;

// --- Evade (DL-016 hybrid: open a distance OR time out) ------------------
/// Disengage below this fraction of max energy.
const LOW_ENERGY_FRAC: f32 = 0.5;
/// Extra distance an evade leg tries to open beyond where it began running.
const EVADE_OPEN_DISTANCE: f32 = 400.0;
/// Floor for the evade goal when a break-off starts very close.
const EVADE_MIN_GOAL_DIST: f32 = 450.0;
/// Timeout cap so a fast player can't drag an evading enemy away forever.
const EVADE_TIMEOUT: f32 = 6.0;

// --- Throttle -------------------------------------------------------------
/// Speed the enemy holds while attacking (units/s). `FlightInput.throttle` is a
/// *rate*, so we bang-bang toward this rather than commanding a constant value.
const ATTACK_SPEED: f32 = 50.0;

/// Cosine of the firing-cone half-angle: the enemy only fires when its nose is
/// within ~14° of the player (`forward·to_player` above this).
const FIRE_CONE_COS: f32 = 0.97;
/// Deadband around `ATTACK_SPEED` (units/s) so the throttle settles to 0 instead
/// of dithering ±1 each frame at the target speed.
const SPEED_DEADBAND: f32 = 2.0;

/// Per-enemy AI state. `pub(crate)` only so tests can assert transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiState {
    /// Close the distance, nose on the player, full throttle.
    Approach,
    /// In range: hold aim at a moderate speed (firing hooks in here later).
    Attack,
    /// Break off — aim away and burn until a distance goal or the timeout.
    Evade,
}

/// AI controller state carried by each enemy ship.
#[derive(Component, Debug)]
pub struct EnemyAi {
    state: AiState,
    /// Counts down the current evade leg (the DL-016 timeout cap).
    evade_timer: Timer,
    /// Squared distance an evade leg is trying to reach (the DL-016 distance goal).
    evade_goal_sq: f32,
}

impl Default for EnemyAi {
    fn default() -> Self {
        Self {
            state: AiState::Approach,
            evade_timer: Timer::from_seconds(EVADE_TIMEOUT, TimerMode::Once),
            evade_goal_sq: 0.0,
        }
    }
}

#[cfg(test)]
impl EnemyAi {
    /// The current AI state — exposed for transition tests only.
    pub(crate) fn state(&self) -> AiState {
        self.state
    }
}

/// Drive every enemy's [`FlightInput`] from its AI state and the player's
/// position. Runs in [`FlightSet::ReadInput`](crate::plugins::flight::FlightSet)
/// — before the integrator — so the command lands the same frame, per the
/// `FlightInput` contract.
pub(super) fn enemy_ai(
    time: Res<Time>,
    player: Query<&Transform, With<Player>>,
    mut enemies: Query<
        (
            &Transform,
            &Ship,
            &Energy,
            &mut EnemyAi,
            &mut FlightInput,
            Option<&mut Weapon>,
        ),
        With<Enemy>,
    >,
) {
    // No unique player → enemies coast and hold fire (zero the persistent command
    // rather than flying/firing the last one forever).
    let Ok(player_tf) = player.single() else {
        for (_, _, _, _, mut input, weapon) in &mut enemies {
            input.rotation = Vec3::ZERO;
            input.throttle = 0.0;
            if let Some(mut weapon) = weapon {
                weapon.set_firing(false);
            }
        }
        return;
    };
    let player_pos = player_tf.translation;
    let dt = Duration::from_secs_f32(time.delta_secs());

    for (tf, ship, energy, mut ai, mut input, weapon) in &mut enemies {
        let dist_sq = tf.translation.distance_squared(player_pos);
        let energy_frac = if energy.max > 0.0 {
            energy.current / energy.max
        } else {
            0.0
        };
        let low_energy = energy_frac < LOW_ENERGY_FRAC;

        // --- Transitions ---------------------------------------------------
        match ai.state {
            AiState::Approach => {
                if low_energy {
                    enter_evade(&mut ai, dist_sq);
                } else if dist_sq <= ATTACK_RANGE * ATTACK_RANGE {
                    ai.state = AiState::Attack;
                }
            }
            AiState::Attack => {
                if low_energy || dist_sq <= EVADE_TOO_CLOSE * EVADE_TOO_CLOSE {
                    enter_evade(&mut ai, dist_sq);
                } else {
                    let reapproach = ATTACK_RANGE * (1.0 + RANGE_HYSTERESIS);
                    if dist_sq > reapproach * reapproach {
                        ai.state = AiState::Approach;
                    }
                }
            }
            AiState::Evade => {
                ai.evade_timer.tick(dt);
                let leg_done = dist_sq >= ai.evade_goal_sq || ai.evade_timer.is_finished();
                if leg_done {
                    // Still hurt → run another leg (no Approach blip — avoids the
                    // per-frame heading thrash an energy threshold without
                    // hysteresis would cause); otherwise re-engage.
                    if low_energy {
                        enter_evade(&mut ai, dist_sq);
                    } else {
                        ai.state = AiState::Approach;
                    }
                }
            }
        }

        // --- Behaviour for the (possibly updated) state --------------------
        let (pitch, roll, throttle) = match ai.state {
            AiState::Approach => {
                let (p, r) = aim_at(tf, player_pos);
                (p, r, 1.0)
            }
            AiState::Attack => {
                let (p, r) = aim_at(tf, player_pos);
                (p, r, speed_throttle(ship.speed, ATTACK_SPEED))
            }
            AiState::Evade => {
                // Aim directly away from the player. Throttle = forward·away gates
                // the burn on heading: while the nose is still toward the player it
                // is negative (decelerate, so a close break-off doesn't accelerate
                // into the ram it's avoiding); it ramps to +1 as the nose comes
                // around to face away.
                let away_dir = (tf.translation - player_pos).normalize_or_zero();
                let (p, r) = aim_at(tf, tf.translation + away_dir);
                let throttle = tf.forward().as_vec3().dot(away_dir);
                (p, r, throttle)
            }
        };

        input.rotation = Vec3::new(pitch, 0.0, roll); // yaw stays 0 (DL-011)
        input.throttle = throttle;

        // Fire only while attacking, with the nose on the player and energy to
        // spare. The weapon system handles the cooldown and the actual shot.
        // Unarmed enemies still fly; they just never set a fire intent.
        if let Some(mut weapon) = weapon {
            let to_player = (player_pos - tf.translation).normalize_or_zero();
            let on_target = tf.forward().as_vec3().dot(to_player) > FIRE_CONE_COS;
            weapon.set_firing(ai.state == AiState::Attack && on_target && !low_energy);
        }
    }
}

/// Begin an evade leg: reset the timeout and set the distance goal from where we
/// started running (floored so a close break-off still opens real space).
fn enter_evade(ai: &mut EnemyAi, dist_sq: f32) {
    ai.state = AiState::Evade;
    ai.evade_timer.reset();
    let goal = (dist_sq.sqrt() + EVADE_OPEN_DISTANCE).max(EVADE_MIN_GOAL_DIST);
    ai.evade_goal_sq = goal * goal;
}

/// Bang-bang throttle *rate* toward a target speed (`FlightInput.throttle` is a
/// rate, not a setpoint, so a constant value would ramp to full), with a deadband
/// so it settles at the target instead of dithering ±1 each frame.
fn speed_throttle(current: f32, target: f32) -> f32 {
    let err = target - current;
    if err.abs() < SPEED_DEADBAND {
        0.0
    } else if err > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// Aim the nose at a world-space `target` using pitch + roll only. Returns
/// `(pitch_cmd, roll_cmd)` in `[-1, 1]`.
///
/// Assumes the ship is a top-level entity, so its `Transform` *is* its world
/// transform. Enemies are spawned unparented today; if one is ever parented
/// (carrier, station, formation), switch this to read `GlobalTransform`.
fn aim_at(ship_tf: &Transform, target: Vec3) -> (f32, f32) {
    let to_target = target - ship_tf.translation;
    if to_target.length_squared() < 1.0e-8 {
        return (0.0, 0.0); // coincident — hold attitude
    }
    // Target direction in the ship's local frame (nose = −Z, up = +Y, right = +X).
    let local = ship_tf.rotation.inverse() * to_target.normalize_or_zero();
    aim_commands(local)
}

/// The pure steering math: given a unit target direction in the ship's **local**
/// frame, return `(pitch_cmd, roll_cmd)`.
fn aim_commands(local_dir: Vec3) -> (f32, f32) {
    // Roll so the target's projection in the local up/right plane swings up to +Y.
    // Sign verified against `rotate_local_z(+) = roll left`: a +X (right) target
    // must roll negative. Hence `(-x).atan2(y)`, NOT `x.atan2(y)`.
    let roll_err = (-local_dir.x).atan2(local_dir.y);
    let roll_cmd = (roll_err * ROLL_GAIN).clamp(-1.0, 1.0);

    // Pitch the nose toward the target's elevation above the nose (−Z is ahead).
    let pitch_err = local_dir.y.atan2(-local_dir.z);
    // Gate pitch while badly mis-rolled so the two axes don't fight (cos → 0 at
    // 90° of roll error, negative on the far side).
    let align = roll_err.cos().max(0.0);
    let pitch_cmd = (pitch_err * PITCH_GAIN * align).clamp(-1.0, 1.0);

    (pitch_cmd, roll_cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_targets_roll_in_opposite_directions() {
        // CORRECTED sign: target on the local RIGHT (+X) rolls negative; LEFT (−X)
        // rolls positive. The inverted sign the design first proposed would flip
        // these and the enemy would never aim.
        let (_, roll_right) = aim_commands(Vec3::new(1.0, 0.0, 0.0));
        let (_, roll_left) = aim_commands(Vec3::new(-1.0, 0.0, 0.0));
        assert!(
            roll_right < 0.0,
            "right target rolls negative, got {roll_right}"
        );
        assert!(
            roll_left > 0.0,
            "left target rolls positive, got {roll_left}"
        );
    }

    #[test]
    fn in_plane_target_needs_no_roll_and_pitches_up() {
        let (pitch, roll) = aim_commands(Vec3::new(0.0, 1.0, 0.0)); // straight up
        assert!(
            roll.abs() < 1.0e-3,
            "overhead target needs no roll, got {roll}"
        );
        assert!(pitch > 0.0, "overhead target pitches up, got {pitch}");

        let (pitch2, roll2) = aim_commands(Vec3::new(0.0, 0.6, -0.8)); // above & ahead
        assert!(
            roll2.abs() < 1.0e-3,
            "in-plane target needs no roll, got {roll2}"
        );
        assert!(
            pitch2 > 0.0,
            "above-and-ahead target pitches up, got {pitch2}"
        );
    }

    #[test]
    fn pitch_is_gated_while_badly_misrolled() {
        // Target straight down: align (cos) → 0, so pitch is held near zero while a
        // strong roll swings it up.
        let (pitch, roll) = aim_commands(Vec3::new(0.0, -1.0, 0.0));
        assert!(
            pitch.abs() < 1.0e-3,
            "pitch gated for a below target, got {pitch}"
        );
        assert!(
            roll.abs() > 0.5,
            "below target commands a strong roll, got {roll}"
        );
    }

    #[test]
    fn coincident_target_is_safe() {
        let tf = Transform::default();
        assert_eq!(aim_at(&tf, tf.translation), (0.0, 0.0));
    }
}
