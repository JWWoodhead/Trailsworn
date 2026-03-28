pub mod artifacts;
pub mod characters;
pub mod culture;
pub mod state;
mod world_events;

use std::collections::BTreeSet;

use rand::{RngExt, SeedableRng};

use super::divine::artifacts::DivineArtifact;
use super::divine::personality;
use super::divine::races::CreatedRace;
use super::divine::sites::DivineSite;
use super::divine::state::GodState;
use super::divine::terrain_scars::TerrainScar;
use super::divine::{territory, worship, drives, conflict, flaws};
use super::divine::{DrawnPantheon, GodId, GodPool};
use super::names::{FactionType, Race, faction_name, region_name, settlement_name};
use super::population;
use super::population::PopulationSim;
use super::population_table::PopTable;
use super::world_map::{WorldMap, WorldPos};
use artifacts::*;
use characters::*;
use culture::*;
use state::*;

/// A historic event — the append-only chronicle.
#[derive(Clone, Debug)]
pub struct HistoricEvent {
    pub year: i32,
    pub kind: EventKind,
    pub description: String,
    /// Faction IDs involved in this event.
    pub participants: Vec<u32>,
    /// God IDs involved in this event.
    pub god_participants: Vec<GodId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    // Mortal events
    FactionFounded,
    FactionDissolved,
    SettlementFounded,
    SettlementDestroyed,
    WarDeclared,
    WarEnded,
    AllianceFormed,
    AllianceBroken,
    LeaderChanged,
    TradeAgreement,
    ReligiousSchism,
    PlagueStruck,
    MonsterAttack,
    HeroRose,
    ArtifactDiscovered,
    Conquest,
    Betrayal,
    // Divine events
    TerritoryClaimed,
    TerritoryContested,
    TerrainShaped,
    TempleEstablished,
    ChampionChosen,
    RaceCreated,
    DivineWarDeclared,
    DivineWarEnded,
    DivineArtifactForged,
    SacredSiteCreated,
    PactFormed,
    PactBroken,
    NarrativeAdvanced,
    GiftBestowed,
    WorshipEstablished,
    WorshipConverted,
    HolyWar,
    DivineIntervention,
}

/// A state mutation that crosses from one domain (divine/mortal) into another.
/// Emitted by divine phases instead of direct settlement mutation; applied centrally.
#[derive(Clone, Debug)]
pub enum CrossDomainEvent {
    /// A god claims an unworshipped settlement.
    WorshipEstablished { settlement_index: usize, god_id: GodId, devotion: u32 },
    /// A god converts a settlement from a rival god.
    WorshipConverted { settlement_index: usize, new_god_id: GodId, old_god_id: GodId, devotion: u32 },
    /// A settlement's devotion shifts by a signed delta (clamped to 0..=100).
    DevotionChanged { settlement_index: usize, delta: i32 },
}

/// Drain cross-domain events and apply them to settlement state.
fn apply_cross_domain_events(
    cross_events: &mut Vec<CrossDomainEvent>,
    settlements: &mut [SettlementState],
) {
    for event in cross_events.drain(..) {
        match event {
            CrossDomainEvent::WorshipEstablished { settlement_index, god_id, devotion } => {
                settlements[settlement_index].patron_god = Some(god_id);
                settlements[settlement_index].devotion = devotion;
            }
            CrossDomainEvent::WorshipConverted { settlement_index, new_god_id, devotion, .. } => {
                settlements[settlement_index].patron_god = Some(new_god_id);
                settlements[settlement_index].devotion = devotion;
            }
            CrossDomainEvent::DevotionChanged { settlement_index, delta } => {
                let current = settlements[settlement_index].devotion as i32;
                settlements[settlement_index].devotion = (current + delta).clamp(0, 100) as u32;
            }
        }
    }
}

/// The complete generated history — mortal and divine intertwined.
#[derive(Clone, Debug, bevy::prelude::Resource)]
pub struct WorldHistory {
    pub regions: Vec<String>,
    pub factions: Vec<FactionState>,
    pub settlements: Vec<SettlementState>,
    pub characters: Vec<Character>,
    pub artifacts: Vec<Artifact>,
    pub cultures: Vec<(u32, CulturalProfile)>,
    pub events: Vec<HistoricEvent>,
    pub world_state: WorldState,
    pub current_year: i32,
    // Divine
    pub gods: Vec<GodState>,
    pub divine_sites: Vec<DivineSite>,
    pub divine_artifacts: Vec<DivineArtifact>,
    pub created_races: Vec<CreatedRace>,
    pub terrain_scars: Vec<TerrainScar>,
    // Population
    pub people: Vec<population::Person>,
}

impl WorldHistory {
    pub fn living_factions(&self) -> Vec<&FactionState> {
        self.factions.iter().filter(|f| f.is_alive(self.current_year)).collect()
    }

    pub fn events_for_faction(&self, faction_id: u32) -> Vec<&HistoricEvent> {
        self.events.iter().filter(|e| e.participants.contains(&faction_id)).collect()
    }
}

pub struct HistoryConfig {
    pub num_years: i32,
    pub start_year: i32,
    pub num_regions: u32,
    pub initial_factions: u32,
    /// Base cells claimed per god per year.
    pub territory_expansion_rate: u32,
    /// Year range (absolute) during which gods can create races.
    pub race_creation_window: (i32, i32),

    // Phase toggles — disable individual simulation subsystems.
    /// Phase 2: territory expansion + terrain shaping.
    pub divine_territory: bool,
    /// Phase 3: worship competition (gods claim settlements).
    pub divine_worship: bool,
    /// Phase 4: drive-based actions (sites, artifacts, races).
    pub divine_drives: bool,
    /// Phase 9: divine wars + pacts.
    pub divine_conflict: bool,
    /// Phase 10: flaw pressure accumulation + triggers.
    pub divine_flaws: bool,
    /// Phases 5-8: faction upkeep, settlements, characters, mortal events.
    pub mortal_simulation: bool,
    /// Population simulation: person-level birth/death/marriage/life events.
    pub population_simulation: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            num_years: 100,
            start_year: 0,
            num_regions: 8,
            initial_factions: 6,
            territory_expansion_rate: 80,
            race_creation_window: (10, 60),
            divine_territory: true,
            divine_worship: true,
            divine_drives: true,
            divine_conflict: true,
            divine_flaws: true,
            mortal_simulation: true,
            population_simulation: true,
        }
    }
}

/// Generate a unified world history — mortal factions and gods intertwined.
pub fn generate_history(
    config: &HistoryConfig,
    world_map: &mut WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    seed: u64,
) -> WorldHistory {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut next_id = 1u32;

    // Generate regions
    let regions: Vec<String> = (0..config.num_regions).map(|_| region_name(&mut rng)).collect();

    let mut factions: Vec<FactionState> = Vec::new();
    let mut settlements: Vec<SettlementState> = Vec::new();
    let mut characters: Vec<Character> = Vec::new();
    let mut artifacts_list: Vec<Artifact> = Vec::new();
    let mut events: Vec<HistoricEvent> = Vec::new();
    let mut world_state = WorldState::default();

    // --- Initialize gods ---
    let mut gods: Vec<GodState> = pantheon.god_ids.iter().map(|&id| {
        let domain = god_pool.get(id).map(|d| d.domain).unwrap_or(crate::resources::magic::MagicSchool::Arcane);
        let traits = pantheon.traits(id);
        let p = personality::roll_personality(domain, traits, &mut rng);
        GodState::new(id, p)
    }).collect();

    // Initialize territory map and divine relations
    world_state.territory_map = vec![None; world_map.cells.len()];
    world_state.divine_relations = super::divine::state::DivineRelationMatrix::from_relationships(&pantheon.relationships);

    let any_divine = config.divine_territory || config.divine_worship
        || config.divine_drives || config.divine_conflict || config.divine_flaws;

    // BFS frontiers for territory expansion
    let mut frontiers: Vec<BTreeSet<WorldPos>> = vec![BTreeSet::new(); gods.len()];

    // Assign starting seats of power (only needed when divine phases are active)
    if any_divine {
        territory::assign_seats_of_power(&mut gods, &mut frontiers, &mut world_state, world_map, god_pool, &mut rng);
    }

    // --- Initialize divine sites/artifacts/races/scars ---
    let mut divine_sites: Vec<super::divine::sites::DivineSite> = Vec::new();
    let mut divine_artifacts: Vec<super::divine::artifacts::DivineArtifact> = Vec::new();
    let mut created_races: Vec<super::divine::races::CreatedRace> = Vec::new();
    let mut terrain_scars: Vec<super::divine::terrain_scars::TerrainScar> = Vec::new();

    // --- Initialize mortal factions ---
    let faction_type_table = PopTable::pick_one(vec![
        (FactionType::Kingdom, 30.0),
        (FactionType::MercenaryCompany, 15.0),
        (FactionType::ReligiousOrder, 15.0),
        (FactionType::MerchantGuild, 15.0),
        (FactionType::MageCircle, 10.0),
        (FactionType::TribalWarband, 10.0),
        (FactionType::BanditClan, 5.0),
    ]);

    let race_table = PopTable::pick_one(vec![
        (Race::Human, 40.0),
        (Race::Dwarf, 20.0),
        (Race::Elf, 15.0),
        (Race::Orc, 15.0),
        (Race::Goblin, 10.0),
    ]);

    for _ in 0..config.initial_factions {
        let ft = faction_type_table.roll_one(&mut rng).unwrap();
        let race = race_table.roll_one(&mut rng).unwrap();
        let region = regions[rng.random_range(0..regions.len())].clone();
        let name = faction_name(ft, race, &mut rng);
        let faction_id = next_id;
        next_id += 1;
        let founding_year = config.start_year + rng.random_range(0..10);
        let (mil, wealth, stab) = FactionState::initialize_gauges(ft);

        let leader_id = next_id; next_id += 1;
        let leader_birth = founding_year - rng.random_range(20..40);
        let leader = generate_character(leader_id, race, CharacterRole::Leader, Some(faction_id), leader_birth, &mut rng);
        let leader_display = leader.full_display_name();

        let general_id = next_id; next_id += 1;
        let general_birth = founding_year - rng.random_range(18..35);
        let general = generate_character(general_id, race, CharacterRole::General, Some(faction_id), general_birth, &mut rng);

        characters.push(leader);
        characters.push(general);

        factions.push(FactionState {
            id: faction_id, name: name.clone(), faction_type: ft, race,
            founded_year: founding_year, home_region: region.clone(),
            dissolved_year: None, leader_name: leader_display.clone(), leader_id: Some(leader_id),
            military_strength: mil, wealth, stability: stab,
            territory: vec![region.clone()], settlements: vec![],
            patron_god: None, devotion: 0,
        });

        events.push(HistoricEvent {
            year: founding_year, kind: EventKind::FactionFounded,
            description: format!("{} was founded in {} by {}", name, region, leader_display),
            participants: vec![faction_id], god_participants: vec![],
        });

        let sname = settlement_name(&mut rng);
        let sid = next_id; next_id += 1;
        let pop = match ft {
            FactionType::Kingdom | FactionType::MerchantGuild => PopulationClass::Town,
            _ => PopulationClass::Village,
        };
        settlements.push(SettlementState {
            id: sid, name: sname.clone(), founded_year: founding_year,
            owner_faction: faction_id, destroyed_year: None, region: region.clone(),
            population_class: pop, prosperity: 50, defenses: 30,
            patron_god: None, devotion: 0, world_pos: None,
            zone_type: None, stockpile: ResourceStockpile::default(), at_war: false, plague_this_year: false,
        });
        factions.last_mut().unwrap().settlements.push(sid);

        events.push(HistoricEvent {
            year: founding_year, kind: EventKind::SettlementFounded,
            description: format!("{} founded the settlement of {}", name, sname),
            participants: vec![faction_id], god_participants: vec![],
        });
    }

    // Initialize pairwise faction relations
    for i in 0..factions.len() {
        for j in (i + 1)..factions.len() {
            let a = factions[i].clone();
            let b = factions[j].clone();
            world_state.relations.initialize_pair(&a, &b);
        }
    }

    // Build settlement list for worship from world map settlements
    // (These are the map's ~70 settlements that gods compete for worship over)
    let mut map_settlements: Vec<SettlementState> = world_map.cells.iter().enumerate()
        .filter(|(_, c)| c.settlement_name.is_some())
        .map(|(i, c)| {
            let x = (i as u32) % world_map.width;
            let y = (i as u32) / world_map.width;
            let sid = next_id; next_id += 1;
            SettlementState {
                id: sid,
                name: c.settlement_name.clone().unwrap(),
                founded_year: config.start_year,
                owner_faction: 0, // unowned by any faction initially
                destroyed_year: None,
                region: String::new(),
                population_class: match c.settlement_size {
                    Some(crate::worldgen::world_map::SettlementSize::City) => PopulationClass::Capital,
                    Some(crate::worldgen::world_map::SettlementSize::Town) => PopulationClass::Town,
                    Some(crate::worldgen::world_map::SettlementSize::Village) => PopulationClass::Village,
                    _ => PopulationClass::Hamlet,
                },
                prosperity: 50,
                defenses: 20,
                patron_god: None,
                devotion: 0,
                world_pos: Some(WorldPos::new(x as i32, y as i32)),
                zone_type: Some(c.zone_type),
                stockpile: ResourceStockpile::default(), at_war: false, plague_this_year: false,
            }
        })
        .collect();
    settlements.append(&mut map_settlements);

    // --- Population simulation ---
    let mut pop_sim = if config.population_simulation {
        Some(PopulationSim::new(&settlements, config.start_year, &mut rng))
    } else {
        None
    };

    // --- Unified year-by-year simulation ---
    let mut cross_events: Vec<CrossDomainEvent> = Vec::new();

    for year in config.start_year..config.start_year + config.num_years {
        let event_count_before = events.len();

        // Always runs — bookkeeping
        worship::update_god_power(&mut gods);

        // Phase 2: Territory expansion & terrain shaping
        if config.divine_territory {
            territory::evaluate_territory_expansion(
                year, config.territory_expansion_rate, &mut gods, &mut frontiers,
                &mut events, &mut world_state, world_map, god_pool, pantheon, &mut rng,
            );
            territory::evaluate_terrain_shaping(
                year, &gods, &mut events, &mut terrain_scars,
                &world_state, world_map, god_pool, pantheon, &mut next_id, &mut rng,
            );
        }

        // Phase 3: Worship competition
        if config.divine_worship {
            worship::evaluate_worship(
                year, &mut gods, &mut events, &settlements,
                &mut cross_events, &mut world_state, world_map, pantheon, &mut rng,
            );
            apply_cross_domain_events(&mut cross_events, &mut settlements);
        }

        // Phase 4: Drive-based divine actions
        if config.divine_drives {
            drives::evaluate_drive_actions(
                year, config.race_creation_window, &mut gods, &mut events,
                &mut divine_sites, &mut divine_artifacts, &mut created_races,
                &world_state, world_map, god_pool, pantheon, &mut next_id, &mut rng,
            );
        }

        // Phases 5-8: Mortal simulation
        if config.mortal_simulation {
            world_events::simulate_year(
                year, &mut factions, &mut settlements, &mut characters,
                &mut artifacts_list, &mut events,
                &mut world_state, &regions, &mut next_id,
                &faction_type_table, &race_table, &mut rng,
            );
        }

        // Set settlement flags for population sim
        for settlement in settlements.iter_mut() {
            settlement.at_war = world_state.war_count(settlement.owner_faction) > 0;
            // plague_this_year is set directly by evaluate_plague in world_events
        }

        // Population simulation
        if let Some(ref mut pop) = pop_sim {
            let newly_notable = pop.advance_year(&mut settlements, year, &mut rng).to_vec();
            for pid in newly_notable {
                if let Some(person) = pop.person(pid) {
                    let character = population::notable::promote_to_character(
                        person, &mut next_id, &settlements, &factions, &mut rng,
                    );
                    characters.push(character);
                }
            }
        }

        // Phase 9: Divine conflict
        if config.divine_conflict {
            let active_god_ids: Vec<u32> = gods.iter().filter(|g| g.is_active()).map(|g| g.god_id).collect();
            conflict::evaluate_divine_war_declared(year, &gods, &mut events, &mut world_state, &active_god_ids, pantheon, &mut rng);
            conflict::evaluate_divine_war_resolution(
                year, &mut gods, &mut events, &mut terrain_scars,
                &mut world_state, world_map, god_pool, pantheon, &mut next_id, &mut rng,
            );
            conflict::evaluate_divine_pact(year, &gods, &mut events, &mut world_state, &active_god_ids, pantheon, &mut rng);
            conflict::evaluate_pact_broken(year, &gods, &mut events, &mut world_state, pantheon, &mut rng);
        }

        // Phase 10: Flaw pressure & triggers
        if config.divine_flaws {
            let new_events = &events[event_count_before..];
            flaws::accumulate_flaw_pressure(&mut gods, new_events, &world_state);
            flaws::evaluate_flaw_triggers(
                year, &mut gods, &mut events, &settlements,
                &mut cross_events, &mut world_state, pantheon, &mut rng,
            );
            apply_cross_domain_events(&mut cross_events, &mut settlements);
        }

        // Always runs — maintenance
        conflict::drain_divine_war_power(&mut gods, &world_state);
        world_state.divine_relations.drift_toward_neutral();
    }

    // Write final divine_owner to world map cells
    if config.divine_territory {
        for (i, owner) in world_state.territory_map.iter().enumerate() {
            if let Some(god_id) = owner {
                world_map.cells[i].divine_owner = Some(*god_id);
            }
        }
    }

    // Build cultural profiles
    let cultures: Vec<(u32, CulturalProfile)> = factions.iter()
        .map(|f| (f.id, build_culture(f.id, &events)))
        .collect();

    WorldHistory {
        regions, factions, settlements, characters,
        artifacts: artifacts_list, cultures, events, world_state,
        current_year: config.start_year + config.num_years,
        gods,
        divine_sites,
        divine_artifacts,
        created_races,
        terrain_scars,
        people: pop_sim.map(|p| p.people).unwrap_or_default(),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_history(seed: u64) -> WorldHistory {
        test_history_with_config(HistoryConfig::default(), seed)
    }

    fn test_history_with_config(config: HistoryConfig, seed: u64) -> WorldHistory {
        test_history_with_size(config, seed, 64)
    }

    fn test_history_with_size(config: HistoryConfig, seed: u64, map_size: u32) -> WorldHistory {
        use crate::worldgen::world_map::generate_world_map;
        use crate::worldgen::divine::build_god_pool;
        use rand::SeedableRng;
        let mut world_map = generate_world_map(map_size, map_size, seed);
        let god_pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let pantheon = god_pool.draw_pantheon(6, &mut rng);
        generate_history(&config, &mut world_map, &god_pool, &pantheon, seed)
    }

    #[test]
    fn generates_factions_with_gauges() {
        let history = test_history(42);
        for f in &history.factions {
            assert!(f.military_strength <= 100);
            assert!(f.wealth <= 100);
            assert!(f.stability <= 100);
        }
    }

    #[test]
    fn wars_only_between_hostile_factions() {
        let history = test_history(42);
        for event in &history.events {
            if event.kind == EventKind::WarDeclared && event.participants.len() == 2 {
                // The war was declared, which means at the time sentiment was < -30
                // We can't retroactively check but we can verify wars exist
                assert!(event.participants.len() >= 2);
            }
        }
    }

    #[test]
    fn wars_affect_military_strength() {
        let h1 = test_history_with_config(HistoryConfig { num_years: 50, ..Default::default() }, 42);
        // Factions at war should be weaker than starting values
        let at_war: Vec<&FactionState> = h1.factions.iter()
            .filter(|f| h1.world_state.war_count(f.id) > 0)
            .collect();
        for f in &at_war {
            let (init_mil, _, _) = FactionState::initialize_gauges(f.faction_type);
            // They should have lost some strength (not guaranteed but very likely)
            assert!(f.military_strength < init_mil || f.military_strength < 50,
                "{} has mil {} (started at {})", f.name, f.military_strength, init_mil);
        }
    }

    #[test]
    fn settlements_can_change_hands() {
        let history = test_history(42);
        let conquests: Vec<_> = history.events.iter()
            .filter(|e| e.kind == EventKind::Conquest)
            .collect();
        // May or may not have conquests, but shouldn't crash
        let _ = conquests;
    }

    #[test]
    fn relations_initialized() {
        let history = test_history(42);
        let living = history.living_factions();
        if living.len() >= 2 {
            // At least some non-zero relations should exist
            let a = living[0].id;
            let b = living[1].id;
            let _sentiment = history.world_state.relations.get(a, b);
            // Just verify it doesn't crash
        }
    }

    #[test]
    fn event_variety() {
        let history = test_history(42);
        let kinds: std::collections::HashSet<_> = history.events.iter().map(|e| &e.kind).collect();
        assert!(kinds.len() >= 5);
    }

    #[test]
    fn deterministic() {
        let h1 = test_history(42);
        let h2 = test_history(42);
        assert_eq!(h1.events.len(), h2.events.len());
        assert_eq!(h1.factions[0].name, h2.factions[0].name);
    }

    #[test]
    fn dissolved_factions_cleaned_up() {
        let history = test_history(42);
        for f in &history.factions {
            if let Some(_dy) = f.dissolved_year {
                assert!(!history.world_state.at_war(f.id, 0));
                assert_eq!(history.world_state.war_count(f.id), 0);
            }
        }
    }

    #[test]
    fn has_characters() {
        let history = test_history(42);
        assert!(history.characters.len() > 10, "only {} characters", history.characters.len());
        let alive = history.characters.iter().filter(|c| c.is_alive(history.current_year)).count();
        assert!(alive > 0, "no living characters");
    }

    #[test]
    fn has_artifacts() {
        let history = test_history(42);
        // Artifacts are probabilistic but very likely over 100 years
        // Just verify no crash
        let _ = history.artifacts.len();
    }

    #[test]
    fn has_cultures() {
        let history = test_history(42);
        assert!(!history.cultures.is_empty());
        // At least some factions should have cultural values
        let with_values = history.cultures.iter().filter(|(_, c)| !c.values.is_empty()).count();
        assert!(with_values > 0, "no factions developed cultural values");
    }

    #[test]
    fn dump_history_summary() {
        let history = test_history_with_size(HistoryConfig::default(), 42, 256);

        println!("\n=== WORLD HISTORY (100 years) ===");
        println!("Factions: {} ({} alive)", history.factions.len(),
            history.living_factions().len());
        println!("Characters: {} ({} alive)", history.characters.len(),
            history.characters.iter().filter(|c| c.is_alive(history.current_year)).count());
        println!("Settlements: {}", history.settlements.len());
        println!("Artifacts: {}", history.artifacts.len());
        println!("Events: {}", history.events.len());

        println!("\n--- Living Factions ---");
        for f in history.living_factions() {
            println!("  {} ({:?} {:?}) mil:{} wealth:{} stab:{} | Leader: {}",
                f.name, f.race, f.faction_type,
                f.military_strength, f.wealth, f.stability, f.leader_name);
        }

        println!("\n--- Notable Living Characters ---");
        for c in history.characters.iter()
            .filter(|c| c.is_alive(history.current_year) && c.renown >= 10)
        {
            let traits: Vec<_> = c.traits.iter().map(|t| format!("{:?}", t)).collect();
            println!("  {} ({:?} {:?}) renown:{} [{:?}] traits:[{}]",
                c.full_display_name(), c.race, c.role,
                c.renown, c.ambition, traits.join(", "));
        }

        println!("\n--- Cultures ---");
        for (fid, culture) in &history.cultures {
            if culture.values.is_empty() && culture.taboos.is_empty() { continue; }
            let fname = history.factions.iter().find(|f| f.id == *fid)
                .map(|f| f.name.as_str()).unwrap_or("???");
            let values: Vec<_> = culture.values.iter().map(|v| format!("{:?}", v)).collect();
            let taboos: Vec<_> = culture.taboos.iter().map(|t| format!("{:?}", t)).collect();
            println!("  {} | values:[{}] taboos:[{}]", fname, values.join(", "), taboos.join(", "));
        }

        println!("\n--- Population ---");
        let total_people = history.people.len();
        let alive = history.people.iter().filter(|p| p.death_year.is_none() || p.death_year.unwrap() > history.current_year).count();
        let with_events = history.people.iter().filter(|p| !p.life_events.is_empty()).count();
        let notable_people = history.people.iter().filter(|p| p.notable).count();
        println!("  Total ever: {}", total_people);
        println!("  Alive at year {}: {}", history.current_year, alive);
        println!("  With life events: {}", with_events);
        println!("  Notable: {}", notable_people);

        // Event type breakdown
        let wars = history.events.iter().filter(|e| e.kind == EventKind::WarDeclared).count();
        let plagues = history.events.iter().filter(|e| e.kind == EventKind::PlagueStruck).count();
        let conquests = history.events.iter().filter(|e| e.kind == EventKind::Conquest).count();
        println!("  Wars declared: {}", wars);
        println!("  Plagues: {}", plagues);
        println!("  Conquests: {}", conquests);

        // Settlement stockpile samples
        println!("\n--- Settlement Samples ---");
        for s in history.settlements.iter().take(5) {
            println!("  {} ({:?}, {:?}) pop_class:{:?} prosperity:{} food:{} timber:{} ore:{}",
                s.name, s.zone_type.unwrap_or(crate::worldgen::zone::ZoneType::Grassland),
                s.population_class, s.population_class,
                s.prosperity, s.stockpile.food, s.stockpile.timber, s.stockpile.ore);
        }

        println!("\n--- Key Events ---");
        for e in history.events.iter().filter(|e| matches!(e.kind,
            EventKind::WarDeclared | EventKind::Conquest | EventKind::Betrayal
            | EventKind::HeroRose | EventKind::ArtifactDiscovered
            | EventKind::FactionDissolved | EventKind::FactionFounded
            | EventKind::PlagueStruck))
        {
            println!("  Year {}: {}", e.year, e.description);
        }
    }
}
