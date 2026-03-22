use bevy::prelude::*;

use crate::resources::map::MapSettings;

/// Draw grid lines over the tilemap.
pub fn draw_grid(map_settings: Res<MapSettings>, mut gizmos: Gizmos) {
    let ts = map_settings.tile_size;
    let w = map_settings.width as f32 * ts;
    let h = map_settings.height as f32 * ts;
    let half_ts = ts * 0.5;

    let color = Color::srgba(1.0, 1.0, 1.0, 0.15);

    // Vertical lines
    for x in 0..=map_settings.width {
        let xf = x as f32 * ts - half_ts;
        gizmos.line_2d(Vec2::new(xf, -half_ts), Vec2::new(xf, h - half_ts), color);
    }

    // Horizontal lines
    for y in 0..=map_settings.height {
        let yf = y as f32 * ts - half_ts;
        gizmos.line_2d(Vec2::new(-half_ts, yf), Vec2::new(w - half_ts, yf), color);
    }
}
