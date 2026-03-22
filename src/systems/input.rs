use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::pathfinding::astar_tile_grid;
use crate::resources::map::{GridPosition, MapSettings, TileWorld};
use crate::resources::movement::MovePath;
use crate::systems::camera::MainCamera;
use crate::systems::spawning::PlayerControlled;

/// Right-click to move selected pawn to target tile.
pub fn click_to_move(
    mouse: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    tile_world: Res<TileWorld>,
    map_settings: Res<MapSettings>,
    mut commands: Commands,
    mut pawn_query: Query<(Entity, &GridPosition), With<PlayerControlled>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Convert world position to tile coordinates.
    // Tile centers are at (x * tile_size, y * tile_size), so use round.
    let tile_x = (world_pos.x / map_settings.tile_size).round() as i32;
    let tile_y = (world_pos.y / map_settings.tile_size).round() as i32;

    info!(
        "Click: cursor={cursor_pos:?} world={world_pos:?} tile=({tile_x}, {tile_y}) tile_size={}",
        map_settings.tile_size
    );

    // Bounds check
    if tile_x < 0
        || tile_y < 0
        || tile_x >= tile_world.width as i32
        || tile_y >= tile_world.height as i32
    {
        return;
    }

    let goal = (tile_x as u32, tile_y as u32);

    // Check walkable
    if tile_world.walk_cost[tile_world.idx(goal.0, goal.1)] <= 0.0 {
        return;
    }

    for (entity, grid_pos) in &mut pawn_query {
        let start = (grid_pos.x, grid_pos.y);
        if start == goal {
            continue;
        }

        if let Some(path) = astar_tile_grid(
            start,
            goal,
            tile_world.width,
            tile_world.height,
            &tile_world.walk_cost,
            5000,
        ) {
            commands.entity(entity).insert(MovePath::new(path));
        }
    }
}
