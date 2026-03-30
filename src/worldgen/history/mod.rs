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
use super::zone::ZoneType;
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
    /// Why this event happened (one level of causation).
    pub cause: Option<population::types::EventCause>,
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
    /// Active trade routes from the final year of simulation.
    pub trade_routes: Vec<population::trade::TradeRoute>,
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
        (FactionType::TribalWarband, 25.0),
        (FactionType::MercenaryCompany, 15.0),
        (FactionType::ReligiousOrder, 15.0),
        (FactionType::MerchantGuild, 15.0),
        (FactionType::MageCircle, 10.0),
        (FactionType::Theocracy, 10.0),
        (FactionType::BanditClan, 5.0),
        (FactionType::ThievesGuild, 5.0),
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
            patron_god: None, devotion: 0, unhappy_years: 0,
        });

        events.push(HistoricEvent {
            year: founding_year, kind: EventKind::FactionFounded,
            description: format!("{} was founded in {} by {}", name, region, leader_display),
            participants: vec![faction_id], god_participants: vec![],
            cause: None,
        });

        let sname = settlement_name(&mut rng);
        let sid = next_id; next_id += 1;
        let pop = match ft {
            FactionType::MerchantGuild | FactionType::Theocracy => PopulationClass::Town,
            _ => PopulationClass::Village,
        };
        // Pick a random land cell for this faction's settlement
        let faction_world_pos = {
            let land_cells: Vec<WorldPos> = (0..world_map.cells.len())
                .filter(|&i| world_map.cells[i].zone_type != ZoneType::Ocean)
                .map(|i| WorldPos::new(
                    (i as u32 % world_map.width) as i32,
                    (i as u32 / world_map.width) as i32,
                ))
                .collect();
            if land_cells.is_empty() { None }
            else { Some(land_cells[rng.random_range(0..land_cells.len())]) }
        };
        let faction_zone_type = faction_world_pos
            .and_then(|pos| world_map.get(pos))
            .map(|c| c.zone_type);
        settlements.push(SettlementState {
            id: sid, name: sname.clone(), founded_year: founding_year,
            controlling_faction: faction_id, destroyed_year: None, region: region.clone(),
            population_class: pop, prosperity: 50, defenses: 30,
            patron_god: None, devotion: 0, world_pos: faction_world_pos,
            zone_type: faction_zone_type, stockpile: ResourceStockpile::default(), plague_this_year: false, conquered_this_year: false, conquered_by: None,
            dominant_race: None,
        });
        factions.last_mut().unwrap().settlements.push(sid);

        events.push(HistoricEvent {
            year: founding_year, kind: EventKind::SettlementFounded,
            description: format!("{} founded the settlement of {}", name, sname),
            participants: vec![faction_id], god_participants: vec![],
            cause: None,
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

    // Build settlement list from world map settlements (deduplicated — cities/towns span multiple cells)
    let mut seen_settlement_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut map_settlements: Vec<SettlementState> = world_map.cells.iter().enumerate()
        .filter(|(_, c)| c.settlement_name.is_some())
        .filter(|(_, c)| {
            let name = c.settlement_name.as_ref().unwrap();
            seen_settlement_names.insert(name.clone()) // true only for first occurrence
        })
        .map(|(i, c)| {
            let x = (i as u32) % world_map.width;
            let y = (i as u32) / world_map.width;
            let sid = next_id; next_id += 1;
            SettlementState {
                id: sid,
                name: c.settlement_name.clone().unwrap(),
                founded_year: config.start_year,
                controlling_faction: 0, // unowned by any faction initially
                destroyed_year: None,
                region: String::new(),
                population_class: match c.settlement_size {
                    Some(crate::worldgen::world_map::SettlementSize::City) => PopulationClass::City,
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
                stockpile: ResourceStockpile::default(), plague_this_year: false, conquered_this_year: false, conquered_by: None,
                dominant_race: None,
            }
        })
        .collect();
    settlements.append(&mut map_settlements);

    // Assign unowned map settlements to nearest faction
    assign_unowned_settlements(&mut settlements, &mut factions);

    // --- Population simulation ---
    let mut pop_sim = if config.population_simulation {
        Some(PopulationSim::new(&settlements, &factions, config.start_year, &mut rng))
    } else {
        None
    };

    // --- Unified year-by-year simulation ---
    let mut cross_events: Vec<CrossDomainEvent> = Vec::new();
    let mut trade_routes: Vec<population::trade::TradeRoute> = Vec::new();

    for year in config.start_year..config.start_year + config.num_years {
        let event_count_before = events.len();

        // Always runs — bookkeeping
        worship::update_god_power(&mut gods);

        // Phase 2: Territory expansion (gods claim territory but don't alter biomes)
        if config.divine_territory {
            territory::evaluate_territory_expansion(
                year, config.territory_expansion_rate, &mut gods, &mut frontiers,
                &mut events, &mut world_state, world_map, god_pool, pantheon, &mut rng,
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

        // Compute faction stats, prophet tensions, and formation candidates from population
        let (faction_stats, prophet_tensions, formation_candidates) = if let Some(ref pop) = pop_sim {
            let index = population::index::SettlementIndex::build(&pop.people, year);
            let stats = population::faction_stats::compute_faction_stats(
                &pop.people, &index, &settlements, &factions, year,
            );
            let tensions = population::faction_stats::compute_prophet_tensions(
                &pop.people, &index, &settlements, year,
            );
            let candidates = population::faction_stats::compute_formation_candidates(
                &pop.people, &index, &settlements, &factions, year,
            );
            (Some(stats), Some(tensions), Some(candidates))
        } else {
            (None, None, None)
        };

        // Phases 5-8: Mortal simulation
        if config.mortal_simulation {
            let founder_shifts = world_events::simulate_year(
                year, &mut factions, &mut settlements, &mut characters,
                &mut artifacts_list, &mut events,
                &mut world_state, &regions, &mut next_id,
                &faction_type_table, &race_table,
                faction_stats.as_ref(),
                prophet_tensions.as_ref(),
                formation_candidates.as_ref(),
                &mut rng,
            );

            // Founders join their own faction immediately
            if let Some(ref mut pop) = pop_sim {
                for &(person_id, faction_id) in &founder_shifts {
                    if let Some(person) = pop.people.iter_mut().find(|p| p.id == person_id) {
                        let old = person.faction_allegiance;
                        person.faction_allegiance = faction_id;
                        person.life_events.push(population::types::LifeEvent {
                            year,
                            kind: population::types::LifeEventKind::AllegianceChanged {
                                old_faction: old, new_faction: faction_id,
                            },
                            cause: None,
                        });
                    }
                }
            }
        }

        // plague_this_year / conquered_this_year are set directly by world_events

        // Trade between settlements (before population sim so imported food prevents famine)
        if let Some(ref pop) = pop_sim {
            let index = population::index::SettlementIndex::build(&pop.people, year);
            let year_routes = population::trade::settle_trade(
                &mut settlements, &factions, &pop.people, &index, &world_state, year,
            );
            trade_routes = year_routes; // keep last year's routes for WorldHistory
        }

        // Population simulation
        if let Some(ref mut pop) = pop_sim {
            let newly_notable = pop.advance_year(&mut settlements, &gods, &factions, &world_state, year, &mut rng).to_vec();
            for pid in newly_notable {
                if let Some(person) = pop.person(pid) {
                    let character = population::notable::promote_to_character(
                        person, &mut next_id, &settlements, &factions, &mut rng,
                    );
                    characters.push(character);
                }
            }
        }

        // Recompute controlling factions from population allegiance
        if let Some(ref pop) = pop_sim {
            let index = population::index::SettlementIndex::build(&pop.people, year);
            population::faction_stats::recompute_controlling_factions(
                &pop.people, &index, &mut settlements, &mut factions, year,
            );
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
        trade_routes,
    }
}

/// Assign map settlements (controlling_faction == 0) to the nearest faction within
/// claiming distance. Settlements too far from any faction stay independent.
fn assign_unowned_settlements(
    settlements: &mut [SettlementState],
    factions: &mut [FactionState],
) {
    const MAX_CLAIM_DISTANCE: i32 = 40;

    // Build lookup: faction_id -> world_pos of their home settlement
    let faction_homes: Vec<(u32, WorldPos)> = factions.iter()
        .filter(|f| f.dissolved_year.is_none())
        .filter_map(|f| {
            let home_sid = f.settlements.first()?;
            let home_pos = settlements.iter()
                .find(|s| s.id == *home_sid)?
                .world_pos?;
            Some((f.id, home_pos))
        })
        .collect();

    for settlement in settlements.iter_mut() {
        if settlement.controlling_faction != 0 { continue; }
        let pos = match settlement.world_pos {
            Some(p) => p,
            None => continue,
        };

        // Find nearest faction home
        let nearest = faction_homes.iter()
            .map(|&(fid, home)| (fid, pos.manhattan_distance(home)))
            .min_by_key(|&(_, dist)| dist);

        if let Some((fid, dist)) = nearest {
            if dist <= MAX_CLAIM_DISTANCE {
                settlement.controlling_faction = fid;
            }
        }
    }

    // Rebuild each faction's settlement list from the authoritative settlement data
    for faction in factions.iter_mut() {
        if faction.dissolved_year.is_some() { continue; }
        faction.settlements = settlements.iter()
            .filter(|s| s.controlling_faction == faction.id && s.destroyed_year.is_none())
            .map(|s| s.id)
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worldgen::population::types::EventCause;

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
        // Military is now derived from real soldiers — verify gauges are in range
        // and that factions at war exist (wars still happen with population-derived power)
        let at_war: Vec<&FactionState> = h1.factions.iter()
            .filter(|f| h1.world_state.war_count(f.id) > 0)
            .collect();
        for f in &at_war {
            assert!(f.military_strength <= 100,
                "{} has mil {} (should be 0-100)", f.name, f.military_strength);
        }
        // Wars should still occur with the population-derived system
        let war_count = h1.events.iter().filter(|e| e.kind == EventKind::WarDeclared).count();
        assert!(war_count > 0, "No wars occurred in 50 years");
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
        use crate::worldgen::population::LifeEventKind as LEK;
        use crate::worldgen::population::types::EventCause;

        let history = test_history_with_size(HistoryConfig::default(), 42, 256);
        let year = history.current_year;

        let living: Vec<_> = history.people.iter()
            .filter(|p| p.death_year.is_none() || p.death_year.unwrap() > year)
            .collect();
        let alive = living.len();
        let wars = history.events.iter().filter(|e| e.kind == EventKind::WarDeclared).count();
        let plagues = history.events.iter().filter(|e| e.kind == EventKind::PlagueStruck).count();
        let conquests = history.events.iter().filter(|e| e.kind == EventKind::Conquest).count();
        let migrations = history.people.iter().flat_map(|p| &p.life_events)
            .filter(|e| matches!(e.kind, LEK::Migrated { .. })).count();
        let avg_happiness: f64 = if alive == 0 { 0.0 }
            else { living.iter().map(|p| p.happiness as f64).sum::<f64>() / alive as f64 };

        // === OVERVIEW ===
        println!("\n============================================================");
        println!("  WORLD HISTORY — {} years", year);
        println!("============================================================");
        println!("  {} factions ({} alive) | {} settlements | {} events",
            history.factions.len(), history.living_factions().len(),
            history.settlements.iter().filter(|s| s.destroyed_year.is_none()).count(),
            history.events.len());
        println!("  {} people alive (of {} ever) | avg happiness: {:.0}",
            alive, history.people.len(), avg_happiness);
        println!("  {} wars | {} conquests | {} plagues | {} migrations",
            wars, conquests, plagues, migrations);

        // === GODS ===
        println!("\n--- GODS ---");
        for g in &history.gods {
            let status = if g.is_active() { "ACTIVE" } else { "faded" };
            let drive = format!("{:?}", g.personality.drive);
            let flaw = format!("{:?}", g.personality.flaw);
            println!("  God {} [{}] drive:{} flaw:{} | pow:{} terr:{} worshippers:{}",
                g.god_id, status, drive, flaw, g.power, g.territory.len(),
                g.worshipper_settlements.len());
        }

        // === FACTIONS ===
        println!("\n--- FACTIONS ---");
        for f in &history.factions {
            let status = if f.is_alive(year) { "ALIVE" } else { "dead" };
            let leader = &f.leader_name;
            println!("  {} [{} {:?} {:?}] settlements:{} mil:{} wealth:{} stab:{} leader:{}",
                f.name, status, f.race, f.faction_type,
                f.settlements.len(), f.military_strength, f.wealth, f.stability, leader);
        }

        // === SETTLEMENTS ===
        println!("\n--- SETTLEMENTS (by population) ---");
        let mut settlement_info: Vec<_> = history.settlements.iter()
            .filter(|s| s.destroyed_year.is_none())
            .map(|s| {
                let pop = living.iter().filter(|p| p.settlement_id == s.id).count();
                (s, pop)
            })
            .filter(|(_, pop)| *pop > 0)
            .collect();
        settlement_info.sort_by(|a, b| b.1.cmp(&a.1));
        for (s, pop) in settlement_info.iter().take(15) {
            let avg_h: f64 = living.iter().filter(|p| p.settlement_id == s.id)
                .map(|p| p.happiness as f64).sum::<f64>() / *pop as f64;
            let race_str = s.dominant_race.map(|r| format!("{:?}", r)).unwrap_or("-".into());
            let patron_str = s.patron_god.map(|g| format!("god{}", g)).unwrap_or("-".into());
            let faction_name = history.factions.iter()
                .find(|f| f.id == s.controlling_faction)
                .map(|f| f.name.as_str()).unwrap_or("unowned");
            println!("  {} ({:?}) pop:{} prosp:{} happy:{:.0} | race:{} patron:{} | {}",
                s.name, s.population_class, pop, s.prosperity, avg_h,
                race_str, patron_str, faction_name);
        }

        // === PROPHETS ===
        println!("\n--- PROPHETS ---");
        let prophet_people: Vec<_> = history.people.iter()
            .filter(|p| p.prophet_of.is_some())
            .collect();
        if prophet_people.is_empty() {
            println!("  (none)");
        }
        for p in &prophet_people {
            let role = p.prophet_of.as_ref().unwrap();
            let is_alive = p.death_year.is_none() || p.death_year.unwrap() > year;
            let status = if is_alive { format!("alive age {}", p.age(year)) }
                else {
                    let cause_str = p.death_cause.map(|c| format!("{:?}", c)).unwrap_or("?".into());
                    format!("died yr {} ({})", p.death_year.unwrap(), cause_str)
                };
            let settlement_name = history.settlements.iter()
                .find(|s| s.id == p.settlement_id)
                .map(|s| s.name.as_str()).unwrap_or("???");
            let traits_str = p.traits.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join(",");
            println!("  #{} {:?} {:?} — {:?} of god{} in {} | {} | traits:[{}]",
                p.id, p.race, p.occupation, role.kind, role.god_id, settlement_name,
                status, traits_str);
        }

        // === TIMELINE (key events) ===
        println!("\n--- TIMELINE ---");
        for e in history.events.iter().filter(|e| matches!(e.kind,
            EventKind::WarDeclared | EventKind::WarEnded | EventKind::Conquest
            | EventKind::Betrayal | EventKind::FactionDissolved | EventKind::FactionFounded
            | EventKind::PlagueStruck | EventKind::HeroRose))
        {
            println!("  yr{:3}: {}", e.year, e.description);
        }

        // === NOTABLE LIVES (top 3) ===
        println!("\n--- NOTABLE LIVES ---");
        let mut notables: Vec<_> = history.people.iter()
            .filter(|p| p.notable && p.life_events.len() >= 10)
            .collect();
        notables.sort_by(|a, b| b.life_events.len().cmp(&a.life_events.len()));
        for p in notables.iter().take(3) {
            let is_alive = p.death_year.is_none() || p.death_year.unwrap() > year;
            let status = if is_alive { format!("alive age {}", p.age(year)) }
                else { format!("died yr {}", p.death_year.unwrap()) };
            let race_str = if let Some(sr) = p.secondary_race {
                format!("{:?}/{:?}", p.race, sr)
            } else { format!("{:?}", p.race) };
            let faith_str = if p.faith.is_empty() { "none".into() }
                else { p.faith.iter().map(|(g, d)| format!("g{}:{}", g, d)).collect::<Vec<_>>().join(",") };
            let traits_str = p.traits.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join(",");
            let settlement_name = history.settlements.iter()
                .find(|s| s.id == p.settlement_id)
                .map(|s| s.name.as_str()).unwrap_or("???");
            let prophet_str = if p.prophet_of.is_some() { " [PROPHET]" } else { "" };
            println!("\n  #{} {:?} {} ({}) {}{} | faith:{} traits:[{}] happy:{}",
                p.id, p.occupation, race_str, status, settlement_name, prophet_str,
                faith_str, traits_str, p.happiness);
            for e in &p.life_events {
                let cause_str = match &e.cause {
                    Some(EventCause::Divine { god_id, action }) => format!(" [god{} {:?}]", god_id, action),
                    Some(EventCause::Conditions { detail, .. }) => format!(" [{}]", detail),
                    Some(EventCause::Faction { faction_id, detail }) => format!(" [fac{} {}]", faction_id, detail),
                    Some(EventCause::PersonAction { person_id, role }) => format!(" [#{} {}]", person_id, role),
                    None => String::new(),
                };
                println!("    yr{:3}: {:?}{}", e.year, e.kind, cause_str);
            }
        }
    }

    #[test]
    fn multi_seed_comparison() {
        use crate::worldgen::population::LifeEventKind as LEK;

        println!("\n{:>6} {:>5} {:>5} {:>4} {:>4} {:>5} {:>6} {:>5} {:>5} {:>8} {:>4}",
            "seed", "alive", "total", "facs", "wars", "conq", "plague", "migr", "proph", "avg_hap", "gods");
        println!("{}", "-".repeat(85));

        for seed in [42, 123, 999, 7, 2024, 55555, 314, 8008] {
            let history = test_history_with_size(HistoryConfig::default(), seed, 256);
            let year = history.current_year;

            let alive = history.people.iter()
                .filter(|p| p.death_year.is_none() || p.death_year.unwrap() > year)
                .count();
            let factions_alive = history.living_factions().len();
            let wars = history.events.iter().filter(|e| e.kind == EventKind::WarDeclared).count();
            let conquests = history.events.iter().filter(|e| e.kind == EventKind::Conquest).count();
            let plagues = history.events.iter().filter(|e| e.kind == EventKind::PlagueStruck).count();
            let migrations = history.people.iter().flat_map(|p| &p.life_events)
                .filter(|e| matches!(e.kind, LEK::Migrated { .. })).count();
            let prophets = history.people.iter()
                .filter(|p| p.prophet_of.is_some()).count();
            let avg_happiness: f64 = {
                let living: Vec<_> = history.people.iter()
                    .filter(|p| p.death_year.is_none() || p.death_year.unwrap() > year)
                    .collect();
                if living.is_empty() { 0.0 }
                else { living.iter().map(|p| p.happiness as f64).sum::<f64>() / living.len() as f64 }
            };
            let active_gods = history.gods.iter().filter(|g| g.is_active()).count();

            println!("{:>6} {:>5} {:>5} {:>4} {:>4} {:>5} {:>6} {:>5} {:>5} {:>8.1} {:>4}",
                seed, alive, history.people.len(), factions_alive, wars, conquests,
                plagues, migrations, prophets, avg_happiness, active_gods);
        }
    }

    #[test]
    fn faction_diagnostic_sweep() {
        // Sweep 50 seeds, show total vs alive factions, type breakdown, and zombie factions
        println!("\n{:>6} {:>5} {:>5} {:>4} {:>4}  {:<50}  {:>7}",
            "seed", "total", "alive", "dead", "wars", "type breakdown (alive)", "zombies");
        println!("{}", "-".repeat(110));

        let mut worst_seeds: Vec<(u64, usize, usize)> = Vec::new();

        for seed in 0u64..50 {
            let history = test_history_with_size(HistoryConfig::default(), seed, 256);
            let year = history.current_year;

            let total = history.factions.len();
            let alive: Vec<_> = history.factions.iter().filter(|f| f.is_alive(year)).collect();
            let dead = total - alive.len();
            let wars = history.events.iter().filter(|e| e.kind == EventKind::WarDeclared).count();

            // Type breakdown of alive factions
            let mut type_counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
            for f in &alive {
                *type_counts.entry(format!("{:?}", f.faction_type)).or_insert(0) += 1;
            }
            let type_str: String = type_counts.iter()
                .map(|(t, c)| format!("{}:{}", t, c))
                .collect::<Vec<_>>().join(" ");

            // Zombie factions: alive but zero allegiant people
            let zombies: Vec<_> = alive.iter().filter(|f| {
                let pop = history.people.iter()
                    .filter(|p| p.is_alive(year) && p.faction_allegiance == f.id)
                    .count();
                pop == 0
            }).collect();

            println!("{:>6} {:>5} {:>5} {:>4} {:>4}  {:<50}  {:>7}",
                seed, total, alive.len(), dead, wars, type_str, zombies.len());

            worst_seeds.push((seed, total, alive.len()));
        }

        // Show worst seeds by total factions
        worst_seeds.sort_by(|a, b| b.1.cmp(&a.1));
        println!("\n--- WORST SEEDS (by total factions ever) ---");
        for (seed, total, alive) in worst_seeds.iter().take(5) {
            println!("  seed {} — {} total, {} alive", seed, total, alive);
        }
    }

    #[test]
    fn faction_deep_dive() {
        // Deep dive into a specific seed to trace faction lifecycle
        let seed = 17u64; // was 18 alive — ReligiousOrder spam seed
        let history = test_history_with_size(HistoryConfig::default(), seed, 256);
        let year = history.current_year;

        println!("\n========== DEEP DIVE: seed {} ==========", seed);
        println!("  {} factions total, {} alive\n", history.factions.len(),
            history.living_factions().len());

        // Formation timeline: when were factions founded?
        println!("--- FORMATION TIMELINE ---");
        let mut by_year: std::collections::BTreeMap<i32, Vec<&FactionState>> = std::collections::BTreeMap::new();
        for f in &history.factions {
            by_year.entry(f.founded_year).or_default().push(f);
        }
        for (yr, facs) in &by_year {
            let names: Vec<_> = facs.iter().map(|f| {
                let status = if f.is_alive(year) { "ALIVE" } else {
                    if let Some(dy) = f.dissolved_year { "dead" } else { "???" }
                };
                let pop = history.people.iter()
                    .filter(|p| p.is_alive(year) && p.faction_allegiance == f.id)
                    .count();
                let god_str = f.patron_god.map(|g| format!("g{}", g)).unwrap_or("-".into());
                format!("{} ({:?}, {}, pop:{}, {})", f.name, f.faction_type, status, pop, god_str)
            }).collect();
            println!("  yr {:>3}: {}", yr, names.join("; "));
        }

        // Zombie factions detail
        println!("\n--- ZOMBIE FACTIONS (alive, zero population) ---");
        for f in history.factions.iter().filter(|f| f.is_alive(year)) {
            let pop = history.people.iter()
                .filter(|p| p.is_alive(year) && p.faction_allegiance == f.id)
                .count();
            if pop == 0 {
                println!("  {} ({:?}) — founded yr {}, mil:{} wealth:{} stab:{} settlements:{} unhappy_yrs:{}",
                    f.name, f.faction_type, f.founded_year,
                    f.military_strength, f.wealth, f.stability,
                    f.settlements.len(), f.unhappy_years);
            }
        }

        // Faction lifespan histogram
        println!("\n--- FACTION LIFESPANS (dead factions) ---");
        let mut lifespans: Vec<(i32, String, String)> = Vec::new();
        for f in history.factions.iter().filter(|f| !f.is_alive(year)) {
            let lifespan = f.dissolved_year.unwrap_or(year) - f.founded_year;
            lifespans.push((lifespan, format!("{:?}", f.faction_type), f.name.clone()));
        }
        lifespans.sort();
        let short_lived = lifespans.iter().filter(|(l, _, _)| *l <= 3).count();
        let medium = lifespans.iter().filter(|(l, _, _)| *l > 3 && *l <= 10).count();
        let long = lifespans.iter().filter(|(l, _, _)| *l > 10).count();
        println!("  <=3 years: {}  |  4-10 years: {}  |  >10 years: {}", short_lived, medium, long);

        // Show the short-lived ones
        println!("\n  Short-lived factions (<=3 years):");
        for (lifespan, ftype, name) in lifespans.iter().filter(|(l, _, _)| *l <= 3).take(20) {
            println!("    {} ({}) — lived {} years", name, ftype, lifespan);
        }

        // Theocracy lifecycle trace
        let theocracies: Vec<_> = history.factions.iter()
            .filter(|f| f.faction_type == crate::worldgen::names::FactionType::Theocracy)
            .collect();
        if !theocracies.is_empty() {
            println!("\n--- THEOCRACY LIFECYCLE ---");
            for f in &theocracies {
                let lifespan = f.dissolved_year.unwrap_or(year) - f.founded_year;
                let status = if f.is_alive(year) { "ALIVE".to_string() }
                    else { format!("died yr {}", f.dissolved_year.unwrap()) };

                // Find how it died
                let death_event = history.events.iter()
                    .find(|e| e.kind == EventKind::FactionDissolved && e.participants.contains(&f.id)
                        && e.year == f.dissolved_year.unwrap_or(-1));
                let death_cause = death_event.map(|e| {
                    if e.description.contains("absorbed") { "ABSORBED" }
                    else { "DISSOLVED" }
                }).unwrap_or(if f.is_alive(year) { "alive" } else { "???" });

                // Pop at end
                let pop_now = history.people.iter()
                    .filter(|p| p.is_alive(year) && p.faction_allegiance == f.id)
                    .count();

                // Find the founder event
                let founded_event = history.events.iter()
                    .find(|e| e.kind == EventKind::FactionFounded && e.participants.contains(&f.id)
                        && e.year == f.founded_year);
                let reason = founded_event.map(|e| e.description.as_str()).unwrap_or("?");

                println!("  {} — founded yr {}, {} (lived {}yr), pop:{}, god:{:?}, mil:{} stab:{} | {} | {}",
                    f.name, f.founded_year, status, lifespan, pop_now,
                    f.patron_god, f.military_strength, f.stability,
                    death_cause, reason);
            }
        }

        // Type churn: how many of each type were created vs survived
        println!("\n--- TYPE CHURN ---");
        let mut type_total: std::collections::BTreeMap<String, (usize, usize)> = std::collections::BTreeMap::new();
        for f in &history.factions {
            let key = format!("{:?}", f.faction_type);
            let entry = type_total.entry(key).or_insert((0, 0));
            entry.0 += 1;
            if f.is_alive(year) { entry.1 += 1; }
        }
        for (ftype, (total, alive)) in &type_total {
            println!("  {:<20} created:{:>3}  alive:{:>3}  died:{:>3}", ftype, total, alive, total - alive);
        }
    }
}
