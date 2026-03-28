use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::resources::abilities::{
    AbilityRegistry, AbilitySlots, CastTarget, Mana, Stamina, TargetType,
};
use crate::resources::casting::validate_cast;
use crate::resources::damage::EquippedWeapon;
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::input::{Action, ActionState};
use crate::resources::map::{CursorPosition, GridPosition, MapSettings, TileWorld, render_layers};
use crate::resources::selection::{DragSelection, Selected, TargetingMode};
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::task::{self, CurrentTask, Engaging, Task, TaskSource};
use crate::systems::camera::MainCamera;
use crate::systems::spawning::PlayerControlled;

/// Visual indicator for selected entities.
#[derive(Component)]
pub struct SelectionRing {
    pub owner: Entity,
}

/// Handle left-click: drag select, single-click select, or resolve targeting mode.
/// When targeting mode is active, left-click resolves the target into a cast task.
/// Otherwise, left-click performs normal selection (drag or single-click).
pub fn selection_input(
    actions: Res<ActionState>,
    cursor: Res<CursorPosition>,
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

            for (entity, grid_pos) in &player_entities {
                if grid_pos.x as i32 == tile_x && grid_pos.y as i32 == tile_y {
                    commands.entity(entity).insert(Selected);
                }
            }
        }

        drag.reset();
    }
}

/// Resolve a left-click during targeting mode into a cast task.
fn resolve_targeting_click(
    cursor: &CursorPosition,
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
            let mut found = None;
            for (entity, pos, _) in targetable_entities.iter() {
                if pos.x == tile_x && pos.y == tile_y {
                    found = Some(entity);
                    break;
                }
            }
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
    faction_relations: Res<FactionRelations>,
    mut commands: Commands,
    selected_query: Query<
        (Entity, &Faction, &EquippedWeapon),
        With<Selected>,
    >,
    target_query: Query<(Entity, &GridPosition, &Faction), Without<PlayerControlled>>,
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

    // Check if there's a hostile entity at the clicked tile
    let mut hostile_target = None;
    for (target_entity, target_pos, target_faction) in &target_query {
        if target_pos.x as i32 == tile_x && target_pos.y as i32 == tile_y {
            for (_, selected_faction, _) in &selected_query {
                if faction_relations.is_hostile(selected_faction.0, target_faction.0) {
                    hostile_target = Some(target_entity);
                    break;
                }
            }
            if hostile_target.is_some() {
                break;
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

/// Spawn/despawn selection ring sprites on selected entities.
pub fn update_selection_visuals(
    mut commands: Commands,
    theme: Res<crate::resources::theme::Theme>,
    selected: Query<Entity, Added<Selected>>,
    mut deselected: RemovedComponents<Selected>,
    rings: Query<(Entity, &SelectionRing)>,
) {
    for entity in &selected {
        let ring = commands
            .spawn((
                Sprite {
                    color: theme.primary.with_alpha(0.5),
                    custom_size: Some(Vec2::new(60.0, 60.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, 0.0, render_layers::ENTITIES - 0.1)),
                SelectionRing { owner: entity },
            ))
            .id();
        commands.entity(entity).add_child(ring);
    }

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
            if validate_cast(ability, slots, slot_index, mana, stamina, &cc_flags, (0, 0), None).is_err() {
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

