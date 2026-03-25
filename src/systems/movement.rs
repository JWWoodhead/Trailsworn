use bevy::prelude::*;

use crate::pathfinding::astar_tile_grid;
use crate::resources::movement::RepathTimer;
use crate::resources::game_time::{GameTime, TICK_DURATION};
use crate::resources::map::{GridPosition, MapSettings, TileWorld};
use crate::resources::movement::{FacingDirection, MovePath, MovementSpeed, PathOffset, PendingPath};
use crate::resources::task::{Action, CurrentTask};

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
                        let traveled = path.tiles_traveled + path.current_index as f32 + path.progress;
                        *path = MovePath::new(pending_path.waypoints[start..].to_vec());
                        path.tiles_traveled = traveled;
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

        let map_height_px = map_settings.height as f32 * ts;
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        transform.translation.z = crate::resources::map::render_layers::y_sorted_z(
            pos.y, map_height_px, crate::resources::map::render_layers::ENTITIES,
        );
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

// ---------------------------------------------------------------------------
// resolve_movement — pathfinding driven by current task action
// ---------------------------------------------------------------------------

/// Convert movement-related actions into A* pathfinding.
pub fn resolve_movement(
    game_time: Res<GameTime>,
    tile_world: Res<TileWorld>,
    mut query: Query<(
        Entity,
        &GridPosition,
        &CurrentTask,
        &mut RepathTimer,
        Option<&MovePath>,
        Option<&crate::systems::spawning::PlayerControlled>,
    )>,
    target_positions: Query<&GridPosition>,
    mut commands: Commands,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    for (entity, grid_pos, current_task, mut repath_timer, current_path, player_controlled) in &mut query {
        // Tick repath timer
        for _ in 0..game_time.ticks_this_frame {
            repath_timer.tick();
        }

        // Extract movement goal from current action
        let Some((goal, in_range)) = extract_movement_goal(current_task, grid_pos, &tile_world, &target_positions) else {
            // No movement needed — clear any active path
            if current_path.is_some() {
                commands.entity(entity).remove::<MovePath>();
                commands.entity(entity).remove::<PendingPath>();
            }
            repath_timer.reset();
            continue;
        };

        if in_range {
            if current_path.is_some() {
                commands.entity(entity).remove::<MovePath>();
                commands.entity(entity).remove::<PendingPath>();
            }
            repath_timer.reset();
            continue;
        }

        // Repath throttling
        let is_player = player_controlled.is_some();
        let needs_initial_path = current_path.is_none();

        if !needs_initial_path {
            if is_player {
                let current_dest = current_path.and_then(|p| p.destination());
                if current_dest == Some(goal) {
                    continue;
                }
            } else if !repath_timer.should_repath() {
                continue;
            }
        }

        // Mid-movement handling
        let mid_movement = current_path.is_some_and(|p| p.progress > 0.0);
        let old_progress = current_path.map(|p| p.progress).unwrap_or(0.0);
        let old_traveled = current_path
            .map(|p| p.tiles_traveled + p.current_index as f32)
            .unwrap_or(0.0);

        let (start, prepend_current) = if mid_movement {
            match current_path.and_then(|p| p.next_tile()) {
                Some(n) => (n, is_player),
                None => ((grid_pos.x, grid_pos.y), false),
            }
        } else {
            ((grid_pos.x, grid_pos.y), false)
        };

        if start == goal {
            continue;
        }

        if let Some(mut path) = astar_tile_grid(
            start, goal, tile_world.width, tile_world.height,
            &tile_world.walk_cost, 5000,
        ) {
            if prepend_current {
                path.insert(0, (grid_pos.x, grid_pos.y));
                let mut mp = MovePath::new(path);
                mp.progress = old_progress;
                mp.tiles_traveled = old_traveled;
                commands.entity(entity).insert(mp);
            } else if mid_movement {
                commands.entity(entity).insert(PendingPath { waypoints: path });
            } else {
                commands.entity(entity).insert(MovePath::new(path));
            }
            repath_timer.reset();
        }
    }
}

/// Extract the movement goal from the brain's current action.
/// Returns `(goal_tile, in_range)` or `None` if the action doesn't need movement.
fn extract_movement_goal(
    current_task: &CurrentTask,
    grid_pos: &GridPosition,
    tile_world: &TileWorld,
    target_positions: &Query<&GridPosition>,
) -> Option<((u32, u32), bool)> {
    let action = current_task.current_action()?;

    match action {
        Action::MoveToEntity { target, range } | Action::EngageTarget { target, attack_range: range } => {
            let tp = target_positions.get(*target).ok()?;
            let dx = grid_pos.x as f32 - tp.x as f32;
            let dy = grid_pos.y as f32 - tp.y as f32;
            Some(((tp.x, tp.y), (dx * dx + dy * dy).sqrt() <= *range))
        }

        Action::MoveToPosition { x, y } => {
            Some(((*x, *y), grid_pos.x == *x && grid_pos.y == *y))
        }

        Action::FleeFrom { threat } => {
            let tp = target_positions.get(*threat).ok()?;
            let dx = grid_pos.x as i32 - tp.x as i32;
            let dy = grid_pos.y as i32 - tp.y as i32;
            let flee_x = (grid_pos.x as i32 + dx.signum() * 10)
                .clamp(0, tile_world.width as i32 - 1) as u32;
            let flee_y = (grid_pos.y as i32 + dy.signum() * 10)
                .clamp(0, tile_world.height as i32 - 1) as u32;
            Some(((flee_x, flee_y), false))
        }

        Action::FollowEntity { leader, distance } => {
            let lp = target_positions.get(*leader).ok()?;
            let dx = grid_pos.x as f32 - lp.x as f32;
            let dy = grid_pos.y as f32 - lp.y as f32;
            Some(((lp.x, lp.y), (dx * dx + dy * dy).sqrt() <= *distance))
        }

        // Non-movement actions
        Action::Wait { .. } | Action::CastAbility { .. } => None,
    }
}
