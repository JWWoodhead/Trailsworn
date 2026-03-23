use bevy::prelude::*;

use crate::worldgen::WorldPos;

/// Tracks the current zone the player is in.
#[derive(Resource)]
pub struct CurrentZone {
    pub world_pos: WorldPos,
    /// Seed used for the current zone's generation (derived from world seed + position).
    pub zone_seed: u64,
    /// The world seed.
    pub world_seed: u64,
}

impl CurrentZone {
    pub fn new(world_seed: u64, pos: WorldPos) -> Self {
        Self {
            world_pos: pos,
            zone_seed: zone_seed(world_seed, pos),
            world_seed,
        }
    }

    pub fn move_to(&mut self, pos: WorldPos) {
        self.world_pos = pos;
        self.zone_seed = zone_seed(self.world_seed, pos);
    }
}

/// Derive a deterministic zone seed from world seed + position.
pub fn zone_seed(world_seed: u64, pos: WorldPos) -> u64 {
    let mut seed = world_seed;
    seed ^= (pos.x as u64).wrapping_mul(0x517cc1b727220a95);
    seed ^= (pos.y as u64).wrapping_mul(0x6c62272e07bb0142);
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xff51afd7ed558ccd);
    seed ^= seed >> 33;
    seed
}

/// Event fired when the player transitions to a new zone.
#[derive(Message, Clone, Debug)]
pub struct ZoneTransitionEvent {
    pub new_pos: WorldPos,
    /// Which edge the player exited from, to determine spawn position in the new zone.
    pub entry_edge: EntryEdge,
}

#[derive(Clone, Copy, Debug)]
pub enum EntryEdge {
    North,
    South,
    East,
    West,
    /// Spawning fresh (game start or cave entrance/exit).
    Center,
}
