use bevy::prelude::*;

use crate::resources::game_time::GameTime;

/// Handle player input for pause and speed controls.
/// Runs every frame regardless of pause state.
pub fn game_speed_input(keyboard: Res<ButtonInput<KeyCode>>, mut game_time: ResMut<GameTime>) {
    if keyboard.just_pressed(KeyCode::Space) {
        game_time.paused = !game_time.paused;
    }

    if keyboard.just_pressed(KeyCode::Digit1) {
        game_time.speed = 1.0;
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        game_time.speed = 2.0;
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        game_time.speed = 3.0;
    }
}

/// Accumulate real time into simulation ticks.
/// Must run before any simulation system.
pub fn advance_game_time(time: Res<Time>, mut game_time: ResMut<GameTime>) {
    game_time.ticks_this_frame = game_time.accumulate(time.delta_secs());
}
