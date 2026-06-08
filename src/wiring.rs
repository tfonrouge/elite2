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
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use crate::plugins::camera::CockpitCamera;
use crate::plugins::core::CorePlugin;
use crate::plugins::flight::{Player, Ship};
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
fn control_directions_match_the_documented_mapping() {
    // The pitch/yaw/roll signs are the contract the whole flight feel rests on.
    // A future flip in `axis()` or a swapped `KeyCode` would otherwise pass
    // fmt/clippy/build/test and silently invert a control.
    let cases: [(KeyCode, fn(Vec3, Vec3) -> bool, &str); 6] = [
        (KeyCode::KeyW, |fwd, _up| fwd.y < 0.0, "W = nose down (-Y)"),
        (KeyCode::KeyS, |fwd, _up| fwd.y > 0.0, "S = nose up (+Y)"),
        (KeyCode::KeyA, |fwd, _up| fwd.x < 0.0, "A = yaw left (-X)"),
        (KeyCode::KeyD, |fwd, _up| fwd.x > 0.0, "D = yaw right (+X)"),
        (
            KeyCode::KeyQ,
            |_fwd, up| up.x < 0.0,
            "Q = roll left (up -X)",
        ),
        (
            KeyCode::KeyE,
            |_fwd, up| up.x > 0.0,
            "E = roll right (up +X)",
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
        let (fwd, up) = (t.forward().as_vec3(), t.up().as_vec3());
        assert!(
            predicate(fwd, up),
            "wrong control direction for {description}: forward={fwd:?} up={up:?}"
        );
    }
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

    let speed = {
        let mut query = app.world_mut().query_filtered::<&Ship, With<Player>>();
        query
            .iter(app.world())
            .next()
            .expect("a Player exists")
            .speed
    };
    let z = player_transform(&mut app).translation.z;

    assert!(
        speed > 0.0,
        "holding throttle-up should build speed, got {speed}"
    );
    assert!(z < 0.0, "the ship should move forward along -Z, got z={z}");
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
