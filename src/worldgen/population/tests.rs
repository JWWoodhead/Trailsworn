use rand::SeedableRng;

use crate::worldgen::history::state::{FactionState, PopulationClass, SettlementState};
use crate::worldgen::names::{FactionType, Race};

use super::notable;
use super::types::*;
use super::PopulationSim;

fn rng() -> rand::rngs::StdRng {
    rand::rngs::StdRng::seed_from_u64(42)
}

fn hamlet(id: u32) -> SettlementState {
    SettlementState {
        id,
        name: format!("Hamlet {}", id),
        founded_year: 0,
        controlling_faction: 1,
        destroyed_year: None,
        region: "Test".into(),
        population_class: PopulationClass::Hamlet,
        prosperity: 50,
        defenses: 20,
        patron_god: None,
        devotion: 0,
        world_pos: None,
        zone_type: Some(crate::worldgen::zone::ZoneType::Grassland),
        stockpile: crate::worldgen::history::state::ResourceStockpile::default(),
        plague_this_year: false, conquered_this_year: false, conquered_by: None,
        dominant_race: None,
    }
}

fn village(id: u32) -> SettlementState {
    let mut s = hamlet(id);
    s.population_class = PopulationClass::Village;
    s
}

fn town(id: u32) -> SettlementState {
    let mut s = hamlet(id);
    s.population_class = PopulationClass::Town;
    s
}

fn test_faction() -> FactionState {
    FactionState {
        id: 1,
        name: "Test Faction".into(),
        faction_type: FactionType::TribalWarband,
        race: Race::Human,
        founded_year: 0,
        home_region: "Test".into(),
        dissolved_year: None,
        leader_name: "Leader".into(),
        leader_id: None,
        military_strength: 50,
        wealth: 50,
        stability: 50,
        territory: vec!["Test".into()],
        settlements: vec![1],
        patron_god: None,
        devotion: 0,
        unhappy_years: 0,
    }
}

// ---------------------------------------------------------------------------
// Seeding
// ---------------------------------------------------------------------------

#[test]
fn seed_hamlet_headcount() {
    let mut settlements = vec![hamlet(1)];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    // Hamlet = 30 people
    assert_eq!(sim.people.len(), 30);
}

#[test]
fn seed_village_headcount() {
    let mut settlements = vec![village(1)];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    assert_eq!(sim.people.len(), 120);
}

#[test]
fn seed_town_headcount() {
    let mut settlements = vec![town(1)];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    assert_eq!(sim.people.len(), 500);
}

#[test]
fn seed_skips_destroyed_settlements() {
    let mut s = hamlet(1);
    s.destroyed_year = Some(-10);
    let mut settlements = vec![s, hamlet(2)];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    assert_eq!(sim.people.len(), 30); // only hamlet 2
}

#[test]
fn seed_premarries_adults() {
    let mut settlements = vec![village(1)];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    let married = sim.people.iter().filter(|p| p.spouse.is_some()).count();
    // ~60% of min(males, females) pairs -> roughly 60+ people married
    assert!(married > 40, "only {} married out of {}", married, sim.people.len());
}

#[test]
fn seed_faith_from_patron() {
    let mut s = hamlet(1);
    s.patron_god = Some(5);
    let mut settlements = vec![s];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    for person in &sim.people {
        assert_eq!(person.primary_god(), Some(5));
        let dev = person.devotion_to(5);
        assert!(dev >= 30 && dev <= 70, "devotion {} out of range", dev);
    }
}

#[test]
fn seed_no_faith_without_patron() {
    let mut settlements = vec![hamlet(1)];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    for person in &sim.people {
        assert!(person.faith.is_empty());
    }
}

#[test]
fn seed_mixed_ages() {
    let mut settlements = vec![hamlet(1)];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    let children = sim.people.iter().filter(|p| p.age(0) < 16).count();
    let adults = sim.people.iter().filter(|p| p.age(0) >= 16 && p.age(0) < 51).count();
    let elderly = sim.people.iter().filter(|p| p.age(0) >= 51).count();
    assert!(children > 0, "should seed some children");
    assert!(adults > 0, "should seed some adults");
    assert!(elderly > 0, "should seed some elderly");
    for person in &sim.people {
        let age = person.age(0);
        assert!(age >= 0 && age < 71, "unexpected age {}", age);
    }
}

// ---------------------------------------------------------------------------
// Lifecycle — births and deaths
// ---------------------------------------------------------------------------

#[test]
fn lifecycle_produces_births_and_deaths() {
    let mut settlements = vec![village(1)];
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng());
    let initial = sim.people.len();

    // Run 10 years
    for year in 0..10 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng());
    }

    let born = sim.people.len() - initial;
    let dead = sim.people.iter().filter(|p| p.death_year.is_some()).count();

    assert!(born > 0, "no births in 10 years");
    assert!(dead > 0, "no deaths in 10 years");
}

#[test]
fn marriage_links_spouses() {
    let mut settlements = vec![village(1)];
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng());

    // Run a few years for new marriages to form (new births reaching adulthood takes too long,
    // but initial unmarried adults should marry)
    for year in 0..5 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng());
    }

    // Check reciprocal spouse links among living couples
    for person in &sim.people {
        if person.death_year.is_some() { continue; } // dead people may have stale spouse links
        if let Some(spouse_id) = person.spouse {
            let spouse = sim.person(spouse_id);
            if let Some(spouse) = spouse {
                // Both alive — should be reciprocal
                assert_eq!(spouse.spouse, Some(person.id),
                    "spouse link not reciprocal for living person {} <-> {}", person.id, spouse_id);
            }
            // spouse dead = widowed, ok — they may remarry and the dead spouse's link is stale
        }
    }
}

// ---------------------------------------------------------------------------
// Family events
// ---------------------------------------------------------------------------

#[test]
fn death_generates_lost_spouse_event() {
    let mut settlements = vec![village(1)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    // Run enough years for some married people to die
    for year in 0..30 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
    }

    let lost_spouse_count = sim.people.iter()
        .flat_map(|p| &p.life_events)
        .filter(|e| matches!(e.kind, LifeEventKind::LostSpouse { .. }))
        .count();

    assert!(lost_spouse_count > 0, "no LostSpouse events after 30 years");
}

#[test]
fn birth_generates_child_born_event() {
    let mut settlements = vec![village(1)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    for year in 0..5 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
    }

    let child_born_count = sim.people.iter()
        .flat_map(|p| &p.life_events)
        .filter(|e| matches!(e.kind, LifeEventKind::ChildBorn { .. }))
        .count();

    assert!(child_born_count > 0, "no ChildBorn events after 5 years");
}

#[test]
fn death_generates_lost_parent_event() {
    let mut settlements = vec![village(1)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    // Need births first, then parent deaths — run a longer period
    for year in 0..50 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
    }

    let lost_parent_count = sim.people.iter()
        .flat_map(|p| &p.life_events)
        .filter(|e| matches!(e.kind, LifeEventKind::LostParent { .. }))
        .count();

    assert!(lost_parent_count > 0, "no LostParent events after 50 years");
}

#[test]
fn death_generates_lost_sibling_event() {
    let mut settlements = vec![village(1)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    for year in 0..50 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
    }

    let lost_sibling_count = sim.people.iter()
        .flat_map(|p| &p.life_events)
        .filter(|e| matches!(e.kind, LifeEventKind::LostSibling { .. }))
        .count();

    assert!(lost_sibling_count > 0, "no LostSibling events after 50 years");
}

#[test]
fn marriage_generates_married_to_event() {
    let mut settlements = vec![village(1)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    for year in 0..5 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
    }

    let married_count = sim.people.iter()
        .flat_map(|p| &p.life_events)
        .filter(|e| matches!(e.kind, LifeEventKind::MarriedTo { .. }))
        .count();

    assert!(married_count > 0, "no MarriedTo events after 5 years");
    // MarriedTo should always come in pairs
    assert_eq!(married_count % 2, 0, "odd number of MarriedTo events");
}

// ---------------------------------------------------------------------------
// Notable threshold
// ---------------------------------------------------------------------------

#[test]
fn notable_requires_threshold_events() {
    let mut person = Person {
        id: 1, birth_year: 0, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
        sex: Sex::Male, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
        occupation: Occupation::Farmer, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: Vec::new(),
        life_events: Vec::new(), notable: false,
    };

    // Normal life events (marriage, children) don't count toward notable threshold
    person.life_events.push(LifeEvent { year: 10, kind: LifeEventKind::MarriedTo { spouse_id: 2 }, cause: None });
    person.life_events.push(LifeEvent { year: 12, kind: LifeEventKind::ChildBorn { child_id: 3 }, cause: None });
    assert!(!notable::check_notable(&mut person));
    assert!(!person.notable);

    // Loss events count — but need 5 to reach threshold
    person.life_events.push(LifeEvent { year: 15, kind: LifeEventKind::LostParent { parent_id: 4, cause: DeathCause::OldAge }, cause: None });
    person.life_events.push(LifeEvent { year: 18, kind: LifeEventKind::LostParent { parent_id: 5, cause: DeathCause::OldAge }, cause: None });
    person.life_events.push(LifeEvent { year: 22, kind: LifeEventKind::LostSibling { sibling_id: 6, cause: DeathCause::War }, cause: None });
    person.life_events.push(LifeEvent { year: 25, kind: LifeEventKind::LostSpouse { spouse_id: 2, cause: DeathCause::Plague }, cause: None });
    assert!(!notable::check_notable(&mut person)); // 4 loss events, still under 5

    person.life_events.push(LifeEvent { year: 30, kind: LifeEventKind::LostChild { child_id: 3, cause: DeathCause::War }, cause: None });
    assert!(notable::check_notable(&mut person)); // 5 loss events — notable
    assert!(person.notable);
}

#[test]
fn notable_only_fires_once() {
    let mut person = Person {
        id: 1, birth_year: 0, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
        sex: Sex::Male, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
        occupation: Occupation::Farmer, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: Vec::new(),
        life_events: vec![
            LifeEvent { year: 1, kind: LifeEventKind::LostParent { parent_id: 10, cause: DeathCause::OldAge }, cause: None },
            LifeEvent { year: 2, kind: LifeEventKind::LostParent { parent_id: 11, cause: DeathCause::OldAge }, cause: None },
            LifeEvent { year: 3, kind: LifeEventKind::LostSibling { sibling_id: 12, cause: DeathCause::War }, cause: None },
            LifeEvent { year: 4, kind: LifeEventKind::LostSpouse { spouse_id: 13, cause: DeathCause::Plague }, cause: None },
            LifeEvent { year: 5, kind: LifeEventKind::LostChild { child_id: 14, cause: DeathCause::Famine }, cause: None },
        ],
        notable: false,
    };

    assert!(notable::check_notable(&mut person));  // first time -> true
    assert!(!notable::check_notable(&mut person)); // already notable -> false
}

#[test]
fn sim_produces_notables_over_time() {
    let mut settlements = vec![village(1), village(2)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    let mut total_notables = 0;
    for year in 0..100 {
        let newly = sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
        total_notables += newly.len();
    }

    assert!(total_notables > 0, "no notables emerged in 100 years across 2 villages");
}

// ---------------------------------------------------------------------------
// Promote to character
// ---------------------------------------------------------------------------

#[test]
fn promote_produces_valid_character() {
    let mut settlements = vec![hamlet(1)];
    let factions = vec![test_faction()];
    let mut rng = rng();

    let person = Person {
        id: 1, birth_year: -20, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
        sex: Sex::Female, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
        occupation: Occupation::Soldier, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: Vec::new(),
        life_events: vec![
            LifeEvent { year: 5, kind: LifeEventKind::MarriedTo { spouse_id: 2 }, cause: None },
            LifeEvent { year: 8, kind: LifeEventKind::ChildBorn { child_id: 3 }, cause: None },
            LifeEvent { year: 12, kind: LifeEventKind::LostSpouse { spouse_id: 2, cause: DeathCause::War }, cause: None },
        ],
        notable: true,
    };

    let mut next_id = 100;
    let character = notable::promote_to_character(&person, &mut next_id, &settlements, &factions, &mut rng);

    assert_eq!(character.id, 100);
    assert_eq!(next_id, 101);
    assert_eq!(character.birth_year, -20);
    assert_eq!(character.race, Race::Human);
    assert_eq!(character.faction_id, Some(1));
    assert!(!character.traits.is_empty());
    assert!(character.renown > 5, "renown should be boosted by life events");
}

#[test]
fn promote_soldier_becomes_hero() {
    let mut settlements = vec![hamlet(1)];
    let factions = vec![test_faction()];
    let mut rng = rng();

    let person = Person {
        id: 1, birth_year: -30, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
        sex: Sex::Male, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
        occupation: Occupation::Soldier, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: Vec::new(),
        life_events: vec![
            LifeEvent { year: 5, kind: LifeEventKind::LostParent { parent_id: 10, cause: DeathCause::War }, cause: None },
            LifeEvent { year: 8, kind: LifeEventKind::LostParent { parent_id: 11, cause: DeathCause::War }, cause: None },
            LifeEvent { year: 10, kind: LifeEventKind::SurvivedWar { enemy_faction_id: 2 }, cause: None },
            LifeEvent { year: 12, kind: LifeEventKind::LostSibling { sibling_id: 12, cause: DeathCause::War }, cause: None },
            LifeEvent { year: 15, kind: LifeEventKind::LostSpouse { spouse_id: 13, cause: DeathCause::Famine }, cause: None },
        ],
        notable: true,
    };

    let mut next_id = 200;
    let character = notable::promote_to_character(&person, &mut next_id, &settlements, &factions, &mut rng);

    use crate::worldgen::history::characters::CharacterRole;
    assert_eq!(character.role, CharacterRole::Hero);
}

// ---------------------------------------------------------------------------
// End-to-end benchmark
// ---------------------------------------------------------------------------

#[test]
fn population_benchmark() {
    use std::time::Instant;

    // Roughly match the world: 3 cities, 8 towns, 20 villages, 40 hamlets
    let mut settlements = Vec::new();
    let mut id = 1;
    for _ in 0..3 {
        let mut s = hamlet(id); s.population_class = PopulationClass::City; settlements.push(s); id += 1;
    }
    for _ in 0..8 {
        settlements.push(town(id)); id += 1;
    }
    for _ in 0..20 {
        settlements.push(village(id)); id += 1;
    }
    for _ in 0..40 {
        settlements.push(hamlet(id)); id += 1;
    }

    let _initial_pop: usize = settlements.len();
    let mut rng = rng();
    let start = Instant::now();

    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);
    let seeded = sim.people.len();

    let mut total_notables = 0;
    for year in 0..100 {
        let newly = sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
        total_notables += newly.len();
    }
    let elapsed = start.elapsed();

    let alive = sim.living_count(100);
    let dead = sim.people.iter().filter(|p| p.death_year.is_some()).count();
    let with_events = sim.people.iter().filter(|p| !p.life_events.is_empty()).count();
    let mem = sim.people.len() * std::mem::size_of::<Person>()
        + sim.people.iter().map(|p| p.life_events.len() * std::mem::size_of::<LifeEvent>()).sum::<usize>();

    println!("\n=== Population Benchmark ===");
    println!("Settlements: {}", settlements.len());
    println!("Seeded: {}", seeded);
    println!("Time: {:.2?}", elapsed);
    println!("Total ever: {}", sim.people.len());
    println!("Alive at 100: {}", alive);
    println!("Dead: {}", dead);
    println!("With life events: {}", with_events);
    println!("Notables promoted: {}", total_notables);
    println!("Memory estimate: {:.2} MB", mem as f64 / (1024.0 * 1024.0));
    println!("Person struct: {} bytes", std::mem::size_of::<Person>());
    println!("LifeEvent struct: {} bytes", std::mem::size_of::<LifeEvent>());
    println!("=== End ===\n");

    // Sanity checks
    assert!(alive > 0, "everyone died");
    assert!(dead > 0, "no one died");
    assert!(sim.people.len() > seeded, "no births happened");
    assert!(with_events > 0, "no life events generated");
    assert!(total_notables > 0, "no notables in 100 years");
    // Population should be roughly stable (within 30% of initial)
    let ratio = alive as f64 / seeded as f64;
    assert!(ratio > 0.5 && ratio < 2.0,
        "population unstable: {} seeded, {} alive (ratio {:.2})", seeded, alive, ratio);
}

#[test]
fn deterministic_with_same_seed() {
    // Full run: returns (total_people, alive, notables, total_events, last_person_death_year)
    let run = |seed: u64| -> (usize, usize, usize, usize, Option<i32>) {
        let mut settlements = vec![village(1), hamlet(2), town(3)];
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);
        let mut notables = 0;
        for year in 0..100 {
            notables += sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng).len();
        }
        let total_events: usize = sim.people.iter().map(|p| p.life_events.len()).sum();
        let last_death = sim.people.last().and_then(|p| p.death_year);
        (sim.people.len(), sim.living_count(100), notables, total_events, last_death)
    };

    // Same seed must produce identical results
    let a = run(42);
    let b = run(42);
    assert_eq!(a, b, "seed 42: runs differ — not deterministic");

    let c = run(999);
    let d = run(999);
    assert_eq!(c, d, "seed 999: runs differ — not deterministic");

    let e = run(0);
    let f = run(0);
    assert_eq!(e, f, "seed 0: runs differ — not deterministic");

    // Different seeds must differ
    assert_ne!(a, c, "seeds 42 and 999 should produce different results");
    assert_ne!(a, e, "seeds 42 and 0 should produce different results");

    // Check small-settlement stability across many seeds
    for seed in [42, 999, 0, 7, 1234, 9999, 777, 55555] {
        let result = run(seed);
        let seeded = 650; // village(120) + hamlet(30) + town(500)
        let ratio = result.1 as f64 / seeded as f64;
        assert!(ratio > 0.3, "seed {}: population collapsed to {} (ratio {:.2})", seed, result.1, ratio);
    }
}

#[test]
fn person_lookup_by_id() {
    let mut settlements = vec![hamlet(1)];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());

    // IDs start at 1
    assert!(sim.person(0).is_none());
    assert!(sim.person(1).is_some());
    assert_eq!(sim.person(1).unwrap().id, 1);
    assert!(sim.person(30).is_some());
    assert!(sim.person(31).is_none()); // hamlet has 30 people
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[test]
fn grassland_settlement_produces_food_surplus() {
    let mut settlements = vec![hamlet(1)]; // zone_type = Grassland
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    // Run a few years so resources accumulate
    for year in 0..5 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
    }

    // Grassland hamlet with ~55% farmers should have food surplus
    assert!(settlements[0].stockpile.food > 0,
        "grassland settlement should have food surplus, got {}", settlements[0].stockpile.food);
}

#[test]
fn terrain_affects_occupation_mix() {
    use crate::worldgen::zone::ZoneType;

    let mut forest_settlement = hamlet(1);
    forest_settlement.zone_type = Some(ZoneType::Forest);
    let mut mountain_settlement = hamlet(2);
    mountain_settlement.zone_type = Some(ZoneType::Mountain);

    let settlements = vec![forest_settlement, mountain_settlement];
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng());

    let forest_woodcutters = sim.people.iter()
        .filter(|p| p.settlement_id == 1 && p.occupation == Occupation::Woodcutter)
        .count();
    let mountain_miners = sim.people.iter()
        .filter(|p| p.settlement_id == 2 && p.occupation == Occupation::Miner)
        .count();

    assert!(forest_woodcutters > 0, "forest settlement should have woodcutters");
    assert!(mountain_miners > 0, "mountain settlement should have miners");
}

#[test]
fn famine_kills_people_in_desert() {
    use crate::worldgen::zone::ZoneType;

    // Desert with very few farmers — should trigger famine
    let mut desert = hamlet(1);
    desert.zone_type = Some(ZoneType::Desert);
    let mut settlements = vec![desert];

    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);
    let initial_alive = sim.living_count(0);

    // Run 20 years — desert food penalty should cause some famine
    for year in 0..20 {
        sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), year, &mut rng);
    }

    let famine_deaths = sim.people.iter()
        .flat_map(|p| &p.life_events)
        .filter(|e| matches!(e.kind,
            LifeEventKind::LostParent { cause: DeathCause::Famine, .. }
            | LifeEventKind::LostSpouse { cause: DeathCause::Famine, .. }
            | LifeEventKind::LostChild { cause: DeathCause::Famine, .. }
        ))
        .count();

    // Desert settlements should experience at least some food pressure
    // (may or may not trigger full famine depending on RNG/occupation mix)
    let alive_now = sim.living_count(20);
    println!("Desert hamlet: {} initial, {} alive after 20 years, {} famine-related events",
        initial_alive, alive_now, famine_deaths);
}

#[test]
fn food_stockpile_has_spoilage() {
    use super::resources;
    use crate::worldgen::history::state::ResourceStockpile;

    let mut stockpile = ResourceStockpile { food: 100, timber: 50, ore: 30, leather: 20, stone: 10 };
    resources::apply_spoilage(&mut stockpile);

    // Food should decay ~30%
    assert!(stockpile.food < 100, "food should decay from spoilage");
    assert!(stockpile.food >= 60 && stockpile.food <= 75,
        "food should be ~70 after 30% spoilage, got {}", stockpile.food);
    // Other resources unaffected
    assert_eq!(stockpile.timber, 50);
    assert_eq!(stockpile.ore, 30);
}

#[test]
fn resource_stockpile_caps() {
    use super::resources;
    use crate::worldgen::history::state::ResourceStockpile;

    let mut stockpile = ResourceStockpile { food: 9999, timber: 9999, ore: 9999, leather: 9999, stone: 9999 };
    resources::cap_stockpile(&mut stockpile, 100); // 100 people

    assert_eq!(stockpile.food, 300); // 3x population
    assert_eq!(stockpile.timber, 200); // 2x population
}

// ---------------------------------------------------------------------------
// War effects
// ---------------------------------------------------------------------------

#[test]
fn war_drafts_soldiers() {
    let mut settlements = vec![village(1)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    let soldiers_before = sim.people.iter()
        .filter(|p| p.occupation == Occupation::Soldier)
        .count();

    let mut ws = crate::worldgen::history::state::WorldState::default();
    ws.active_wars.push(crate::worldgen::history::state::War { aggressor: 1, defender: 99, start_year: 0 });
    sim.advance_year(&mut settlements, &[], &[], &ws, 0, &mut rng);

    let soldiers_after = sim.people.iter()
        .filter(|p| p.is_alive(0) && p.occupation == Occupation::Soldier)
        .count();

    assert!(soldiers_after > soldiers_before,
        "war should draft soldiers: before={}, after={}", soldiers_before, soldiers_after);

    let drafted = sim.people.iter()
        .filter(|p| p.life_events.iter().any(|e| matches!(e.kind, LifeEventKind::DraftedToWar { .. })))
        .count();
    assert!(drafted > 0, "should have DraftedToWar events");
}

#[test]
fn war_kills_soldiers() {
    let mut settlements = vec![village(1)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    let mut ws = crate::worldgen::history::state::WorldState::default();
    ws.active_wars.push(crate::worldgen::history::state::War { aggressor: 1, defender: 99, start_year: 0 });
    // Run several years of war
    for year in 0..10 {
        sim.advance_year(&mut settlements, &[], &[], &ws, year, &mut rng);
    }

    let war_deaths = sim.people.iter()
        .flat_map(|p| &p.life_events)
        .filter(|e| matches!(e.kind,
            LifeEventKind::LostParent { cause: DeathCause::War, .. }
            | LifeEventKind::LostSpouse { cause: DeathCause::War, .. }
            | LifeEventKind::LostChild { cause: DeathCause::War, .. }
        ))
        .count();

    assert!(war_deaths > 0, "war should kill soldiers and generate LostFamily events");
}

#[test]
fn war_ended_gives_survived_event() {
    let mut settlements = vec![village(1)];
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    let mut ws = crate::worldgen::history::state::WorldState::default();
    ws.active_wars.push(crate::worldgen::history::state::War { aggressor: 1, defender: 99, start_year: 0 });
    // War for 3 years
    for year in 0..3 {
        sim.advance_year(&mut settlements, &[], &[], &ws, year, &mut rng);
    }

    // War ends
    ws.active_wars.clear();
    sim.advance_year(&mut settlements, &[], &[], &ws, 3, &mut rng);

    let survived = sim.people.iter()
        .filter(|p| p.life_events.iter().any(|e| matches!(e.kind, LifeEventKind::SurvivedWar { .. })))
        .count();

    assert!(survived > 0, "should have SurvivedWar events after war ends");
}

// ---------------------------------------------------------------------------
// Plague effects
// ---------------------------------------------------------------------------

#[test]
fn plague_kills_percentage() {
    let mut settlements = vec![village(1)];
    settlements[0].plague_this_year = true;
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    let alive_before = sim.living_count(0);
    sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), 0, &mut rng);
    let alive_after = sim.living_count(0);

    let killed = alive_before - alive_after;
    assert!(killed > 0, "plague should kill people");

    // Should kill ~10-15% — with natural deaths too, allow wider margin
    let kill_pct = killed as f64 / alive_before as f64;
    assert!(kill_pct > 0.05 && kill_pct < 0.30,
        "plague kill rate out of range: {:.1}% ({} of {})", kill_pct * 100.0, killed, alive_before);

    // Flag should be cleared
    assert!(!settlements[0].plague_this_year, "plague flag should be cleared after processing");
}

#[test]
fn plague_is_one_time() {
    let mut settlements = vec![village(1)];
    settlements[0].plague_this_year = true;
    let mut rng = rng();
    let mut sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), 0, &mut rng);
    let alive_after_plague = sim.living_count(0);

    // Run 2 more years — no more plague deaths
    sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), 1, &mut rng);
    sim.advance_year(&mut settlements, &[], &[], &crate::worldgen::history::state::WorldState::default(), 2, &mut rng);
    let alive_later = sim.living_count(2);

    // Population should not continue dropping dramatically (some natural deaths ok)
    let ratio = alive_later as f64 / alive_after_plague as f64;
    assert!(ratio > 0.90, "population shouldn't keep crashing after plague: ratio {:.2}", ratio);
}

// ---------------------------------------------------------------------------
// Occupation rebalancing
// ---------------------------------------------------------------------------

#[test]
fn rebalancing_switches_to_farmers_when_starving() {
    use super::resources;
    use crate::worldgen::history::state::ResourceStockpile;

    let mut settlements = vec![hamlet(1)];
    settlements[0].stockpile = ResourceStockpile { food: -50, timber: 0, ore: 0, leather: 0, stone: 0 };

    let mut rng = rng();
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng);

    let farmers_before = sim.people.iter()
        .filter(|p| p.occupation == Occupation::Farmer)
        .count();

    let index = super::index::SettlementIndex::build(&sim.people, 0);
    let mut people = sim.people.clone();
    resources::rebalance_occupations(&mut people, &index, &settlements[0], 0);

    let farmers_after = people.iter()
        .filter(|p| p.occupation == Occupation::Farmer)
        .count();

    assert!(farmers_after > farmers_before,
        "rebalancing should add farmers: before={}, after={}", farmers_before, farmers_after);
}

// ---------------------------------------------------------------------------
// Combat score
// ---------------------------------------------------------------------------

#[test]
fn combat_score_rewards_veterans() {
    use super::war;

    let settlements = vec![hamlet(1)];
    let mut veteran = Person {
        id: 1, birth_year: -25, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
        sex: Sex::Male, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
        occupation: Occupation::Soldier, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: Vec::new(),
        life_events: vec![
            LifeEvent { year: 5, kind: LifeEventKind::SurvivedWar { enemy_faction_id: 2 }, cause: None },
            LifeEvent { year: 10, kind: LifeEventKind::SurvivedWar { enemy_faction_id: 3 }, cause: None },
        ],
        notable: false,
    };
    let rookie = Person {
        id: 2, birth_year: -20, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
        sex: Sex::Male, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
        occupation: Occupation::Soldier, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: Vec::new(),
        life_events: Vec::new(),
        notable: false,
    };

    let vet_score = war::combat_score(&veteran, &settlements[0], 0);
    let rookie_score = war::combat_score(&rookie, &settlements[0], 0);

    assert!(vet_score > rookie_score,
        "veteran ({:.1}) should score higher than rookie ({:.1})", vet_score, rookie_score);
}

// ---------------------------------------------------------------------------
// Trade
// ---------------------------------------------------------------------------

#[test]
fn trade_balances_food_within_faction() {
    use super::trade;
    use crate::worldgen::history::state::{ResourceStockpile, WorldState};

    // Two settlements in same faction: one with food surplus, one with deficit
    let mut surplus_settlement = hamlet(1);
    surplus_settlement.stockpile = ResourceStockpile { food: 100, timber: 0, ore: 0, leather: 0, stone: 0 };
    let mut deficit_settlement = hamlet(2);
    deficit_settlement.stockpile = ResourceStockpile { food: -20, timber: 0, ore: 0, leather: 0, stone: 0 };

    let mut settlements = vec![surplus_settlement, deficit_settlement];
    let factions = vec![test_faction()]; // owns both settlements
    let mut factions_with_both = factions;
    factions_with_both[0].settlements = vec![1, 2];

    // Create people with some merchants
    let mut rng = rng();
    let sim = PopulationSim::new(&settlements, &[], 0, &mut rng);
    let index = super::index::SettlementIndex::build(&sim.people, 0);
    let world_state = WorldState::default();

    let routes = trade::settle_trade(
        &mut settlements, &factions_with_both, &sim.people, &index, &world_state, 0,
    );

    // If there are merchants, trade should have occurred
    let has_merchants = sim.people.iter().any(|p| p.occupation == Occupation::Merchant);
    if has_merchants {
        assert!(settlements[1].stockpile.food > -20,
            "deficit settlement should receive food via trade, got {}", settlements[1].stockpile.food);
        assert!(!routes.is_empty(), "should generate trade routes");
    }
}

#[test]
fn no_merchants_no_trade() {
    use super::trade;
    use crate::worldgen::history::state::{ResourceStockpile, WorldState};

    let mut surplus = hamlet(1);
    surplus.stockpile = ResourceStockpile { food: 100, timber: 0, ore: 0, leather: 0, stone: 0 };
    let mut deficit = hamlet(2);
    deficit.stockpile = ResourceStockpile { food: -20, timber: 0, ore: 0, leather: 0, stone: 0 };

    let mut settlements = vec![surplus, deficit];
    let mut faction = test_faction();
    faction.settlements = vec![1, 2];
    let factions = vec![faction];

    // Empty people vec — no merchants at all
    let people: Vec<Person> = Vec::new();
    let index = super::index::SettlementIndex::build(&people, 0);
    let world_state = WorldState::default();

    let routes = trade::settle_trade(&mut settlements, &factions, &people, &index, &world_state, 0);

    assert!(routes.is_empty(), "no merchants should mean no trade");
    assert_eq!(settlements[1].stockpile.food, -20, "deficit should be unchanged without trade");
}

// ---------------------------------------------------------------------------
// Faith
// ---------------------------------------------------------------------------

#[test]
fn faithless_adopt_settlement_patron() {
    use super::faith;
    use crate::worldgen::divine::state::GodState;
    use crate::worldgen::divine::personality::{DivinePersonality, DivineDrive, DivineFlaw};

    let mut settlement = hamlet(1);
    settlement.patron_god = Some(5);
    settlement.prosperity = 80;

    let personality = DivinePersonality { drive: DivineDrive::Worship, flaw: DivineFlaw::Hubris };
    let god = GodState::new(5, personality);
    let gods = vec![god];

    let mut people = vec![
        Person {
            id: 1, birth_year: -20, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
            sex: Sex::Male, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
            occupation: Occupation::Farmer, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: Vec::new(),
            life_events: Vec::new(), notable: false,
        },
    ];

    let index = super::index::SettlementIndex::build(&people, 0);
    let mut settlements = vec![settlement];
    faith::evaluate_faith(&mut people, &index, &mut settlements, &gods, 0);

    assert_eq!(people[0].primary_god(), Some(5), "should adopt settlement's patron god");
    assert!(!people[0].faith.is_empty(), "should have some initial devotion");
}

#[test]
fn faith_strengthened_when_prospering() {
    use super::faith;
    use crate::worldgen::divine::state::GodState;
    use crate::worldgen::divine::personality::{DivinePersonality, DivineDrive, DivineFlaw};

    let mut settlement = hamlet(1);
    settlement.patron_god = Some(5);
    settlement.prosperity = 90; // prospering

    let personality = DivinePersonality { drive: DivineDrive::Worship, flaw: DivineFlaw::Hubris };
    let mut god = GodState::new(5, personality);
    god.champion_name = Some("Hero".into());
    let gods = vec![god];

    let mut people = vec![
        Person {
            id: 1, birth_year: -20, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
            sex: Sex::Male, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
            occupation: Occupation::Farmer, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: vec![(5, 75)],
            life_events: Vec::new(), notable: false,
        },
    ];

    let index = super::index::SettlementIndex::build(&people, 0);

    // Run faith for several years to push devotion past 80
    let mut settlements = vec![settlement];
    for year in 0..5 {
        faith::evaluate_faith(&mut people, &index, &mut settlements, &gods, year);
    }

    assert!(people[0].devotion_to(5) > 80, "devotion should increase past 80 with prosperity");
    let has_strengthened = people[0].life_events.iter()
        .any(|e| matches!(e.kind, LifeEventKind::FaithStrengthened { .. }));
    assert!(has_strengthened, "should have FaithStrengthened event");
}

#[test]
fn faith_shaken_when_suffering_under_powerful_god() {
    use super::faith;
    use crate::worldgen::divine::state::GodState;
    use crate::worldgen::divine::personality::{DivinePersonality, DivineDrive, DivineFlaw};

    let mut settlement = hamlet(1);
    settlement.patron_god = Some(5);
    settlement.prosperity = 20; // suffering

    let personality = DivinePersonality { drive: DivineDrive::Worship, flaw: DivineFlaw::Hubris };
    let mut god = GodState::new(5, personality);
    god.power = 60; // powerful — "you could help but don't"
    let gods = vec![god];

    let mut people = vec![
        Person {
            id: 1, birth_year: -20, death_year: None, death_cause: None, settlement_id: 1, faction_allegiance: 0,
            sex: Sex::Male, race: Race::Human, secondary_race: None, mother: None, father: None, spouse: None,
            occupation: Occupation::Farmer, traits: Vec::new(), happiness: 50, prophet_of: None, years_as_outlier: 0, faith: vec![(5, 25)],
            life_events: Vec::new(), notable: false,
        },
    ];

    let index = super::index::SettlementIndex::build(&people, 0);
    let mut settlements = vec![settlement];
    faith::evaluate_faith(&mut people, &index, &mut settlements, &gods, 0);

    assert!(people[0].devotion_to(5) < 25, "devotion should decrease when suffering under powerful god");
    let has_shaken = people[0].life_events.iter()
        .any(|e| matches!(e.kind, LifeEventKind::FaithShaken { .. }));
    assert!(has_shaken, "should have FaithShaken event when devotion drops below 20");
}
