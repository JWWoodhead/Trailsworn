use std::collections::HashMap;

use crate::worldgen::divine::state::{DivineRelationMatrix, DivineWar, DivinePact};
use crate::worldgen::divine::gods::GodId;
use crate::worldgen::names::{FactionType, Race};
use crate::worldgen::world_map::WorldPos;
use crate::worldgen::zone::ZoneType;

/// Pairwise faction sentiment. Range -100 (blood feud) to +100 (sworn brothers).
#[derive(Clone, Debug, Default)]
pub struct RelationMatrix {
    sentiments: HashMap<(u32, u32), i32>,
}

impl RelationMatrix {
    fn key(a: u32, b: u32) -> (u32, u32) {
        if a <= b { (a, b) } else { (b, a) }
    }

    pub fn get(&self, a: u32, b: u32) -> i32 {
        if a == b { return 100; }
        self.sentiments.get(&Self::key(a, b)).copied().unwrap_or(0)
    }

    pub fn set(&mut self, a: u32, b: u32, value: i32) {
        if a == b { return; }
        self.sentiments.insert(Self::key(a, b), value.clamp(-100, 100));
    }

    pub fn modify(&mut self, a: u32, b: u32, delta: i32) {
        let current = self.get(a, b);
        self.set(a, b, current + delta);
    }

    pub fn is_hostile(&self, a: u32, b: u32) -> bool {
        self.get(a, b) < -30
    }

    pub fn is_friendly(&self, a: u32, b: u32) -> bool {
        self.get(a, b) > 30
    }

    /// Find the most hostile pair among the given faction IDs.
    pub fn most_hostile_pair(&self, faction_ids: &[u32]) -> Option<(u32, u32, i32)> {
        let mut worst: Option<(u32, u32, i32)> = None;
        for i in 0..faction_ids.len() {
            for j in (i + 1)..faction_ids.len() {
                let a = faction_ids[i];
                let b = faction_ids[j];
                let s = self.get(a, b);
                if worst.is_none() || s < worst.unwrap().2 {
                    worst = Some((a, b, s));
                }
            }
        }
        worst
    }

    /// Initialize sentiments between two factions based on racial/type defaults.
    pub fn initialize_pair(&mut self, a: &FactionState, b: &FactionState) {
        let mut base = 0i32;

        // Same race bonus
        if a.race == b.race {
            base += 10;
        }

        // Historical racial tensions
        match (a.race, b.race) {
            (Race::Elf, Race::Orc) | (Race::Orc, Race::Elf) => base -= 15,
            (Race::Dwarf, Race::Goblin) | (Race::Goblin, Race::Dwarf) => base -= 15,
            (Race::Dwarf, Race::Elf) | (Race::Elf, Race::Dwarf) => base += 5,
            (Race::Orc, Race::Goblin) | (Race::Goblin, Race::Orc) => base += 5,
            _ => {}
        }

        // Same region proximity
        if a.home_region == b.home_region {
            base += 5;
        }

        // Type-based friction
        match (a.faction_type, b.faction_type) {
            (FactionType::Kingdom, FactionType::BanditClan)
            | (FactionType::BanditClan, FactionType::Kingdom) => {
                if a.home_region == b.home_region {
                    base -= 20;
                }
            }
            (FactionType::ReligiousOrder, FactionType::MageCircle)
            | (FactionType::MageCircle, FactionType::ReligiousOrder) => base -= 10,
            (FactionType::MerchantGuild, _) | (_, FactionType::MerchantGuild) => base += 5,
            _ => {}
        }

        self.set(a.id, b.id, base);
    }

    /// Drift all sentiments 1 point toward 0 (grudges/friendships fade).
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

/// The mutable state of a faction during simulation.
#[derive(Clone, Debug)]
pub struct FactionState {
    // Identity (immutable after creation)
    pub id: u32,
    pub name: String,
    pub faction_type: FactionType,
    pub race: Race,
    pub founded_year: i32,
    pub home_region: String,

    // Mutable state
    pub dissolved_year: Option<i32>,
    pub leader_name: String,
    pub leader_id: Option<u32>, // Character id, filled in step 4
    /// Abstract military power, 1-100.
    pub military_strength: u32,
    /// Abstract economic power, 1-100.
    pub wealth: u32,
    /// Internal cohesion, 1-100. Low = coups, schisms.
    pub stability: u32,
    /// Regions controlled.
    pub territory: Vec<String>,
    /// Settlement IDs owned.
    pub settlements: Vec<u32>,
    /// The god this faction worships, if any.
    pub patron_god: Option<GodId>,
    /// Faction's devotion to their patron (0-100).
    pub devotion: u32,
}

impl FactionState {
    pub fn is_alive(&self, year: i32) -> bool {
        self.dissolved_year.is_none() || self.dissolved_year.unwrap() > year
    }

    /// Update gauges from population-derived stats. Factions not in the stats
    /// map (e.g. just spawned, no settlements yet) keep their current values.
    pub fn update_from_stats(&mut self, stats: &crate::worldgen::population::faction_stats::FactionStats) {
        if let Some(m) = stats.military(self.id) {
            self.military_strength = m;
        }
        if let Some(w) = stats.wealth(self.id) {
            self.wealth = w;
        }
        if let Some(s) = stats.stability(self.id) {
            self.stability = s;
        }
        if let Some(g) = stats.patron_god(self.id) {
            self.patron_god = Some(g);
        }
    }

    /// Initial gauge values based on faction type.
    /// Initial gauge values (military, wealth, stability) based on faction type.
    pub fn initialize_gauges(faction_type: FactionType) -> (u32, u32, u32) {
        match faction_type {
            FactionType::Kingdom => (50, 60, 65),
            FactionType::MercenaryCompany => (55, 40, 50),
            FactionType::ReligiousOrder => (25, 45, 70),
            FactionType::ThievesGuild => (20, 55, 40),
            FactionType::MerchantGuild => (15, 70, 55),
            FactionType::MageCircle => (25, 50, 60),
            FactionType::BanditClan => (45, 30, 35),
            FactionType::TribalWarband => (50, 25, 40),
        }
    }
}

/// An active war between two factions.
#[derive(Clone, Debug)]
pub struct War {
    pub aggressor: u32,
    pub defender: u32,
    pub start_year: i32,
}

/// An active alliance between two factions.
#[derive(Clone, Debug)]
pub struct Alliance {
    pub faction_a: u32,
    pub faction_b: u32,
    pub formed_year: i32,
}

/// A trade treaty between two factions.
#[derive(Clone, Debug)]
pub struct Treaty {
    pub faction_a: u32,
    pub faction_b: u32,
    pub formed_year: i32,
}

/// Per-settlement resource stockpile. Values can go negative (unmet demand).
#[derive(Clone, Debug, Default)]
pub struct ResourceStockpile {
    pub food: i32,
    pub timber: i32,
    pub ore: i32,
    pub leather: i32,
    pub stone: i32,
}

/// The settlement's mutable simulation state.
#[derive(Clone, Debug)]
pub struct SettlementState {
    pub id: u32,
    pub name: String,
    pub founded_year: i32,
    pub owner_faction: u32,
    pub destroyed_year: Option<i32>,
    pub region: String,
    pub population_class: PopulationClass,
    pub prosperity: u32,
    pub defenses: u32,
    /// Which god this settlement worships.
    pub patron_god: Option<GodId>,
    /// Devotion level (0-100).
    pub devotion: u32,
    /// World map position.
    pub world_pos: Option<WorldPos>,
    /// Terrain type at this settlement's location.
    pub zone_type: Option<ZoneType>,
    /// Resource stockpile.
    pub stockpile: ResourceStockpile,
    /// Owner faction is currently at war (set each year by history loop).
    pub at_war: bool,
    /// Hit by plague this year (one-time pulse, cleared after population processes it).
    pub plague_this_year: bool,
    /// Conquered this year — new faction took over (set by world_events, cleared after population processes).
    pub conquered_this_year: bool,
    /// Dominant race among living residents (computed yearly from population).
    pub dominant_race: Option<Race>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PopulationClass {
    Hamlet,
    Village,
    Town,
    City,
}

impl PopulationClass {
    pub fn grow(self) -> Self {
        match self {
            Self::Hamlet => Self::Village,
            Self::Village => Self::Town,
            Self::Town => Self::City,
            Self::City => Self::City,
        }
    }

    pub fn shrink(self) -> Self {
        match self {
            Self::Hamlet => Self::Hamlet,
            Self::Village => Self::Hamlet,
            Self::Town => Self::Village,
            Self::City => Self::Town,
        }
    }
}

/// The complete mutable world state during simulation.
#[derive(Clone, Debug, Default)]
pub struct WorldState {
    pub relations: RelationMatrix,
    pub active_wars: Vec<War>,
    pub active_alliances: Vec<Alliance>,
    pub active_treaties: Vec<Treaty>,
    /// Pairwise god relationships.
    pub divine_relations: DivineRelationMatrix,
    /// Active wars between gods.
    pub divine_wars: Vec<DivineWar>,
    /// Active pacts between gods.
    pub divine_pacts: Vec<DivinePact>,
    /// Which god owns each world map cell (parallel to WorldMap.cells).
    pub territory_map: Vec<Option<GodId>>,
}

impl WorldState {
    /// Check if two factions are currently at war.
    pub fn at_war(&self, a: u32, b: u32) -> bool {
        self.active_wars.iter().any(|w| {
            (w.aggressor == a && w.defender == b) || (w.aggressor == b && w.defender == a)
        })
    }

    /// Check if two factions have an active alliance.
    pub fn allied(&self, a: u32, b: u32) -> bool {
        self.active_alliances.iter().any(|al| {
            (al.faction_a == a && al.faction_b == b) || (al.faction_a == b && al.faction_b == a)
        })
    }

    /// Count how many wars a faction is currently in.
    pub fn war_count(&self, faction_id: u32) -> usize {
        self.active_wars.iter().filter(|w| {
            w.aggressor == faction_id || w.defender == faction_id
        }).count()
    }

    /// Check if two gods are currently at war.
    pub fn gods_at_war(&self, a: GodId, b: GodId) -> bool {
        self.divine_wars.iter().any(|w| {
            (w.aggressor == a && w.defender == b)
                || (w.aggressor == b && w.defender == a)
        })
    }

    /// Check if two gods have an active pact.
    pub fn gods_have_pact(&self, a: GodId, b: GodId) -> bool {
        self.divine_pacts.iter().any(|p| {
            (p.god_a == a && p.god_b == b) || (p.god_a == b && p.god_b == a)
        })
    }

    /// Count how many wars a god is currently in.
    pub fn god_war_count(&self, god_id: GodId) -> usize {
        self.divine_wars.iter().filter(|w| {
            w.aggressor == god_id || w.defender == god_id
        }).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_matrix_symmetric() {
        let mut rm = RelationMatrix::default();
        rm.set(1, 2, 50);
        assert_eq!(rm.get(1, 2), 50);
        assert_eq!(rm.get(2, 1), 50);
    }

    #[test]
    fn relation_matrix_clamps() {
        let mut rm = RelationMatrix::default();
        rm.set(1, 2, 200);
        assert_eq!(rm.get(1, 2), 100);
        rm.set(1, 2, -200);
        assert_eq!(rm.get(1, 2), -100);
    }

    #[test]
    fn relation_matrix_modify() {
        let mut rm = RelationMatrix::default();
        rm.set(1, 2, 10);
        rm.modify(1, 2, -25);
        assert_eq!(rm.get(1, 2), -15);
    }

    #[test]
    fn hostile_threshold() {
        let mut rm = RelationMatrix::default();
        rm.set(1, 2, -31);
        assert!(rm.is_hostile(1, 2));
        rm.set(1, 2, -30);
        assert!(!rm.is_hostile(1, 2));
    }

    #[test]
    fn self_sentiment_is_max() {
        let rm = RelationMatrix::default();
        assert_eq!(rm.get(5, 5), 100);
    }

    #[test]
    fn most_hostile_pair_finds_worst() {
        let mut rm = RelationMatrix::default();
        rm.set(1, 2, -50);
        rm.set(1, 3, -10);
        rm.set(2, 3, -80);
        let (a, b, s) = rm.most_hostile_pair(&[1, 2, 3]).unwrap();
        assert_eq!(s, -80);
        assert!((a == 2 && b == 3) || (a == 3 && b == 2));
    }

    #[test]
    fn drift_toward_neutral() {
        let mut rm = RelationMatrix::default();
        rm.set(1, 2, 50);
        rm.set(3, 4, -30);
        rm.drift_toward_neutral();
        assert_eq!(rm.get(1, 2), 49);
        assert_eq!(rm.get(3, 4), -29);
    }

    #[test]
    fn racial_initialization() {
        let dwarf = FactionState {
            id: 1, name: "D".into(), faction_type: FactionType::Kingdom,
            race: Race::Dwarf, founded_year: 0, home_region: "North".into(),
            dissolved_year: None, leader_name: "L".into(), leader_id: None,
            military_strength: 50, wealth: 50, stability: 50,
            territory: vec![], settlements: vec![],
            patron_god: None, devotion: 0,
        };
        let goblin = FactionState {
            id: 2, name: "G".into(), faction_type: FactionType::BanditClan,
            race: Race::Goblin, founded_year: 0, home_region: "North".into(),
            dissolved_year: None, leader_name: "L".into(), leader_id: None,
            military_strength: 50, wealth: 50, stability: 50,
            territory: vec![], settlements: vec![],
            patron_god: None, devotion: 0,
        };
        let mut rm = RelationMatrix::default();
        rm.initialize_pair(&dwarf, &goblin);
        // Dwarf vs Goblin: -15 racial + 5 same region + (-20 Kingdom vs BanditClan same region) = -30
        assert!(rm.get(1, 2) < 0);
    }

    #[test]
    fn faction_initial_gauges() {
        let (m, w, s) = FactionState::initialize_gauges(FactionType::Kingdom);
        assert_eq!(m, 50);
        assert_eq!(w, 60);
        assert_eq!(s, 65);
    }

    #[test]
    fn population_class_grow_shrink() {
        assert_eq!(PopulationClass::Hamlet.grow(), PopulationClass::Village);
        assert_eq!(PopulationClass::City.grow(), PopulationClass::City);
        assert_eq!(PopulationClass::Village.shrink(), PopulationClass::Hamlet);
        assert_eq!(PopulationClass::Hamlet.shrink(), PopulationClass::Hamlet);
    }

    #[test]
    fn world_state_war_tracking() {
        let mut ws = WorldState::default();
        assert!(!ws.at_war(1, 2));
        ws.active_wars.push(War { aggressor: 1, defender: 2, start_year: 10 });
        assert!(ws.at_war(1, 2));
        assert!(ws.at_war(2, 1));
        assert!(!ws.at_war(1, 3));
        assert_eq!(ws.war_count(1), 1);
        assert_eq!(ws.war_count(3), 0);
    }
}
