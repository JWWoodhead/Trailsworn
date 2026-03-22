use bevy::prelude::*;

/// Base movement speed in tiles per second (before modifiers).
#[derive(Component, Clone, Copy, Debug)]
pub struct MovementSpeed {
    pub tiles_per_second: f32,
}

impl Default for MovementSpeed {
    fn default() -> Self {
        Self {
            tiles_per_second: 4.0,
        }
    }
}

/// Direction the entity is facing. Affects sprites and ranged attacks.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FacingDirection {
    #[default]
    South,
    North,
    East,
    West,
}

/// Active movement path being followed by an entity.
/// Attached when a move command is issued, removed on arrival or interruption.
#[derive(Component, Debug)]
pub struct MovePath {
    pub waypoints: Vec<(u32, u32)>,
    pub current_index: usize,
    /// Interpolation progress between current tile and next (0.0 to 1.0).
    pub progress: f32,
}

impl MovePath {
    pub fn new(waypoints: Vec<(u32, u32)>) -> Self {
        Self {
            waypoints,
            current_index: 0,
            progress: 0.0,
        }
    }

    pub fn current_tile(&self) -> Option<(u32, u32)> {
        self.waypoints.get(self.current_index).copied()
    }

    pub fn next_tile(&self) -> Option<(u32, u32)> {
        self.waypoints.get(self.current_index + 1).copied()
    }

    pub fn is_finished(&self) -> bool {
        self.current_index + 1 >= self.waypoints.len()
    }

    pub fn advance(&mut self) {
        self.current_index += 1;
        self.progress = 0.0;
    }
}
