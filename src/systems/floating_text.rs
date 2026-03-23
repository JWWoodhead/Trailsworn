use bevy::prelude::*;

use crate::resources::events::{AttackMissedEvent, DamageDealtEvent};
use crate::resources::map::render_layers;
use crate::resources::theme::Theme;

/// A floating text entity that drifts upward and fades out.
#[derive(Component)]
pub struct FloatingText {
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub velocity: Vec2,
}

const FLOAT_SPEED: f32 = 60.0;
const FLOAT_LIFETIME: f32 = 1.0;
const FONT_SIZE: f32 = 18.0;
const CRIT_FONT_SIZE: f32 = 24.0;

/// Spawn floating damage numbers when damage is dealt.
pub fn spawn_damage_numbers(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageDealtEvent>,
    mut miss_events: MessageReader<AttackMissedEvent>,
    theme: Res<Theme>,
    transforms: Query<&Transform>,
) {
    for event in damage_events.read() {
        let Ok(target_transform) = transforms.get(event.target) else {
            continue;
        };

        let pos = target_transform.translation;

        // Vary x slightly so overlapping numbers spread out
        let x_offset = (event.amount * 7.3 % 20.0) - 10.0;

        let (text, color, size) = if event.target_killed {
            ("KILLED".to_string(), theme.secondary, CRIT_FONT_SIZE)
        } else if event.part_destroyed {
            (
                format!("{:.0}!", event.amount),
                theme.secondary,
                CRIT_FONT_SIZE,
            )
        } else {
            (
                format!("{:.0}", event.amount),
                theme.text_parchment,
                FONT_SIZE,
            )
        };

        commands.spawn((
            Text2d::new(text),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            Transform::from_translation(Vec3::new(
                pos.x + x_offset,
                pos.y + 30.0,
                render_layers::UI_OVERLAY + 1.0,
            )),
            FloatingText {
                lifetime: FLOAT_LIFETIME,
                max_lifetime: FLOAT_LIFETIME,
                velocity: Vec2::new(x_offset * 0.5, FLOAT_SPEED),
            },
        ));
    }

    for event in miss_events.read() {
        let Ok(target_transform) = transforms.get(event.target) else {
            continue;
        };
        let pos = target_transform.translation;

        commands.spawn((
            Text2d::new("Miss"),
            TextFont {
                font_size: FONT_SIZE,
                ..default()
            },
            TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
            Transform::from_translation(Vec3::new(
                pos.x,
                pos.y + 30.0,
                render_layers::UI_OVERLAY + 1.0,
            )),
            FloatingText {
                lifetime: FLOAT_LIFETIME * 0.7,
                max_lifetime: FLOAT_LIFETIME * 0.7,
                velocity: Vec2::new(0.0, FLOAT_SPEED * 0.8),
            },
        ));
    }
}

/// Animate floating text: drift upward and fade out, then despawn.
pub fn animate_floating_text(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut FloatingText, &mut Transform, &mut TextColor)>,
) {
    let dt = time.delta_secs();

    for (entity, mut ft, mut transform, mut color) in &mut query {
        ft.lifetime -= dt;

        if ft.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Drift upward
        transform.translation.x += ft.velocity.x * dt;
        transform.translation.y += ft.velocity.y * dt;

        // Fade out
        let alpha = (ft.lifetime / ft.max_lifetime).clamp(0.0, 1.0);
        let c = color.0.to_srgba();
        color.0 = Color::srgba(c.red, c.green, c.blue, alpha);
    }
}
