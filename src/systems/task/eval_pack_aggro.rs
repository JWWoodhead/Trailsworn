use std::collections::HashMap;

use bevy::prelude::*;

use crate::resources::pack::PackId;
use crate::resources::threat::ThreatTable;

/// Propagate threat across pack members. When any member of a pack has threat,
/// all other members with empty threat tables receive a seed threat entry.
/// This causes the entire pack to aggro via the existing `engage_combat` evaluator.
pub fn propagate_pack_aggro(mut query: Query<(Entity, &PackId, &mut ThreatTable)>) {
    // Pass 1: collect threat targets per pack.
    let mut pack_targets: HashMap<u32, Vec<Entity>> = HashMap::new();

    for (_entity, pack_id, threat_table) in &query {
        if threat_table.is_empty() {
            continue;
        }
        let targets = threat_table.all_targets();
        pack_targets
            .entry(pack_id.0)
            .or_default()
            .extend(targets);
    }

    // Deduplicate targets per pack.
    for targets in pack_targets.values_mut() {
        targets.sort_unstable();
        targets.dedup();
    }

    // Pass 2: seed threat on pack members that have empty threat tables.
    for (_entity, pack_id, mut threat_table) in &mut query {
        if !threat_table.is_empty() {
            continue;
        }
        if let Some(targets) = pack_targets.get(&pack_id.0) {
            for &target in targets {
                threat_table.add_threat(target, 1.0);
            }
        }
    }
}
