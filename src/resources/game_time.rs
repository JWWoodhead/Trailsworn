use bevy::prelude::*;

/// Fixed simulation tick rate (ticks per second at 1x speed).
pub const TICKS_PER_SECOND: u32 = 60;

/// Duration of one simulation tick at 1x speed.
pub const TICK_DURATION: f32 = 1.0 / TICKS_PER_SECOND as f32;

/// Global game time state controlling simulation speed and pause.
#[derive(Resource)]
pub struct GameTime {
    pub speed: f32,
    pub paused: bool,
    /// Number of simulation ticks to run this frame. Set by advance_game_time.
    pub ticks_this_frame: u32,
    /// Accumulated real time since last tick, used for fixed timestep.
    accumulator: f32,
}

impl Default for GameTime {
    fn default() -> Self {
        Self {
            speed: 1.0,
            paused: false,
            ticks_this_frame: 0,
            accumulator: 0.0,
        }
    }
}

impl GameTime {
    /// Accumulate real delta time scaled by game speed.
    /// Returns the number of simulation ticks to run this frame.
    pub fn accumulate(&mut self, real_dt: f32) -> u32 {
        if self.paused {
            return 0;
        }

        self.accumulator += real_dt * self.speed;

        let ticks = (self.accumulator / TICK_DURATION) as u32;
        self.accumulator -= ticks as f32 * TICK_DURATION;

        // Cap to prevent spiral of death if framerate tanks
        ticks.min(10)
    }

    /// Fractional progress into the next tick (0.0 to 1.0).
    /// Used for visual interpolation between simulation states.
    pub fn interpolation_alpha(&self) -> f32 {
        self.accumulator / TICK_DURATION
    }
}
