use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::resources::ai::{AiState, MovementIntent, PlayerCommand};
use crate::resources::damage::EquippedWeapon;
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::input::{Action, ActionState};
use crate::resources::map::{GridPosition, MapSettings, TileWorld, render_layers};
use crate::resources::selection::{DragSelection, Selected};
use crate::systems::camera::MainCamera;
use crate::systems::spawning::PlayerControlled;

/// Visual indicator for selected entities.
#[derive(Component)]
pub struct SelectionRing {
    pub owner: Entity,
}

/// Handle left-click: start drag or select single entity.
pub fn selection_input(
    actions: Res<ActionState>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    map_settings: Res<MapSettings>,
    mut drag: ResMut<DragSelection>,
    mut commands: Commands,
    player_entities: Query<(Entity, &GridPosition), With<PlayerControlled>>,
    already_selected: Query<Entity, With<Selected>>,
) {
    let Ok(window) = window_query.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };

    if actions.just_pressed(Action::Select) {
        drag.begin(cursor_pos);
    }

    if actions.pressed(Action::Select) {
        drag.update(cursor_pos);
    }

    if actions.just_released(Action::Select) {
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
