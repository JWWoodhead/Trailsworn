use bevy::prelude::*;

use crate::resources::abilities::{AbilityRegistry, CastingState};
use crate::resources::map::render_layers;
use crate::resources::theme::Theme;

/// Marker on the entity itself to prevent re-spawning the cast bar.
#[derive(Component)]
pub struct HasCastBar;

/// Marker for the cast bar background sprite (child of the caster entity).
#[derive(Component)]
pub struct CastBarBackground {
    pub owner: Entity,
}

/// Marker for the cast bar fill sprite (child of the background).
#[derive(Component)]
pub struct CastBarFill {
    pub owner: Entity,
}

/// Marker for the cast bar ability name text.
#[derive(Component)]
pub struct CastBarText {
    pub owner: Entity,
}

const BAR_WIDTH: f32 = 48.0;
const BAR_HEIGHT: f32 = 4.0;
const BAR_Y_OFFSET: f32 = 50.0; // Above health bar (which is at 40.0)

/// Spawn cast bars for entities that are casting but don't have a cast bar yet.
pub fn spawn_cast_bars(
    mut commands: Commands,
    theme: Res<Theme>,
    ability_registry: Res<AbilityRegistry>,
    query: Query<(Entity, &CastingState), Without<HasCastBar>>,
) {
    for (entity, casting) in &query {
        // Skip instant casts (cast_time == 0, they resolve immediately)
        let ability = match ability_registry.get(casting.ability_id) {
            Some(a) => a,
            None => continue,
        };
        if ability.cast_time_ticks == 0 {
            continue;
        }

        // Background bar
        let bg = commands
            .spawn((
                Sprite {
                    color: theme.hp_bar_bg,
                    custom_size: Some(Vec2::new(BAR_WIDTH, BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, BAR_Y_OFFSET, render_layers::UI_OVERLAY)),
                CastBarBackground { owner: entity },
            ))
            .id();

        // Fill bar (orange/amber for casting)
        let fill = commands
            .spawn((
                Sprite {
                    color: Color::srgb(0.9, 0.6, 0.2),
                    custom_size: Some(Vec2::new(0.0, BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                CastBarFill { owner: entity },
            ))
            .id();

        // Ability name text above the bar
        let text = commands
            .spawn((
                Text2d::new(ability.name.clone()),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(theme.text_parchment),
                Transform::from_translation(Vec3::new(0.0, BAR_HEIGHT + 2.0, 0.1)),
                CastBarText { owner: entity },
            ))
            .id();

        commands.entity(bg).add_child(fill);
        commands.entity(bg).add_child(text);
        commands.entity(entity).insert(HasCastBar).add_child(bg);
    }
}

/// Update cast bar fill based on casting progress.
pub fn update_cast_bars(
    ability_registry: Res<AbilityRegistry>,
    casting_query: Query<&CastingState>,
    mut fill_query: Query<(&CastBarFill, &mut Sprite, &mut Transform)>,
) {
    for (fill, mut sprite, mut transform) in &mut fill_query {
        let Ok(casting) = casting_query.get(fill.owner) else {
            continue;
        };
        let ability = match ability_registry.get(casting.ability_id) {
            Some(a) => a,
            None => continue,
        };
        if ability.cast_time_ticks == 0 {
            continue;
        }

        let progress = 1.0 - (casting.remaining_ticks as f32 / ability.cast_time_ticks as f32);
        let fill_width = BAR_WIDTH * progress.clamp(0.0, 1.0);
        sprite.custom_size = Some(Vec2::new(fill_width, BAR_HEIGHT));
        transform.translation.x = (fill_width - BAR_WIDTH) * 0.5;
    }
}

/// Remove cast bars when the entity stops casting.
pub fn cleanup_cast_bars(
    mut commands: Commands,
    bg_query: Query<(Entity, &CastBarBackground)>,
    casting_query: Query<&CastingState>,
    has_bar_query: Query<Entity, With<HasCastBar>>,
) {
    for (bg_entity, bg) in &bg_query {
        if casting_query.get(bg.owner).is_err() {
            commands.entity(bg_entity).despawn();
            // Remove the marker so a new cast bar can be spawned on next cast
            if has_bar_query.get(bg.owner).is_ok() {
                commands.entity(bg.owner).remove::<HasCastBar>();
            }
        }
    }
}
