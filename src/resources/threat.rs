use std::collections::HashMap;

use bevy::prelude::*;

/// Per-entity threat table. Enemies track how much threat each attacker has generated.
#[derive(Component, Clone, Debug, Default)]
pub struct ThreatTable {
    entries: HashMap<Entity, f32>,
}

impl ThreatTable {
    pub fn add_threat(&mut self, source: Entity, amount: f32) {
        *self.entries.entry(source).or_insert(0.0) += amount;
    }

    /// Get the entity with the highest threat.
    pub fn highest_threat(&self) -> Option<Entity> {
        self.entries
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&entity, _)| entity)
    }

    pub fn get_threat(&self, source: Entity) -> f32 {
        self.entries.get(&source).copied().unwrap_or(0.0)
    }

    /// Remove a dead or despawned entity from the table.
    pub fn remove(&mut self, entity: Entity) {
        self.entries.remove(&entity);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All entities this entity has threat against.
    pub fn all_targets(&self) -> Vec<Entity> {
        self.entries.keys().copied().collect()
    }
}
