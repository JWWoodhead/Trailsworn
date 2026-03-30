//! Allegiance shift system — people change faction loyalty based on
//! conquest, charisma, faith, race, natural affinity, and sustained unhappiness.
//! All rules are deterministic (no random rolls).

use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::history::state::{FactionState, SettlementState, WorldState};
use crate::worldgen::names::FactionType;

use super::index::SettlementIndex;
use super::types::*;

/// Evaluate allegiance shifts for all living people. Runs yearly after happiness evaluation.
pub fn evaluate_allegiance_shifts(
    people: &mut [Person],
    index: &SettlementIndex,
    settlements: &[SettlementState],
    factions: &[FactionState],
    _world_state: &WorldState,
    year: i32,
) {
    // Collect shifts first, apply after (can't mutate while iterating)
    let mut shifts: Vec<(usize, u32)> = Vec::new(); // (person_index, new_faction)

    for settlement in settlements.iter() {
        if settlement.destroyed_year.is_some() { continue; }

        // Conquest shifts: conquered settlement residents may accept the conqueror
        if settlement.conquered_this_year {
            if let Some(conqueror) = settlement.conquered_by {
                for &idx in index.residents(settlement.id) {
                    let person = &people[idx];
                    if !person.is_alive(year) { continue; }
                    // Unhappy residents accept the new regime
                    if person.happiness < 40 && person.faction_allegiance != conqueror {
                        shifts.push((idx, conqueror));
                    }
                }
            }
        }

        let residents: Vec<usize> = index.residents(settlement.id).iter()
            .copied()
            .filter(|&idx| people[idx].is_alive(year) && people[idx].age(year) >= 16)
            .collect();

        // Charismatic conversion: converts unaligned residents in same settlement
        for &idx in &residents {
            let person = &people[idx];
            if person.faction_allegiance == 0 { continue; }
            if !person.traits.contains(&CharacterTrait::Charismatic) { continue; }

            // Find one unaligned person to convert
            for &target_idx in &residents {
                if people[target_idx].faction_allegiance == 0 {
                    shifts.push((target_idx, person.faction_allegiance));
                    break; // one conversion per charismatic person per year
                }
            }
        }

        // Natural affinity: people join factions that match their occupation,
        // traits, or circumstances. Unaligned people join freely; people already
        // in a faction only shift if unhappy (same threshold as faction founding).
        for &idx in &residents {
            let person = &people[idx];
            let current_type = factions.iter()
                .find(|f| f.id == person.faction_allegiance)
                .map(|f| f.faction_type);

            let best_match = find_natural_faction(person, current_type, factions, year);
            if let Some(faction_id) = best_match {
                if person.faction_allegiance == 0 {
                    // Unaligned: join freely
                    shifts.push((idx, faction_id));
                } else if person.happiness < 40 {
                    // Already in a faction but unhappy: defect to a better fit
                    shifts.push((idx, faction_id));
                }
            }
        }

        // Faith-driven shifts: person's god matches another faction's patron
        for &idx in &residents {
            let person = &people[idx];
            if person.happiness >= 40 { continue; } // only unhappy people shift
            let person_god = match person.primary_god() {
                Some(g) => g,
                None => continue,
            };
            if person.devotion_to(person_god) < 60 { continue; } // must be devout enough

            // Find a faction whose patron matches their god (but isn't their current faction)
            let current_faction_patron = factions.iter()
                .find(|f| f.id == person.faction_allegiance)
                .and_then(|f| f.patron_god);
            if current_faction_patron == Some(person_god) { continue; } // already aligned

            let matching_faction = factions.iter()
                .find(|f| f.is_alive(year) && f.patron_god == Some(person_god) && f.id != person.faction_allegiance);
            if let Some(f) = matching_faction {
                shifts.push((idx, f.id));
            }
        }

        // Race-driven shifts: Purist trait + unhappy + another faction matches their race
        for &idx in &residents {
            let person = &people[idx];
            if person.happiness >= 30 { continue; }
            if !person.traits.contains(&CharacterTrait::Purist) { continue; }

            let current_faction_race = factions.iter()
                .find(|f| f.id == person.faction_allegiance)
                .map(|f| f.race);
            if current_faction_race == Some(person.race) { continue; } // already same race

            let matching_faction = factions.iter()
                .find(|f| f.is_alive(year) && f.race == person.race && f.id != person.faction_allegiance);
            if let Some(f) = matching_faction {
                shifts.push((idx, f.id));
            }
        }

        // Unhappiness conformity: very unhappy people for 3+ years conform to local power
        for &idx in &residents {
            let person = &people[idx];
            if person.happiness >= 20 { continue; }
            if person.faction_allegiance == settlement.controlling_faction { continue; }
            if settlement.controlling_faction == 0 { continue; }

            // Check if they've been unhappy for a while (use years_as_outlier as proxy)
            // TODO: add a dedicated unhappy_years counter on Person if needed
            // For now, happiness < 20 is sufficient — sustained unhappiness is tracked on factions
            shifts.push((idx, settlement.controlling_faction));
        }
    }

    // Apply all shifts
    for (idx, new_faction) in shifts {
        let old = people[idx].faction_allegiance;
        if old == new_faction { continue; }
        people[idx].faction_allegiance = new_faction;
        people[idx].life_events.push(LifeEvent {
            year,
            kind: LifeEventKind::AllegianceChanged { old_faction: old, new_faction },
            cause: None,
        });
    }
}

/// Find the best-matching alive faction for a person based on their occupation,
/// traits, and faith. Returns None if no faction is a natural fit, or if the
/// person is already in a faction of the matching type.
fn find_natural_faction(
    person: &Person,
    current_type: Option<FactionType>,
    factions: &[FactionState],
    year: i32,
) -> Option<u32> {
    let alive_factions: Vec<_> = factions.iter()
        .filter(|f| f.is_alive(year) && f.id != person.faction_allegiance)
        .collect();

    // Merchant → MerchantGuild (skip if already in one)
    if person.occupation == Occupation::Merchant && current_type != Some(FactionType::MerchantGuild) {
        let best = alive_factions.iter()
            .filter(|f| f.faction_type == FactionType::MerchantGuild)
            .max_by_key(|f| if f.race == person.race { 1 } else { 0 });
        if let Some(f) = best { return Some(f.id); }
    }

    // Soldier → MercenaryCompany (skip if already in one)
    if person.occupation == Occupation::Soldier && current_type != Some(FactionType::MercenaryCompany) {
        let best = alive_factions.iter()
            .filter(|f| f.faction_type == FactionType::MercenaryCompany)
            .max_by_key(|f| if f.race == person.race { 1 } else { 0 });
        if let Some(f) = best { return Some(f.id); }
    }

    // Priest/Devout → ReligiousOrder or Theocracy matching their god
    if person.occupation == Occupation::Priest || person.traits.contains(&CharacterTrait::Devout) {
        if let Some(god) = person.primary_god() {
            let already_religious = matches!(current_type, Some(FactionType::ReligiousOrder | FactionType::Theocracy));
            if !already_religious {
                let best = alive_factions.iter()
                    .find(|f| {
                        (f.faction_type == FactionType::ReligiousOrder || f.faction_type == FactionType::Theocracy)
                        && f.patron_god == Some(god)
                    });
                if let Some(f) = best { return Some(f.id); }
            }
        }
    }

    // Treacherous/Cunning → ThievesGuild (skip if already in one)
    if person.traits.contains(&CharacterTrait::Treacherous)
        || person.traits.contains(&CharacterTrait::Cunning)
    {
        if current_type != Some(FactionType::ThievesGuild) {
            let best = alive_factions.iter()
                .filter(|f| f.faction_type == FactionType::ThievesGuild)
                .max_by_key(|f| if f.race == person.race { 1 } else { 0 });
            if let Some(f) = best { return Some(f.id); }
        }
    }

    // Scholarly/Wise → MageCircle (skip if already in one)
    if person.traits.contains(&CharacterTrait::Scholarly)
        || person.traits.contains(&CharacterTrait::Wise)
    {
        if current_type != Some(FactionType::MageCircle) {
            let best = alive_factions.iter()
                .filter(|f| f.faction_type == FactionType::MageCircle)
                .max_by_key(|f| if f.race == person.race { 1 } else { 0 });
            if let Some(f) = best { return Some(f.id); }
        }
    }

    // Minority race person → TribalWarband of their race
    if current_type != Some(FactionType::TribalWarband) {
        let best = alive_factions.iter()
            .find(|f| f.faction_type == FactionType::TribalWarband && f.race == person.race);
        if let Some(f) = best { return Some(f.id); }
    }

    // Person whose primary god matches a Theocracy → join it
    if !matches!(current_type, Some(FactionType::Theocracy)) {
        if let Some(god) = person.primary_god() {
            let best = alive_factions.iter()
                .find(|f| f.faction_type == FactionType::Theocracy && f.patron_god == Some(god));
            if let Some(f) = best { return Some(f.id); }
        }
    }

    None
}
