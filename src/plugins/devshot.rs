//! DevShotPlugin — automated screenshot capture for headless visual checks.
//!
//! Inert on normal runs. When the environment variable `ELITE_SHOT_DIR` is set,
//! this plugin captures the primary window to PNG files in that directory at a
//! few timed intervals (so transient startup state has settled and any motion is
//! visible), then exits the app. This lets a tooling/agent session "see the
//! graphics" without OS screen-recording permissions.
//!
//! This is a development convenience only — it spawns no entities and runs no
//! systems unless `ELITE_SHOT_DIR` is present in the environment.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

/// Capture instants (seconds after startup) and the matching file names.
const SHOTS: &[(f32, &str)] = &[
    (2.0, "elite_01.png"),
    (3.5, "elite_02.png"),
    (5.0, "elite_03.png"),
];

#[derive(Resource)]
struct ShotPlan {
    dir: PathBuf,
    elapsed: f32,
    next: usize,
}

pub struct DevShotPlugin;

impl Plugin for DevShotPlugin {
    fn build(&self, app: &mut App) {
        let Ok(dir) = std::env::var("ELITE_SHOT_DIR") else {
            return; // inert unless explicitly requested
        };
        app.insert_resource(ShotPlan {
            dir: PathBuf::from(dir),
            elapsed: 0.0,
            next: 0,
        })
        .add_systems(Update, capture);
    }
}

fn capture(
    mut plan: ResMut<ShotPlan>,
    time: Res<Time>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    plan.elapsed += time.delta_secs();

    while plan.next < SHOTS.len() && plan.elapsed >= SHOTS[plan.next].0 {
        let path = plan.dir.join(SHOTS[plan.next].1);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        plan.next += 1;
    }

    if plan.next >= SHOTS.len() {
        // Give the last screenshot a moment to flush to disk before quitting.
        if plan.elapsed >= SHOTS[SHOTS.len() - 1].0 + 1.0 {
            exit.write(AppExit::Success);
        }
    }
}
