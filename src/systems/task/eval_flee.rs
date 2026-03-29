use bevy::prelude::*;

use crate::resources::combat_behavior::CombatBehavior;
use crate::resources::body::Health;
use crate::resources::task::{
    Action, AiBrain, Task, TaskEvaluator, TaskSource, COMBAT_EVAL_INTERVAL,
};
use crate::resources::threat::ThreatTable;

/// Propose fleeing when HP drops below threshold.
pub fn flee(
    mut query: Query<(&Health, &CombatBehavior, Option<&ThreatTable>, &mut AiBrain)>,
) {
    for (health, behavior, threat_table, mut brain) in &mut query {
        if brain.combat_eval_cooldown != 0 {
            continue;
        }
        if !brain.evaluators.iter().any(|e| matches!(e, TaskEvaluator::Flee)) {
            continue;
        }
        if let Some(proposal) = evaluate(health, behavior, threat_table) {
            brain.proposals.push(proposal);
        }
    }
}

fn evaluate(
    health: &Health,
    behavior: &CombatBehavior,
    threat_table: Option<&ThreatTable>,
) -> Option<Task> {
    if behavior.flee_hp_threshold <= 0.0 {
        return None;
    }
    let hp_fraction = health.fraction();
    if hp_fraction >= behavior.flee_hp_threshold {
        return None;
    }

    let actions = if let Some(threat) = threat_table.and_then(|t| t.highest_threat()) {
        vec![Action::FleeFrom { threat }]
    } else {
        vec![Action::Wait { ticks: COMBAT_EVAL_INTERVAL * 2, elapsed: 0 }]
    };

    Some(Task::new("flee", 90, TaskSource::Evaluator, actions))
}
