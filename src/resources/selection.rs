use bevy::prelude::*;

use super::abilities::{AbilityId, TargetType};

/// Marker for currently selected entities.
#[derive(Component)]
pub struct Selected;

/// Active targeting mode for ability casting.
#[derive(Resource, Default)]
pub enum TargetingMode {
    #[default]
    None,
    /// Waiting for the player to click a target for an ability.
    AwaitingTarget {
        caster: Entity,
        ability_id: AbilityId,
        slot_index: usize,
        target_type: TargetType,
        range: f32,
        aoe_radius: f32,
    },
}

/// Tracks the drag selection box state.
#[derive(Resource, Default)]
pub struct DragSelection {
    /// Screen-space start position of the drag.
    pub start: Option<Vec2>,
    /// Whether we're actively dragging (past a small threshold).
    pub active: bool,
}

const DRAG_THRESHOLD: f32 = 5.0;

impl DragSelection {
    pub fn begin(&mut self, pos: Vec2) {
        self.start = Some(pos);
        self.active = false;
    }

    pub fn update(&mut self, current_pos: Vec2) {
        if let Some(start) = self.start {
            if start.distance(current_pos) > DRAG_THRESHOLD {
                self.active = true;
            }
        }
    }

    pub fn reset(&mut self) {
        self.start = None;
        self.active = false;
    }

    /// Get the screen-space rect of the selection box (min corner, max corner).
    pub fn rect(&self, current_pos: Vec2) -> Option<(Vec2, Vec2)> {
        if !self.active {
            return None;
        }
        let start = self.start?;
        let min = Vec2::new(start.x.min(current_pos.x), start.y.min(current_pos.y));
        let max = Vec2::new(start.x.max(current_pos.x), start.y.max(current_pos.y));
        Some((min, max))
    }
}
