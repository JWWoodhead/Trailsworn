use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use super::items::{EquipSlot, ItemInstanceId};
use crate::worldgen::WorldPos;

// ---------------------------------------------------------------------------
// Zone spawn index — deterministic entity identification
// ---------------------------------------------------------------------------

/// Assigned to each zone-spawned entity during POI iteration.
/// Matches the deterministic spawn order so entities can be identified
/// across zone transitions without Entity references.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ZoneSpawnIndex(pub u32);

// ---------------------------------------------------------------------------
// Entity snapshot — mutable state delta from base spawn
// ---------------------------------------------------------------------------

/// Snapshot of a single alive entity's mutable state.
/// Only stores state that can change from the deterministic base spawn:
/// position, body HP, and resources. Everything else (attributes, abilities,
/// AI config) is reconstructed from the base generation.
#[derive(Clone, Debug)]
pub struct EntitySnapshot {
    /// Current tile position (may have moved from spawn point).
    pub position: (u32, u32),
    /// Per-body-part current HP (parallel to BodyTemplate.parts).
    pub body_part_hp: Vec<f32>,
    /// Per-body-part destroyed flag (parallel to BodyTemplate.parts).
    pub body_part_destroyed: Vec<bool>,
    /// Current mana.
    pub mana_current: f32,
    /// Current stamina.
    pub stamina_current: f32,
    /// Equipped item instance IDs to preserve across zone transitions.
    pub equipment_instance_ids: Vec<(EquipSlot, ItemInstanceId)>,
}

// ---------------------------------------------------------------------------
// Zone snapshot — per-zone delta from deterministic base
// ---------------------------------------------------------------------------

/// Complete delta from the deterministic base state of a zone.
/// Stored when the player leaves a zone, applied when they return.
#[derive(Clone, Debug, Default)]
pub struct ZoneSnapshot {
    /// Spawn indices of entities that have been killed.
    pub dead_indices: HashSet<u32>,
    /// Per-entity state overrides for surviving entities.
    pub alive_overrides: HashMap<u32, EntitySnapshot>,
    /// ItemInstanceIds that belong to this zone's alive entities.
    /// These must NOT be cleaned up from ItemInstanceRegistry during transitions.
    pub preserved_item_instances: HashSet<ItemInstanceId>,
}

// ---------------------------------------------------------------------------
// Zone state cache — global resource
// ---------------------------------------------------------------------------

/// Per-zone state storage, keyed by world position.
/// Populated as the player moves between zones.
#[derive(Resource, Default)]
pub struct ZoneStateCache {
    snapshots: HashMap<WorldPos, ZoneSnapshot>,
}

impl ZoneStateCache {
    pub fn get(&self, pos: &WorldPos) -> Option<&ZoneSnapshot> {
        self.snapshots.get(pos)
    }

    pub fn get_or_create_mut(&mut self, pos: WorldPos) -> &mut ZoneSnapshot {
        self.snapshots.entry(pos).or_default()
    }

    pub fn insert(&mut self, pos: WorldPos, snapshot: ZoneSnapshot) {
        self.snapshots.insert(pos, snapshot);
    }

    pub fn remove(&mut self, pos: &WorldPos) -> Option<ZoneSnapshot> {
        self.snapshots.remove(pos)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_insert_and_get() {
        let mut cache = ZoneStateCache::default();
        let pos = WorldPos::new(2, 3);
        let mut snapshot = ZoneSnapshot::default();
        snapshot.dead_indices.insert(0);
        cache.insert(pos, snapshot);

        let retrieved = cache.get(&pos).unwrap();
        assert!(retrieved.dead_indices.contains(&0));
        assert!(cache.get(&WorldPos::new(0, 0)).is_none());
    }

    #[test]
    fn cache_overwrite() {
        let mut cache = ZoneStateCache::default();
        let pos = WorldPos::new(1, 1);
        let mut snap1 = ZoneSnapshot::default();
        snap1.dead_indices.insert(0);
        cache.insert(pos, snap1);

        let mut snap2 = ZoneSnapshot::default();
        snap2.dead_indices.insert(5);
        cache.insert(pos, snap2);

        let retrieved = cache.get(&pos).unwrap();
        assert!(!retrieved.dead_indices.contains(&0));
        assert!(retrieved.dead_indices.contains(&5));
    }

    #[test]
    fn get_or_create_mut_creates_default() {
        let mut cache = ZoneStateCache::default();
        let pos = WorldPos::new(3, 3);
        let snapshot = cache.get_or_create_mut(pos);
        assert!(snapshot.dead_indices.is_empty());
        snapshot.dead_indices.insert(2);

        assert!(cache.get(&pos).unwrap().dead_indices.contains(&2));
    }

    #[test]
    fn snapshot_tracks_dead_and_alive() {
        let mut snapshot = ZoneSnapshot::default();
        snapshot.dead_indices.insert(0);
        snapshot.dead_indices.insert(3);

        snapshot.alive_overrides.insert(1, EntitySnapshot {
            position: (50, 60),
            body_part_hp: vec![25.0, 10.0],
            body_part_destroyed: vec![false, false],
            mana_current: 30.0,
            stamina_current: 45.0,
            equipment_instance_ids: vec![(EquipSlot::MainHand, ItemInstanceId(42))],
        });

        assert!(snapshot.dead_indices.contains(&0));
        assert!(snapshot.dead_indices.contains(&3));
        assert!(!snapshot.dead_indices.contains(&1));

        let alive = snapshot.alive_overrides.get(&1).unwrap();
        assert_eq!(alive.position, (50, 60));
        assert!((alive.mana_current - 30.0).abs() < 0.001);
        assert_eq!(alive.equipment_instance_ids[0].1, ItemInstanceId(42));
    }

    #[test]
    fn preserved_item_instances_tracked() {
        let mut snapshot = ZoneSnapshot::default();
        snapshot.preserved_item_instances.insert(ItemInstanceId(1));
        snapshot.preserved_item_instances.insert(ItemInstanceId(2));

        assert!(snapshot.preserved_item_instances.contains(&ItemInstanceId(1)));
        assert!(!snapshot.preserved_item_instances.contains(&ItemInstanceId(99)));
    }
}
