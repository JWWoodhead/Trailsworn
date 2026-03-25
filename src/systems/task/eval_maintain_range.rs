use bevy::prelude::*;

use crate::resources::combat_behavior::CombatBehavior;
use crate::resources::map::GridPosition;
use crate::resources::task::{Action, AiBrain, Engaging, Task, TaskEvaluator, TaskSource};

/// Propose retreating when the current target is closer than `preferred_min_range`.
/// Priority 65: above EngageTarget (60) so the caster repositions, but below
/// UseAbility (70) so it still casts when in spell range.
pub fn maintain_range(
    mut query: Query<(
        &GridPosition,
        &CombatBehavior,
        &mut AiBrain,
        Option<&Engaging>,
    )>,
    target_positions: Query<&GridPosition>,
) {
    for (grid_pos, behavior, mut brain, engaging) in &mut query {
        if brain.combat_eval_cooldown != 0 {
            continue;
        }
        if !brain.evaluators.iter().any(|e| matches!(e, TaskEvaluator::MaintainRange)) {
            continue;
        }

        let min_range = match behavior.preferred_min_range {
            Some(r) => r,
            None => continue,
        };

        // Only retreat if we're engaged with a target
        let target = match engaging {
            Some(e) => e.target,
            None => continue,
        };

        let target_pos = match target_positions.get(target) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let dx = grid_pos.x as f32 - target_pos.x as f32;
        let dy = grid_pos.y as f32 - target_pos.y as f32;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist >= min_range {
            continue; // Already at safe distance
        }

        // Calculate retreat position: move away from target
        let (dir_x, dir_y) = if dist > 0.1 {
            (dx / dist, dy / dist)
        } else {
            // On top of target, pick arbitrary direction
            (1.0, 0.0)
        };

        // Move to preferred_min_range + a small buffer so we don't immediately re-trigger
        let retreat_dist = min_range + 2.0;
        let retreat_x = (target_pos.x as f32 + dir_x * retreat_dist).clamp(0.0, 249.0) as u32;
        let retreat_y = (target_pos.y as f32 + dir_y * retreat_dist).clamp(0.0, 249.0) as u32;

        brain.proposals.push(Task::new(
            "maintain_range", 65, TaskSource::Evaluator,
            vec![Action::MoveToPosition { x: retreat_x, y: retreat_y }],
        ));
    }
}
