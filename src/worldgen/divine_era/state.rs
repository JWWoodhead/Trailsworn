use std::collections::HashMap;

use crate::worldgen::gods::{GodId, GodRelationship};
use crate::worldgen::names::Race;
use crate::worldgen::world_map::WorldPos;

use super::personality::{DivineDrive, DivineFlaw, DivinePersonality};

/// Per-god mutable state during the divine era simulation.
#[derive(Clone, Debug)]
pub struct GodState {
    pub god_id: GodId,

    // Personality
    /// The god's core drive and flaw — what they want, how it breaks them.
    pub personality: DivinePersonality,

    // Territory
    /// All cells this god has claimed.
    pub territory: Vec<WorldPos>,
    /// First ~20 cells near seat of power.
    pub core_territory: Vec<WorldPos>,
    /// The god's seat of power (first claimed cell).
    pub seat_of_power: Option<WorldPos>,

    // Gauges (0-100)
    /// Divine power — scales with worshippers, determines ability to act.
    pub power: u32,
    /// Index into propp_tendencies — current narrative position.
    pub narrative_phase: usize,

    // State
    /// True if the god has faded (zero worshippers for too long). Faded gods cannot act.
    pub faded: bool,
    /// How many consecutive years this god has had zero worshippers.
    pub years_without_worship: u32,
    /// ID of the race this god created, if any.
    pub created_race_id: Option<u32>,
    /// Mortal champion chosen by this god.
    pub champion_name: Option<String>,
    pub champion_race: Option<Race>,
    /// Settlements that worship this god (by world position).
    pub worshipper_settlements: Vec<WorldPos>,
    /// Pressure building toward flaw triggering (0-100). Triggers at 80+.
    pub flaw_pressure: u32,

    // Counters
    pub wars_fought: u32,
    pub wars_won: u32,
    pub artifacts_created: u32,
    pub sites_created: u32,
}

impl GodState {
    /// Create initial god state for the start of the divine era.
    pub fn new(god_id: GodId, personality: DivinePersonality) -> Self {
        Self {
            god_id,
            personality,
            territory: Vec::new(),
            core_territory: Vec::new(),
            seat_of_power: None,
            power: 80,
            narrative_phase: 0,
            faded: false,
            years_without_worship: 0,
            created_race_id: None,
            champion_name: None,
            champion_race: None,
            worshipper_settlements: Vec::new(),
            flaw_pressure: 0,
            wars_fought: 0,
            wars_won: 0,
            artifacts_created: 0,
            sites_created: 0,
        }
    }

    pub fn drive(&self) -> DivineDrive {
        self.personality.drive
    }

    pub fn flaw(&self) -> DivineFlaw {
        self.personality.flaw
    }

    pub fn is_active(&self) -> bool {
        !self.faded
    }
}

/// Pairwise god relationships — mutable during simulation.
/// Initialized from DrawnPantheon relationships, then drifts.
#[derive(Clone, Debug, Default)]
pub struct DivineRelationMatrix {
    sentiments: HashMap<(GodId, GodId), i32>,
}

impl DivineRelationMatrix {
    fn key(a: GodId, b: GodId) -> (GodId, GodId) {
        if a <= b { (a, b) } else { (b, a) }
    }

    pub fn get(&self, a: GodId, b: GodId) -> i32 {
        if a == b { return 100; }
        self.sentiments.get(&Self::key(a, b)).copied().unwrap_or(0)
    }

    pub fn set(&mut self, a: GodId, b: GodId, value: i32) {
        if a == b { return; }
        self.sentiments.insert(Self::key(a, b), value.clamp(-100, 100));
    }

    pub fn modify(&mut self, a: GodId, b: GodId, delta: i32) {
        let current = self.get(a, b);
        self.set(a, b, current + delta);
    }

    pub fn is_hostile(&self, a: GodId, b: GodId) -> bool {
        self.get(a, b) < -30
    }

    pub fn is_friendly(&self, a: GodId, b: GodId) -> bool {
        self.get(a, b) > 30
    }

    /// Find the most hostile pair among the given god IDs.
    pub fn most_hostile_pair(&self, god_ids: &[GodId]) -> Option<(GodId, GodId, i32)> {
        let mut worst: Option<(GodId, GodId, i32)> = None;
        for i in 0..god_ids.len() {
            for j in (i + 1)..god_ids.len() {
                let a = god_ids[i];
                let b = god_ids[j];
                let s = self.get(a, b);
                if worst.is_none() || s < worst.unwrap().2 {
                    worst = Some((a, b, s));
                }
            }
        }
        worst
    }

    /// Initialize from the DrawnPantheon's computed relationships.
    pub fn from_relationships(relationships: &[GodRelationship]) -> Self {
        let mut matrix = Self::default();
        for rel in relationships {
            matrix.set(rel.god_a, rel.god_b, rel.affinity);
        }
        matrix
    }

    /// Drift all sentiments 1 point toward 0.
    pub fn drift_toward_neutral(&mut self) {
        for (_, sentiment) in self.sentiments.iter_mut() {
            if *sentiment > 0 {
                *sentiment -= 1;
            } else if *sentiment < 0 {
                *sentiment += 1;
            }
        }
    }
}

/// An active divine war.
#[derive(Clone, Debug)]
pub struct DivineWar {
    pub aggressor: GodId,
    pub defender: GodId,
    pub start_year: i32,
    /// Cells being fought over.
    pub contested_cells: Vec<WorldPos>,
}

/// A pact between two gods.
#[derive(Clone, Debug)]
pub struct DivinePact {
    pub god_a: GodId,
    pub god_b: GodId,
    pub formed_year: i32,
    pub kind: PactKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PactKind {
    NonAggression,
    SharedDomain,
    MutualDefense,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn god_state_defaults() {
        let p = DivinePersonality { drive: DivineDrive::Supremacy, flaw: DivineFlaw::Hubris };
        let gs = GodState::new(1, p);
        assert_eq!(gs.power, 80);
        assert!(gs.is_active());
        assert!(!gs.faded);
        assert!(gs.territory.is_empty());
        assert!(gs.seat_of_power.is_none());
        assert_eq!(gs.drive(), DivineDrive::Supremacy);
        assert_eq!(gs.flaw(), DivineFlaw::Hubris);
        assert!(gs.worshipper_settlements.is_empty());
        assert_eq!(gs.flaw_pressure, 0);
    }

    #[test]
    fn relation_matrix_symmetric() {
        let mut rm = DivineRelationMatrix::default();
        rm.set(1, 2, 50);
        assert_eq!(rm.get(1, 2), 50);
        assert_eq!(rm.get(2, 1), 50);
    }

    #[test]
    fn relation_matrix_clamps() {
        let mut rm = DivineRelationMatrix::default();
        rm.set(1, 2, 200);
        assert_eq!(rm.get(1, 2), 100);
        rm.set(1, 2, -200);
        assert_eq!(rm.get(1, 2), -100);
    }

    #[test]
    fn relation_matrix_modify() {
        let mut rm = DivineRelationMatrix::default();
        rm.set(1, 2, 10);
        rm.modify(1, 2, -25);
        assert_eq!(rm.get(1, 2), -15);
    }

    #[test]
    fn hostile_threshold() {
        let mut rm = DivineRelationMatrix::default();
        rm.set(1, 2, -31);
        assert!(rm.is_hostile(1, 2));
        rm.set(1, 2, -30);
        assert!(!rm.is_hostile(1, 2));
    }

    #[test]
    fn self_sentiment_is_max() {
        let rm = DivineRelationMatrix::default();
        assert_eq!(rm.get(5, 5), 100);
    }

    #[test]
    fn most_hostile_pair_finds_worst() {
        let mut rm = DivineRelationMatrix::default();
        rm.set(1, 2, -50);
        rm.set(1, 3, -10);
        rm.set(2, 3, -80);
        let (a, b, s) = rm.most_hostile_pair(&[1, 2, 3]).unwrap();
        assert_eq!(s, -80);
        assert!((a == 2 && b == 3) || (a == 3 && b == 2));
    }

    #[test]
    fn drift_toward_neutral() {
        let mut rm = DivineRelationMatrix::default();
        rm.set(1, 2, 50);
        rm.set(3, 4, -30);
        rm.drift_toward_neutral();
        assert_eq!(rm.get(1, 2), 49);
        assert_eq!(rm.get(3, 4), -29);
    }

    #[test]
    fn from_relationships_init() {
        let rels = vec![
            GodRelationship { god_a: 1, god_b: 2, affinity: -40, reason: "test".into() },
            GodRelationship { god_a: 1, god_b: 3, affinity: 30, reason: "test".into() },
        ];
        let rm = DivineRelationMatrix::from_relationships(&rels);
        assert_eq!(rm.get(1, 2), -40);
        assert_eq!(rm.get(1, 3), 30);
        assert_eq!(rm.get(2, 3), 0); // not set
    }

}
