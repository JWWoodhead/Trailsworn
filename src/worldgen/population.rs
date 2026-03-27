//! Population simulation prototype.
//! Simulates every person in the world: birth, death, family, settlement.
//! Notable characters emerge from this population fabric.

use rand::{RngExt, SeedableRng};

// ---------------------------------------------------------------------------
// Data model — intentionally minimal to benchmark the lifecycle cost.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sex {
    Male,
    Female,
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
}

impl Person {
    pub fn is_alive(&self, year: i32) -> bool {
        self.death_year.is_none() || self.death_year.unwrap() > year
    }

    pub fn age(&self, year: i32) -> i32 {
        year - self.birth_year
    }
}

/// A settlement with a population capacity used to seed initial population.
#[derive(Clone, Debug)]
pub struct SettlementPop {
    pub id: u32,
    pub initial_population: u32,
}

/// Result of a population simulation run, for benchmarking.
#[derive(Debug)]
pub struct PopulationStats {
    pub total_ever_lived: u32,
    pub alive_at_end: u32,
    pub peak_living: u32,
    pub total_births: u32,
    pub total_deaths: u32,
    pub memory_bytes_estimate: usize,
}

// ---------------------------------------------------------------------------
// Simulation
// ---------------------------------------------------------------------------

/// Simulate a full population lifecycle across settlements for `num_years`.
pub fn simulate_population(
    settlements: &[SettlementPop],
    num_years: i32,
    seed: u64,
) -> (Vec<Person>, PopulationStats) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut people: Vec<Person> = Vec::new();
    let mut next_id = 1u32;

    // Seed initial population — adults of varying ages
    for settlement in settlements {
        for _ in 0..settlement.initial_population {
            let sex = if rng.random::<bool>() { Sex::Male } else { Sex::Female };
            let age = rng.random_range(16..55);
            people.push(Person {
                id: next_id,
                birth_year: -age,
                death_year: None,
                settlement_id: settlement.id,
                sex,
                mother: None,
                father: None,
                spouse: None,
            });
            next_id += 1;
        }
    }

    // Pre-marry some initial adults (pair up ~60% of adults in same settlement)
    let mut unmarried_by_settlement: std::collections::HashMap<u32, (Vec<u32>, Vec<u32>)> =
        std::collections::HashMap::new();
    for p in &people {
        let entry = unmarried_by_settlement
            .entry(p.settlement_id)
            .or_insert_with(|| (Vec::new(), Vec::new()));
        match p.sex {
            Sex::Male => entry.0.push(p.id),
            Sex::Female => entry.1.push(p.id),
        }
    }
    for (_sid, (males, females)) in &unmarried_by_settlement {
        let pairs = males.len().min(females.len()) * 6 / 10;
        for i in 0..pairs {
            let m_id = males[i];
            let f_id = females[i];
            if let Some(m) = people.iter_mut().find(|p| p.id == m_id) {
                m.spouse = Some(f_id);
            }
            if let Some(f) = people.iter_mut().find(|p| p.id == f_id) {
                f.spouse = Some(m_id);
            }
        }
    }

    let mut stats = PopulationStats {
        total_ever_lived: people.len() as u32,
        alive_at_end: 0,
        peak_living: people.len() as u32,
        total_births: 0,
        total_deaths: 0,
        memory_bytes_estimate: 0,
    };

    // Year-by-year simulation
    for year in 0..num_years {
        let living_count_start = people.iter().filter(|p| p.is_alive(year)).count();

        // --- Death pass ---
        for person in people.iter_mut() {
            if !person.is_alive(year) { continue; }
            let age = person.age(year);

            // Mortality curve: very low until 40, ramps up sharply after 60
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
                stats.total_deaths += 1;
            }
        }

        // --- Marriage pass ---
        // Collect unmarried living adults by settlement
        let mut eligible_males: std::collections::HashMap<u32, Vec<u32>> =
            std::collections::HashMap::new();
        let mut eligible_females: std::collections::HashMap<u32, Vec<u32>> =
            std::collections::HashMap::new();

        for p in people.iter() {
            if !p.is_alive(year) { continue; }
            if p.spouse.is_some() { continue; }
            let age = p.age(year);
            if age < 16 || age > 45 { continue; }
            match p.sex {
                Sex::Male => eligible_males.entry(p.settlement_id).or_default().push(p.id),
                Sex::Female => eligible_females.entry(p.settlement_id).or_default().push(p.id),
            }
        }

        let mut new_marriages: Vec<(u32, u32)> = Vec::new();
        for (sid, males) in &eligible_males {
            if let Some(females) = eligible_females.get(sid) {
                let pairs = males.len().min(females.len());
                // ~30% of eligible pairs marry per year
                let marriages = (pairs as f32 * 0.30) as usize;
                for i in 0..marriages.min(males.len()).min(females.len()) {
                    new_marriages.push((males[i], females[i]));
                }
            }
        }

        for (m_id, f_id) in &new_marriages {
            if let Some(m) = people.iter_mut().find(|p| p.id == *m_id) {
                m.spouse = Some(*f_id);
            }
            if let Some(f) = people.iter_mut().find(|p| p.id == *f_id) {
                f.spouse = Some(*m_id);
            }
        }

        // --- Birth pass ---
        let mut new_births: Vec<Person> = Vec::new();

        for p in people.iter() {
            if !p.is_alive(year) { continue; }
            if p.sex != Sex::Female { continue; }
            if p.spouse.is_none() { continue; }
            let age = p.age(year);
            if age < 16 || age > 42 { continue; }

            // Fertility rate: ~0.15 per year for women 16-30, drops off after
            let fertility = if age <= 30 {
                0.15
            } else if age <= 38 {
                0.08
            } else {
                0.03
            };

            if rng.random::<f64>() < fertility {
                let sex = if rng.random::<bool>() { Sex::Male } else { Sex::Female };
                new_births.push(Person {
                    id: next_id,
                    birth_year: year,
                    death_year: None,
                    settlement_id: p.settlement_id,
                    sex,
                    mother: Some(p.id),
                    father: p.spouse,
                    spouse: None,
                });
                next_id += 1;
                stats.total_births += 1;
            }
        }

        people.extend(new_births);
        stats.total_ever_lived = (next_id - 1) as u32;

        let living_now = people.iter().filter(|p| p.is_alive(year)).count() as u32;
        if living_now > stats.peak_living {
            stats.peak_living = living_now;
        }
    }

    stats.alive_at_end = people.iter().filter(|p| p.is_alive(num_years)).count() as u32;
    stats.memory_bytes_estimate = people.len() * std::mem::size_of::<Person>();

    (people, stats)
}

// ---------------------------------------------------------------------------
// Tests / Benchmarks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn default_settlements() -> Vec<SettlementPop> {
        let mut settlements = Vec::new();
        let mut id = 1u32;

        // ~3 cities (3000-10000 people)
        for _ in 0..3 {
            settlements.push(SettlementPop { id, initial_population: 5000 });
            id += 1;
        }
        // ~8 towns (500-2000)
        for _ in 0..8 {
            settlements.push(SettlementPop { id, initial_population: 1000 });
            id += 1;
        }
        // ~20 villages (50-200)
        for _ in 0..20 {
            settlements.push(SettlementPop { id, initial_population: 120 });
            id += 1;
        }
        // ~40 hamlets (10-50)
        for _ in 0..40 {
            settlements.push(SettlementPop { id, initial_population: 30 });
            id += 1;
        }

        settlements
    }

    #[test]
    fn population_benchmark() {
        let settlements = default_settlements();
        let initial_pop: u32 = settlements.iter().map(|s| s.initial_population).sum();
        println!("\n=== Population Simulation Benchmark ===");
        println!("Settlements: {}", settlements.len());
        println!("Initial population: {}", initial_pop);

        let start = Instant::now();
        let (people, stats) = simulate_population(&settlements, 100, 42);
        let elapsed = start.elapsed();

        println!("\n--- Results (100 years) ---");
        println!("Time: {:.2?}", elapsed);
        println!("Total ever lived: {}", stats.total_ever_lived);
        println!("Alive at end: {}", stats.alive_at_end);
        println!("Peak living: {}", stats.peak_living);
        println!("Total births: {}", stats.total_births);
        println!("Total deaths: {}", stats.total_deaths);
        println!("Memory (Person vec): {:.2} MB", stats.memory_bytes_estimate as f64 / (1024.0 * 1024.0));
        println!("Person struct size: {} bytes", std::mem::size_of::<Person>());

        // Sanity checks
        assert!(stats.alive_at_end > 0, "Everyone died");
        assert!(stats.total_births > 0, "No births happened");
        assert!(stats.total_deaths > 0, "No deaths happened");
        assert!(stats.alive_at_end < stats.total_ever_lived, "No one ever died");

        // Check family links exist
        let has_parents = people.iter().filter(|p| p.mother.is_some()).count();
        println!("People with parent links: {} ({:.1}%)",
            has_parents, has_parents as f64 / people.len() as f64 * 100.0);

        let has_spouse = people.iter().filter(|p| p.spouse.is_some()).count();
        println!("People who married: {} ({:.1}%)",
            has_spouse, has_spouse as f64 / people.len() as f64 * 100.0);

        println!("=== Benchmark Complete ===\n");
    }

    #[test]
    fn population_scales_with_settlements() {
        // Test that doubling settlements roughly doubles work but stays fast
        let small: Vec<SettlementPop> = (1..=10).map(|id| SettlementPop { id, initial_population: 100 }).collect();
        let large: Vec<SettlementPop> = (1..=70).map(|id| SettlementPop { id, initial_population: 100 }).collect();

        let start = Instant::now();
        let (_, stats_small) = simulate_population(&small, 100, 42);
        let small_time = start.elapsed();

        let start = Instant::now();
        let (_, stats_large) = simulate_population(&large, 100, 42);
        let large_time = start.elapsed();

        println!("\n10 settlements (1000 initial): {:.2?}, {} ever lived", small_time, stats_small.total_ever_lived);
        println!("70 settlements (7000 initial): {:.2?}, {} ever lived", large_time, stats_large.total_ever_lived);

        assert!(stats_large.total_ever_lived > stats_small.total_ever_lived);
    }
}
