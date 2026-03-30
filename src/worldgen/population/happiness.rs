//! Happiness evaluation — yearly assessment of personal satisfaction.
//! Driven by settlement conditions, personal traits, faith alignment, and family.

use crate::worldgen::history::characters::CharacterTrait::*;
use crate::worldgen::history::state::{PopulationClass, SettlementState, WorldState};

use super::index::SettlementIndex;
use super::types::*;

/// Evaluate happiness for all living people.
pub fn evaluate_happiness(
    people: &mut [Person],
    index: &SettlementIndex,
    settlements: &[SettlementState],
    world_state: &WorldState,
    year: i32,
) {
    for settlement in settlements {
        if settlement.destroyed_year.is_some() { continue; }
        for &idx in index.residents(settlement.id) {
            if !people[idx].is_alive(year) { continue; }
            let at_war = world_state.war_count(people[idx].faction_allegiance) > 0;

            // Read person state immutably first
            let spouse_id = people[idx].spouse;
            let primary_god = people[idx].primary_god();
            let traits = people[idx].traits.clone();
            let happiness = people[idx].happiness;
            let life_events_this_year: Vec<_> = people[idx].life_events.iter()
                .filter(|e| e.year == year)
                .map(|e| e.kind.clone())
                .collect();

            let has_spouse = spouse_id
                .and_then(|sid| people.get((sid.checked_sub(1)?) as usize))
                .is_some_and(|s| s.is_alive(year));

            let mut delta: i16 = 0;

            // --- Positive ---

            if settlement.prosperity > 70 {
                delta += 3;
            }
            if settlement.stockpile.food > 0 {
                delta += 2;
            }
            if has_spouse {
                delta += 2;
            }

            if let Some(patron) = settlement.patron_god {
                if primary_god == Some(patron) {
                    delta += 2;
                }
            }

            // Trait fit — ambitious in a city
            if traits.contains(&Ambitious) {
                match settlement.population_class {
                    PopulationClass::City => delta += 2,
                    _ => {}
                }
            }

            // --- Negative ---

            if settlement.prosperity < 30 { delta -= 5; }
            if settlement.stockpile.food < 0 { delta -= 3; }
            if at_war { delta -= 3; }

            // Recent child/spouse death (this year)
            if life_events_this_year.iter().any(|k| matches!(k, LifeEventKind::LostChild { .. })) {
                delta -= 10;
            }
            if life_events_this_year.iter().any(|k| matches!(k, LifeEventKind::LostSpouse { .. })) {
                delta -= 8;
            }

            // Faith misaligned
            if let Some(patron) = settlement.patron_god {
                if let Some(primary) = primary_god {
                    if primary != patron {
                        let dev = people[idx].devotion_to(primary);
                        if dev > 50 { delta -= 3; }
                    }
                }
            }

            if traits.contains(&Skeptical) && settlement.devotion > 70 { delta -= 2; }
            if traits.contains(&Devout) && settlement.patron_god.is_none() { delta -= 2; }

            // Trait misfit
            if traits.contains(&Ambitious) {
                if matches!(settlement.population_class, PopulationClass::Hamlet) { delta -= 2; }
            }
            if traits.contains(&Peaceful) && at_war { delta -= 3; }

            // Race mismatch with settlement's dominant race
            let person_race = people[idx].race;
            if let Some(dominant) = settlement.dominant_race {
                if person_race != dominant {
                    delta -= 2; // minority race in this settlement
                }
            }
            // Purist in a mixed-race settlement
            if traits.contains(&Purist) {
                if let Some(dominant) = settlement.dominant_race {
                    if person_race != dominant {
                        delta -= 2; // purist AND minority — very unhappy
                    }
                }
            }

            // --- Apply ---

            // Drift toward 50
            if happiness > 50 { delta -= 1; }
            else if (happiness as i16) < 50 { delta += 1; }

            people[idx].happiness = (happiness as i16 + delta).clamp(0, 100) as u8;
        }
    }
}
