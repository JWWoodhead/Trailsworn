//! Plague effects on population: one-time kill pulse.
//! Plague is a sharp event — kills 10-15% of a settlement, then it's over.

use rand::{Rng, RngExt};

use crate::worldgen::history::state::SettlementState;

use super::index::SettlementIndex;
use super::types::*;

/// Apply plague to a settlement. Returns IDs of people who died.
pub fn apply_plague(
    people: &mut [Person],
    index: &SettlementIndex,
    settlement: &mut SettlementState,
    year: i32,
    rng: &mut impl Rng,
) -> Vec<u32> {
    let residents = index.residents(settlement.id);
    let living: Vec<(usize, i32)> = residents.iter()
        .filter(|&&idx| people[idx].is_alive(year))
        .map(|&idx| (idx, people[idx].age(year)))
        .collect();

    if living.is_empty() { return Vec::new(); }

    // Kill 10-15% of population
    let kill_pct = rng.random_range(10..=15) as f32 / 100.0;
    let kill_count = ((living.len() as f32 * kill_pct).ceil() as usize).max(1);

    // Sort: infants and elderly die first (same priority as famine)
    let mut victims = living;
    victims.sort_by(|a, b| {
        let priority_a = if a.1 < 5 { 0 } else if a.1 > 60 { 1 } else { 2 };
        let priority_b = if b.1 < 5 { 0 } else if b.1 > 60 { 1 } else { 2 };
        priority_a.cmp(&priority_b)
            .then_with(|| if a.1 > 60 && b.1 > 60 { b.1.cmp(&a.1) } else { std::cmp::Ordering::Equal })
    });

    let mut dead_ids = Vec::new();
    for &(idx, _) in victims.iter().take(kill_count) {
        people[idx].death_year = Some(year);
        dead_ids.push(people[idx].id);
    }

    // Survivors get SurvivedPlague event
    for &(idx, _) in victims.iter().skip(kill_count) {
        if people[idx].is_alive(year) {
            people[idx].life_events.push(LifeEvent {
                year,
                kind: LifeEventKind::SurvivedPlague,
            });
        }
    }

    // Destroy 50% of food stockpile (contamination, rats)
    settlement.stockpile.food = settlement.stockpile.food / 2;

    // Clear the flag — plague is a one-time pulse
    settlement.plague_this_year = false;

    dead_ids
}
