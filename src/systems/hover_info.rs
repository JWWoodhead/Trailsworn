use bevy::prelude::*;

use crate::resources::body::{Body, BodyTemplates, Health};
use crate::resources::damage::EquippedWeapon;
use crate::resources::map::CursorPosition;
use crate::resources::stats::Attributes;
use crate::resources::theme::Theme;
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
    cursor: Res<CursorPosition>,
    _body_templates: Res<BodyTemplates>,
    entities: Query<(
        &crate::resources::map::GridPosition,
        &EntityName,
        &Body,
        &Health,
        &Attributes,
        Option<&EquippedWeapon>,
    )>,
    mut tooltip_query: Query<&mut Node, With<HoverTooltip>>,
    mut text_query: Query<&mut Text, With<HoverTooltipText>>,
) {
    let Some((tile_x, tile_y)) = cursor.tile else {
        hide_tooltip(&mut tooltip_query);
        return;
    };

    // Find entity at this tile
    let mut found = None;
    for (grid_pos, name, body, health, attrs, weapon) in &entities {
        if grid_pos.x as i32 == tile_x && grid_pos.y as i32 == tile_y {
            found = Some((name, body, health, attrs, weapon));
            break;
        }
    }

    let Ok(mut node) = tooltip_query.single_mut() else {
        return;
    };

    match found {
        Some((name, _body, health, attrs, weapon)) => {
            let hp_fraction = health.fraction();
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

            // Position tooltip near cursor (screen-space)
            if let Some(screen_pos) = cursor.screen {
                node.left = Val::Px(screen_pos.x + 16.0);
                node.top = Val::Px(screen_pos.y + 16.0);
            }
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
