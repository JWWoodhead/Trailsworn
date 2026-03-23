use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::resources::abilities::{
    AbilityRegistry, AbilitySlots, CastTarget, CastingState, Mana, Stamina, TargetType,
};
use crate::resources::ai::{AbilityTarget, AiState, MovementIntent, PlayerCommand};
use crate::resources::damage::EquippedWeapon;
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::input::{Action, ActionState};
use crate::resources::map::{GridPosition, MapSettings, TileWorld, render_layers};
use crate::resources::selection::{DragSelection, Selected, TargetingMode};
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::systems::camera::MainCamera;
use crate::systems::spawning::PlayerControlled;

/// Visual indicator for selected entities.
#[derive(Component)]
pub struct SelectionRing {
    pub owner: Entity,
}

/// Handle left-click: targeting mode resolution, drag select, or single-click select.
pub fn selection_input(
    actions: Res<ActionState>,
    ability_registry: Res<AbilityRegistry>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    map_settings: Res<MapSettings>,
    mut drag: ResMut<DragSelection>,
    mut targeting_mode: ResMut<TargetingMode>,
    mut commands: Commands,
    player_entities: Query<(Entity, &GridPosition), With<PlayerControlled>>,
    already_selected: Query<Entity, With<Selected>>,
    targetable_entities: Query<(Entity, &GridPosition, &Faction), Without<PlayerControlled>>,
    caster_positions: Query<&GridPosition>,
) {
    let Ok(window) = window_query.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };

    // --- Normal selection: drag tracking (only when not targeting) ---
    if matches!(*targeting_mode, TargetingMode::None) {
        if actions.just_pressed(Action::Select) {
            drag.begin(cursor_pos);
        }

        if actions.pressed(Action::Select) {
            drag.update(cursor_pos);
        }
    }

    if actions.just_released(Action::Select) {
        // --- Targeting mode: resolve the target on release ---
        if !matches!(*targeting_mode, TargetingMode::None) {
            resolve_targeting_click(
                cursor_pos,
                &ability_registry,
                &camera_query,
                &map_settings,
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
            if let Some((min, max)) = drag.rect(cursor_pos) {
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
            let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
                drag.reset();
                return;
            };

            let tile_x = (world_pos.x / map_settings.tile_size).round() as i32;
            let tile_y = (world_pos.y / map_settings.tile_size).round() as i32;

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

/// Resolve a targeting-mode click into a cast command.
fn resolve_targeting_click(
    cursor_pos: Vec2,
    ability_registry: &AbilityRegistry,
    camera_query: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    map_settings: &MapSettings,
    targeting_mode: &mut ResMut<TargetingMode>,
    commands: &mut Commands,
    targetable_entities: &Query<(Entity, &GridPosition, &Faction), Without<PlayerControlled>>,
    caster_positions: &Query<&GridPosition>,
) {
    let TargetingMode::AwaitingTarget {
        caster,
        ability_id,
        slot_index,
        target_type,
        ..
    } = &**targeting_mode
    else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return };

    let tile_x = (world_pos.x / map_settings.tile_size).round() as u32;
    let tile_y = (world_pos.y / map_settings.tile_size).round() as u32;

    let caster_entity = *caster;
    let ab_id = *ability_id;
    let sl_idx = *slot_index;
    let tt = *target_type;

    let caster_pos = match caster_positions.get(caster_entity) {
        Ok(p) => p,
        Err(_) => {
            **targeting_mode = TargetingMode::None;
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

    let ability_target = match &cast_target {
        CastTarget::SelfCast => AbilityTarget::SelfCast,
        CastTarget::Entity(e) => AbilityTarget::Entity(*e),
        CastTarget::Position { x, y } => AbilityTarget::Position { x: *x, y: *y },
        CastTarget::Direction { dx, dy } => AbilityTarget::Direction { dx: *dx, dy: *dy },
    };

    let cast_time = ability_registry.get(ab_id).map_or(0, |a| a.cast_time_ticks);
    commands.entity(caster_entity).insert(CastingState {
        ability_id: ab_id,
        slot_index: sl_idx,
        remaining_ticks: cast_time,
        target: cast_target,
    });
    commands.entity(caster_entity).insert(PlayerCommand::CastAbility {
        ability_id: ab_id,
        slot_index: sl_idx,
        target: ability_target,
    });

    **targeting_mode = TargetingMode::None;
}

/// Handle right-click: set MovementIntent (move or attack) on selected entities.
/// Does NOT do pathfinding — resolve_movement_intent handles that for everyone.
pub fn right_click_command(
    actions: Res<ActionState>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    tile_world: Res<TileWorld>,
    map_settings: Res<MapSettings>,
    faction_relations: Res<FactionRelations>,
    mut commands: Commands,
    mut selected_query: Query<
        (Entity, &Faction, &EquippedWeapon, &mut MovementIntent, &mut AiState),
        With<Selected>,
    >,
    target_query: Query<(Entity, &GridPosition, &Faction), Without<PlayerControlled>>,
) {
    if !actions.just_pressed(Action::Command) {
        return;
    }

    let Ok(window) = window_query.single() else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return };

    let tile_x = (world_pos.x / map_settings.tile_size).round() as i32;
    let tile_y = (world_pos.y / map_settings.tile_size).round() as i32;

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
            for (_, selected_faction, _, _, _) in &selected_query {
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

    for (entity, _, weapon, mut intent, mut ai_state) in &mut selected_query {
        if let Some(target) = hostile_target {
            // Attack: set intent to move within weapon range, set AI state to engage
            *intent = MovementIntent::MoveToEntity {
                target,
                desired_range: weapon.weapon.range,
            };
            *ai_state = AiState::Engaging { target };
            commands.entity(entity).insert(PlayerCommand::Attack(target));
        } else {
            // Move: set intent to move to position
            if tile_world.walk_cost[tile_world.idx(target_tile.0, target_tile.1)] <= 0.0 {
                continue;
            }

            *intent = MovementIntent::MoveToPosition {
                x: target_tile.0,
                y: target_tile.1,
            };
            *ai_state = AiState::Idle;
            commands.entity(entity).remove::<PlayerCommand>();
        }
    }
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
            &AiState,
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
        for (entity, slots, mana, stamina, status_effects, _grid_pos, ai_state) in &selected_query {
            if slot_index >= slots.abilities.len() {
                continue;
            }

            let ability_id = slots.abilities[slot_index];
            let ability = match ability_registry.get(ability_id) {
                Some(a) => a,
                None => continue,
            };

            // Quick validation (without target position for now)
            let cc_flags = status_effects.combined_cc_flags(&status_registry);
            if !cc_flags.can_cast() || !slots.is_ready(slot_index) {
                continue;
            }
            if ability.mana_cost > 0 && mana.current < ability.mana_cost as f32 {
                continue;
            }
            if ability.stamina_cost > 0 && stamina.current < ability.stamina_cost as f32 {
                continue;
            }

            match ability.target_type {
                TargetType::SelfOnly => {
                    // Instant self-target: begin cast immediately
                    commands.entity(entity).insert(CastingState {
                        ability_id,
                        slot_index,
                        remaining_ticks: ability.cast_time_ticks,
                        target: CastTarget::SelfCast,
                    });
                    commands.entity(entity).insert(PlayerCommand::CastAbility {
                        ability_id,
                        slot_index,
                        target: AbilityTarget::SelfCast,
                    });
                }
                TargetType::SingleEnemy | TargetType::SingleAlly => {
                    // If already engaging a target, use that target
                    if let AiState::Engaging { target } = ai_state {
                        let target_entity = *target;
                        commands.entity(entity).insert(CastingState {
                            ability_id,
                            slot_index,
                            remaining_ticks: ability.cast_time_ticks,
                            target: CastTarget::Entity(target_entity),
                        });
                        commands.entity(entity).insert(PlayerCommand::CastAbility {
                            ability_id,
                            slot_index,
                            target: AbilityTarget::Entity(target_entity),
                        });
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

/// Resolve targeting mode clicks: left-click picks target, right-click/escape cancels.
pub fn resolve_targeting(
    actions: Res<ActionState>,
    ability_registry: Res<AbilityRegistry>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    map_settings: Res<MapSettings>,
    mut targeting_mode: ResMut<TargetingMode>,
    mut commands: Commands,
    entities_at_tile: Query<(Entity, &GridPosition, &Faction), Without<PlayerControlled>>,
    caster_query: Query<&GridPosition>,
) {
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

    if !actions.just_pressed(Action::Select) {
        return;
    }

    let Ok(window) = window_query.single() else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return };

    let tile_x = (world_pos.x / map_settings.tile_size).round() as u32;
    let tile_y = (world_pos.y / map_settings.tile_size).round() as u32;

    let caster_entity = *caster;
    let ab_id = *ability_id;
    let sl_idx = *slot_index;
    let tt = *target_type;

    // Get caster position for direction-based targeting
    let caster_pos = match caster_query.get(caster_entity) {
        Ok(p) => p,
        Err(_) => {
            *targeting_mode = TargetingMode::None;
            return;
        }
    };

    let cast_target = match tt {
        TargetType::SingleEnemy | TargetType::SingleAlly => {
            // Find entity at clicked tile
            let mut found = None;
            for (entity, pos, _) in &entities_at_tile {
                if pos.x == tile_x && pos.y == tile_y {
                    found = Some(entity);
                    break;
                }
            }
            match found {
                Some(target) => CastTarget::Entity(target),
                None => return, // No valid target at tile, keep targeting mode active
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

    let ability_target = match &cast_target {
        CastTarget::SelfCast => AbilityTarget::SelfCast,
        CastTarget::Entity(e) => AbilityTarget::Entity(*e),
        CastTarget::Position { x, y } => AbilityTarget::Position { x: *x, y: *y },
        CastTarget::Direction { dx, dy } => AbilityTarget::Direction { dx: *dx, dy: *dy },
    };

    let cast_time = ability_registry.get(ab_id).map_or(0, |a| a.cast_time_ticks);
    commands.entity(caster_entity).insert(CastingState {
        ability_id: ab_id,
        slot_index: sl_idx,
        remaining_ticks: cast_time,
        target: cast_target,
    });
    commands.entity(caster_entity).insert(PlayerCommand::CastAbility {
        ability_id: ab_id,
        slot_index: sl_idx,
        target: ability_target,
    });

    *targeting_mode = TargetingMode::None;
}
