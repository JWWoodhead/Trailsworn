use std::collections::HashMap;

use crate::worldgen::gods::{GodId, GodRelationship};
use crate::worldgen::names::Race;
use crate::worldgen::world_map::WorldPos;

use super::artifacts::DivineArtifact;
use super::events::DivineEvent;
use super::personality::{DivineDrive, DivineFlaw, DivinePersonality};
use super::races::CreatedRace;
use super::sites::DivineSite;
use super::terrain_scars::TerrainScar;
use super::the_fall::TheFall;

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
    /// False after being vanquished in divine war.
    pub active: bool,
    pub vanquished_year: Option<i32>,
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
            active: true,
            vanquished_year: None,
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
        self.active
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

/// A mortal settlement during the divine era.
#[derive(Clone, Debug)]
pub struct DivineSettlement {
    pub pos: WorldPos,
    pub name: String,
    /// Which god this settlement worships, if any.
    pub patron_god: Option<GodId>,
    /// How devoted the settlement is to their patron (0-100).
    /// High devotion = hard to convert, low = vulnerable.
    pub devotion: u32,
    /// Year worship was established.
    pub patron_since: Option<i32>,
}

/// The mutable world state during the divine era simulation.
#[derive(Clone, Debug)]
pub struct DivineWorldState {
    pub relations: DivineRelationMatrix,
    pub active_wars: Vec<DivineWar>,
    pub active_pacts: Vec<DivinePact>,
    /// Which god owns each cell, indexed parallel to WorldMap.cells.
    pub territory_map: Vec<Option<GodId>>,
    /// Mortal settlements and their worship status.
    pub settlements: Vec<DivineSettlement>,
}

impl DivineWorldState {
    /// Check if two gods are currently at war.
    pub fn at_war(&self, a: GodId, b: GodId) -> bool {
        self.active_wars.iter().any(|w| {
            (w.aggressor == a && w.defender == b)
                || (w.aggressor == b && w.defender == a)
        })
    }

    /// Check if two gods have an active pact.
    pub fn have_pact(&self, a: GodId, b: GodId) -> bool {
        self.active_pacts.iter().any(|p| {
            (p.god_a == a && p.god_b == b) || (p.god_a == b && p.god_b == a)
        })
    }

    /// Count how many wars a god is currently in.
    pub fn war_count(&self, god_id: GodId) -> usize {
        self.active_wars
            .iter()
            .filter(|w| w.aggressor == god_id || w.defender == god_id)
            .count()
    }

    /// Count how many settlements worship a given god.
    pub fn worshipper_count(&self, god_id: GodId) -> usize {
        self.settlements
            .iter()
            .filter(|s| s.patron_god == Some(god_id))
            .count()
    }

    /// Get settlements that currently have no patron god.
    pub fn unpatronized_settlements(&self) -> Vec<usize> {
        self.settlements
            .iter()
            .enumerate()
            .filter(|(_, s)| s.patron_god.is_none())
            .map(|(i, _)| i)
            .collect()
    }

    /// Get settlements within a god's territory that worship a different god (or none).
    pub fn convertible_settlements(&self, god_id: GodId, territory_map: &[Option<GodId>], world_map: &crate::worldgen::world_map::WorldMap) -> Vec<usize> {
        self.settlements
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.patron_god != Some(god_id)
                    && world_map.idx(s.pos)
                        .is_some_and(|idx| territory_map[idx] == Some(god_id))
            })
            .map(|(i, _)| i)
            .collect()
    }
}

/// The complete output of the divine era simulation.
#[derive(Clone, Debug, bevy::prelude::Resource)]
pub struct DivineHistory {
    pub gods: Vec<GodState>,
    pub events: Vec<DivineEvent>,
    pub sites: Vec<DivineSite>,
    pub artifacts: Vec<DivineArtifact>,
    pub created_races: Vec<CreatedRace>,
    pub terrain_scars: Vec<TerrainScar>,
    pub current_year: i32,
    pub the_fall: Option<TheFall>,
}

impl DivineHistory {
    /// Get all divine sites at a specific world position that persist into the mortal era.
    pub fn sites_at(&self, pos: WorldPos) -> Vec<&DivineSite> {
        self.sites
            .iter()
            .filter(|s| s.world_pos == pos && s.persists)
            .collect()
    }

    /// Get all events for a specific god.
    pub fn events_for_god(&self, god_id: GodId) -> Vec<&DivineEvent> {
        self.events
            .iter()
            .filter(|e| e.participants.contains(&god_id))
            .collect()
    }

    /// Count how many events of a given kind occurred.
    pub fn event_count(&self, kind: &super::events::DivineEventKind) -> usize {
        self.events.iter().filter(|e| &e.kind == kind).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn god_state_defaults() {
        let p = DivinePersonality { drive: DivineDrive::Supremacy, flaw: DivineFlaw::Hubris };
        let gs = GodState::new(1, p);
        assert_eq!(gs.power, 80);
        assert!(gs.active);
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

    #[test]
    fn world_state_war_tracking() {
        let mut ws = DivineWorldState {
            relations: DivineRelationMatrix::default(),
            active_wars: Vec::new(),
            active_pacts: Vec::new(),
            territory_map: Vec::new(),
            settlements: Vec::new(),
        };
        assert!(!ws.at_war(1, 2));
        ws.active_wars.push(DivineWar {
            aggressor: 1,
            defender: 2,
            start_year: -90,
            contested_cells: vec![],
        });
        assert!(ws.at_war(1, 2));
        assert!(ws.at_war(2, 1));
        assert!(!ws.at_war(1, 3));
        assert_eq!(ws.war_count(1), 1);
        assert_eq!(ws.war_count(3), 0);
    }

    #[test]
    fn world_state_pact_tracking() {
        let mut ws = DivineWorldState {
            relations: DivineRelationMatrix::default(),
            active_wars: Vec::new(),
            active_pacts: Vec::new(),
            territory_map: Vec::new(),
            settlements: Vec::new(),
        };
        assert!(!ws.have_pact(1, 2));
        ws.active_pacts.push(DivinePact {
            god_a: 1,
            god_b: 2,
            formed_year: -80,
            kind: PactKind::NonAggression,
        });
        assert!(ws.have_pact(1, 2));
        assert!(ws.have_pact(2, 1));
        assert!(!ws.have_pact(1, 3));
    }
}
