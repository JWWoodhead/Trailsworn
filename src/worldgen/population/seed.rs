//! Initial population seeding from settlement state.

use rand::{Rng, RngExt};

use crate::worldgen::history::state::{PopulationClass, SettlementState};
use crate::worldgen::zone::ZoneType;

use crate::worldgen::names::Race;

use super::types::{Occupation, Person, Sex};

/// Convert PopulationClass to an approximate headcount.
fn headcount(class: PopulationClass) -> u32 {
    match class {
        PopulationClass::Hamlet => 30,
        PopulationClass::Village => 120,
        PopulationClass::Town => 500,
        PopulationClass::City => 3000,
    }
}

/// Terrain-aware occupation assignment.
/// Zone type determines the worker mix — grasslands produce farmers,
/// forests produce woodcutters, mountains produce miners, etc.
pub fn occupation_for_terrain(zone_type: Option<ZoneType>, rng: &mut impl Rng) -> Occupation {
    let roll: f32 = rng.random();
    match zone_type {
        Some(ZoneType::Grassland) => {
            if roll < 0.55 { Occupation::Farmer }
            else if roll < 0.65 { Occupation::Hunter }
            else if roll < 0.75 { Occupation::Soldier }
            else if roll < 0.83 { Occupation::Smith }
            else if roll < 0.90 { Occupation::Merchant }
            else if roll < 0.96 { Occupation::Priest }
            else { Occupation::Scholar }
        }
        Some(ZoneType::Forest) => {
            if roll < 0.30 { Occupation::Farmer }
            else if roll < 0.50 { Occupation::Woodcutter }
            else if roll < 0.65 { Occupation::Hunter }
            else if roll < 0.75 { Occupation::Soldier }
            else if roll < 0.82 { Occupation::Smith }
            else if roll < 0.88 { Occupation::Merchant }
            else if roll < 0.94 { Occupation::Priest }
            else { Occupation::Scholar }
        }
        Some(ZoneType::Mountain) => {
            if roll < 0.25 { Occupation::Farmer }
            else if roll < 0.42 { Occupation::Miner }
            else if roll < 0.55 { Occupation::Quarrier }
            else if roll < 0.65 { Occupation::Soldier }
            else if roll < 0.75 { Occupation::Smith }
            else if roll < 0.83 { Occupation::Merchant }
            else if roll < 0.92 { Occupation::Priest }
            else { Occupation::Scholar }
        }
        Some(ZoneType::Coast) => {
            if roll < 0.45 { Occupation::Farmer } // includes fishing
            else if roll < 0.55 { Occupation::Hunter }
            else if roll < 0.65 { Occupation::Merchant }
            else if roll < 0.75 { Occupation::Soldier }
            else if roll < 0.83 { Occupation::Smith }
            else if roll < 0.92 { Occupation::Priest }
            else { Occupation::Scholar }
        }
        Some(ZoneType::Tundra) => {
            if roll < 0.30 { Occupation::Farmer }
            else if roll < 0.55 { Occupation::Hunter }
            else if roll < 0.65 { Occupation::Woodcutter }
            else if roll < 0.75 { Occupation::Soldier }
            else if roll < 0.83 { Occupation::Smith }
            else if roll < 0.92 { Occupation::Priest }
            else { Occupation::Merchant }
        }
        Some(ZoneType::Desert) => {
            if roll < 0.40 { Occupation::Farmer }
            else if roll < 0.50 { Occupation::Quarrier }
            else if roll < 0.60 { Occupation::Merchant }
            else if roll < 0.70 { Occupation::Soldier }
            else if roll < 0.80 { Occupation::Smith }
            else if roll < 0.90 { Occupation::Priest }
            else { Occupation::Scholar }
        }
        Some(ZoneType::Swamp) => {
            if roll < 0.40 { Occupation::Farmer }
            else if roll < 0.55 { Occupation::Hunter }
            else if roll < 0.65 { Occupation::Woodcutter }
            else if roll < 0.75 { Occupation::Soldier }
            else if roll < 0.83 { Occupation::Smith }
            else if roll < 0.92 { Occupation::Priest }
            else { Occupation::Merchant }
        }
        // Settlement zone or unknown — diverse city-like mix
        _ => {
            if roll < 0.20 { Occupation::Farmer }
            else if roll < 0.35 { Occupation::Merchant }
            else if roll < 0.48 { Occupation::Smith }
            else if roll < 0.58 { Occupation::Soldier }
            else if roll < 0.68 { Occupation::Woodcutter }
            else if roll < 0.76 { Occupation::Miner }
            else if roll < 0.84 { Occupation::Priest }
            else if roll < 0.92 { Occupation::Scholar }
            else { Occupation::Hunter }
        }
    }
}

/// Seed the initial population from settlement states.
/// Returns `(people, next_person_id)`.
pub fn seed_population(
    settlements: &[SettlementState],
    factions: &[crate::worldgen::history::state::FactionState],
    start_year: i32,
    rng: &mut impl Rng,
) -> (Vec<Person>, u32) {
    let total_estimate: u32 = settlements.iter()
        .filter(|s| s.destroyed_year.is_none())
        .map(|s| headcount(s.population_class))
        .sum();
    let mut people: Vec<Person> = Vec::with_capacity(total_estimate as usize);
    let mut next_id = 1u32;

    for settlement in settlements {
        if settlement.destroyed_year.is_some() { continue; }

        let count = headcount(settlement.population_class);
        for _ in 0..count {
            let sex = if rng.random::<bool>() { Sex::Male } else { Sex::Female };
            // Mix of ages: ~30% children (0-15), ~55% working adults (16-50), ~15% elderly (51-70)
            let age = {
                let roll: f32 = rng.random();
                if roll < 0.30 { rng.random_range(0..16) }
                else if roll < 0.85 { rng.random_range(16..51) }
                else { rng.random_range(51..71) }
            };
            people.push(Person {
                id: next_id,
                birth_year: start_year - age,
                death_year: None,
                death_cause: None,
                settlement_id: settlement.id,
                faction_allegiance: settlement.controlling_faction,
                sex,
                race: factions.iter()
                    .find(|f| f.id == settlement.controlling_faction)
                    .map(|f| f.race)
                    .unwrap_or(Race::Human),
                secondary_race: None,
                mother: None,
                father: None,
                spouse: None,
                occupation: occupation_for_terrain(settlement.zone_type, rng),
                traits: super::traits::seed_traits(None, None, rng),
                happiness: 50, prophet_of: None, years_as_outlier: 0,
                faith: settlement.patron_god
                    .map(|g| vec![(g, rng.random_range(30..=70))])
                    .unwrap_or_default(),
                life_events: Vec::new(),
                notable: false,
            });
            next_id += 1;
        }
    }

    // Pre-marry ~60% of adults (16+) in same settlement
    let mut unmarried: std::collections::HashMap<u32, (Vec<usize>, Vec<usize>)> =
        std::collections::HashMap::new();
    for (i, p) in people.iter().enumerate() {
        if p.age(start_year) < 16 { continue; } // children don't marry
        let entry = unmarried.entry(p.settlement_id).or_default();
        match p.sex {
            Sex::Male => entry.0.push(i),
            Sex::Female => entry.1.push(i),
        }
    }
    for (_sid, (males, females)) in &unmarried {
        let pairs = males.len().min(females.len()) * 6 / 10;
        for i in 0..pairs {
            let m_idx = males[i];
            let f_idx = females[i];
            let m_id = people[m_idx].id;
            let f_id = people[f_idx].id;
            people[m_idx].spouse = Some(f_id);
            people[f_idx].spouse = Some(m_id);
        }
    }

    (people, next_id)
}
