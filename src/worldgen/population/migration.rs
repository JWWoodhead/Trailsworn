//! Migration — unhappy families leave for better settlements.
//! Migration is a family decision: spouse + minor children move together.

use crate::worldgen::history::characters::CharacterTrait::*;
use crate::worldgen::history::state::{FactionState, PopulationClass, SettlementState, WorldState};

use super::index::SettlementIndex;
use super::types::*;

/// Evaluate migration for all unhappy people. Moves families to better settlements.
pub fn evaluate_migration(
    people: &mut Vec<Person>,
    index: &SettlementIndex,
    settlements: &[SettlementState],
    factions: &[FactionState],
    world_state: &WorldState,
    year: i32,
) {
    // Collect migration decisions first to avoid borrow issues
    let mut migrations: Vec<(Vec<u32>, u32, u32, &'static str)> = Vec::new(); // (person_ids, from, to, reason)

    // Track who has already been scheduled to migrate (as part of someone else's family)
    let mut migrating: std::collections::HashSet<u32> = std::collections::HashSet::new();

    for si in 0..settlements.len() {
        let settlement = &settlements[si];
        if settlement.destroyed_year.is_some() { continue; }

        for &idx in index.residents(settlement.id) {
            let person = &people[idx];
            if !person.is_alive(year) { continue; }
            if migrating.contains(&person.id) { continue; }

            // Check migration threshold
            let dominated = should_migrate(person, people, year);
            if !dominated { continue; }

            // Find best destination
            let dest = find_best_destination(
                person, settlement, settlements, factions, world_state,
            );
            let (dest_id, reason) = match dest {
                Some(d) => d,
                None => continue, // nowhere better to go
            };

            // Collect family members to migrate
            let mut family_ids = vec![person.id];

            // Spouse (if alive and in same settlement)
            if let Some(sid) = person.spouse {
                if let Some(spouse) = people.get((sid - 1) as usize) {
                    if spouse.is_alive(year) && spouse.settlement_id == settlement.id {
                        family_ids.push(sid);
                    }
                }
            }

            // Minor children (<16) in same settlement
            for p in people.iter() {
                if !p.is_alive(year) { continue; }
                if p.settlement_id != settlement.id { continue; }
                if p.age(year) >= 16 { continue; }
                if p.mother == Some(person.id) || p.father == Some(person.id) {
                    family_ids.push(p.id);
                }
            }

            for &id in &family_ids {
                migrating.insert(id);
            }
            migrations.push((family_ids, settlement.id, dest_id, reason));
        }
    }

    // Apply migrations
    for (family_ids, from_id, to_id, reason) in migrations {
        let _to_zone = settlements.iter().find(|s| s.id == to_id).and_then(|s| s.zone_type);

        for &pid in &family_ids {
            let pidx = (pid - 1) as usize;
            if pidx >= people.len() { continue; }
            let person = &mut people[pidx];
            person.settlement_id = to_id;
            person.happiness = 50; // fresh start

            // Determine cause
            let cause = if reason == "famine" || reason == "food deficit" {
                Some(EventCause::Conditions { settlement_id: from_id, detail: reason })
            } else if reason == "war" {
                Some(EventCause::Conditions { settlement_id: from_id, detail: reason })
            } else if reason == "faith misalignment" {
                person.primary_god().map(|god_id| EventCause::Divine {
                    god_id,
                    action: DivineAction::WorshipClaimed,
                })
            } else {
                Some(EventCause::Conditions { settlement_id: from_id, detail: reason })
            };

            person.life_events.push(LifeEvent {
                year,
                kind: LifeEventKind::Migrated { from_settlement: from_id, to_settlement: to_id },
                cause,
            });

            // Reassign occupation based on new settlement's terrain (adults only)
            if person.age(year) >= 16 {
                // Only reassign resource workers — keep soldiers, priests, scholars
                match person.occupation {
                    Occupation::Farmer | Occupation::Woodcutter | Occupation::Miner
                    | Occupation::Hunter | Occupation::Quarrier => {
                        // Will be naturally reassigned by the resource system's rebalancing
                    }
                    _ => {} // keep non-resource occupations
                }
            }
        }
    }
}

/// Should this person migrate?
fn should_migrate(person: &Person, people: &[Person], year: i32) -> bool {
    if person.happiness >= 20 { return false; }

    // Check if they have a living spouse in the same settlement
    if let Some(sid) = person.spouse {
        if let Some(spouse) = people.get((sid - 1) as usize) {
            if spouse.is_alive(year) && spouse.settlement_id == person.settlement_id {
                // Both spouses need to be unhappy for family migration
                return spouse.happiness < 25;
            }
        }
    }

    // Single / widowed — can migrate on their own
    true
}

/// Find the best destination settlement. Returns (settlement_id, reason) or None.
fn find_best_destination(
    person: &Person,
    current: &SettlementState,
    settlements: &[SettlementState],
    _factions: &[FactionState],
    world_state: &WorldState,
) -> Option<(u32, &'static str)> {
    let current_pos = current.world_pos?;
    let person_faction = person.faction_allegiance;

    let mut best_score = 0i32;
    let mut best_id = None;
    let mut best_reason = "general unhappiness";

    for candidate in settlements {
        if candidate.id == current.id { continue; }
        if candidate.destroyed_year.is_some() { continue; }

        // Must be same faction or allied (not at war)
        let same_faction = candidate.controlling_faction == person_faction;
        let allied = world_state.allied(candidate.controlling_faction, person_faction);
        if !same_faction && !allied { continue; }
        if world_state.at_war(candidate.controlling_faction, person_faction) { continue; }

        // Must be within trade distance
        let cand_pos = match candidate.world_pos {
            Some(p) => p,
            None => continue,
        };
        let dist = current_pos.manhattan_distance(cand_pos);
        if dist > 100 { continue; }

        // Score the candidate
        let mut score = candidate.prosperity as i32;

        // Food surplus
        if candidate.stockpile.food > 0 { score += 20; }

        // Faith match
        if let Some(primary) = person.primary_god() {
            if candidate.patron_god == Some(primary) { score += 15; }
        }

        // Trait fit
        if person.traits.contains(&Ambitious) {
            match candidate.population_class {
                PopulationClass::City => score += 10,
                _ => {}
            }
        }
        if person.traits.contains(&Scholarly) {
            match candidate.population_class {
                PopulationClass::Town | PopulationClass::City => score += 5,
                _ => {}
            }
        }

        // Closer is better
        score += (100 - dist) / 5;

        // Not at war is better
        if world_state.war_count(candidate.controlling_faction) == 0 { score += 10; }

        if score > best_score {
            best_score = score;
            best_id = Some(candidate.id);

            // Determine the primary reason for leaving
            if current.stockpile.food < 0 {
                best_reason = "food deficit";
            } else if world_state.war_count(current.controlling_faction) > 0 {
                best_reason = "war";
            } else if current.prosperity < 30 {
                best_reason = "poor conditions";
            } else if person.primary_god().is_some() && current.patron_god != person.primary_god() {
                best_reason = "faith misalignment";
            } else {
                best_reason = "seeking better life";
            }
        }
    }

    // Only migrate if destination is meaningfully better than current
    let current_score = current.prosperity as i32;
    if best_score > current_score + 20 {
        best_id.map(|id| (id, best_reason))
    } else {
        None
    }
}
