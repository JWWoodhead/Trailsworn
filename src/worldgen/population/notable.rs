//! Notable promotion: when a person accumulates enough life events,
//! they become notable and can be promoted to a full Character.

use rand::Rng;

use crate::worldgen::history::characters::{
    Character, CharacterRole, generate_character, generate_epithet,
};
use crate::worldgen::history::state::{FactionState, SettlementState};
use crate::worldgen::names::Race;

use super::types::{LifeEventKind, Occupation, Person};

pub const NOTABLE_THRESHOLD: usize = 4;

/// Maximum notables promoted per settlement per generation (~25 years).
pub const MAX_NOTABLES_PER_SETTLEMENT_PER_GEN: usize = 3;

/// Count significant life events — exceptional experiences, not routine life.
/// Excludes: ChildBorn, MarriedTo (normal), LostSibling (too common in big families).
fn significant_event_count(person: &Person) -> usize {
    person.life_events.iter().filter(|e| matches!(
        e.kind,
        // Loss (but not siblings — too common)
        LifeEventKind::LostParent { .. }
        | LifeEventKind::LostSpouse { .. }
        | LifeEventKind::LostChild { .. }
        // War experience
        | LifeEventKind::DraftedToWar { .. }
        | LifeEventKind::SurvivedWar { .. }
        // Survival
        | LifeEventKind::SurvivedPlague
        | LifeEventKind::SettlementConquered { .. }
    )).count()
}

/// Check if a person should be promoted to notable.
/// Returns true if newly promoted this call.
pub fn check_notable(person: &mut Person) -> bool {
    if person.notable { return false; }
    if significant_event_count(person) >= NOTABLE_THRESHOLD {
        person.notable = true;
        return true;
    }
    false
}

/// Derive a CharacterRole from the person's occupation.
fn derive_role(person: &Person) -> CharacterRole {
    match person.occupation {
        Occupation::Soldier => CharacterRole::Hero,
        Occupation::Priest => CharacterRole::Advisor,
        Occupation::Scholar => CharacterRole::Scholar,
        Occupation::Merchant => CharacterRole::Advisor,
        _ => CharacterRole::Hero,
    }
}

/// Promote a notable person to a full Character for the history system.
pub fn promote_to_character(
    person: &Person,
    next_id: &mut u32,
    settlements: &[SettlementState],
    factions: &[FactionState],
    rng: &mut impl Rng,
) -> Character {
    let faction_id = settlements.iter()
        .find(|s| s.id == person.settlement_id)
        .map(|s| s.owner_faction);

    let race = faction_id
        .and_then(|fid| factions.iter().find(|f| f.id == fid))
        .map(|f| f.race)
        .unwrap_or(Race::Human);

    let role = derive_role(person);

    let id = *next_id;
    *next_id += 1;

    let mut character = generate_character(id, race, role, faction_id, person.birth_year, rng);

    // Boost renown based on how eventful their life has been
    character.renown += person.life_events.len() as i32 * 3;

    // Give an epithet if they have enough events
    if person.life_events.len() >= 4 {
        character.epithet = Some(generate_epithet(&character, rng));
    }

    character
}
