use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::resources::body::{Body, BodyTemplates};
use crate::resources::damage::EquippedWeapon;
use crate::resources::map::{GridPosition, MapSettings};
use crate::resources::stats::Attributes;
use crate::resources::theme::Theme;
use crate::systems::camera::MainCamera;
use crate::systems::spawning::EntityName;

/// Marker for the hover tooltip root node.
#[derive(Component)]
pub struct HoverTooltip;

/// Marker for the tooltip text.
#[derive(Component)]
pub struct HoverTooltipText;

/// Spawn the tooltip UI container (hidden by default).
pub fn setup_hover_tooltip(mut commands: Commands, theme: Res<Theme>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::all(Val::Px(8.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(theme.surface),
            HoverTooltip,
        ))
        .with_child((
            Text::new(""),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(theme.text_parchment),
            HoverTooltipText,
        ));
}

/// Update the hover tooltip based on what entity the cursor is over.
pub fn update_hover_tooltip(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    map_settings: Res<MapSettings>,
    body_templates: Res<BodyTemplates>,
    entities: Query<(
        &GridPosition,
        &EntityName,
        &Body,
        &Attributes,
        Option<&EquippedWeapon>,
    )>,
    mut tooltip_query: Query<&mut Node, With<HoverTooltip>>,
    mut text_query: Query<&mut Text, With<HoverTooltipText>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        hide_tooltip(&mut tooltip_query);
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        hide_tooltip(&mut tooltip_query);
        return;
    };

    let tile_x = (world_pos.x / map_settings.tile_size).round() as i32;
    let tile_y = (world_pos.y / map_settings.tile_size).round() as i32;

    // Find entity at this tile
    let mut found = None;
    for (grid_pos, name, body, attrs, weapon) in &entities {
        if grid_pos.x as i32 == tile_x && grid_pos.y as i32 == tile_y {
            found = Some((name, body, attrs, weapon));
            break;
        }
    }

    let Ok(mut node) = tooltip_query.single_mut() else {
        return;
    };

    match found {
        Some((name, body, attrs, weapon)) => {
            let template = match body_templates.get(&body.template_id) {
                Some(t) => t,
                None => return,
            };

            let hp_fraction = 1.0 - body.pain_level(template);
            let hp_pct = (hp_fraction * 100.0).round();

            let weapon_str = weapon
                .map(|w| format!("\n{} ({:.0} dmg)", w.weapon.name, w.weapon.base_damage))
                .unwrap_or_default();

            let info = format!(
                "{}\nHP: {:.0}%\nSTR:{} AGI:{} INT:{}\nTOU:{} WIL:{}{}",
                name.0,
                hp_pct,
                attrs.strength,
                attrs.agility,
                attrs.intellect,
                attrs.toughness,
                attrs.willpower,
                weapon_str,
            );

            if let Ok(mut text) = text_query.single_mut() {
                **text = info;
            }

            node.left = Val::Px(cursor_pos.x + 16.0);
            node.top = Val::Px(cursor_pos.y + 16.0);
            node.display = Display::Flex;
        }
        None => {
            node.display = Display::None;
        }
    }
}

fn hide_tooltip(tooltip_query: &mut Query<&mut Node, With<HoverTooltip>>) {
    if let Ok(mut node) = tooltip_query.single_mut() {
        node.display = Display::None;
    }
}
