//! Headless wiring + behavior tests.
//!
//! Boots the game's plugins with a *minimal* (no-window, no-GPU) Bevy app and
//! checks two things without a display:
//! - **Wiring:** the scene graph materializes — exactly one `Player`, one
//!   cockpit camera mounted as its child, one HUD, one starfield root (guarding
//!   the single-player Phase 1 invariant and the `PostStartup` camera mount).
//! - **Behavior:** with a fixed timestep and synthesized key presses, the
//!   control directions, throttle→forward motion, and the starfield
//!   translation-only follow (DL-006) hold — turning those contracts from
//!   "verified by reasoning" into enforced invariants.

use std::time::Duration;

use bevy::asset::AssetPlugin;
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use crate::plugins::camera::CockpitCamera;
use crate::plugins::combat::CombatPlugin;
use crate::plugins::combat::ai::{AiState, EnemyAi};
use crate::plugins::combat::components::{
    Bounty, CollisionRadius, Enemy, Energy, Faction, Shields,
};
use crate::plugins::combat::events::ShipDestroyed;
use crate::plugins::core::CorePlugin;
use crate::plugins::flight::{FlightInput, Player, Ship, YawAssist};
use crate::plugins::hud::HudText;
use crate::plugins::starfield::StarfieldRoot;
use crate::plugins::{
    camera::CameraPlugin, flight::FlightPlugin, hud::HudPlugin, starfield::StarfieldPlugin,
    world::WorldPlugin,
};

/// Build the same plugin stack `main` uses, but on a headless app: no window,
/// renderer, or audio. We supply only the infrastructure the game systems
/// actually touch — time (via `MinimalPlugins`), the keyboard input resource,
/// and the asset collections meshes/materials are added to.
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(InputPlugin)
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins((
            CorePlugin,
            FlightPlugin,
            CameraPlugin,
            WorldPlugin,
            StarfieldPlugin,
            HudPlugin,
            CombatPlugin,
        ));
    app
}

/// Count entities matching a query filter, isolating the mutable/immutable
/// world borrows so they don't overlap.
fn count<F: bevy::ecs::query::QueryFilter>(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<Entity, F>();
    query.iter(app.world()).count()
}

#[test]
fn wiring_spawns_exactly_one_of_each_singleton() {
    let mut app = headless_app();
    // First tick runs the startup schedules (Startup spawns the ship/scene,
    // PostStartup mounts the camera) plus one Update; a second tick exercises
    // the per-frame systems once more.
    app.update();
    app.update();

    assert_eq!(count::<With<Player>>(&mut app), 1, "exactly one Player");
    assert_eq!(
        count::<With<CockpitCamera>>(&mut app),
        1,
        "exactly one cockpit camera"
    );
    assert_eq!(count::<With<HudText>>(&mut app), 1, "exactly one HUD text");
    assert_eq!(
        count::<With<StarfieldRoot>>(&mut app),
        1,
        "exactly one starfield root"
    );
}

#[test]
fn cockpit_camera_is_a_child_of_the_player() {
    let mut app = headless_app();
    app.update();

    let player = {
        let mut query = app.world_mut().query_filtered::<Entity, With<Player>>();
        query
            .iter(app.world())
            .next()
            .expect("a Player entity exists")
    };
    let camera_parent = {
        let mut query = app
            .world_mut()
            .query_filtered::<&ChildOf, With<CockpitCamera>>();
        query
            .iter(app.world())
            .next()
            .expect("the cockpit camera exists")
            .parent()
    };

    assert_eq!(
        camera_parent, player,
        "cockpit camera must be parented to the player ship"
    );
}

#[test]
fn player_is_initialised_as_a_combatant() {
    // CombatPlugin's PostStartup system tags the player's faction AND gives it the
    // shield/energy/collision set, so recharge and (Step 5) enemy weapons treat it
    // like any other ship instead of silently skipping it. This also proves the
    // plugin boots under MinimalPlugins (no `GizmoConfigStore`) without panicking.
    let mut app = headless_app();
    app.update(); // Startup spawns the player; PostStartup initialises it.

    // The query matches only if the player carries all of Shields + Energy +
    // CollisionRadius (the filters) as well as Faction (read back).
    let faction = {
        let mut query = app.world_mut().query_filtered::<&Faction, (
            With<Player>,
            With<Shields>,
            With<Energy>,
            With<CollisionRadius>,
        )>();
        query.iter(app.world()).next().copied()
    };
    assert_eq!(
        faction,
        Some(Faction::Player),
        "player is tagged Player and carries the shield/energy/collision set"
    );
}

#[test]
fn debug_key_spawns_a_pirate_enemy_with_combat_components() {
    let mut app = headless_app();
    app.update();
    assert_eq!(
        count::<With<Enemy>>(&mut app),
        0,
        "no enemy before the spawn key"
    );

    send_key(&mut app, KeyCode::KeyB, ButtonState::Pressed);
    app.update();

    assert_eq!(
        count::<With<Enemy>>(&mut app),
        1,
        "one enemy after pressing B"
    );
    let (faction, shields, energy, radius, bounty) = {
        let mut query = app
            .world_mut()
            .query_filtered::<(&Faction, &Shields, &Energy, &CollisionRadius, &Bounty), With<Enemy>>();
        let (f, s, e, r, b) = query.iter(app.world()).next().expect("an enemy exists");
        (*f, *s, *e, r.0, b.0)
    };
    assert_eq!(faction, Faction::Pirate, "the debug enemy is a pirate");
    assert!(
        shields.fore > 0.0 && shields.aft > 0.0 && shields.max > 0.0,
        "enemy has shields"
    );
    assert!(
        energy.current > 0.0 && energy.max > 0.0,
        "enemy has energy banks"
    );
    assert!(radius > 0.0, "enemy has a collision radius");
    assert!(bounty > 0.0, "enemy carries a bounty");
}

#[test]
fn sustained_laser_fire_destroys_an_enemy() {
    let mut app = timed_app(0.1);
    app.update(); // startup + PostStartup (the player is armed with a Weapon)

    // A stationary target dead ahead on the player's -Z axis: faction Pirate so the
    // player's faction-based fire can hit it, and no `Ship`/`EnemyAi` so it holds
    // position — testing the laser in isolation from the AI's movement.
    app.world_mut().spawn((
        Enemy,
        Faction::Pirate,
        Shields::default(),
        Energy::default(),
        CollisionRadius(8.0),
        Bounty(15.0),
        Transform::from_xyz(0.0, 0.0, -50.0),
    ));
    app.update();
    assert_eq!(count::<With<Enemy>>(&mut app), 1, "a target exists");

    // Hold the fire key; the cooldown-gated laser pulses until the target is gone.
    send_key(&mut app, KeyCode::Space, ButtonState::Pressed);
    for _ in 0..40 {
        app.update();
    }

    assert_eq!(
        count::<With<Enemy>>(&mut app),
        0,
        "sustained laser fire destroys the target"
    );
}

#[test]
fn enemy_fire_damages_the_player() {
    // The first test of the player TAKING damage: an enemy in Attack, nose-on,
    // fires and drains the player's shields (which it got in init_player_combat).
    let mut app = timed_app(0.05);
    app.update();

    // Spawn a full armed pirate via the debug key, then place it within attack
    // range facing the player at the origin.
    send_key(&mut app, KeyCode::KeyB, ButtonState::Pressed);
    app.update();
    let enemy = {
        let mut query = app.world_mut().query_filtered::<Entity, With<Enemy>>();
        query.iter(app.world()).next().expect("an enemy exists")
    };
    *app.world_mut().get_mut::<Transform>(enemy).unwrap() =
        Transform::from_xyz(0.0, 0.0, -300.0).looking_at(Vec3::ZERO, Vec3::Y);

    // Let it aim, get nose-on, and fire for a few seconds.
    for _ in 0..80 {
        app.update();
    }

    let shields = {
        let mut query = app.world_mut().query_filtered::<&Shields, With<Player>>();
        *query.iter(app.world()).next().expect("the player exists")
    };
    assert!(
        shields.fore < 64.0 || shields.aft < 64.0,
        "the player took shield damage from enemy fire (fore {}, aft {})",
        shields.fore,
        shields.aft
    );
}

#[test]
fn the_player_is_not_despawned_when_destroyed() {
    // The player's loss is an escape pod / game over in a later phase — handle_death
    // must NOT despawn the player, or one unlucky frame would delete the ship and
    // orphan the camera/AI.
    let mut app = headless_app();
    app.update();

    let player = {
        let mut query = app.world_mut().query_filtered::<Entity, With<Player>>();
        query.iter(app.world()).next().expect("a Player exists")
    };
    app.world_mut().write_message(ShipDestroyed {
        ship: player,
        bounty: 0.0,
    });
    app.update();

    assert_eq!(
        count::<With<Player>>(&mut app),
        1,
        "the player is not despawned on destruction"
    );
}

#[test]
fn the_first_shot_fires_immediately() {
    // The refire timer starts ready, so a held trigger lands a hit on the very
    // first frame rather than after a full cooldown.
    let mut app = timed_app(0.05);
    app.update(); // player armed

    app.world_mut().spawn((
        Enemy,
        Faction::Pirate,
        Shields::default(),
        Energy::default(),
        CollisionRadius(8.0),
        Bounty(15.0),
        Transform::from_xyz(0.0, 0.0, -50.0),
    ));
    app.update();

    send_key(&mut app, KeyCode::Space, ButtonState::Pressed);
    app.update(); // a single fire frame

    let shields = {
        let mut query = app.world_mut().query_filtered::<&Shields, With<Enemy>>();
        *query.iter(app.world()).next().expect("a target exists")
    };
    assert!(
        shields.fore < 64.0 || shields.aft < 64.0,
        "the first shot lands on frame one (fore {}, aft {})",
        shields.fore,
        shields.aft
    );
}

#[test]
fn a_ship_does_not_fire_on_itself_or_allies() {
    // Faction targeting must exclude the shooter itself and same-faction ships —
    // the rule all friendly-fire safety rests on.
    let mut app = timed_app(0.05);
    app.update(); // player armed, faction Player

    // A same-faction ally directly ahead of the player.
    let ally = app
        .world_mut()
        .spawn((
            Faction::Player,
            Shields::default(),
            Energy::default(),
            CollisionRadius(8.0),
            Transform::from_xyz(0.0, 0.0, -50.0),
        ))
        .id();
    app.update();

    // Hold fire with only the player and its ally in the line of sight.
    send_key(&mut app, KeyCode::Space, ButtonState::Pressed);
    for _ in 0..10 {
        app.update();
    }

    let player_shields = {
        let mut query = app.world_mut().query_filtered::<&Shields, With<Player>>();
        *query.iter(app.world()).next().expect("the player exists")
    };
    let ally_shields = *app.world().get::<Shields>(ally).expect("ally shields");
    assert_eq!(
        player_shields.fore, 64.0,
        "the player never fires on itself"
    );
    assert_eq!(player_shields.aft, 64.0, "the player never fires on itself");
    assert_eq!(ally_shields.fore, 64.0, "the player never fires on an ally");
    assert_eq!(ally_shields.aft, 64.0, "the player never fires on an ally");
}

#[test]
fn overlapping_ships_take_collision_damage() {
    // DL-014: two ships whose spheres overlap ram each other, and the damage flows
    // through the same apply_damage chokepoint (so both shields drain).
    let mut app = timed_app(0.05);
    app.update(); // player at the origin, CollisionRadius 6

    // Another ship overlapping the player (5 u apart < 6 + 8 = 14).
    let other = app
        .world_mut()
        .spawn((
            Faction::Pirate,
            Shields::default(),
            Energy::default(),
            CollisionRadius(8.0),
            Transform::from_xyz(0.0, 0.0, -5.0),
        ))
        .id();

    for _ in 0..10 {
        app.update();
    }

    let player_shields = {
        let mut query = app.world_mut().query_filtered::<&Shields, With<Player>>();
        *query.iter(app.world()).next().expect("the player exists")
    };
    let other_shields = *app.world().get::<Shields>(other).expect("other ship");
    assert!(
        player_shields.fore < 64.0 || player_shields.aft < 64.0,
        "the player is rammed"
    );
    assert!(
        other_shields.fore < 64.0 || other_shields.aft < 64.0,
        "the other ship is rammed"
    );
}

#[test]
fn distant_ships_do_not_collide() {
    let mut app = timed_app(0.05);
    app.update();

    // Far from the player — well beyond the summed radii.
    let other = app
        .world_mut()
        .spawn((
            Faction::Pirate,
            Shields::default(),
            Energy::default(),
            CollisionRadius(8.0),
            Transform::from_xyz(0.0, 0.0, -100.0),
        ))
        .id();

    for _ in 0..10 {
        app.update();
    }

    let other_shields = *app.world().get::<Shields>(other).expect("other ship");
    assert_eq!(other_shields.fore, 64.0, "no collision at range");
    assert_eq!(other_shields.aft, 64.0, "no collision at range");
}

#[test]
fn enemy_ai_turns_the_nose_toward_the_player() {
    // The no-yaw aimer must steer TOWARD the player (the sign the adversarial
    // review caught inverted). Place the player far to the enemy's local right and
    // assert the enemy rotates to face it — and never commands yaw (DL-011).
    let mut app = timed_app(0.05);
    app.update(); // startup

    // Move the player well off to the side (kept out of attack range / reach).
    {
        let mut query = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Player>>();
        let mut iter = query.iter_mut(app.world_mut());
        let mut player = iter.next().expect("a Player exists");
        *player = Transform::from_xyz(1500.0, 0.0, 0.0);
    }
    // Spawn an enemy at the origin facing -Z (so the player is ~90° to its right).
    let enemy = app
        .world_mut()
        .spawn((
            Enemy,
            Ship::default(),
            Energy::default(),
            EnemyAi::default(),
            Transform::default(),
        ))
        .id();

    // Run ~6 s of the AI → integrator pipeline.
    for _ in 0..120 {
        app.update();
    }

    let player_pos = {
        let mut query = app.world_mut().query_filtered::<&Transform, With<Player>>();
        query.iter(app.world()).next().expect("player").translation
    };
    let enemy_tf = *app
        .world()
        .get::<Transform>(enemy)
        .expect("enemy transform");
    let yaw_cmd = app
        .world()
        .get::<FlightInput>(enemy)
        .expect("enemy FlightInput")
        .rotation
        .y;

    let to_player = (player_pos - enemy_tf.translation).normalize_or_zero();
    let aim = enemy_tf.forward().as_vec3().dot(to_player);
    assert!(
        aim > 0.7,
        "enemy should turn to face the player (forward·dir = {aim})"
    );
    assert_eq!(yaw_cmd, 0.0, "DL-011: the NPC never commands yaw");
}

/// The AI state of a specific enemy (test-only accessor).
fn ai_state(app: &App, enemy: Entity) -> AiState {
    app.world()
        .get::<EnemyAi>(enemy)
        .expect("enemy has EnemyAi")
        .state()
}

/// Spawn a bare AI enemy (no mesh) at `pos` and return its entity.
fn spawn_ai_enemy(app: &mut App, pos: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Enemy,
            Ship::default(),
            Energy::default(),
            EnemyAi::default(),
            Transform::from_translation(pos),
        ))
        .id()
}

#[test]
fn a_healthy_enemy_in_range_enters_attack() {
    let mut app = timed_app(0.05);
    app.update(); // player at the origin

    // 200 u away — inside ATTACK_RANGE (350) — and at full energy.
    let enemy = spawn_ai_enemy(&mut app, Vec3::new(0.0, 0.0, -200.0));
    app.update(); // one AI tick: Approach -> Attack

    assert_eq!(
        ai_state(&app, enemy),
        AiState::Attack,
        "a healthy enemy within range attacks"
    );
}

#[test]
fn an_evading_enemy_opens_distance_then_re_engages() {
    // Exercises the DL-016 hybrid evade: low energy breaks the enemy off, it opens
    // a distance goal, and (with energy restored) the leg ends and it re-engages.
    let mut app = timed_app(0.05);
    app.update();

    // Place the enemy so the player sits behind its nose — evading just means
    // burning straight ahead, which opens distance cleanly.
    let enemy = spawn_ai_enemy(&mut app, Vec3::new(0.0, 0.0, -200.0));
    app.update();

    // Cripple its energy → it must break off into Evade.
    app.world_mut().get_mut::<Energy>(enemy).unwrap().current = 10.0;
    app.update();
    assert_eq!(
        ai_state(&app, enemy),
        AiState::Evade,
        "low energy breaks the enemy off into Evade"
    );

    let player_pos = Vec3::ZERO; // player starts at the origin and does not move
    let start_dist = app
        .world()
        .get::<Transform>(enemy)
        .unwrap()
        .translation
        .distance(player_pos);

    // Heal it so the leg can complete and it re-engages rather than running forever.
    app.world_mut().get_mut::<Energy>(enemy).unwrap().current = 150.0;

    let mut max_dist = start_dist;
    let mut re_engaged = false;
    for _ in 0..200 {
        app.update();
        let dist = app
            .world()
            .get::<Transform>(enemy)
            .unwrap()
            .translation
            .distance(player_pos);
        max_dist = max_dist.max(dist);
        if ai_state(&app, enemy) != AiState::Evade {
            re_engaged = true;
            break;
        }
    }

    assert!(
        re_engaged,
        "the evade leg ends (distance goal or timeout) and the enemy re-engages"
    );
    assert!(
        max_dist > start_dist + 50.0,
        "evade opened distance (max {max_dist} from start {start_dist})"
    );
}

#[test]
fn an_evade_leg_ends_on_the_timeout_when_distance_cannot_open() {
    // The *other* DL-016 branch: when the enemy can't open the distance goal (a
    // player keeping pace), the leg must still end on the timeout cap. The
    // sibling test above covers the distance branch; this covers the timeout.
    let mut app = timed_app(0.05);
    app.update();

    let enemy = spawn_ai_enemy(&mut app, Vec3::new(0.0, 0.0, -200.0));
    app.update();

    // Break it off into Evade, then heal so the leg ends into Approach (not a
    // fresh low-energy leg).
    app.world_mut().get_mut::<Energy>(enemy).unwrap().current = 10.0;
    app.update();
    assert_eq!(ai_state(&app, enemy), AiState::Evade, "low energy -> Evade");
    app.world_mut().get_mut::<Energy>(enemy).unwrap().current = 150.0;

    let mut exited = false;
    for _ in 0..400 {
        // Chase: pin the player a short, constant distance from the enemy each
        // frame, so the distance goal can never be met — the only remaining exit
        // is the timeout.
        let enemy_pos = app.world().get::<Transform>(enemy).unwrap().translation;
        {
            let mut query = app
                .world_mut()
                .query_filtered::<&mut Transform, With<Player>>();
            let mut iter = query.iter_mut(app.world_mut());
            iter.next().unwrap().translation = enemy_pos + Vec3::new(0.0, 0.0, 100.0);
        }
        app.update();
        if ai_state(&app, enemy) != AiState::Evade {
            exited = true;
            break;
        }
    }
    assert!(exited, "the evade leg ends even when distance never opens");

    let player_pos = {
        let mut query = app.world_mut().query_filtered::<&Transform, With<Player>>();
        query.iter(app.world()).next().expect("player").translation
    };
    let final_dist = app
        .world()
        .get::<Transform>(enemy)
        .unwrap()
        .translation
        .distance(player_pos);
    // The player stayed ~100 u away, far below the ~600 u distance goal, so the
    // leg can only have ended on the DL-016 timeout — not the distance branch.
    assert!(
        final_dist < 300.0,
        "leg ended on the timeout, not the distance goal (dist {final_dist})"
    );
}

// --- Behavior tests -------------------------------------------------------
// These drive the app with a fixed timestep so `Time::delta_secs()` is
// deterministic, press keys, and assert on the resulting motion — turning the
// flight contract from "verified by reasoning" into an enforced invariant.

/// Headless app with a fixed per-update timestep (deterministic `delta_secs`).
fn timed_app(step_secs: f32) -> App {
    let mut app = headless_app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        step_secs,
    )));
    app
}

fn player_transform(app: &mut App) -> Transform {
    let mut query = app.world_mut().query_filtered::<&Transform, With<Player>>();
    *query.iter(app.world()).next().expect("a Player exists")
}

#[test]
fn default_controls_are_pitch_and_roll_with_no_yaw() {
    // The faithful 1984 flight model is two-axis: pitch + roll, NO yaw (DL-011).
    // W/S pitch and A/D roll; Q/E (the yaw-assist keys) must produce *no*
    // rotation until assist is toggled on. These signs are the contract the
    // whole flight feel rests on — a flip in `axis()` or a swapped `KeyCode`
    // would otherwise pass fmt/clippy/build and silently invert a control.
    let cases: [(KeyCode, fn(Transform) -> bool, &str); 6] = [
        (KeyCode::KeyW, |t| t.forward().y < 0.0, "W = nose down (-Y)"),
        (KeyCode::KeyS, |t| t.forward().y > 0.0, "S = nose up (+Y)"),
        (KeyCode::KeyA, |t| t.up().x < 0.0, "A = roll left (up -X)"),
        (KeyCode::KeyD, |t| t.up().x > 0.0, "D = roll right (up +X)"),
        (
            KeyCode::KeyQ,
            |t| is_near_identity(t.rotation),
            "Q = no yaw by default",
        ),
        (
            KeyCode::KeyE,
            |t| is_near_identity(t.rotation),
            "E = no yaw by default",
        ),
    ];

    for (key, predicate, description) in cases {
        let mut app = timed_app(0.2);
        app.update(); // startup spawns the ship; no input this frame
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update(); // one 0.2 s step with the control held

        let t = player_transform(&mut app);
        assert!(
            predicate(t),
            "wrong control for {description}: rot={:?} forward={:?} up={:?}",
            t.rotation,
            t.forward(),
            t.up()
        );
    }
}

#[test]
fn yaw_assist_toggle_adds_yaw_on_qe() {
    // With yaw assist enabled (DL-011), Q/E re-enable a yaw axis. We set the
    // resource directly rather than simulating the `Y` keypress so the test
    // doesn't depend on `just_pressed` edge timing across the headless schedule.
    let cases: [(KeyCode, fn(f32) -> bool, &str); 2] = [
        (KeyCode::KeyQ, |x| x < 0.0, "Q = yaw left (forward -X)"),
        (KeyCode::KeyE, |x| x > 0.0, "E = yaw right (forward +X)"),
    ];

    for (key, predicate, description) in cases {
        let mut app = timed_app(0.2);
        app.update();
        app.world_mut().resource_mut::<YawAssist>().enabled = true;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();

        let fwd_x = player_transform(&mut app).forward().x;
        assert!(predicate(fwd_x), "{description}: forward.x={fwd_x}");
    }
}

/// True when a rotation is within a tiny tolerance of no rotation at all — used
/// to assert that the yaw keys leave attitude untouched while assist is off.
fn is_near_identity(rotation: Quat) -> bool {
    rotation.angle_between(Quat::IDENTITY) < 1.0e-3
}

#[test]
fn pressing_y_toggles_yaw_assist() {
    // Exercises the *real* input path — `just_pressed(KeyCode::KeyY)` in
    // `player_input` — which the resource-level yaw test bypasses. A mistyped
    // keycode or a `pressed`-vs-`just_pressed` slip would pass every other test
    // but fail here. We inject `KeyboardInput` messages the way winit does (a
    // manual `ButtonInput::press` is wiped by the keyboard system's per-frame
    // clear before `Update` runs); `just_pressed` only fires on a fresh press, so
    // we release between the two toggles.
    let mut app = timed_app(0.2);
    app.update(); // startup
    assert!(
        !app.world().resource::<YawAssist>().enabled,
        "yaw assist starts off (faithful no-yaw default)"
    );

    send_key(&mut app, KeyCode::KeyY, ButtonState::Pressed);
    app.update();
    assert!(
        app.world().resource::<YawAssist>().enabled,
        "first Y press enables yaw assist"
    );

    send_key(&mut app, KeyCode::KeyY, ButtonState::Released);
    app.update();
    send_key(&mut app, KeyCode::KeyY, ButtonState::Pressed);
    app.update();
    assert!(
        !app.world().resource::<YawAssist>().enabled,
        "a second Y press toggles yaw assist back off"
    );
}

/// Inject a keyboard event the way the windowing backend does, so the keyboard
/// system sets `just_pressed`/`just_released` for the frame. `logical_key` is
/// irrelevant to the key-code path, so we use the trivially-constructed
/// `Key::Dead(None)`.
fn send_key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Dead(None),
        state,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

fn ship_speed(app: &mut App) -> f32 {
    let mut query = app.world_mut().query_filtered::<&Ship, With<Player>>();
    query
        .iter(app.world())
        .next()
        .expect("a Player exists")
        .speed
}

#[test]
fn throttle_accelerates_the_ship_forward() {
    let mut app = timed_app(0.1);
    app.update(); // startup
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyR); // throttle up
    for _ in 0..10 {
        app.update();
    }

    let speed = ship_speed(&mut app);
    let z = player_transform(&mut app).translation.z;

    assert!(
        speed > 0.0,
        "holding throttle-up should build speed, got {speed}"
    );
    assert!(z < 0.0, "the ship should move forward along -Z, got z={z}");
}

#[test]
fn throttle_down_decelerates_the_ship() {
    // The throttle path was rewritten from press-based to `axis(R, F)`-integrated;
    // lock the *deceleration* direction (F lowers speed) the way the rotation test
    // locks the pitch/roll signs. Also exercises release(R) so the two keys aren't
    // both held (axis would net to zero).
    let mut app = timed_app(0.1);
    app.update(); // startup

    // Build speed with throttle-up.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyR);
    for _ in 0..10 {
        app.update();
    }
    let fast = ship_speed(&mut app);
    assert!(fast > 0.0, "throttle-up should build speed, got {fast}");

    // Switch to throttle-down.
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyR);
        keys.press(KeyCode::KeyF);
    }
    for _ in 0..10 {
        app.update();
    }
    let slow = ship_speed(&mut app);
    assert!(
        slow < fast,
        "holding throttle-down should reduce speed: {slow} !< {fast}"
    );
}

#[test]
fn starfield_root_follows_translation_but_not_rotation() {
    // DL-006: the root copies the player's translation only. Copying rotation
    // too would defeat the "rotation reads, no translation parallax" design.
    let mut app = timed_app(0.05);
    app.update(); // spawn ship + starfield

    let target = Vec3::new(123.0, -45.0, 67.0);
    {
        let mut query = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Player>>();
        let mut iter = query.iter_mut(app.world_mut());
        let mut player = iter.next().expect("a Player exists");
        player.translation = target;
        player.rotation = Quat::from_rotation_y(1.0); // arbitrary, non-identity
    }
    app.update(); // follow_player runs

    let root = {
        let mut query = app
            .world_mut()
            .query_filtered::<&Transform, With<StarfieldRoot>>();
        *query
            .iter(app.world())
            .next()
            .expect("a StarfieldRoot exists")
    };

    assert!(
        (root.translation - target).length() < 1e-4,
        "root should follow the player's translation, got {:?}",
        root.translation
    );
    assert_eq!(
        root.rotation,
        Quat::IDENTITY,
        "root must NOT inherit the player's rotation"
    );
}
