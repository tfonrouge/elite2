//! The player's ship and how it flies.
//!
//! Flight model: **damped arcade** (chosen in DESIGN.md). Throttle sets a
//! target speed the ship accelerates toward; rotation is driven by an angular
//! velocity that ramps toward the player's input and *damps back to zero when
//! the input is released*, so the ship self-stabilizes. Rotation is in the
//! ship's local frame — **pitch and roll by default**, plus an *optional* yaw
//! axis (see [`YawAssist`]) — and the ship always moves along its own forward
//! axis.
//!
//! Controls (cockpit): **W/S pitch · A/D roll · R/F throttle ±**. The 1984 Cobra
//! flies on two rotational axes only — pitch and roll, *no yaw* (DL-011) — so yaw
//! is off by default. Pressing `Y` toggles a "yaw assist" mode that adds
//! **Q/E yaw** for players who want a third axis.

use bevy::prelude::*;

/// Marker for the single player-controlled entity.
///
/// **Phase 1 invariant: exactly one `Player` exists.** The camera, HUD, and
/// player-input/combat systems read it through `Single<.. With<Player>>`, which
/// *silently skips* its system if there are zero or more than one (it does not
/// panic). If a future phase introduces multiple controllable ships, those
/// `Single` queries must be revisited.
#[derive(Component)]
pub struct Player;

/// Dynamic flight state carried by every ship — player and NPC alike.
///
/// `#[require(..)]` pulls in [`FlightConfig`] + [`FlightInput`]: spawning any
/// entity with a `Ship` auto-inserts both (via their `Default`) unless the spawn
/// bundle overrides them, so the shared [`ship_movement`] integrator — whose
/// query needs all three — always matches. This structurally prevents the
/// "spawned NPC never moves" trap that a partial bundle would otherwise cause.
#[derive(Component, Debug, Default)]
#[require(FlightConfig, FlightInput)]
pub struct Ship {
    /// Throttle setting in `0.0..=1.0` (fraction of max speed).
    pub throttle: f32,
    /// Current forward speed in units/second.
    pub speed: f32,
    /// Angular velocity in rad/s about the local axes: `(pitch, yaw, roll)`.
    /// The yaw component stays zero unless [`YawAssist`] is enabled (DL-011).
    pub angular_velocity: Vec3,
}

/// Tunable flight characteristics, **per ship** — a `Component`, not a global
/// resource. The player and each NPC hull carry their own `FlightConfig`, so the
/// shared [`ship_movement`] integrator flies them all. (This completes the DL-004
/// Phase-2 prep: Resource → Component, `Single` → `Query`.)
#[derive(Component, Debug, Clone)]
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

/// Per-frame flight command for one ship, written by a controller (the player's
/// [`player_input`], or — in Phase 2 — an AI controller) and consumed by the
/// shared [`ship_movement`] integrator. Separating *what the pilot commands* from
/// *how the hull responds* lets a single integrator fly the player and every NPC.
///
/// **Contract:** this is persistent state, not a transient event — the integrator
/// reads it every frame and never clears it. A controller must therefore
/// (re)write its ship's `FlightInput` every frame (or zero it on early-out), or
/// the ship keeps flying the last command it was issued.
#[derive(Component, Debug, Default)]
pub struct FlightInput {
    /// Commanded rotation in `-1.0..=1.0` about the local axes: `(pitch, yaw, roll)`.
    /// The yaw component stays zero for the player unless [`YawAssist`] is on.
    pub rotation: Vec3,
    /// Commanded throttle change in `-1.0..=1.0` (player R/F, or AI).
    pub throttle: f32,
}

/// Whether the optional **yaw-assist** axis is active (DL-011).
///
/// The 1984 *Elite* Cobra has no yaw — flight is pitch + roll only — so this is
/// **off by default**, giving the faithful two-axis model. Toggling it on (the
/// `Y` key) re-enables a yaw axis on `Q`/`E` for players who want it. When it is
/// switched back off, any residual yaw rate damps to zero like every other axis.
///
/// This is a global **player input mode**, not per-ship tuning, so it is a
/// `Resource` (read only by [`player_input`]); the shared [`ship_movement`]
/// integrator and any AI controller never touch it.
#[derive(Resource, Debug, Default)]
pub struct YawAssist {
    pub enabled: bool,
}

/// Ordering for the flight pipeline. Controllers write each ship's
/// [`FlightInput`] in `ReadInput`; the shared [`ship_movement`] integrator
/// applies it in `Integrate`. Systems that *read* the player's `Transform` in the
/// same frame (the HUD, the combat readout, combat) order themselves
/// `.after(FlightSet::Integrate)` so they observe the current frame's motion.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightSet {
    /// Controllers (player input, AI) write each ship's `FlightInput`. Any
    /// controller — *including AI systems in other plugins* (e.g. combat) — must
    /// join this set, so it writes before `ship_movement` integrates; otherwise
    /// that ship integrates last frame's command (a one-frame input lag).
    ReadInput,
    /// The shared integrator applies `FlightInput` to motion.
    Integrate,
}

pub struct FlightPlugin;

impl Plugin for FlightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<YawAssist>()
            .add_systems(Startup, spawn_player)
            .configure_sets(Update, (FlightSet::ReadInput, FlightSet::Integrate).chain())
            .add_systems(Update, player_input.in_set(FlightSet::ReadInput))
            .add_systems(Update, ship_movement.in_set(FlightSet::Integrate));
    }
}

/// Spawn the player's ship at the origin, nose pointing down `-Z` (toward the
/// reference station). The cockpit camera is mounted as a child of this entity
/// by [`super::camera`] in `PostStartup`.
fn spawn_player(mut commands: Commands) {
    // `Ship` requires `FlightConfig` + `FlightInput` (see its docs), so the player
    // gets the default Cobra tuning and a zeroed command without listing them.
    // `Visibility` (and the `InheritedVisibility`/`ViewVisibility` it requires) is
    // included so visibility propagates to the child cockpit camera — without it
    // Bevy logs hierarchy warning B0004.
    commands.spawn((
        Player,
        Ship::default(),
        Transform::default(),
        Visibility::default(),
    ));
}

/// Read the keyboard and write the player's [`FlightInput`] for this frame.
///
/// Faithful two-axis default — pitch (W/S) and roll (A/D), plus throttle (R/F).
/// `Y` toggles the optional yaw axis (DL-011); while on, Q/E command yaw.
fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut yaw_assist: ResMut<YawAssist>,
    mut input: Single<&mut FlightInput, With<Player>>,
) {
    // A just-flipped toggle takes effect the same frame.
    if keys.just_pressed(KeyCode::KeyY) {
        yaw_assist.enabled = !yaw_assist.enabled;
    }

    let pitch_in = axis(&keys, KeyCode::KeyS, KeyCode::KeyW); // S nose-up, W nose-down
    let roll_in = axis(&keys, KeyCode::KeyA, KeyCode::KeyD); // A roll-left, D roll-right
    let yaw_in = if yaw_assist.enabled {
        axis(&keys, KeyCode::KeyQ, KeyCode::KeyE) // Q yaw-left, E yaw-right (assist only)
    } else {
        0.0
    };

    input.rotation = Vec3::new(pitch_in, yaw_in, roll_in);
    input.throttle = axis(&keys, KeyCode::KeyR, KeyCode::KeyF); // R up, F down
}

/// Integrate the damped-arcade flight model for **every** ship from its
/// [`FlightInput`] + [`FlightConfig`]. Shared by the player and all NPCs, so
/// behaviour is identical whoever (or whatever) is at the controls.
fn ship_movement(
    time: Res<Time>,
    mut ships: Query<(&mut Transform, &mut Ship, &FlightConfig, &FlightInput)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut ship, cfg, input) in &mut ships {
        // Ramp angular velocity toward the commanded target; with a zero command
        // this damps back to a stop — the "auto-stabilize" of the arcade model.
        let target = Vec3::new(
            input.rotation.x * cfg.pitch_rate,
            input.rotation.y * cfg.yaw_rate,
            input.rotation.z * cfg.roll_rate,
        );
        let blend = 1.0 - (-cfg.turn_responsiveness * dt).exp();
        ship.angular_velocity = ship.angular_velocity.lerp(target, blend);

        transform.rotate_local_x(ship.angular_velocity.x * dt);
        transform.rotate_local_y(ship.angular_velocity.y * dt);
        transform.rotate_local_z(ship.angular_velocity.z * dt);

        // Throttle and forward speed.
        ship.throttle = (ship.throttle + input.throttle * cfg.throttle_rate * dt).clamp(0.0, 1.0);
        let target_speed = ship.throttle * cfg.max_speed;
        ship.speed = move_toward(ship.speed, target_speed, cfg.acceleration * dt);

        // Translate along the ship's local forward axis.
        let forward = transform.forward();
        transform.translation += forward * (ship.speed * dt);
    }
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
