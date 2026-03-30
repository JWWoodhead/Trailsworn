//! Core data types for the population simulation.

use crate::worldgen::divine::gods::GodId;
use crate::worldgen::history::characters::CharacterTrait;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sex {
    Male,
    Female,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occupation {
    Farmer,
    Woodcutter,
    Miner,
    Hunter,
    Quarrier,
    Soldier,
    Smith,
    Merchant,
    Priest,
    Scholar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCause {
    OldAge,
    /// Infant or child death (under 5).
    Illness,
    /// Working-age accident — mining collapse, logging, drowning, fall.
    Accident,
    /// Mother died during or shortly after childbirth.
    Childbirth,
    Famine,
    War,
    Plague,
    DivineFlaw,
    Violence,
    Monster,
}

/// What divine action caused an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivineAction {
    FlawTriggered,
    TempleBuilt,
    ChampionChosen,
    ArtifactForged,
    SiteCreated,
    TerritoryLost,
    Faded,
    WorshipClaimed,
}

/// What a prophet preaches — derived from their god's drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Doctrine {
    SpreadTheWord,      // Worship drive
    ConquerForTheGod,   // Dominion drive
    SeekTruth,          // Knowledge drive
    PunishUnbelievers,  // Vindication drive
    ProtectTheFaithful, // Love drive
    PurifyTheLand,      // Justice drive
    BreakTheChains,     // Freedom drive
    BuildForEternity,   // Legacy drive
    ProveSupremacy,     // Supremacy drive
    AchievePerfection,  // Perfection drive
    GodsAreFalse,       // Heretic — reject all gods
    GodHasAbandoned,    // Heretic — this god failed us
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProphetKind {
    Zealot,  // more devout than community, spreads faith
    Heretic, // less devout, undermines faith
}

/// A person who has become a prophet — actively influences their settlement's faith.
#[derive(Clone, Debug)]
pub struct ProphetRole {
    pub god_id: GodId,
    pub kind: ProphetKind,
    pub doctrine: Doctrine,
    pub became_prophet_year: i32,
}

/// Why an event happened — one level of causation.
/// The full causal chain is reconstructed by reading a person's life_events in order.
#[derive(Clone, Debug)]
pub enum EventCause {
    /// A god's action or nature caused this.
    Divine { god_id: GodId, action: DivineAction },
    /// A specific person's action caused this.
    PersonAction { person_id: u32, role: &'static str },
    /// Settlement conditions caused this.
    Conditions { settlement_id: u32, detail: &'static str },
    /// A faction's decision caused this.
    Faction { faction_id: u32, detail: &'static str },
}

#[derive(Clone, Debug)]
pub struct LifeEvent {
    pub year: i32,
    pub kind: LifeEventKind,
    pub cause: Option<EventCause>,
}

#[derive(Clone, Debug)]
pub enum LifeEventKind {
    ChildBorn { child_id: u32 },
    MarriedTo { spouse_id: u32 },
    LostParent { parent_id: u32, cause: DeathCause },
    LostSpouse { spouse_id: u32, cause: DeathCause },
    LostChild { child_id: u32, cause: DeathCause },
    LostSibling { sibling_id: u32, cause: DeathCause },
    DraftedToWar { enemy_faction_id: u32 },
    SurvivedWar { enemy_faction_id: u32 },
    SettlementConquered { new_faction_id: u32 },
    SurvivedPlague,
    // Faith events
    FaithStrengthened { god_id: GodId },
    FaithShaken { god_id: GodId },
    ConvertedFaith { old_god: Option<GodId>, new_god: GodId },
    AbandonedFaith { god_id: GodId },
    Migrated { from_settlement: u32, to_settlement: u32 },
    BecameProphet { god_id: GodId, kind: ProphetKind },
    WitnessedMartyrdom { prophet_id: u32, god_id: GodId },
    AllegianceChanged { old_faction: u32, new_faction: u32 },
}

/// A single person in the world.
#[derive(Clone, Debug)]
pub struct Person {
    pub id: u32,
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub death_cause: Option<DeathCause>,
    pub settlement_id: u32,
    /// Which faction this person is personally loyal to (0 = unaligned).
    pub faction_allegiance: u32,
    pub sex: Sex,
    pub race: crate::worldgen::names::Race,
    pub secondary_race: Option<crate::worldgen::names::Race>,
    pub mother: Option<u32>,
    pub father: Option<u32>,
    pub spouse: Option<u32>,
    pub occupation: Occupation,
    /// Personality traits — seeded at birth (2), earned from life events (up to 5).
    pub traits: Vec<CharacterTrait>,
    /// Personal happiness (0-100). Drives migration decisions.
    pub happiness: u8,
    /// Prophet status — if this person is actively preaching.
    pub prophet_of: Option<ProphetRole>,
    /// How many consecutive years this person has been a faith outlier in their settlement.
    pub years_as_outlier: u8,
    /// Relationship with each god the person has been exposed to. (god_id, devotion 0-100).
    /// Most people have 1-2 entries. Primary faith = highest devotion.
    pub faith: Vec<(GodId, u8)>,
    pub life_events: Vec<LifeEvent>,
    pub notable: bool,
}

impl Person {
    pub fn is_alive(&self, year: i32) -> bool {
        self.death_year.is_none() || self.death_year.unwrap() > year
    }

    pub fn age(&self, year: i32) -> i32 {
        year - self.birth_year
    }

    /// The god this person is most devoted to, if any (devotion > 0).
    pub fn primary_god(&self) -> Option<GodId> {
        self.faith.iter()
            .filter(|(_, d)| *d > 0)
            .max_by_key(|(_, d)| *d)
            .map(|(g, _)| *g)
    }

    /// Get devotion to a specific god (0 if no relationship).
    pub fn devotion_to(&self, god_id: GodId) -> u8 {
        self.faith.iter()
            .find(|(g, _)| *g == god_id)
            .map(|(_, d)| *d)
            .unwrap_or(0)
    }

    /// Set devotion to a specific god, adding the entry if needed.
    pub fn set_devotion(&mut self, god_id: GodId, devotion: u8) {
        if let Some(entry) = self.faith.iter_mut().find(|(g, _)| *g == god_id) {
            entry.1 = devotion;
        } else if devotion > 0 {
            self.faith.push((god_id, devotion));
        }
    }
}
