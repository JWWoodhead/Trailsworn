use bevy::prelude::*;

use crate::resources::abilities::{AbilityRegistry, AbilitySlots, CastingState, Mana, Stamina};
use crate::resources::selection::{Selected, TargetingMode};
use crate::resources::theme::Theme;
use crate::systems::spawning::PlayerControlled;

/// Marker for the ability bar root node.
#[derive(Component)]
pub struct AbilityBarRoot;

/// Marker for an individual ability slot in the bar.
#[derive(Component)]
pub struct AbilitySlotUi {
    pub slot_index: usize,
}

/// Marker for the cooldown overlay within a slot.
#[derive(Component)]
pub struct CooldownOverlay {
    pub slot_index: usize,
}

/// Marker for the ability name text within a slot.
#[derive(Component)]
pub struct AbilitySlotText {
    pub slot_index: usize,
}

/// Marker for the cast bar root.
#[derive(Component)]
pub struct CastBarRoot;

/// Marker for the cast bar fill.
#[derive(Component)]
pub struct CastBarFill;

/// Marker for the cast bar text.
#[derive(Component)]
pub struct CastBarText;

/// Marker for the mana bar fill.
#[derive(Component)]
pub struct ManaBarFill;

/// Marker for the stamina bar fill.
#[derive(Component)]
pub struct StaminaBarFill;

/// Marker for the resource bars root.
#[derive(Component)]
pub struct ResourceBarsRoot;

const SLOT_SIZE: f32 = 48.0;
const SLOT_GAP: f32 = 4.0;
const NUM_SLOTS: usize = 6;
const HOTKEY_LABELS: [&str; 6] = ["Q", "E", "R", "T", "F", "G"];

const MANA_COLOR: Color = Color::srgb(0.2, 0.4, 0.8);
const STAMINA_COLOR: Color = Color::srgb(0.3, 0.7, 0.3);

const BAR_WIDTH: f32 = 200.0;
const BAR_HEIGHT: f32 = 8.0;

/// Setup the ability bar, cast bar, and resource bars. Called once at startup.
pub fn setup_ability_bar(mut commands: Commands, theme: Res<Theme>) {
    // --- Ability Bar (bottom center) ---
    let bar_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                left: Val::Percent(50.0),
                // Use margin to center
                margin: UiRect::left(Val::Px(
                    -((SLOT_SIZE * NUM_SLOTS as f32 + SLOT_GAP * (NUM_SLOTS - 1) as f32) / 2.0),
                )),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(SLOT_GAP),
                ..default()
            },
            AbilityBarRoot,
        ))
        .id();

    for i in 0..NUM_SLOTS {
        let slot = commands
            .spawn((
                Node {
                    width: Val::Px(SLOT_SIZE),
                    height: Val::Px(SLOT_SIZE),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(theme.surface),
                AbilitySlotUi { slot_index: i },
            ))
            .id();

        // Hotkey label (top-left corner)
        let hotkey = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(2.0),
                    top: Val::Px(1.0),
                    ..default()
                },
                Text::new(HOTKEY_LABELS[i]),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.961, 0.961, 0.863, 0.5)),
            ))
            .id();

        // Ability name text (centered)
        let name_text = commands
            .spawn((
                Text::new(""),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(theme.text_parchment),
                AbilitySlotText { slot_index: i },
            ))
            .id();

        // Cooldown overlay (fills from bottom to top)
        let cooldown = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(0.0), // 0% when ready
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                CooldownOverlay { slot_index: i },
            ))
            .id();

        commands.entity(slot).add_children(&[hotkey, name_text, cooldown]);
        commands.entity(bar_root).add_child(slot);
    }

    // --- Cast Bar (above ability bar) ---
    let cast_bar_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(72.0), // above ability bar
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-BAR_WIDTH / 2.0)),
                width: Val::Px(BAR_WIDTH),
                height: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                display: Display::None, // hidden by default
                ..default()
            },
            BackgroundColor(theme.surface),
            CastBarRoot,
        ))
        .id();

    let cast_fill = commands
        .spawn((
            Node {
                width: Val::Percent(0.0),
                height: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(theme.primary),
            CastBarFill,
        ))
        .id();

    let cast_text = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Text::new(""),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(theme.text_parchment),
            CastBarText,
        ))
        .id();

    commands.entity(cast_bar_root).add_children(&[cast_fill, cast_text]);

    // --- Resource Bars (above cast bar) ---
    let resource_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(92.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-BAR_WIDTH / 2.0)),
                width: Val::Px(BAR_WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                display: Display::None,
                ..default()
            },
            ResourceBarsRoot,
        ))
        .id();

    // Mana bar
    let mana_bg = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(BAR_HEIGHT),
                ..default()
            },
            BackgroundColor(Color::srgba(0.075, 0.075, 0.075, 0.85)),
        ))
        .id();
    let mana_fill = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(MANA_COLOR),
            ManaBarFill,
        ))
        .id();
    commands.entity(mana_bg).add_child(mana_fill);

    // Stamina bar
    let stam_bg = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(BAR_HEIGHT),
                ..default()
            },
            BackgroundColor(Color::srgba(0.075, 0.075, 0.075, 0.85)),
        ))
        .id();
    let stam_fill = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(STAMINA_COLOR),
            StaminaBarFill,
        ))
        .id();
    commands.entity(stam_bg).add_child(stam_fill);

    commands.entity(resource_root).add_children(&[mana_bg, stam_bg]);
}

/// Update ability bar slot contents based on the first selected player entity.
pub fn update_ability_bar(
    ability_registry: Res<AbilityRegistry>,
    theme: Res<Theme>,
    selected: Query<(&AbilitySlots, &Mana, &Stamina), (With<Selected>, With<PlayerControlled>)>,
    mut slot_bgs: Query<(&AbilitySlotUi, &mut BackgroundColor)>,
    mut slot_texts: Query<(&AbilitySlotText, &mut Text, &mut TextColor)>,
    mut cooldown_overlays: Query<(&CooldownOverlay, &mut Node), Without<AbilitySlotUi>>,
    targeting_mode: Res<TargetingMode>,
) {
    let selected_data = selected.iter().next();

    for (slot_ui, mut bg) in &mut slot_bgs {
        let i = slot_ui.slot_index;

        let (has_ability, on_cooldown, affordable) = if let Some((slots, mana, stamina)) = selected_data {
            if i < slots.abilities.len() {
                let ability_id = slots.abilities[i];
                if let Some(ability) = ability_registry.get(ability_id) {
                    let can_afford = (ability.mana_cost == 0 || mana.current >= ability.mana_cost as f32)
                        && (ability.stamina_cost == 0 || stamina.current >= ability.stamina_cost as f32);
                    (true, !slots.is_ready(i), can_afford)
                } else {
                    (false, false, false)
                }
            } else {
                (false, false, false)
            }
        } else {
            (false, false, false)
        };

        // Check if this slot is the one being targeted
        let is_targeting = matches!(
            &*targeting_mode,
            TargetingMode::AwaitingTarget { slot_index, .. } if *slot_index == i
        );

        // Background color
        if is_targeting {
            bg.0 = theme.primary_container;
        } else if !has_ability {
            bg.0 = theme.surface;
        } else if on_cooldown || !affordable {
            bg.0 = Color::srgb(0.05, 0.05, 0.05);
        } else {
            bg.0 = theme.surface_bright;
        }
    }

    for (slot_text, mut text, mut color) in &mut slot_texts {
        let i = slot_text.slot_index;

        if let Some((slots, _mana, _stamina)) = selected_data {
            if i < slots.abilities.len() {
                let ability_id = slots.abilities[i];
                if let Some(ability) = ability_registry.get(ability_id) {
                    // Abbreviate: first 5 chars of ability name
                    let abbrev: String = ability.name.chars().take(5).collect();
                    **text = abbrev;
                    color.0 = theme.text_parchment;
                    continue;
                }
            }
        }
        **text = String::new();
        color.0 = theme.text_parchment;
    }

    // Update cooldown overlays
    for (cd_overlay, mut node) in &mut cooldown_overlays {
        let i = cd_overlay.slot_index;
        if let Some((slots, _, _)) = selected_data {
            if i < slots.cooldowns.len() && slots.cooldowns[i] > 0 {
                let ability_id = slots.abilities[i];
                if let Some(ability) = ability_registry.get(ability_id) {
                    let fraction = slots.cooldowns[i] as f32 / ability.cooldown_ticks as f32;
                    node.height = Val::Percent(fraction * 100.0);
                    continue;
                }
            }
        }
        node.height = Val::Percent(0.0);
    }
}

/// Update cast bar visibility and progress.
pub fn update_cast_bar(
    ability_registry: Res<AbilityRegistry>,
    selected: Query<&CastingState, (With<Selected>, With<PlayerControlled>)>,
    mut cast_root: Query<&mut Node, With<CastBarRoot>>,
    mut cast_fill: Query<&mut Node, (With<CastBarFill>, Without<CastBarRoot>)>,
    mut cast_text: Query<&mut Text, With<CastBarText>>,
) {
    let casting = selected.iter().next();
    let Ok(mut root_node) = cast_root.single_mut() else { return };

    match casting {
        Some(state) => {
            if let Some(ability) = ability_registry.get(state.ability_id) {
                if ability.cast_time_ticks == 0 {
                    root_node.display = Display::None;
                    return;
                }

                root_node.display = Display::Flex;

                let progress = if ability.cast_time_ticks > 0 {
                    1.0 - (state.remaining_ticks as f32 / ability.cast_time_ticks as f32)
                } else {
                    1.0
                };

                if let Ok(mut fill_node) = cast_fill.single_mut() {
                    fill_node.width = Val::Percent(progress * 100.0);
                }
                if let Ok(mut text) = cast_text.single_mut() {
                    **text = ability.name.clone();
                }
            } else {
                root_node.display = Display::None;
            }
        }
        None => {
            root_node.display = Display::None;
        }
    }
}

/// Update mana and stamina bars for the first selected player entity.
pub fn update_resource_bars(
    selected: Query<(&Mana, &Stamina), (With<Selected>, With<PlayerControlled>)>,
    mut resource_root: Query<&mut Node, With<ResourceBarsRoot>>,
    mut mana_fill: Query<&mut Node, (With<ManaBarFill>, Without<ResourceBarsRoot>, Without<StaminaBarFill>)>,
    mut stam_fill: Query<&mut Node, (With<StaminaBarFill>, Without<ResourceBarsRoot>, Without<ManaBarFill>)>,
) {
    let Ok(mut root_node) = resource_root.single_mut() else { return };

    if let Some((mana, stamina)) = selected.iter().next() {
        root_node.display = Display::Flex;

        let mana_frac = if mana.max > 0.0 { mana.current / mana.max } else { 0.0 };
        let stam_frac = if stamina.max > 0.0 { stamina.current / stamina.max } else { 0.0 };

        if let Ok(mut mana_node) = mana_fill.single_mut() {
            mana_node.width = Val::Percent(mana_frac * 100.0);
        }
        if let Ok(mut stam_node) = stam_fill.single_mut() {
            stam_node.width = Val::Percent(stam_frac * 100.0);
        }
    } else {
        root_node.display = Display::None;
    }
}

/// Draw a targeting reticle when in targeting mode.
pub fn draw_targeting_reticle(
    targeting_mode: Res<TargetingMode>,
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::systems::camera::MainCamera>>,
    map_settings: Res<crate::resources::map::MapSettings>,
    mut gizmos: Gizmos,
) {
    let TargetingMode::AwaitingTarget { aoe_radius, .. } = &*targeting_mode else {
        return;
    };

    let Ok(window) = window_query.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return };

    // Snap to tile center
    let tile_x = (world_pos.x / map_settings.tile_size).round();
    let tile_y = (world_pos.y / map_settings.tile_size).round();
    let center = Vec2::new(tile_x * map_settings.tile_size, tile_y * map_settings.tile_size);

    // Draw crosshair
    let size = map_settings.tile_size * 0.5;
    let color = Color::srgba(0.949, 0.792, 0.314, 0.7);
    gizmos.line_2d(center - Vec2::new(size, 0.0), center + Vec2::new(size, 0.0), color);
    gizmos.line_2d(center - Vec2::new(0.0, size), center + Vec2::new(0.0, size), color);

    // Draw AoE radius if applicable
    if *aoe_radius > 0.0 {
        let radius_px = *aoe_radius * map_settings.tile_size;
        gizmos.circle_2d(
            Isometry2d::from_translation(center),
            radius_px,
            Color::srgba(0.949, 0.792, 0.314, 0.3),
        );
    }
}
