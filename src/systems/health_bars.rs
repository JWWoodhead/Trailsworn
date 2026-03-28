use bevy::prelude::*;

use crate::resources::body::{Body, BodyTemplates};
use crate::resources::combat::Dead;
use crate::resources::map::render_layers;
use crate::resources::theme::Theme;

/// Marker for the health bar background sprite.
#[derive(Component)]
pub struct HealthBarBackground;

/// Marker for the health bar fill sprite. Child of the background.
#[derive(Component)]
pub struct HealthBarFill {
    pub owner: Entity,
}

const BAR_WIDTH: f32 = 48.0;
const BAR_HEIGHT: f32 = 6.0;
const BAR_Y_OFFSET: f32 = 40.0;

/// Spawn health bars for entities that have a Body but no health bar yet.
pub fn spawn_health_bars(
    mut commands: Commands,
    theme: Res<Theme>,
    query: Query<Entity, (With<Body>, Without<HealthBarBackground>, Without<Dead>)>,
) {
    for entity in &query {
        let bg = commands
            .spawn((
                Sprite {
                    color: theme.hp_bar_bg,
                    custom_size: Some(Vec2::new(BAR_WIDTH, BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, BAR_Y_OFFSET, render_layers::UI_OVERLAY)),
                HealthBarBackground,
            ))
            .id();

        let fill = commands
            .spawn((
                Sprite {
                    color: theme.hp_full,
                    custom_size: Some(Vec2::new(BAR_WIDTH, BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                HealthBarFill { owner: entity },
            ))
            .id();

        commands.entity(bg).add_child(fill);
        commands.entity(entity).insert(HealthBarBackground).add_child(bg);
    }
}

/// Update health bar fill width and color based on current HP.
pub fn update_health_bars(
    theme: Res<Theme>,
    body_templates: Res<BodyTemplates>,
    body_query: Query<&Body>,
    mut fill_query: Query<(&HealthBarFill, &mut Sprite, &mut Transform)>,
) {
    for (fill, mut sprite, mut transform) in &mut fill_query {
        let Ok(body) = body_query.get(fill.owner) else {
            continue;
        };
        let template = match body_templates.get(&body.template_id) {
            Some(t) => t,
            None => continue,
        };

        let hp_fraction = (1.0 - body.pain_level(template)).clamp(0.0, 1.0);

        let fill_width = BAR_WIDTH * hp_fraction;
        sprite.custom_size = Some(Vec2::new(fill_width, BAR_HEIGHT));
        transform.translation.x = (fill_width - BAR_WIDTH) * 0.5;
        sprite.color = theme.hp_color(hp_fraction);
    }
}

/// Remove health bars for dead or despawned entities.
pub fn cleanup_orphaned_health_bars(
    mut commands: Commands,
    fill_query: Query<(Entity, &HealthBarFill)>,
    body_query: Query<&Body, Without<Dead>>,
) {
    for (fill_entity, fill) in &fill_query {
        // Despawn health bar if owner is despawned OR dead
        if body_query.get(fill.owner).is_err() {
            commands.entity(fill_entity).despawn();
        }
    }
}
