use bevy::prelude::*;

use crate::resources::ai::{AiState, CombatBehavior};
use crate::resources::map::{GridPosition, MapSettings};
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
        };

        for flag in flags_str.split(',') {
            match flag.trim() {
                "all" => {
                    flags.grid = true;
                    flags.pathing = true;
                    flags.aggro_radius = true;
                    flags.ai_state = true;
                    flags.profiling = true;
                }
                "grid" => flags.grid = true,
                "pathing" | "path" => flags.pathing = true,
                "aggro" | "aggro_radius" => flags.aggro_radius = true,
                "ai" | "ai_state" => flags.ai_state = true,
                "profiling" | "perf" => flags.profiling = true,
                _ => eprintln!("Unknown debug flag: {flag}"),
            }
        }

        Some(flags)
    }

    pub fn any_active(&self) -> bool {
        self.grid || self.pathing || self.aggro_radius || self.ai_state || self.profiling
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
    query: Query<(&GridPosition, &AiState, Option<&EntityName>)>,
) {
    if !flags.ai_state {
        return;
    }

    let ts = map_settings.tile_size;
    let color_idle = Color::srgba(0.5, 0.5, 0.5, 0.6);
    let color_engage = Color::srgba(1.0, 0.4, 0.2, 0.8);
    let color_flee = Color::srgba(1.0, 1.0, 0.2, 0.8);

    for (grid_pos, ai_state, _name) in &query {
        let pos = grid_pos.to_world(ts);
        let offset = Vec2::new(0.0, -30.0);

        let (color, size) = match ai_state {
            AiState::Idle => (color_idle, 3.0),
            AiState::Engaging { .. } => (color_engage, 5.0),
            AiState::Fleeing => (color_flee, 4.0),
            AiState::Following { .. } => (color_idle, 3.0),
        };

        // Small indicator dot below the entity
        gizmos.circle_2d(
            Isometry2d::from_translation(pos + offset),
            size,
            color,
        );
    }
}
