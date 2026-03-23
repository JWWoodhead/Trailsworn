use bevy::prelude::*;

use crate::resources::game_time::{GameTime, TICK_DURATION};
use crate::resources::map::{GridPosition, MapSettings, TileWorld};
use crate::resources::movement::{FacingDirection, MovePath, MovementSpeed, PathOffset, PendingPath};

/// Advance entities along their movement paths. Runs simulation ticks.
pub fn movement(
    mut commands: Commands,
    game_time: Res<GameTime>,
    tile_world: Res<TileWorld>,
    mut query: Query<(Entity, &mut MovePath, &mut GridPosition, &MovementSpeed, &mut FacingDirection, Option<&PendingPath>)>,
) {
    let ticks = game_time.ticks_this_frame;
    if ticks == 0 {
        return;
    }

    for (entity, mut path, mut grid_pos, speed, mut facing, pending) in &mut query {
        for _ in 0..ticks {
            if path.is_finished() {
                commands.entity(entity).remove::<MovePath>();
                commands.entity(entity).remove::<PendingPath>();
                break;
            }

            let next = match path.next_tile() {
                Some(t) => t,
                None => {
                    commands.entity(entity).remove::<MovePath>();
                    commands.entity(entity).remove::<PendingPath>();
                    break;
                }
            };

            // Cost of entering the next tile
            let tile_cost = tile_world.walk_cost[tile_world.idx(next.0, next.1)];
            if tile_cost <= 0.0 {
                commands.entity(entity).remove::<MovePath>();
                commands.entity(entity).remove::<PendingPath>();
                break;
            }

            // Progress per tick, with whole-path ease-in/ease-out
            let ease = path.ease_speed_multiplier();
            let progress_per_tick = speed.tiles_per_second * TICK_DURATION / tile_cost * ease;
            path.progress += progress_per_tick;

            if path.progress >= 1.0 {
                grid_pos.x = next.0;
                grid_pos.y = next.1;

                // Update facing based on movement direction
                let cur = path.current_tile().unwrap_or(next);
                *facing = facing_from_movement(cur, next);

                // If there's a pending path, swap to it now.
                // The pending path was calculated from the entity's grid_pos at repath
                // time. Find our current tile in it and start from there. If we can't
                // find it (entity followed a different route than expected), the pending
                // path is stale — discard it and keep the current path.
                if let Some(pending_path) = pending {
                    let current = (grid_pos.x, grid_pos.y);
                    if let Some(start) = pending_path.waypoints.iter().position(|&wp| wp == current) {
                        *path = MovePath::new(pending_path.waypoints[start..].to_vec());
                    } else {
                        // Stale pending path — just continue on the current path
                        path.advance();
                    }
                    commands.entity(entity).remove::<PendingPath>();
                } else {
                    path.advance();
                }
            }
        }
    }
}

/// Set entity transforms from simulation state.
/// Runs every frame (not tick-locked).
pub fn sync_transforms(
    map_settings: Res<MapSettings>,
    mut query: Query<(&GridPosition, Option<&MovePath>, Option<&PathOffset>, &mut Transform)>,
) {
    let ts = map_settings.tile_size;

    for (grid_pos, move_path, offset, mut transform) in &mut query {
        let offset_px = offset.map_or(Vec2::ZERO, |o| Vec2::new(o.x * ts, o.y * ts));
        let base = grid_pos.to_world(ts) + offset_px;

        let pos = if let Some(path) = move_path {
            if let Some(next) = path.next_tile() {
                let next_world = Vec2::new(
                    next.0 as f32 * ts,
                    next.1 as f32 * ts,
                ) + offset_px;
                base.lerp(next_world, path.progress.clamp(0.0, 1.0))
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
