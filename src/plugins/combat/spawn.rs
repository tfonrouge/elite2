//! Bringing ships into the world.
//!
//! DL-017 (debug command-spawn + in-system hook): for the Phase-2 slice, pressing
//! `B` spawns one pirate enemy ahead of the player but **offset off the −Z
//! station axis**, so it doesn't materialise inside the reference ring station at
//! `(0, 0, -300)`. The real in-system spawner (procedural, by system lawlessness
//! and legal status) lands in Phase 3 and reuses [`spawn_enemy`].

use bevy::prelude::*;

use crate::plugins::flight::{Player, Ship};

use super::ai::EnemyAi;
use super::components::{Bounty, CollisionRadius, Enemy, Energy, Faction, Shields};

/// Debug spawn key — press to drop a pirate enemy into the world for testing.
const SPAWN_ENEMY_KEY: KeyCode = KeyCode::KeyB;

/// Where the debug enemy appears: ahead of the player's start, pushed off the
/// −Z axis so it clears the ring station and asteroids.
const DEBUG_ENEMY_POS: Vec3 = Vec3::new(30.0, 8.0, -120.0);

/// Player collision radius — the targetable size enemy weapons (Step 5) test
/// against. Comparable in scale to the debug enemy's.
const PLAYER_RADIUS: f32 = 6.0;

/// Make the player ship a combatant once it exists (`PostStartup`, after `flight`
/// spawns it in `Startup` — mirrors how the cockpit camera mounts): tag its
/// [`Faction`] and give it the shield/energy/collision set, so the recharge,
/// damage, and (Step 5) enemy-weapon systems treat it like any other ship rather
/// than silently skipping it.
pub(super) fn init_player_combat(mut commands: Commands, player: Single<Entity, With<Player>>) {
    commands.entity(*player).insert((
        Faction::Player,
        Shields::default(),
        Energy::default(),
        CollisionRadius(PLAYER_RADIUS),
    ));
}

/// On the debug key, spawn one pirate enemy. A single press (`just_pressed`)
/// makes exactly one, so holding the key doesn't spam them.
pub(super) fn debug_spawn_enemy(
    keys: Res<ButtonInput<KeyCode>>,
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    if keys.just_pressed(SPAWN_ENEMY_KEY) {
        spawn_enemy(
            commands,
            meshes,
            materials,
            Faction::Pirate,
            DEBUG_ENEMY_POS,
        );
    }
}

/// Spawn one NPC ship of the given faction at `position`. The in-system spawner
/// (Phase 3) calls this too — it is the single place an enemy's component set is
/// defined, so the debug path and the real path can never drift apart.
///
/// `Ship` pulls in `FlightConfig` + `FlightInput` via its `#[require(..)]`, so the
/// shared `ship_movement` integrator flies the NPC; with the default (zeroed)
/// `FlightInput` it simply holds station until the AI controller drives it.
///
/// The NPC currently inherits the **default (Cobra) `FlightConfig`** via that
/// `#[require]` — i.e. player-identical handling. Per-hull/bestiary tuning
/// (DL-015, `GAMEPLAY.md`) will pass an explicit `FlightConfig` in here.
pub(super) fn spawn_enemy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    faction: Faction,
    position: Vec3,
) {
    let hull = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.25, 0.2),
        perceptual_roughness: 0.5,
        metallic: 0.3,
        ..default()
    });
    commands.spawn((
        Enemy,
        faction,
        EnemyAi::default(),
        Ship::default(),
        Shields::default(),
        Energy::default(),
        CollisionRadius(8.0),
        Bounty(15.0),
        Mesh3d(meshes.add(Cuboid::new(6.0, 4.0, 12.0))),
        MeshMaterial3d(hull),
        Transform::from_translation(position),
        Visibility::default(),
    ));
}
