use bevy::prelude::*;

use crate::resources::game_time::GameTime;
use crate::resources::input::{Action, ActionState};

/// Handle player input for pause and speed controls.
pub fn game_speed_input(actions: Res<ActionState>, mut game_time: ResMut<GameTime>) {
    if actions.just_pressed(Action::Pause) {
        game_time.paused = !game_time.paused;
    }
    if actions.just_pressed(Action::Speed1) {
        game_time.speed = 1.0;
    }
    if actions.just_pressed(Action::Speed2) {
        game_time.speed = 2.0;
    }
    if actions.just_pressed(Action::Speed3) {
        game_time.speed = 3.0;
    }
}

/// Accumulate real time into simulation ticks.
pub fn advance_game_time(time: Res<Time>, mut game_time: ResMut<GameTime>) {
    game_time.ticks_this_frame = game_time.accumulate(time.delta_secs());
}
