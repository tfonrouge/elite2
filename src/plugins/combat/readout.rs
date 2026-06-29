//! Combat HUD readout — the player's shields/energy banks and the nearest
//! target's status, as a text overlay in the top-right (separate from the
//! flight HUD in `plugins::hud`, which sits top-left).
//!
//! Text-based, so it is **headless-safe** — unlike a `Gizmos`-based overlay it
//! needs no `GizmoConfigStore`, so the wiring tests run it under `MinimalPlugins`
//! without panicking.

use bevy::prelude::*;

use crate::plugins::flight::Player;

use super::components::{Enemy, Energy, Shields};

/// Marker for the combat-readout text node. `pub` so the wiring test can assert
/// exactly one exists.
#[derive(Component)]
pub struct CombatReadout;

pub(super) fn setup_combat_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.8, 0.4)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(12.0),
            ..default()
        },
        CombatReadout,
    ));
}

/// Refresh the readout from the player's shields/energy and the nearest enemy.
/// Runs after the combat pipeline so it shows this frame's damage.
pub(super) fn update_combat_hud(
    player: Single<(&Transform, &Shields, &Energy), With<Player>>,
    hostiles: Query<(&Transform, &Shields, &Energy), With<Enemy>>,
    mut hud: Single<&mut Text, With<CombatReadout>>,
) {
    let (player_tf, shields, energy) = player.into_inner();

    // The nearest hostile: its distance and a rough surviving-health fraction.
    let nearest = hostiles
        .iter()
        .map(|(tf, s, e)| {
            let dist = player_tf.translation.distance(tf.translation);
            let hp = (s.fore + s.aft + e.current) / (2.0 * s.max + e.max);
            (dist, hp)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));

    let target = match nearest {
        Some((dist, hp)) => format!("{dist:.0}u   hull {:.0}%", hp * 100.0),
        None => "-- (no contacts)".to_string(),
    };

    hud.0 = format!(
        "SHIELDS  F:{:>3.0} A:{:>3.0}\n\
         ENERGY   {:>3.0} / {:.0}\n\
         TARGET   {target}",
        shields.fore, shields.aft, energy.current, energy.max,
    );
}
