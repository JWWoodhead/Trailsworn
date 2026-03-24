use bevy::prelude::*;

use crate::resources::game_time::GameTime;
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::task::{
    AiBrain, CurrentTask, COMBAT_EVAL_INTERVAL, ROUTINE_EVAL_INTERVAL,
};

/// Decrement evaluation cooldowns and clear stale proposals.
/// Runs before evaluators each frame.
pub fn advance_eval_timers(
    game_time: Res<GameTime>,
    mut query: Query<&mut AiBrain>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }
    for mut brain in &mut query {
        for _ in 0..game_time.ticks_this_frame {
            brain.combat_eval_cooldown = brain.combat_eval_cooldown.saturating_sub(1);
            brain.routine_eval_cooldown = brain.routine_eval_cooldown.saturating_sub(1);
        }
        brain.proposals.clear();
    }
}

/// Pick the best proposal from each AiBrain and assign it as the current task.
/// Runs after all evaluators.
pub fn assign_task(
    status_registry: Res<StatusEffectRegistry>,
    mut query: Query<(Entity, &ActiveStatusEffects, &mut AiBrain, Option<&CurrentTask>)>,
    mut commands: Commands,
) {
    for (entity, status_effects, mut brain, current_task) in &mut query {
        let eval_combat = brain.combat_eval_cooldown == 0;
        let eval_routine = brain.routine_eval_cooldown == 0;

        // CC — incapacitated entities discard proposals but don't reset cooldowns
        // so they evaluate immediately after recovering
        let cc_flags = status_effects.combined_cc_flags(&status_registry);
        if cc_flags.is_incapacitated() {
            brain.proposals.clear();
            continue;
        }

        if !eval_combat && !eval_routine {
            brain.proposals.clear();
            continue;
        }

        let best = brain.proposals.drain(..).max_by_key(|t| t.priority);

        if let Some(proposal) = best {
            let should_set = match &current_task {
                None => true,
                Some(ct) => ct.should_replace(&proposal),
            };
            if should_set {
                commands.entity(entity).insert(CurrentTask::new(proposal));
            }
        }

        if eval_combat {
            brain.combat_eval_cooldown = COMBAT_EVAL_INTERVAL;
        }
        if eval_routine {
            brain.routine_eval_cooldown = ROUTINE_EVAL_INTERVAL;
        }
    }
}
