use bevy::prelude::*;

use crate::resources::combat_behavior::CombatBehavior;
use crate::resources::task::{Action, CurrentTask};
use crate::resources::map::{GridPosition, MapSettings, TileWorld};
use crate::resources::movement::MovePath;
use crate::systems::spawning::EntityName;

/// Individual debug visualization flags. Controlled via CLI and runtime keys.
#[derive(Resource)]
pub struct DebugFlags {
    pub grid: bool,
    pub pathing: bool,
    pub aggro_radius: bool,
    pub ai_state: bool,
    pub profiling: bool,
    pub obstacles: bool,
}

impl DebugFlags {
    /// Parse from CLI argument. Accepts "all" or comma-separated flag names.
    /// Returns None if --debug was not passed.
    pub fn from_args() -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        let debug_idx = args.iter().position(|a| a == "--debug")?;
        let flags_str = args.get(debug_idx + 1).map(|s| s.as_str()).unwrap_or("all");

        let mut flags = Self {
            grid: false,
            pathing: false,
            aggro_radius: false,
            ai_state: false,
            profiling: false,
            obstacles: false,
        };

        for flag in flags_str.split(',') {
            match flag.trim() {
                "all" => {
                    flags.grid = true;
                    flags.pathing = true;
                    flags.aggro_radius = true;
                    flags.ai_state = true;
                    flags.profiling = true;
                    flags.obstacles = true;
                }
                "grid" => flags.grid = true,
                "pathing" | "path" => flags.pathing = true,
                "aggro" | "aggro_radius" => flags.aggro_radius = true,
                "ai" | "ai_state" => flags.ai_state = true,
                "profiling" | "perf" => flags.profiling = true,
                "obstacles" | "obs" => flags.obstacles = true,
                _ => eprintln!("Unknown debug flag: {flag}"),
            }
        }

        Some(flags)
    }

    pub fn any_active(&self) -> bool {
        self.grid || self.pathing || self.aggro_radius || self.ai_state || self.profiling || self.obstacles
    }
}

/// Runtime toggle of debug flags via action system.
pub fn debug_key_toggles(
    actions: Res<crate::resources::input::ActionState>,
    mut flags: ResMut<DebugFlags>,
) {
    use crate::resources::input::Action;
    if actions.just_pressed(Action::DebugGrid) {
        flags.grid = !flags.grid;
    }
    if actions.just_pressed(Action::DebugPathing) {
        flags.pathing = !flags.pathing;
    }
    if actions.just_pressed(Action::DebugAggro) {
        flags.aggro_radius = !flags.aggro_radius;
    }
    if actions.just_pressed(Action::DebugAiState) {
        flags.ai_state = !flags.ai_state;
    }
    if actions.just_pressed(Action::DebugProfiling) {
        flags.profiling = !flags.profiling;
    }
    if actions.just_pressed(Action::DebugObstacles) {
        flags.obstacles = !flags.obstacles;
    }
}

/// Draw grid lines over the tilemap.
pub fn draw_grid(
    flags: Res<DebugFlags>,
    map_settings: Res<MapSettings>,
    mut gizmos: Gizmos,
) {
    if !flags.grid {
        return;
    }

    let ts = map_settings.tile_size;
    let w = map_settings.width as f32 * ts;
    let h = map_settings.height as f32 * ts;
    let half_ts = ts * 0.5;

    let color = Color::srgba(1.0, 1.0, 1.0, 0.15);

    for x in 0..=map_settings.width {
        let xf = x as f32 * ts - half_ts;
        gizmos.line_2d(Vec2::new(xf, -half_ts), Vec2::new(xf, h - half_ts), color);
    }

    for y in 0..=map_settings.height {
        let yf = y as f32 * ts - half_ts;
        gizmos.line_2d(Vec2::new(-half_ts, yf), Vec2::new(w - half_ts, yf), color);
    }
}

/// Draw movement paths as lines from entity to each waypoint.
pub fn draw_pathing(
    flags: Res<DebugFlags>,
    map_settings: Res<MapSettings>,
    mut gizmos: Gizmos,
    query: Query<(&GridPosition, &MovePath)>,
) {
    if !flags.pathing {
        return;
    }

    let ts = map_settings.tile_size;
    let color = Color::srgba(0.3, 0.8, 1.0, 0.6);

    for (grid_pos, path) in &query {
        let mut prev = grid_pos.to_world(ts);

        for i in (path.current_index + 1)..path.waypoints.len() {
            let wp = path.waypoints[i];
            let next = Vec2::new(wp.0 as f32 * ts, wp.1 as f32 * ts);
            gizmos.line_2d(prev, next, color);
            prev = next;
        }

        // Draw a small diamond at the destination
        if let Some(dest) = path.destination() {
            let dest_pos = Vec2::new(dest.0 as f32 * ts, dest.1 as f32 * ts);
            let s = 4.0;
            gizmos.line_2d(dest_pos + Vec2::new(0.0, s), dest_pos + Vec2::new(s, 0.0), color);
            gizmos.line_2d(dest_pos + Vec2::new(s, 0.0), dest_pos + Vec2::new(0.0, -s), color);
            gizmos.line_2d(dest_pos + Vec2::new(0.0, -s), dest_pos + Vec2::new(-s, 0.0), color);
            gizmos.line_2d(dest_pos + Vec2::new(-s, 0.0), dest_pos + Vec2::new(0.0, s), color);
        }
    }
}

/// Draw aggro/engage radius circles around entities with CombatBehavior.
pub fn draw_aggro_radius(
    flags: Res<DebugFlags>,
    map_settings: Res<MapSettings>,
    mut gizmos: Gizmos,
    query: Query<(&GridPosition, &CombatBehavior)>,
) {
    if !flags.aggro_radius {
        return;
    }

    let ts = map_settings.tile_size;
    let color = Color::srgba(1.0, 0.3, 0.3, 0.3);

    for (grid_pos, behavior) in &query {
        let pos = grid_pos.to_world(ts);
        let radius = behavior.aggro_range * ts;
        gizmos.circle_2d(Isometry2d::from_translation(pos), radius, color);
    }
}

/// Draw AI state labels above entities.
pub fn draw_ai_state(
    flags: Res<DebugFlags>,
    map_settings: Res<MapSettings>,
    mut gizmos: Gizmos,
    query: Query<(&GridPosition, Option<&CurrentTask>, Option<&EntityName>)>,
) {
    if !flags.ai_state {
        return;
    }

    let ts = map_settings.tile_size;
    let color_idle = Color::srgba(0.5, 0.5, 0.5, 0.6);
    let color_engage = Color::srgba(1.0, 0.4, 0.2, 0.8);
    let color_flee = Color::srgba(1.0, 1.0, 0.2, 0.8);

    for (grid_pos, current_task, _name) in &query {
        let pos = grid_pos.to_world(ts);
        let offset = Vec2::new(0.0, -30.0);

        let (color, size) = match current_task.and_then(|ct| ct.current_action()) {
            Some(Action::EngageTarget { .. }) => (color_engage, 5.0),
            Some(Action::FleeFrom { .. }) => (color_flee, 4.0),
            _ => (color_idle, 3.0),
        };

        // Small indicator dot below the entity
        gizmos.circle_2d(
            Isometry2d::from_translation(pos + offset),
            size,
            color,
        );
    }
}

/// Draw red outlines on impassable tiles (walk_cost <= 0).
pub fn draw_obstacles(
    flags: Res<DebugFlags>,
    map_settings: Res<MapSettings>,
    tile_world: Res<TileWorld>,
    mut gizmos: Gizmos,
    camera_query: Query<&Transform, With<Camera2d>>,
) {
    if !flags.obstacles {
        return;
    }

    let ts = map_settings.tile_size;
    let half = ts * 0.5;
    let color = Color::srgba(1.0, 0.2, 0.2, 0.4);

    // Only draw obstacles within a generous viewport around the camera
    let Ok(cam_transform) = camera_query.single() else { return };
    let cam_pos = cam_transform.translation.truncate();
    let view_radius = 1200.0 * cam_transform.scale.x; // generous estimate
    let min_x = ((cam_pos.x - view_radius) / ts).floor().max(0.0) as u32;
    let max_x = ((cam_pos.x + view_radius) / ts).ceil().min(map_settings.width as f32) as u32;
    let min_y = ((cam_pos.y - view_radius) / ts).floor().max(0.0) as u32;
    let max_y = ((cam_pos.y + view_radius) / ts).ceil().min(map_settings.height as f32) as u32;

    for y in min_y..max_y {
        for x in min_x..max_x {
            let idx = tile_world.idx(x, y);
            if tile_world.walk_cost[idx] <= 0.0 {
                let cx = x as f32 * ts;
                let cy = y as f32 * ts;
                let tl = Vec2::new(cx - half, cy + half);
                let tr = Vec2::new(cx + half, cy + half);
                let br = Vec2::new(cx + half, cy - half);
                let bl = Vec2::new(cx - half, cy - half);
                gizmos.line_2d(tl, tr, color);
                gizmos.line_2d(tr, br, color);
                gizmos.line_2d(br, bl, color);
                gizmos.line_2d(bl, tl, color);
            }
        }
    }
}
