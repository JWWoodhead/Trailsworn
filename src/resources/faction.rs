use std::collections::HashMap;

use bevy::prelude::*;

pub type FactionId = u32;

/// Sentinel value for entities with no faction.
pub const FACTION_NONE: FactionId = 0;
/// The player's party faction.
pub const FACTION_PLAYER: FactionId = 1;
/// Wildlife faction — neutral until attacked.
pub const FACTION_WILDLIFE: FactionId = 3;

/// Which faction an entity belongs to.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Faction(pub FactionId);

/// How one faction feels about another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Disposition {
    Hostile,
    Neutral,
    Friendly,
}

/// Stores pairwise faction relationships.
/// Lookups are order-independent: relation(A, B) == relation(B, A).
#[derive(Resource, Default)]
pub struct FactionRelations {
    relations: HashMap<(FactionId, FactionId), Disposition>,
}

impl FactionRelations {
    fn key(a: FactionId, b: FactionId) -> (FactionId, FactionId) {
        if a <= b { (a, b) } else { (b, a) }
    }

    pub fn set(&mut self, a: FactionId, b: FactionId, disposition: Disposition) {
        self.relations.insert(Self::key(a, b), disposition);
    }

    pub fn get(&self, a: FactionId, b: FactionId) -> Disposition {
        if a == b {
            return Disposition::Friendly;
        }
        self.relations
            .get(&Self::key(a, b))
            .copied()
            .unwrap_or(Disposition::Neutral)
    }

    pub fn is_hostile(&self, a: FactionId, b: FactionId) -> bool {
        self.get(a, b) == Disposition::Hostile
    }

    pub fn is_friendly(&self, a: FactionId, b: FactionId) -> bool {
        self.get(a, b) == Disposition::Friendly
    }
}
