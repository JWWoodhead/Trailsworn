use bevy::prelude::*;

use crate::resources::combat_behavior::CombatBehavior;
use crate::resources::task::PartyMode;
use crate::resources::body::Body;
use crate::resources::combat::{Dead, InCombat};
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::map::GridPosition;
use crate::resources::task::{Action, AiBrain, Task, TaskEvaluator, TaskSource};
use crate::resources::threat::ThreatTable;

/// Propose engaging the nearest hostile target.
pub fn engage_combat(
    faction_relations: Res<FactionRelations>,
    mut query: Query<(
        Entity,
        &GridPosition,
        &Faction,
        &CombatBehavior,
        &mut AiBrain,
        Option<&PartyMode>,
        Option<&ThreatTable>,
    )>,
    potential_targets: Query<(Entity, &GridPosition, &Faction, &Body), (With<InCombat>, Without<Dead>)>,
) {
    for (entity, grid_pos, faction, behavior, mut brain, party_mode, threat_table) in &mut query {
        if brain.combat_eval_cooldown != 0 {
            continue;
        }
        if !brain.evaluators.iter().any(|e| matches!(e, TaskEvaluator::EngageCombat)) {
            continue;
        }
        let target = match super::select_target(
            entity, grid_pos, faction, behavior.aggro_range,
            party_mode, threat_table, &faction_relations, &potential_targets,
        ) {
            Some(t) => t,
            None => continue,
        };
        brain.proposals.push(Task::new(
            "engage", 60, TaskSource::Evaluator,
            vec![Action::EngageTarget { target, attack_range: behavior.attack_range }],
        ));
    }
}
