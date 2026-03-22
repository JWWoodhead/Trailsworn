use bevy::camera::Projection;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::resources::map::MapSettings;

/// Marker for the main game camera.
#[derive(Component)]
pub struct MainCamera;

const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 3.0;
const KEYBOARD_PAN_SPEED: f32 = 500.0;
const EDGE_SCROLL_SPEED: f32 = 400.0;
const EDGE_SCROLL_MARGIN: f32 = 20.0;
const ZOOM_SPEED: f32 = 0.1;

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
    keyboard: Res<ButtonInput<KeyCode>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<(&mut Transform, &Projection), With<MainCamera>>,
    map_settings: Res<MapSettings>,
) {
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

    // Keyboard panning
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        delta.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        delta.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        delta.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        delta.x += 1.0;
    }

    if delta != Vec2::ZERO {
        delta = delta.normalize() * KEYBOARD_PAN_SPEED * dt * scale;
    }

    // Edge scrolling
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
        // Window cursor y is top-down, camera y is bottom-up
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

    // Clamp to map bounds
    let map_w = map_settings.width as f32 * map_settings.tile_size;
    let map_h = map_settings.height as f32 * map_settings.tile_size;
    transform.translation.x = transform.translation.x.clamp(0.0, map_w);
    transform.translation.y = transform.translation.y.clamp(0.0, map_h);
}

pub fn camera_zoom(
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    mut camera_query: Query<&mut Projection, With<MainCamera>>,
) {
    let Ok(mut projection) = camera_query.single_mut() else {
        return;
    };

    for event in scroll_events.read() {
        if let Projection::Orthographic(ref mut ortho) = *projection {
            let zoom_delta = -event.y * ZOOM_SPEED;
            ortho.scale = (ortho.scale + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
}
