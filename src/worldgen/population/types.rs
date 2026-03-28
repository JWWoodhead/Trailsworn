//! Core data types for the population simulation.

use crate::worldgen::divine::gods::GodId;

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
    Famine,
    War,
    Plague,
    DivineFlaw,
    Violence,
    Monster,
}

#[derive(Clone, Debug)]
pub struct LifeEvent {
    pub year: i32,
    pub kind: LifeEventKind,
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
}

/// A single person in the world.
#[derive(Clone, Debug)]
pub struct Person {
    pub id: u32,
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub settlement_id: u32,
    pub sex: Sex,
    pub mother: Option<u32>,
    pub father: Option<u32>,
    pub spouse: Option<u32>,
    pub occupation: Occupation,
    pub faith: Option<GodId>,
    pub devotion: u8,
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
}
