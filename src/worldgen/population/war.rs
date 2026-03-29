//! War effects on population: combat scoring, drafting, casualties, and war-end events.

use rand::Rng;

use crate::worldgen::history::state::SettlementState;

use super::index::SettlementIndex;
use super::types::*;

/// Per-soldier combat effectiveness score.
pub fn combat_score(person: &Person, settlement: &SettlementState, year: i32) -> f32 {
    if person.occupation != Occupation::Soldier { return 0.0; }

    let age = person.age(year);
    combat_score_at_age(person, age, settlement)
}

/// Combat score for a given age (avoids needing year everywhere).
fn combat_score_at_age(person: &Person, age: i32, settlement: &SettlementState) -> f32 {
    let mut score = 1.0f32;

    // Age modifier: peak 20-40
    if age >= 20 && age <= 40 {
        score += 0.5;
    } else if age < 18 || age > 50 {
        score -= 0.5;
    }

    // Veteran bonus: each survived war adds experience
    let wars_survived = person.life_events.iter()
        .filter(|e| matches!(e.kind, LifeEventKind::SurvivedWar { .. }))
        .count();
    score += wars_survived as f32 * 0.3;

    // Equipment bonus: settlement has smiths and ore surplus
    if settlement.stockpile.ore > 0 {
        score += 0.2;
    }

    score.max(0.1) // minimum score
}

/// Compute total military power for a faction across all its settlements.
pub fn faction_military_power(
    people: &[Person],
    index: &SettlementIndex,
    settlements: &[SettlementState],
    faction_id: u32,
    year: i32,
) -> f32 {
    let mut total = 0.0f32;
    for settlement in settlements.iter().filter(|s| s.owner_faction == faction_id && s.destroyed_year.is_none()) {
        for &idx in index.residents(settlement.id) {
            let person = &people[idx];
            if !person.is_alive(year) { continue; }
            if person.occupation != Occupation::Soldier { continue; }
            let age = person.age(year);
            total += combat_score_at_age(person, age, settlement);
        }
    }
    total
}

/// Apply war effects for one settlement: drafting and casualties.
/// Returns IDs of people who died from war this year.
pub fn apply_war_effects(
    people: &mut [Person],
    index: &SettlementIndex,
    settlement: &SettlementState,
    enemy_faction_id: u32,
    year: i32,
    _rng: &mut impl Rng,
) -> Vec<u32> {
    let residents = index.residents(settlement.id);
    let mut dead_ids = Vec::new();

    // Count fighting-age adults and current soldiers
    let mut fighting_age = 0u32;
    let mut soldier_count = 0u32;
    for &idx in residents {
        let p = &people[idx];
        if !p.is_alive(year) { continue; }
        let age = p.age(year);
        if age >= 16 && age <= 50 {
            fighting_age += 1;
            if p.occupation == Occupation::Soldier {
                soldier_count += 1;
            }
        }
    }

    // Draft if fewer than 20% are soldiers
    if fighting_age > 0 && (soldier_count as f32 / fighting_age as f32) < 0.20 {
        let draft_target = (fighting_age as f32 * 0.20) as u32 - soldier_count;
        let draft_priority = [
            Occupation::Quarrier, Occupation::Woodcutter, Occupation::Miner,
            Occupation::Merchant, Occupation::Scholar,
        ];
        let mut drafted = 0u32;
        for &occ in &draft_priority {
            if drafted >= draft_target { break; }
            for &idx in residents {
                if drafted >= draft_target { break; }
                let p = &mut people[idx];
                if !p.is_alive(year) { continue; }
                if p.occupation != occ { continue; }
                let age = p.age(year);
                if age < 16 || age > 50 { continue; }
                p.occupation = Occupation::Soldier;
                p.life_events.push(LifeEvent {
                    year,
                    kind: LifeEventKind::DraftedToWar { enemy_faction_id },
                    cause: None,
                });
                drafted += 1;
            }
        }
    }

    // War casualties: ~3% of soldiers die, weakest first
    let mut soldiers: Vec<(usize, f32)> = residents.iter()
        .filter(|&&idx| {
            let p = &people[idx];
            p.is_alive(year) && p.occupation == Occupation::Soldier
        })
        .map(|&idx| {
            let score = combat_score_at_age(&people[idx], people[idx].age(year), settlement);
            (idx, score)
        })
        .collect();

    // Sort weakest first (lowest combat score dies first)
    soldiers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let casualties = ((soldiers.len() as f32 * 0.03).ceil() as usize).max(if soldiers.is_empty() { 0 } else { 1 });
    for &(idx, _) in soldiers.iter().take(casualties) {
        people[idx].death_year = Some(year);
        people[idx].death_cause = Some(DeathCause::War);
        dead_ids.push(people[idx].id);
    }

    dead_ids
}

/// When war ends, give SurvivedWar events to soldiers who were drafted.
pub fn apply_war_ended(
    people: &mut [Person],
    index: &SettlementIndex,
    settlement: &SettlementState,
    enemy_faction_id: u32,
    year: i32,
) {
    for &idx in index.residents(settlement.id) {
        let p = &mut people[idx];
        if !p.is_alive(year) { continue; }
        // Only soldiers who were drafted get the SurvivedWar event
        let was_drafted = p.life_events.iter().any(|e| matches!(e.kind, LifeEventKind::DraftedToWar { .. }));
        if p.occupation == Occupation::Soldier && was_drafted {
            p.life_events.push(LifeEvent {
                year,
                kind: LifeEventKind::SurvivedWar { enemy_faction_id },
                cause: None,
            });
        }
    }
}
