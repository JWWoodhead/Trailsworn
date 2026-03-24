use bevy::prelude::*;

use crate::resources::combat_behavior::CombatBehavior;
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::task::{
    Action, AiBrain, Task, TaskEvaluator, TaskSource, COMBAT_EVAL_INTERVAL,
};
use crate::resources::threat::ThreatTable;

/// Propose fleeing when HP drops below threshold.
pub fn flee(
    body_templates: Res<BodyTemplates>,
    mut query: Query<(&Body, &CombatBehavior, Option<&ThreatTable>, &mut AiBrain)>,
) {
    for (body, behavior, threat_table, mut brain) in &mut query {
        if brain.combat_eval_cooldown != 0 {
            continue;
        }
        if !brain.evaluators.iter().any(|e| matches!(e, TaskEvaluator::Flee)) {
            continue;
        }
        if let Some(proposal) = evaluate(body, behavior, threat_table, &body_templates) {
            brain.proposals.push(proposal);
        }
    }
}

fn evaluate(
    body: &Body,
    behavior: &CombatBehavior,
    threat_table: Option<&ThreatTable>,
    body_templates: &BodyTemplates,
) -> Option<Task> {
    if behavior.flee_hp_threshold <= 0.0 {
        return None;
    }
    let template = body_templates.get(&body.template_id)?;
    let hp_fraction = 1.0 - body.pain_level(template);
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
