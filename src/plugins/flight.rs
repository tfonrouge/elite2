//! The player's ship and how it flies.
//!
//! Flight model: **damped arcade** (chosen in DESIGN.md). Throttle sets a
//! target speed the ship accelerates toward; rotation is driven by an angular
//! velocity that ramps toward the player's input and *damps back to zero when
//! the input is released*, so the ship self-stabilizes. All rotation is in the
//! ship's local frame (pitch/yaw/roll relative to where the nose points), and
//! the ship always moves along its own forward axis.
//!
//! Controls (cockpit): W/S pitch · A/D yaw · Q/E roll · R/F throttle ±.

use bevy::prelude::*;

/// Marker for the single player-controlled entity.
///
/// **Phase 1 invariant: exactly one `Player` exists.** The camera, HUD, and
/// starfield read it through `Single<.. With<Player>>`, which *silently skips*
/// its system if there are zero or more than one (it does not panic). If a
/// future phase introduces multiple controllable ships, those `Single` queries
/// must be revisited.
#[derive(Component)]
pub struct Player;

/// Dynamic flight state carried by the player's ship.
#[derive(Component, Debug, Default)]
pub struct Ship {
    /// Throttle setting in `0.0..=1.0` (fraction of max speed).
    pub throttle: f32,
    /// Current forward speed in units/second.
    pub speed: f32,
    /// Angular velocity in rad/s about the local axes: `(pitch, yaw, roll)`.
    pub angular_velocity: Vec3,
}

/// Tunable flight characteristics, kept as a single global resource for Phase 1.
///
/// Per-ship-hull tuning later means a Resource → Component move plus
/// generalizing the integrator from `Single<.. With<Player>>` to a `Query`;
/// tracked as a known, deliberate Phase-1 shortcut in `DECISIONS.md` (DL-004).
#[derive(Resource, Debug, Clone)]
pub struct FlightConfig {
    /// Top speed at full throttle, units/second.
    pub max_speed: f32,
    /// How fast actual speed converges to the throttle's target, units/s².
    pub acceleration: f32,
    /// How fast the throttle moves per second while R/F is held (fraction/s).
    pub throttle_rate: f32,
    /// Maximum pitch rate, rad/s.
    pub pitch_rate: f32,
    /// Maximum yaw rate, rad/s.
    pub yaw_rate: f32,
    /// Maximum roll rate, rad/s.
    pub roll_rate: f32,
    /// Angular responsiveness: higher = snappier ramp toward target rotation
    /// and faster damping back to zero on release.
    pub turn_responsiveness: f32,
}

impl Default for FlightConfig {
    fn default() -> Self {
        Self {
            max_speed: 120.0,
            acceleration: 80.0,
            throttle_rate: 0.6,
            pitch_rate: 1.3,
            yaw_rate: 1.0,
            roll_rate: 2.2,
            turn_responsiveness: 6.0,
        }
    }
}

/// System set wrapping the flight integrator. Systems that *read* the player's
/// `Transform` in the same frame (cockpit follow-readers, HUD, starfield) order
/// themselves `.after(FlightSet::Integrate)` so they observe the current
/// frame's motion rather than the previous frame's.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightSet {
    Integrate,
}

pub struct FlightPlugin;

impl Plugin for FlightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlightConfig>()
            .add_systems(Startup, spawn_player)
            .add_systems(Update, flight_controls.in_set(FlightSet::Integrate));
    }
}

/// Spawn the player's ship at the origin, nose pointing down `-Z` (toward the
/// reference station). The cockpit camera is mounted as a child of this entity
/// by [`super::camera`] in `PostStartup`.
fn spawn_player(mut commands: Commands) {
    // `Visibility` (and the `InheritedVisibility`/`ViewVisibility` it requires)
    // is included so visibility propagates consistently to the child cockpit
    // camera — without it Bevy logs hierarchy warning B0004.
    commands.spawn((
        Player,
        Ship::default(),
        Transform::default(),
        Visibility::default(),
    ));
}

/// Read input and integrate the damped-arcade flight model for one frame.
fn flight_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    cfg: Res<FlightConfig>,
    player: Single<(&mut Transform, &mut Ship), With<Player>>,
) {
    let dt = time.delta_secs();
    let (mut transform, mut ship) = player.into_inner();

    // --- Rotation: ramp angular velocity toward the input target -----------
    let pitch_in = axis(&keys, KeyCode::KeyS, KeyCode::KeyW); // S nose-up, W nose-down
    let yaw_in = axis(&keys, KeyCode::KeyA, KeyCode::KeyD); // A left, D right
    let roll_in = axis(&keys, KeyCode::KeyQ, KeyCode::KeyE); // Q left, E right

    let target = Vec3::new(
        pitch_in * cfg.pitch_rate,
        yaw_in * cfg.yaw_rate,
        roll_in * cfg.roll_rate,
    );
    // Exponential smoothing toward the target; with target = 0 (no input) this
    // damps the ship back to a stop — the "auto-stabilize" of the arcade model.
    let blend = 1.0 - (-cfg.turn_responsiveness * dt).exp();
    ship.angular_velocity = ship.angular_velocity.lerp(target, blend);

    transform.rotate_local_x(ship.angular_velocity.x * dt);
    transform.rotate_local_y(ship.angular_velocity.y * dt);
    transform.rotate_local_z(ship.angular_velocity.z * dt);

    // --- Throttle and forward speed ----------------------------------------
    if keys.pressed(KeyCode::KeyR) {
        ship.throttle += cfg.throttle_rate * dt;
    }
    if keys.pressed(KeyCode::KeyF) {
        ship.throttle -= cfg.throttle_rate * dt;
    }
    ship.throttle = ship.throttle.clamp(0.0, 1.0);

    let target_speed = ship.throttle * cfg.max_speed;
    ship.speed = move_toward(ship.speed, target_speed, cfg.acceleration * dt);

    // --- Translate along the ship's local forward axis ---------------------
    let forward = transform.forward();
    transform.translation += forward * (ship.speed * dt);
}

/// Returns `+1`, `-1`, or `0` from a pair of held keys.
fn axis(keys: &ButtonInput<KeyCode>, positive: KeyCode, negative: KeyCode) -> f32 {
    keys.pressed(positive) as i32 as f32 - keys.pressed(negative) as i32 as f32
}

/// Move `current` toward `target` by at most `max_delta` (no overshoot).
fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    if delta.abs() <= max_delta {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

#[cfg(test)]
mod tests {
    use super::move_toward;

    #[test]
    fn move_toward_steps_without_overshoot() {
        // Accelerating: takes a bounded step.
        assert_eq!(move_toward(0.0, 10.0, 3.0), 3.0);
        // Decelerating: steps downward.
        assert_eq!(move_toward(10.0, 0.0, 3.0), 7.0);
    }

    #[test]
    fn move_toward_snaps_when_within_reach() {
        assert_eq!(move_toward(0.0, 2.0, 3.0), 2.0);
        assert_eq!(move_toward(5.0, 5.0, 1.0), 5.0);
    }

    #[test]
    fn move_toward_with_zero_delta_holds() {
        assert_eq!(move_toward(4.0, 10.0, 0.0), 4.0);
    }
}
