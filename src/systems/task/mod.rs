mod eval_defend_self;
mod eval_engage_combat;
mod eval_flee;
mod eval_follow_leader;
mod eval_use_ability;
mod execute_actions;
mod schedule;

pub use eval_defend_self::defend_self;
pub use eval_engage_combat::engage_combat;
pub use eval_flee::flee;
pub use eval_follow_leader::follow_leader;
pub use eval_use_ability::use_ability;
pub use execute_actions::execute_actions;
pub use schedule::{advance_eval_timers, assign_task};

use bevy::prelude::*;

use crate::resources::body::Body;
use crate::resources::combat::InCombat;
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::map::GridPosition;
use crate::resources::threat::ThreatTable;

/// Shared target selection logic used by engage_combat and defend_self evaluators.
pub(super) fn select_target(
    self_entity: Entity,
    self_pos: &GridPosition,
    self_faction: &Faction,
    aggro_range: f32,
    party_mode: Option<&crate::resources::task::PartyMode>,
    threat_table: Option<&ThreatTable>,
    faction_relations: &FactionRelations,
    potential_targets: &Query<(Entity, &GridPosition, &Faction, &Body), With<InCombat>>,
) -> Option<Entity> {
    if let Some(table) = threat_table {
        if let Some(highest) = table.highest_threat() {
            if let Ok((_, _, target_faction, _)) = potential_targets.get(highest) {
                if faction_relations.is_hostile(self_faction.0, target_faction.0) {
                    return Some(highest);
                }
            }
        }
    }

    if let Some(&crate::resources::task::PartyMode::Defensive) = party_mode {
        if threat_table.is_none_or(|t| t.is_empty()) {
            return None;
        }
    }

    if let Some(&crate::resources::task::PartyMode::Follow) = party_mode {
        return None;
    }

    let aggro_range_sq = (aggro_range * aggro_range) as u32;
    let mut best: Option<(Entity, u32)> = None;
    for (target_entity, target_pos, target_faction, _) in potential_targets.iter() {
        if target_entity == self_entity {
            continue;
        }
        if !faction_relations.is_hostile(self_faction.0, target_faction.0) {
            continue;
        }
        let dx = self_pos.x.abs_diff(target_pos.x);
        let dy = self_pos.y.abs_diff(target_pos.y);
        let dist_sq = dx * dx + dy * dy;
        if dist_sq > aggro_range_sq {
            continue;
        }
        if best.is_none() || dist_sq < best.unwrap().1 {
            best = Some((target_entity, dist_sq));
        }
    }

    best.map(|(e, _)| e)
}
