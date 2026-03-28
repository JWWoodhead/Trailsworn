use bevy::prelude::*;

use crate::resources::combat_behavior::CombatBehavior;
use crate::resources::task::PartyMode;
use crate::resources::body::Body;
use crate::resources::combat::{Dead, InCombat};
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::map::GridPosition;
use crate::resources::task::{Action, AiBrain, Task, TaskEvaluator, TaskSource};
use crate::resources::threat::ThreatTable;

/// Propose engaging only when the entity has been attacked (threat table non-empty).
pub fn defend_self(
    faction_relations: Res<FactionRelations>,
    mut query: Query<(
        Entity,
        &GridPosition,
        &Faction,
        &CombatBehavior,
        Option<&ThreatTable>,
        &mut AiBrain,
    )>,
    potential_targets: Query<(Entity, &GridPosition, &Faction, &Body), (With<InCombat>, Without<Dead>)>,
) {
    for (entity, grid_pos, faction, behavior, threat_table, mut brain) in &mut query {
        if brain.combat_eval_cooldown != 0 {
            continue;
        }
        if !brain.evaluators.iter().any(|e| matches!(e, TaskEvaluator::DefendSelf)) {
            continue;
        }
        if threat_table.is_none_or(|t| t.is_empty()) {
            continue;
        }
        let target = match super::select_target(
            entity, grid_pos, faction, behavior.aggro_range,
            Some(&PartyMode::Defensive), threat_table,
            &faction_relations, &potential_targets,
        ) {
            Some(t) => t,
            None => continue,
        };
        brain.proposals.push(Task::new(
            "defend", 60, TaskSource::Evaluator,
            vec![Action::EngageTarget { target, attack_range: behavior.attack_range }],
        ));
    }
}
