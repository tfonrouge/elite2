//! A minimal flight HUD: speed, throttle, and orientation, drawn as a text
//! overlay in the top-left corner. Deliberately spartan for the Phase 1 slice;
//! it grows into a proper cockpit HUD later.

use bevy::prelude::*;

use crate::plugins::flight::{FlightConfig, Player, Ship};

/// Marker for the HUD text node we update each frame.
#[derive(Component)]
struct HudText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(Update, update_hud);
    }
}

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.4, 1.0, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudText,
    ));
}

fn update_hud(
    cfg: Res<FlightConfig>,
    player: Single<(&Transform, &Ship), With<Player>>,
    mut hud: Single<&mut Text, With<HudText>>,
) {
    let (transform, ship) = player.into_inner();
    // EulerRot::YXZ decomposes to (yaw, pitch, roll).
    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);

    hud.0 = format!(
        "ELITE — flight test\n\
         SPD {:>4.0} / {:.0}    THR {:>3.0}%\n\
         PITCH {:>4.0}   YAW {:>4.0}   ROLL {:>4.0}\n\
         [W/S pitch  A/D yaw  Q/E roll  R/F throttle]",
        ship.speed,
        cfg.max_speed,
        ship.throttle * 100.0,
        pitch.to_degrees(),
        yaw.to_degrees(),
        roll.to_degrees(),
    );
}
