use bevy::prelude::*;
use rand::{Rng, RngExt};

/// Base movement speed in tiles per second (before modifiers).
#[derive(Component, Clone, Copy, Debug)]
pub struct MovementSpeed {
    pub tiles_per_second: f32,
}

impl Default for MovementSpeed {
    fn default() -> Self {
        Self {
            tiles_per_second: 2.0,
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
    /// Total number of segments in this path (for whole-path easing).
    pub total_segments: usize,
}

impl MovePath {
    pub fn new(waypoints: Vec<(u32, u32)>) -> Self {
        let total = if waypoints.len() > 1 { waypoints.len() - 1 } else { 1 };
        Self {
            waypoints,
            current_index: 0,
            progress: 0.0,
            total_segments: total,
        }
    }

    /// How far through the overall path we are (0.0 to 1.0).
    pub fn overall_progress(&self) -> f32 {
        let completed = self.current_index as f32 + self.progress;
        (completed / self.total_segments as f32).clamp(0.0, 1.0)
    }

    /// Speed multiplier for ease-in/ease-out over the whole path.
    /// Ramps up over the first EASE_TILES tiles, ramps down over the last EASE_TILES.
    /// Same feel regardless of path length.
    pub fn ease_speed_multiplier(&self) -> f32 {
        const EASE_TILES: f32 = 1.5;
        const MIN_SPEED: f32 = 0.5;

        let tiles_from_start = self.current_index as f32 + self.progress;
        let tiles_from_end = self.total_segments as f32 - tiles_from_start;

        let ease_in = if tiles_from_start < EASE_TILES {
            MIN_SPEED + (1.0 - MIN_SPEED) * (tiles_from_start / EASE_TILES)
        } else {
            1.0
        };

        let ease_out = if tiles_from_end < EASE_TILES {
            MIN_SPEED + (1.0 - MIN_SPEED) * (tiles_from_end / EASE_TILES)
        } else {
            1.0
        };

        ease_in.min(ease_out)
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

    pub fn destination(&self) -> Option<(u32, u32)> {
        self.waypoints.last().copied()
    }
}

/// A path waiting to replace the current MovePath once the current tile step finishes.
#[derive(Component, Debug)]
pub struct PendingPath {
    pub waypoints: Vec<(u32, u32)>,
}

/// Small random offset from tile center for organic-looking movement.
/// Assigned once per entity, stays consistent.
#[derive(Component, Clone, Copy, Debug)]
pub struct PathOffset {
    pub x: f32,
    pub y: f32,
}

impl PathOffset {
    pub fn random(rng: &mut impl Rng) -> Self {
        // Offset up to ~20% of a tile in each direction
        Self {
            x: rng.random::<f32>() * 0.4 - 0.2,
            y: rng.random::<f32>() * 0.4 - 0.2,
        }
    }
}

impl Default for PathOffset {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}
