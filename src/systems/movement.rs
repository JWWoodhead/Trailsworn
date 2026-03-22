use bevy::prelude::*;

use crate::resources::game_time::{GameTime, TICK_DURATION};
use crate::resources::map::{GridPosition, MapSettings, TileWorld};
use crate::resources::movement::{FacingDirection, MovePath, MovementSpeed};

/// Advance entities along their movement paths. Runs simulation ticks.
pub fn movement(
    mut commands: Commands,
    game_time: Res<GameTime>,
    tile_world: Res<TileWorld>,
    mut query: Query<(Entity, &mut MovePath, &mut GridPosition, &MovementSpeed, &mut FacingDirection)>,
) {
    let ticks = game_time.ticks_this_frame;
    if ticks == 0 {
        return;
    }

    for (entity, mut path, mut grid_pos, speed, mut facing) in &mut query {
        for _ in 0..ticks {
            if path.is_finished() {
                commands.entity(entity).remove::<MovePath>();
                break;
            }

            let next = match path.next_tile() {
                Some(t) => t,
                None => {
                    commands.entity(entity).remove::<MovePath>();
                    break;
                }
            };

            // Cost of entering the next tile
            let tile_cost = tile_world.walk_cost[tile_world.idx(next.0, next.1)];
            if tile_cost <= 0.0 {
                // Path blocked — abort
                commands.entity(entity).remove::<MovePath>();
                break;
            }

            // Progress per tick: speed (tiles/sec) * tick_duration (sec) / tile_cost
            let progress_per_tick = speed.tiles_per_second * TICK_DURATION / tile_cost;
            path.progress += progress_per_tick;

            if path.progress >= 1.0 {
                // Arrived at next tile
                grid_pos.x = next.0;
                grid_pos.y = next.1;

                // Update facing based on movement direction
                let cur = path.current_tile().unwrap_or(next);
                *facing = facing_from_movement(cur, next);

                path.advance();
            }
        }
    }
}

/// Smoothly interpolate entity transforms between tiles for rendering.
/// Runs every frame (not tick-locked) for smooth visuals.
pub fn sync_transforms(
    map_settings: Res<MapSettings>,
    game_time: Res<GameTime>,
    mut query: Query<(&GridPosition, Option<&MovePath>, &mut Transform)>,
) {
    let ts = map_settings.tile_size;

    for (grid_pos, move_path, mut transform) in &mut query {
        let base = grid_pos.to_world(ts);

        let pos = if let Some(path) = move_path {
            if let Some(next) = path.next_tile() {
                let next_world = Vec2::new(
                    next.0 as f32 * ts,
                    next.1 as f32 * ts,
                );
                // Blend between current and next tile using path progress + interpolation alpha
                let alpha = (path.progress + game_time.interpolation_alpha() * 0.016)
                    .clamp(0.0, 1.0);
                base.lerp(next_world, alpha)
            } else {
                base
            }
        } else {
            base
        };

        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }
}

fn facing_from_movement(from: (u32, u32), to: (u32, u32)) -> FacingDirection {
    let dx = to.0 as i32 - from.0 as i32;
    let dy = to.1 as i32 - from.1 as i32;

    // Prefer horizontal facing for diagonals
    if dx.abs() >= dy.abs() {
        if dx > 0 { FacingDirection::East } else { FacingDirection::West }
    } else if dy > 0 {
        FacingDirection::North
    } else {
        FacingDirection::South
    }
}
