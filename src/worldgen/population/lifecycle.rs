//! Yearly lifecycle passes: death, marriage, birth.
//! Ported from the population prototype with structured outcome records.

use std::collections::HashMap;

use rand::{Rng, RngExt};

use crate::worldgen::history::state::SettlementState;

use super::index::SettlementIndex;
use super::types::{DeathCause, Person, Sex};

/// Record of a death during the death pass.
pub struct DeathRecord {
    pub person_index: usize,
    pub person_id: u32,
    pub cause: DeathCause,
}

/// Record of a marriage during the marriage pass.
pub struct MarriageRecord {
    pub male_index: usize,
    pub female_index: usize,
}

/// Record of a birth during the birth pass.
pub struct BirthRecord {
    pub person: Person,
}

/// All outcomes from one year of lifecycle simulation.
pub struct YearOutcome {
    pub deaths: Vec<DeathRecord>,
    pub marriages: Vec<MarriageRecord>,
    pub births: Vec<BirthRecord>,
}

/// Run all lifecycle passes for one year.
pub fn run_lifecycle_year(
    people: &mut Vec<Person>,
    index: &SettlementIndex,
    settlements: &[SettlementState],
    year: i32,
    next_person_id: &mut u32,
    rng: &mut impl Rng,
) -> YearOutcome {
    let deaths = death_pass(people, year, rng);
    let marriages = marriage_pass(people, index, year, rng);
    let births = birth_pass(people, settlements, year, next_person_id, rng);

    // Append newborns to the people vec
    for birth in &births {
        people.push(birth.person.clone());
    }

    YearOutcome { deaths, marriages, births }
}

// ---------------------------------------------------------------------------
// Death pass
// ---------------------------------------------------------------------------

fn death_pass(people: &mut [Person], year: i32, rng: &mut impl Rng) -> Vec<DeathRecord> {
    let mut deaths = Vec::new();

    for (i, person) in people.iter_mut().enumerate() {
        if !person.is_alive(year) { continue; }
        let age = person.age(year);

        let death_chance = if age < 1 {
            0.08 // infant mortality
        } else if age < 5 {
            0.02
        } else if age < 40 {
            0.003
        } else if age < 60 {
            0.01 + (age - 40) as f64 * 0.002
        } else if age < 75 {
            0.05 + (age - 60) as f64 * 0.01
        } else {
            0.15 + (age - 75) as f64 * 0.03
        };

        if rng.random::<f64>() < death_chance {
            person.death_year = Some(year);
            deaths.push(DeathRecord {
                person_index: i,
                person_id: person.id,
                cause: DeathCause::OldAge,
            });
        }
    }

    deaths
}

// ---------------------------------------------------------------------------
// Marriage pass
// ---------------------------------------------------------------------------

fn marriage_pass(
    people: &mut [Person],
    _index: &SettlementIndex,
    year: i32,
    _rng: &mut impl Rng,
) -> Vec<MarriageRecord> {
    // Collect eligible adults by settlement using the index
    let mut eligible_males: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut eligible_females: HashMap<u32, Vec<usize>> = HashMap::new();

    // We need to check all settlements that have residents
    for (i, person) in people.iter().enumerate() {
        if !person.is_alive(year) { continue; }
        // Eligible if never married or widowed (spouse is dead)
        if let Some(spouse_id) = person.spouse {
            let sidx = (spouse_id - 1) as usize;
            if sidx < people.len() && people[sidx].is_alive(year) {
                continue; // spouse alive — already married
            }
        }
        let age = person.age(year);
        if age < 16 || age > 45 { continue; }
        match person.sex {
            Sex::Male => eligible_males.entry(person.settlement_id).or_default().push(i),
            Sex::Female => eligible_females.entry(person.settlement_id).or_default().push(i),
        }
    }

    let mut records = Vec::new();
    for (sid, males) in &eligible_males {
        if let Some(females) = eligible_females.get(sid) {
            let pairs = males.len().min(females.len());
            let marriages = (pairs as f32 * 0.30) as usize;
            for i in 0..marriages.min(males.len()).min(females.len()) {
                records.push(MarriageRecord {
                    male_index: males[i],
                    female_index: females[i],
                });
            }
        }
    }

    // Apply marriages
    for record in &records {
        let m_id = people[record.male_index].id;
        let f_id = people[record.female_index].id;
        people[record.male_index].spouse = Some(f_id);
        people[record.female_index].spouse = Some(m_id);
    }

    records
}

// ---------------------------------------------------------------------------
// Birth pass
// ---------------------------------------------------------------------------

fn birth_pass(
    people: &[Person],
    settlements: &[SettlementState],
    year: i32,
    next_person_id: &mut u32,
    rng: &mut impl Rng,
) -> Vec<BirthRecord> {
    let mut births = Vec::new();

    for p in people.iter() {
        if !p.is_alive(year) { continue; }
        if p.sex != Sex::Female { continue; }
        let age = p.age(year);
        if age < 16 || age > 42 { continue; }

        // Check if spouse is alive
        let has_living_spouse = p.spouse
            .and_then(|sid| people.get((sid - 1) as usize))
            .is_some_and(|s| s.is_alive(year));

        let base_fertility = if age <= 30 {
            0.12
        } else if age <= 38 {
            0.06
        } else {
            0.03
        };

        // Unmarried women have children at 50% the rate
        let fertility = if has_living_spouse { base_fertility } else { base_fertility * 0.5 };

        if rng.random::<f64>() < fertility {
            let sex = if rng.random::<bool>() { Sex::Male } else { Sex::Female };

            // Inherit faith from settlement's current patron god
            let settlement = settlements.iter().find(|s| s.id == p.settlement_id);
            let faith = settlement.and_then(|s| s.patron_god);
            let devotion = if faith.is_some() { rng.random_range(20..=60) } else { 0 };
            let zone_type = settlement.and_then(|s| s.zone_type);

            // Father is living spouse if present, otherwise unknown
            let father = if has_living_spouse { p.spouse } else { None };

            let id = *next_person_id;
            *next_person_id += 1;

            births.push(BirthRecord {
                person: Person {
                    id,
                    birth_year: year,
                    death_year: None,
                    settlement_id: p.settlement_id,
                    sex,
                    mother: Some(p.id),
                    father,
                    spouse: None,
                    occupation: super::seed::occupation_for_terrain(zone_type, rng),
                    faith,
                    devotion,
                    life_events: Vec::new(),
                    notable: false,
                },
            });
        }
    }

    births
}
