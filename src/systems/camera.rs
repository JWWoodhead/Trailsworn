use bevy::camera::Projection;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::resources::input::{Action, ActionState};
use crate::resources::map::{CursorPosition, MapSettings};
use crate::systems::world_map_ui::WorldMapVisible;

/// Marker for the main game camera.
#[derive(Component)]
pub struct MainCamera;

const KEYBOARD_PAN_SPEED: f32 = 500.0;
const EDGE_SCROLL_SPEED: f32 = 400.0;
const EDGE_SCROLL_MARGIN: f32 = 20.0;
const TILE_PX: f32 = 64.0;

/// Pre-computed zoom scales where `TILE_PX / scale` is an integer,
/// guaranteeing tiles always map to whole screen pixels.
/// Covers scale = 64/n for n in 22..=128 (scale ~0.5 to ~2.9).
const PIXEL_PERFECT_SCALES: &[f32] = &{
    // 64/128=0.500, 64/127=0.504, ..., 64/22=2.909
    // Build from largest scale (most zoomed out) to smallest.
    let mut arr = [0.0f32; 107]; // 128 - 22 + 1
    let mut i = 0usize;
    let mut n = 22u32;
    while n <= 128 {
        arr[i] = TILE_PX / n as f32;
        i += 1;
        n += 1;
    }
    arr
};

pub fn setup_camera(mut commands: Commands, map_settings: Res<MapSettings>) {
    let center_x = map_settings.width as f32 * map_settings.tile_size * 0.5;
    let center_y = map_settings.height as f32 * map_settings.tile_size * 0.5;

    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_translation(Vec3::new(center_x, center_y, 999.0)),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

pub fn camera_pan(
    time: Res<Time>,
    actions: Res<ActionState>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<(&mut Transform, &Projection), With<MainCamera>>,
    map_settings: Res<MapSettings>,
    world_map_visible: Res<WorldMapVisible>,
) {
    // Don't pan camera when world map is open (keys control map pan instead)
    if world_map_visible.0 {
        return;
    }
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((mut transform, projection)) = camera_query.single_mut() else {
        return;
    };

    let scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };

    let dt = time.delta_secs();
    let mut delta = Vec2::ZERO;

    if actions.pressed(Action::CameraPanUp) {
        delta.y += 1.0;
    }
    if actions.pressed(Action::CameraPanDown) {
        delta.y -= 1.0;
    }
    if actions.pressed(Action::CameraPanLeft) {
        delta.x -= 1.0;
    }
    if actions.pressed(Action::CameraPanRight) {
        delta.x += 1.0;
    }

    if delta != Vec2::ZERO {
        delta = delta.normalize() * KEYBOARD_PAN_SPEED * dt * scale;
    }

    // Edge scrolling (raw cursor position, not action-mapped)
    if let Some(cursor_pos) = window.cursor_position() {
        let w = window.width();
        let h = window.height();
        let mut edge_delta = Vec2::ZERO;

        if cursor_pos.x < EDGE_SCROLL_MARGIN {
            edge_delta.x -= 1.0;
        }
        if cursor_pos.x > w - EDGE_SCROLL_MARGIN {
            edge_delta.x += 1.0;
        }
        if cursor_pos.y < EDGE_SCROLL_MARGIN {
            edge_delta.y += 1.0;
        }
        if cursor_pos.y > h - EDGE_SCROLL_MARGIN {
            edge_delta.y -= 1.0;
        }

        if edge_delta != Vec2::ZERO {
            delta += edge_delta.normalize() * EDGE_SCROLL_SPEED * dt * scale;
        }
    }

    transform.translation.x += delta.x;
    transform.translation.y += delta.y;

    let map_w = map_settings.width as f32 * map_settings.tile_size;
    let map_h = map_settings.height as f32 * map_settings.tile_size;
    transform.translation.x = transform.translation.x.clamp(0.0, map_w);
    transform.translation.y = transform.translation.y.clamp(0.0, map_h);
}

/// Compute cursor position in screen, world, and tile coordinates.
/// Runs once per frame, early in the Input set. All other systems read the resource.
pub fn update_cursor_position(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    map_settings: Res<MapSettings>,
    mut cursor: ResMut<CursorPosition>,
) {
    cursor.screen = None;
    cursor.world = None;
    cursor.tile = None;

    let Ok(window) = window_query.single() else { return };
    let Some(screen_pos) = window.cursor_position() else { return };
    cursor.screen = Some(screen_pos);

    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, screen_pos) else { return };
    cursor.world = Some(world_pos);

    let tile_x = (world_pos.x / map_settings.tile_size).round() as i32;
    let tile_y = (world_pos.y / map_settings.tile_size).round() as i32;
    cursor.tile = Some((tile_x, tile_y));
}

/// Step one notch up or down in the pixel-perfect zoom table.
fn step_pixel_perfect_scale(current: f32, zoom_in: bool) -> f32 {
    // Table is sorted descending (largest scale = most zoomed out first)
    let mut closest_idx = 0usize;
    let mut closest_dist = f32::MAX;
    for (i, &s) in PIXEL_PERFECT_SCALES.iter().enumerate() {
        let d = (s - current).abs();
        if d < closest_dist {
            closest_dist = d;
            closest_idx = i;
        }
    }
    if zoom_in {
        // Zoom in = smaller scale = higher index in descending table
        if closest_idx + 1 < PIXEL_PERFECT_SCALES.len() {
            PIXEL_PERFECT_SCALES[closest_idx + 1]
        } else {
            PIXEL_PERFECT_SCALES[closest_idx]
        }
    } else {
        // Zoom out = larger scale = lower index
        if closest_idx > 0 {
            PIXEL_PERFECT_SCALES[closest_idx - 1]
        } else {
            PIXEL_PERFECT_SCALES[closest_idx]
        }
    }
}

/// Zoom via scroll wheel, snapping to pixel-perfect scales where tiles
/// map to whole screen pixels (eliminates sub-pixel tile seams).
pub fn camera_zoom(
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    mut camera_query: Query<&mut Projection, With<MainCamera>>,
) {
    let Ok(mut projection) = camera_query.single_mut() else {
        return;
    };

    for event in scroll_events.read() {
        if let Projection::Orthographic(ref mut ortho) = *projection {
            let zoom_in = event.y > 0.0;
            ortho.scale = step_pixel_perfect_scale(ortho.scale, zoom_in);
        }
    }
}

/// Snap camera position to the nearest screen-pixel boundary to prevent
/// sub-pixel tile seams. Must run after camera_pan and camera_zoom.
pub fn snap_camera_to_pixel(
    mut camera_query: Query<(&mut Transform, &Projection), With<MainCamera>>,
) {
    let Ok((mut transform, projection)) = camera_query.single_mut() else {
        return;
    };
    let scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };
    // One screen pixel = `scale` world units. Round to that grid.
    transform.translation.x = (transform.translation.x / scale).round() * scale;
    transform.translation.y = (transform.translation.y / scale).round() * scale;
}
