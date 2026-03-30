use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::resources::abilities::{
    AbilityRegistry, AbilitySlots, CastTarget, Mana, Stamina, TargetType,
};
use crate::resources::casting::validate_cast;
use crate::resources::damage::EquippedWeapon;
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::input::{Action, ActionState};
use crate::resources::map::{CursorPosition, GridPosition, MapSettings, TileWorld};
use crate::resources::movement::MovePath;
use crate::resources::selection::{DragSelection, HoveredTarget, Selected, TargetingMode};
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::task::{self, CurrentTask, Task, TaskSource};
use crate::resources::task::Engaging;
use crate::systems::camera::MainCamera;
use crate::systems::spawning::PlayerControlled;

/// Visual indicator for selected entities.
#[derive(Component)]
pub struct SelectionRing {
    pub owner: Entity,
}

/// Update `HoveredTarget` from Bevy's picking system each frame.
/// Finds the topmost pickable entity under the cursor.
pub fn update_hovered_target(
    mut hovered: ResMut<HoveredTarget>,
    hover_map: Res<bevy::picking::hover::HoverMap>,
    pickable_query: Query<Entity, With<Pickable>>,
) {
    hovered.entity = None;
    let pointer_id = bevy::picking::pointer::PointerId::Mouse;
    if let Some(hits) = hover_map.get(&pointer_id) {
        for (entity, _hit_data) in hits.iter() {
            if pickable_query.get(*entity).is_ok() {
                hovered.entity = Some(*entity);
                break;
            }
        }
    }
}

/// Handle left-click: drag select, single-click select, or resolve targeting mode.
/// When targeting mode is active, left-click resolves the target into a cast task.
/// Otherwise, left-click performs normal selection (drag or single-click).
pub fn selection_input(
    actions: Res<ActionState>,
    cursor: Res<CursorPosition>,
    hovered: Res<HoveredTarget>,
    mut targeting_mode: ResMut<TargetingMode>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    map_settings: Res<MapSettings>,
    mut drag: ResMut<DragSelection>,
    mut commands: Commands,
    player_entities: Query<(Entity, &GridPosition), With<PlayerControlled>>,
    already_selected: Query<Entity, With<Selected>>,
    targetable_entities: Query<(Entity, &GridPosition, &Faction), Without<PlayerControlled>>,
    caster_positions: Query<&GridPosition>,
    ui_interactions: Query<&Interaction, With<Node>>,
) {
    let Some(cursor_screen) = cursor.screen else { return };

    // Don't process world clicks when cursor is over UI
    if ui_interactions.iter().any(|i| matches!(i, Interaction::Hovered | Interaction::Pressed)) {
        return;
    }

    // Drag tracking (only when not targeting)
    if matches!(*targeting_mode, TargetingMode::None) {
        if actions.just_pressed(Action::Select) {
            drag.begin(cursor_screen);
        }

        if actions.pressed(Action::Select) {
            drag.update(cursor_screen);
        }
    }

    if actions.just_released(Action::Select) {
        // Targeting mode active — resolve the targeting click
        if !matches!(*targeting_mode, TargetingMode::None) {
            resolve_targeting_click(
                &cursor,
                &hovered,
                &mut targeting_mode,
                &mut commands,
                &targetable_entities,
                &caster_positions,
            );
            drag.reset();
            return;
        }

        let Ok((camera, camera_transform)) = camera_query.single() else {
            drag.reset();
            return;
        };

        if drag.active {
            if let Some((min, max)) = drag.rect(cursor_screen) {
                for entity in &already_selected {
                    commands.entity(entity).remove::<Selected>();
                }

                for (entity, grid_pos) in &player_entities {
                    let world_pos = grid_pos.to_world(map_settings.tile_size);
                    let world_3d = Vec3::new(world_pos.x, world_pos.y, 0.0);

                    if let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_3d) {
                        if screen_pos.x >= min.x
                            && screen_pos.x <= max.x
                            && screen_pos.y >= min.y
                            && screen_pos.y <= max.y
                        {
                            commands.entity(entity).insert(Selected);
                        }
                    }
                }
            }
        } else {
            let Some((tile_x, tile_y)) = cursor.tile else {
                drag.reset();
                return;
            };

            for entity in &already_selected {
                commands.entity(entity).remove::<Selected>();
            }

            // Use picking: if a player entity is under cursor, select it
            if let Some(picked) = hovered.entity {
                if player_entities.get(picked).is_ok() {
                    commands.entity(picked).insert(Selected);
                }
            } else {
                // Fallback: tile-based selection for entities without sprites
                for (entity, grid_pos) in &player_entities {
                    if grid_pos.x as i32 == tile_x && grid_pos.y as i32 == tile_y {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }
        }

        drag.reset();
    }
}

/// Resolve a left-click during targeting mode into a cast task.
fn resolve_targeting_click(
    cursor: &CursorPosition,
    hovered: &HoveredTarget,
    targeting_mode: &mut TargetingMode,
    commands: &mut Commands,
    targetable_entities: &Query<(Entity, &GridPosition, &Faction), Without<PlayerControlled>>,
    caster_positions: &Query<&GridPosition>,
) {
    let Some((tile_x, tile_y)) = cursor.tile else { return };
    let tile_x = tile_x as u32;
    let tile_y = tile_y as u32;

    let TargetingMode::AwaitingTarget {
        caster,
        ability_id,
        slot_index,
        target_type,
        ..
    } = &*targeting_mode
    else {
        return;
    };

    let caster_entity = *caster;
    let ab_id = *ability_id;
    let sl_idx = *slot_index;
    let tt = *target_type;

    let caster_pos = match caster_positions.get(caster_entity) {
        Ok(p) => p,
        Err(_) => {
            *targeting_mode = TargetingMode::None;
            return;
        }
    };

    let cast_target = match tt {
        TargetType::SingleEnemy | TargetType::SingleAlly => {
            // Use picking to find target under cursor
            let found = hovered.entity.filter(|e| targetable_entities.get(*e).is_ok());
            match found {
                Some(target) => CastTarget::Entity(target),
                None => return, // No valid target — keep targeting mode active
            }
        }
        TargetType::CircleAoE => CastTarget::Position {
            x: tile_x as f32,
            y: tile_y as f32,
        },
        TargetType::ConeAoE | TargetType::LineAoE => {
            let dx = tile_x as f32 - caster_pos.x as f32;
            let dy = tile_y as f32 - caster_pos.y as f32;
            CastTarget::Direction { dx, dy }
        }
        TargetType::SelfOnly => CastTarget::SelfCast,
    };

    commands.entity(caster_entity).insert(CurrentTask::new(Task::new(
        "cast", 100, TaskSource::Player,
        vec![task::Action::CastAbility {
            ability_id: ab_id,
            slot_index: sl_idx,
            target: cast_target,
            initiated: false,
        }],
    )));

    *targeting_mode = TargetingMode::None;
}

/// Handle right-click: create move or attack task on selected entities.
pub fn right_click_command(
    actions: Res<ActionState>,
    cursor: Res<CursorPosition>,
    tile_world: Res<TileWorld>,
    hovered: Res<HoveredTarget>,
    faction_relations: Res<FactionRelations>,
    mut commands: Commands,
    selected_query: Query<
        (Entity, &Faction, &EquippedWeapon),
        With<Selected>,
    >,
    faction_query: Query<&Faction>,
    ui_interactions: Query<&Interaction, With<Node>>,
) {
    if !actions.just_pressed(Action::Command) {
        return;
    }
    // Don't process world clicks when cursor is over UI
    if ui_interactions.iter().any(|i| matches!(i, Interaction::Hovered | Interaction::Pressed)) {
        return;
    }

    let Some((tile_x, tile_y)) = cursor.tile else { return };

    if tile_x < 0 || tile_y < 0
        || tile_x >= tile_world.width as i32
        || tile_y >= tile_world.height as i32
    {
        return;
    }

    let target_tile = (tile_x as u32, tile_y as u32);

    // Use picking to determine if an attackable entity is under the cursor
    let mut hostile_target = None;
    if let Some(hovered_entity) = hovered.entity {
        if let Ok(target_faction) = faction_query.get(hovered_entity) {
            for (_, selected_faction, _) in &selected_query {
                if !faction_relations.is_friendly(selected_faction.0, target_faction.0) {
                    hostile_target = Some(hovered_entity);
                    break;
                }
            }
        }
    }

    // For move commands, spread selected entities into a formation around the target tile
    // so they don't all path to the same spot.
    let selected_entities: Vec<_> = selected_query.iter().collect();
    let formation_offsets = formation_around(
        target_tile,
        selected_entities.len(),
        &tile_world,
    );

    for (i, (entity, _, weapon)) in selected_entities.into_iter().enumerate() {
        let new_task = if let Some(target) = hostile_target {
            Task::new(
                "attack", 100, TaskSource::Player,
                vec![task::Action::EngageTarget {
                    target,
                    attack_range: weapon.weapon.range,
                }],
            )
        } else {
            let (fx, fy) = formation_offsets[i];
            if tile_world.walk_cost[tile_world.idx(fx, fy)] <= 0.0 {
                continue;
            }
            Task::new(
                "move", 100, TaskSource::Player,
                vec![task::Action::MoveToPosition {
                    x: fx,
                    y: fy,
                }],
            )
        };
        commands.entity(entity).insert(CurrentTask::new(new_task));
    }
}

/// Pick `count` distinct walkable tiles near `center`.
/// Placeholder until a proper formation system is added.
fn formation_around(
    center: (u32, u32),
    count: usize,
    tile_world: &TileWorld,
) -> Vec<(u32, u32)> {
    // Center first, then cardinals, then diagonals
    const OFFSETS: [(i32, i32); 9] = [
        (0, 0), (1, 0), (-1, 0), (0, 1), (0, -1),
        (1, 1), (-1, 1), (1, -1), (-1, -1),
    ];

    let w = tile_world.width as i32;
    let h = tile_world.height as i32;
    let mut result = Vec::with_capacity(count);

    for &(dx, dy) in &OFFSETS {
        let nx = center.0 as i32 + dx;
        let ny = center.1 as i32 + dy;
        if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
        let (ux, uy) = (nx as u32, ny as u32);
        if tile_world.walk_cost[tile_world.idx(ux, uy)] <= 0.0 { continue; }
        result.push((ux, uy));
        if result.len() >= count { break; }
    }

    // Fallback: pad with center if not enough walkable neighbours
    while result.len() < count {
        result.push(center);
    }
    result
}

/// Draw selection indicators as gizmo circles under selected entities.
pub fn draw_selection_indicators(
    theme: Res<crate::resources::theme::Theme>,
    mut gizmos: Gizmos,
    selected: Query<&Transform, With<Selected>>,
) {
    let color = theme.primary.with_alpha(0.4);
    for transform in &selected {
        let pos = transform.translation.truncate() + Vec2::new(0.0, -12.0);
        gizmos.circle_2d(Isometry2d::from_translation(pos), 22.0, color);
    }
}

/// Clean up any legacy selection ring sprites on deselection.
pub fn update_selection_visuals(
    mut commands: Commands,
    mut deselected: RemovedComponents<Selected>,
    rings: Query<(Entity, &SelectionRing)>,
) {
    let deselected_entities: Vec<Entity> = deselected.read().collect();
    for (ring_entity, ring) in &rings {
        if deselected_entities.contains(&ring.owner) {
            commands.entity(ring_entity).despawn();
        }
    }
}

/// Draw the drag selection box.
pub fn draw_drag_box(
    drag: Res<DragSelection>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut gizmos: Gizmos,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    if !drag.active {
        return;
    }

    let Ok(window) = window_query.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Some((min, max)) = drag.rect(cursor_pos) else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };

    let Ok(world_min) = camera.viewport_to_world_2d(camera_transform, min) else { return };
    let Ok(world_max) = camera.viewport_to_world_2d(camera_transform, max) else { return };

    let center = (world_min + world_max) * 0.5;
    let size = world_max - world_min;

    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        size,
        Color::srgba(0.949, 0.792, 0.314, 0.4),
    );
}

/// The 6 ability slot actions in order.
const ABILITY_SLOT_ACTIONS: [Action; 6] = [
    Action::AbilitySlot1,
    Action::AbilitySlot2,
    Action::AbilitySlot3,
    Action::AbilitySlot4,
    Action::AbilitySlot5,
    Action::AbilitySlot6,
];

/// Handle ability hotkeys (Q/E/R/T/F/G) for selected player entities.
pub fn ability_input(
    actions: Res<ActionState>,
    ability_registry: Res<AbilityRegistry>,
    status_registry: Res<StatusEffectRegistry>,
    mut targeting_mode: ResMut<TargetingMode>,
    mut commands: Commands,
    selected_query: Query<
        (
            Entity,
            &AbilitySlots,
            &Mana,
            &Stamina,
            &ActiveStatusEffects,
            &GridPosition,
            Option<&Engaging>,
        ),
        (With<Selected>, With<PlayerControlled>),
    >,
) {
    // Cancel targeting with Escape or right-click
    if actions.just_pressed(Action::Cancel) || actions.just_pressed(Action::Command) {
        if !matches!(*targeting_mode, TargetingMode::None) {
            *targeting_mode = TargetingMode::None;
            return;
        }
    }

    // Check each ability slot hotkey
    for (slot_index, &action) in ABILITY_SLOT_ACTIONS.iter().enumerate() {
        if !actions.just_pressed(action) {
            continue;
        }

        // Use the first selected entity that has this ability slot
        for (entity, slots, mana, stamina, status_effects, _grid_pos, engaging) in &selected_query {
            if slot_index >= slots.abilities.len() {
                continue;
            }

            let ability_id = slots.abilities[slot_index];
            let ability = match ability_registry.get(ability_id) {
                Some(a) => a,
                None => continue,
            };

            // Validate cast prerequisites (cooldown, resources, CC)
            let cc_flags = status_effects.combined_cc_flags(&status_registry);
            if validate_cast(ability, slots, slot_index, mana, stamina, &cc_flags, (0, 0), None, None).is_err() {
                continue;
            }

            let engage_target = engaging.map(|e| e.target);

            match ability.target_type {
                TargetType::SelfOnly => {
                    commands.entity(entity).insert(CurrentTask::new(Task::new(
                        "cast", 100, TaskSource::Player,
                        vec![task::Action::CastAbility {
                            ability_id,
                            slot_index,
                            target: CastTarget::SelfCast,
                            initiated: false,
                        }],
                    )));
                }
                TargetType::SingleEnemy | TargetType::SingleAlly => {
                    if let Some(target) = engage_target {
                        commands.entity(entity).insert(CurrentTask::new(Task::new(
                            "cast", 100, TaskSource::Player,
                            vec![task::Action::CastAbility {
                                ability_id,
                                slot_index,
                                target: CastTarget::Entity(target),
                                initiated: false,
                            }],
                        )));
                    } else {
                        // Enter targeting mode
                        *targeting_mode = TargetingMode::AwaitingTarget {
                            caster: entity,
                            ability_id,
                            slot_index,
                            target_type: ability.target_type,
                            range: ability.range,
                            aoe_radius: 0.0,
                        };
                    }
                }
                TargetType::CircleAoE | TargetType::ConeAoE | TargetType::LineAoE => {
                    // Enter targeting mode for ground-targeted abilities
                    *targeting_mode = TargetingMode::AwaitingTarget {
                        caster: entity,
                        ability_id,
                        slot_index,
                        target_type: ability.target_type,
                        range: ability.range,
                        aoe_radius: ability.aoe_radius,
                    };
                }
            }

            break; // Only use first matching selected entity
        }

        break; // Only process one ability key per frame
    }
}

/// Select a party member by index using F1-F4 hotkeys.
pub fn party_hotkey_select(
    actions: Res<ActionState>,
    mut commands: Commands,
    currently_selected: Query<Entity, With<Selected>>,
    party_members: Query<Entity, (With<PlayerControlled>, Without<crate::resources::combat::Dead>)>,
) {
    let hotkeys = [
        Action::SelectPartyMember1,
        Action::SelectPartyMember2,
        Action::SelectPartyMember3,
        Action::SelectPartyMember4,
    ];

    let mut target_index = None;
    for (i, action) in hotkeys.iter().enumerate() {
        if actions.just_pressed(*action) {
            target_index = Some(i);
            break;
        }
    }
    let Some(idx) = target_index else { return };

    // Get the Nth party member (iteration order is deterministic per spawn order)
    let Some(target_entity) = party_members.iter().nth(idx) else { return };

    // Deselect all, select target
    for entity in &currently_selected {
        commands.entity(entity).remove::<Selected>();
    }
    commands.entity(target_entity).insert(Selected);
}

/// Draw a line from each selected entity to its engage target.
pub fn draw_engage_lines(
    mut gizmos: Gizmos,
    time: Res<Time>,
    selected: Query<(&Transform, &Engaging), With<Selected>>,
    targets: Query<&Transform, Without<Selected>>,
) {
    let pulse = 0.3 + 0.2 * (time.elapsed_secs() * 4.0).sin();
    let color = Color::srgba(1.0, 0.3, 0.2, pulse);

    for (attacker_tf, engaging) in &selected {
        let Ok(target_tf) = targets.get(engaging.target) else { continue };
        let a = attacker_tf.translation.truncate();
        let b = target_tf.translation.truncate();
        gizmos.line_2d(a, b, color);

        // Small crosshair on the target
        let s = 6.0;
        gizmos.line_2d(b + Vec2::new(-s, 0.0), b + Vec2::new(s, 0.0), color);
        gizmos.line_2d(b + Vec2::new(0.0, -s), b + Vec2::new(0.0, s), color);
    }
}

/// Draw movement path lines and destination circles for selected entities.
pub fn draw_move_preview(
    map_settings: Res<MapSettings>,
    time: Res<Time>,
    mut gizmos: Gizmos,
    query: Query<(&GridPosition, &MovePath, &Sprite), With<Selected>>,
) {
    let ts = map_settings.tile_size;
    let line_color = Color::srgba(1.0, 1.0, 1.0, 0.15);

    // Pulsing alpha for destination circle
    let pulse = 0.2 + 0.15 * (time.elapsed_secs() * 3.0).sin();

    for (grid_pos, path, sprite) in &query {
        // Draw path line from current position to each waypoint
        let mut prev = grid_pos.to_world(ts);
        for i in (path.current_index + 1)..path.waypoints.len() {
            let wp = path.waypoints[i];
            let next = Vec2::new(wp.0 as f32 * ts, wp.1 as f32 * ts);
            gizmos.line_2d(prev, next, line_color);
            prev = next;
        }

        // Draw pulsing circle at destination using the entity's tint color
        if let Some(dest) = path.destination() {
            let dest_pos = Vec2::new(dest.0 as f32 * ts, dest.1 as f32 * ts);
            let base_color = sprite.color.to_srgba();
            let circle_color = Color::srgba(base_color.red, base_color.green, base_color.blue, pulse);
            gizmos.circle_2d(Isometry2d::from_translation(dest_pos), ts * 0.4, circle_color);
        }
    }
}

/// Draw a subtle highlight circle on the entity under the cursor.
pub fn draw_hover_highlight(
    hovered: Res<HoveredTarget>,
    mut gizmos: Gizmos,
    transforms: Query<&Transform>,
) {
    let Some(entity) = hovered.entity else { return };
    let Ok(transform) = transforms.get(entity) else { return };
    let pos = transform.translation.truncate();
    let color = Color::srgba(1.0, 1.0, 1.0, 0.25);
    gizmos.circle_2d(Isometry2d::from_translation(pos), 28.0, color);
}

