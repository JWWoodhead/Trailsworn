use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::*;

/// A stable identifier that persists across save/load.
/// Unlike `Entity`, this ID is deterministic and serializable.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StableId(pub u64);

/// Global counter for generating unique StableIds.
static NEXT_STABLE_ID: AtomicU64 = AtomicU64::new(1);

impl StableId {
    /// Generate a new unique StableId.
    pub fn next() -> Self {
        Self(NEXT_STABLE_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Set the counter to resume from a specific value (for save/load).
    pub fn set_counter(value: u64) {
        NEXT_STABLE_ID.store(value, Ordering::Relaxed);
    }
}

/// Maps StableId → Entity for fast lookups.
/// Updated when entities with StableId are spawned or despawned.
#[derive(Resource, Default)]
pub struct StableIdRegistry {
    map: HashMap<StableId, Entity>,
}

impl StableIdRegistry {
    pub fn register(&mut self, id: StableId, entity: Entity) {
        self.map.insert(id, entity);
    }

    pub fn remove(&mut self, id: &StableId) {
        self.map.remove(id);
    }

    pub fn get(&self, id: &StableId) -> Option<Entity> {
        self.map.get(id).copied()
    }
}

/// System that registers newly added StableId components.
pub fn register_stable_ids(
    mut registry: ResMut<StableIdRegistry>,
    query: Query<(Entity, &StableId), Added<StableId>>,
) {
    for (entity, id) in &query {
        registry.register(*id, entity);
    }
}

/// System that cleans up despawned entities from the registry.
pub fn cleanup_stable_ids(
    mut registry: ResMut<StableIdRegistry>,
    mut removals: RemovedComponents<StableId>,
) {
    // RemovedComponents doesn't give us the StableId value directly,
    // so we do a reverse lookup. This is rare (only on despawn).
    let removed_entities: Vec<Entity> = removals.read().collect();
    if removed_entities.is_empty() {
        return;
    }
    registry.map.retain(|_, entity| !removed_entities.contains(entity));
}
