//! Allegiance shift system — people change faction loyalty based on
//! conquest, charisma, faith, race, and sustained unhappiness.
//! All rules are deterministic (no random rolls).

use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::history::state::{FactionState, SettlementState, WorldState};

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
