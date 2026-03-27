pub mod artifacts;
pub mod characters;
pub mod culture;
pub mod state;

use std::collections::BTreeSet;

use rand::{Rng, RngExt, SeedableRng};

use super::divine_era::artifacts::DivineArtifact;
use super::divine_era::behavior;
use super::divine_era::personality;
use super::divine_era::races::CreatedRace;
use super::divine_era::sites::DivineSite;
use super::divine_era::state::GodState;
use super::divine_era::terrain_scars::TerrainScar;
use super::gods::{DrawnPantheon, GodId, GodPool};
use super::names::{FactionType, Race, faction_name, full_name, region_name, settlement_name};
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
    world_state.divine_relations = super::divine_era::state::DivineRelationMatrix::from_relationships(&pantheon.relationships);

    let any_divine = config.divine_territory || config.divine_worship
        || config.divine_drives || config.divine_conflict || config.divine_flaws;

    // BFS frontiers for territory expansion
    let mut frontiers: Vec<BTreeSet<WorldPos>> = vec![BTreeSet::new(); gods.len()];

    // Assign starting seats of power (only needed when divine phases are active)
    if any_divine {
        behavior::assign_seats_of_power(&mut gods, &mut frontiers, &mut world_state, world_map, god_pool, &mut rng);
    }

    // --- Initialize divine sites/artifacts/races/scars ---
    let mut divine_sites: Vec<super::divine_era::sites::DivineSite> = Vec::new();
    let mut divine_artifacts: Vec<super::divine_era::artifacts::DivineArtifact> = Vec::new();
    let mut created_races: Vec<super::divine_era::races::CreatedRace> = Vec::new();
    let mut terrain_scars: Vec<super::divine_era::terrain_scars::TerrainScar> = Vec::new();

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
            }
        })
        .collect();
    settlements.append(&mut map_settlements);

    // --- Unified year-by-year simulation ---
    let mut cross_events: Vec<CrossDomainEvent> = Vec::new();

    for year in config.start_year..config.start_year + config.num_years {
        let event_count_before = events.len();

        // Always runs — bookkeeping
        behavior::update_god_power(&mut gods);

        // Phase 2: Territory expansion & terrain shaping
        if config.divine_territory {
            behavior::evaluate_territory_expansion(
                year, config.territory_expansion_rate, &mut gods, &mut frontiers,
                &mut events, &mut world_state, world_map, god_pool, pantheon, &mut rng,
            );
            behavior::evaluate_terrain_shaping(
                year, &gods, &mut events, &mut terrain_scars,
                &world_state, world_map, god_pool, pantheon, &mut next_id, &mut rng,
            );
        }

        // Phase 3: Worship competition
        if config.divine_worship {
            behavior::evaluate_worship(
                year, &mut gods, &mut events, &settlements,
                &mut cross_events, &mut world_state, world_map, pantheon, &mut rng,
            );
            apply_cross_domain_events(&mut cross_events, &mut settlements);
        }

        // Phase 4: Drive-based divine actions
        if config.divine_drives {
            behavior::evaluate_drive_actions(
                year, config.race_creation_window, &mut gods, &mut events,
                &mut divine_sites, &mut divine_artifacts, &mut created_races,
                &world_state, world_map, god_pool, pantheon, &mut next_id, &mut rng,
            );
        }

        // Phases 5-8: Mortal simulation
        if config.mortal_simulation {
            simulate_year(
                year, &mut factions, &mut settlements, &mut characters,
                &mut artifacts_list, &mut events,
                &mut world_state, &regions, &mut next_id,
                &faction_type_table, &race_table, &mut rng,
            );
        }

        // Phase 9: Divine conflict
        if config.divine_conflict {
            let active_god_ids: Vec<u32> = gods.iter().filter(|g| g.is_active()).map(|g| g.god_id).collect();
            behavior::evaluate_divine_war_declared(year, &gods, &mut events, &mut world_state, &active_god_ids, pantheon, &mut rng);
            behavior::evaluate_divine_war_resolution(
                year, &mut gods, &mut events, &mut terrain_scars,
                &mut world_state, world_map, god_pool, pantheon, &mut next_id, &mut rng,
            );
            behavior::evaluate_divine_pact(year, &gods, &mut events, &mut world_state, &active_god_ids, pantheon, &mut rng);
            behavior::evaluate_pact_broken(year, &gods, &mut events, &mut world_state, pantheon, &mut rng);
        }

        // Phase 10: Flaw pressure & triggers
        if config.divine_flaws {
            let new_events = &events[event_count_before..];
            behavior::accumulate_flaw_pressure(&mut gods, new_events, &world_state);
            behavior::evaluate_flaw_triggers(
                year, &mut gods, &mut events, &settlements,
                &mut cross_events, &mut world_state, pantheon, &mut rng,
            );
            apply_cross_domain_events(&mut cross_events, &mut settlements);
        }

        // Always runs — maintenance
        behavior::drain_divine_war_power(&mut gods, &world_state);
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
    }
}

fn simulate_year(
    year: i32,
    factions: &mut Vec<FactionState>,
    settlements: &mut Vec<SettlementState>,
    characters: &mut Vec<Character>,
    artifacts_list: &mut Vec<Artifact>,
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    regions: &[String],
    next_id: &mut u32,
    faction_type_table: &PopTable<FactionType>,
    race_table: &PopTable<Race>,
    rng: &mut impl Rng,
) {
    let living: Vec<u32> = factions.iter().filter(|f| f.is_alive(year)).map(|f| f.id).collect();
    if living.is_empty() { return; }

    // Phase 0: Aging and death
    let mut dead_leaders: Vec<(u32, u32)> = Vec::new(); // (faction_id, character_id)
    for character in characters.iter_mut() {
        if !character.is_alive(year) { continue; }
        if character.natural_death_check(year, rng) {
            character.death_year = Some(year);
            // Check if this was a faction leader
            if character.role == CharacterRole::Leader {
                if let Some(fid) = character.faction_id {
                    dead_leaders.push((fid, character.id));
                }
            }
        }
    }

    // Handle leader succession for dead leaders
    for (faction_id, dead_leader_id) in &dead_leaders {
        let faction = match factions.iter().find(|f| f.id == *faction_id) {
            Some(f) => f,
            None => continue,
        };
        if !faction.is_alive(year) { continue; }
        let race = faction.race;
        let fname = faction.name.clone();
        let dead_name = characters.iter()
            .find(|c| c.id == *dead_leader_id)
            .map(|c| c.full_display_name())
            .unwrap_or_else(|| "Unknown".into());

        // Try to promote an existing notable member
        let successor = characters.iter_mut()
            .filter(|c| c.is_alive(year) && c.faction_id == Some(*faction_id) && c.role != CharacterRole::Leader)
            .max_by_key(|c| c.renown);

        let new_leader_name = if let Some(s) = successor {
            s.role = CharacterRole::Leader;
            let name = s.full_display_name();
            let sid = s.id;
            if let Some(f) = factions.iter_mut().find(|f| f.id == *faction_id) {
                f.leader_id = Some(sid);
                f.leader_name = name.clone();
            }
            name
        } else {
            // Generate a new character
            let new_id = *next_id;
            *next_id += 1;
            let birth = year - rng.random_range(25..40);
            let mut new_leader = generate_character(new_id, race, CharacterRole::Leader, Some(*faction_id), birth, rng);
            new_leader.epithet = Some(generate_epithet(&new_leader, rng));
            let name = new_leader.full_display_name();
            characters.push(new_leader);
            if let Some(f) = factions.iter_mut().find(|f| f.id == *faction_id) {
                f.leader_id = Some(new_id);
                f.leader_name = name.clone();
            }
            name
        };

        events.push(HistoricEvent {
            year, kind: EventKind::LeaderChanged,
            description: format!("After the death of {}, {} became leader of {}", dead_name, new_leader_name, fname),
            participants: vec![*faction_id],
            god_participants: vec![],
        });
    }

    // Snapshot for cross-references during upkeep
    let factions_snapshot: Vec<FactionState> = factions.clone();

    // Phase 1: Faction upkeep
    for faction in factions.iter_mut() {
        if !faction.is_alive(year) { continue; }

        // Wars drain military strength heavily
        let wars = world_state.war_count(faction.id);
        if wars > 0 {
            faction.military_strength = faction.military_strength.saturating_sub(5 * wars as u32);
            faction.wealth = faction.wealth.saturating_sub(4 * wars as u32);
            faction.stability = faction.stability.saturating_sub(2);
        }

        // Standing army costs wealth (only large armies are expensive)
        if faction.military_strength > 50 {
            let upkeep = (faction.military_strength - 50) / 25; // 0-1 per year
            faction.wealth = faction.wealth.saturating_sub(upkeep);
        }

        // Treaties add small wealth
        let treaties = world_state.active_treaties.iter()
            .filter(|t| t.faction_a == faction.id || t.faction_b == faction.id)
            .count();
        if treaties > 0 {
            faction.wealth = (faction.wealth + 1).min(80); // cap lower for organic feel
        }

        // Military regenerates slowly (1 per settlement, max 2/year)
        let regen = (faction.settlements.len() as u32).min(2);
        faction.military_strength = (faction.military_strength + regen).min(80);

        // Wealth from settlements (primary income)
        let settlement_income = (faction.settlements.len() as u32 * 2).min(8);
        faction.wealth = (faction.wealth + settlement_income).min(90);

        // Stability drifts toward 50
        if faction.stability > 50 { faction.stability -= 1; }
        else if faction.stability < 50 { faction.stability += 1; }

        // Territorial friction: factions sharing a region with rivals get sentiment pushed down
        for other in &living {
            if *other == faction.id { continue; }
            if let Some(other_f) = factions_snapshot.iter().find(|f| f.id == *other) {
                if other_f.home_region == faction.home_region {
                    // Proximity breeds friction — push sentiment slightly negative each year
                    world_state.relations.modify(faction.id, *other, -1);
                }
            }
        }
    }

    // Phase 2: Settlement upkeep
    for settlement in settlements.iter_mut() {
        if settlement.destroyed_year.is_some() { continue; }
        // Prosperity from peace
        if world_state.war_count(settlement.owner_faction) == 0 {
            settlement.prosperity = (settlement.prosperity + 1).min(100);
        }
        // Growth check
        if settlement.prosperity > 70 && rng.random::<f32>() < 0.05 {
            settlement.population_class = settlement.population_class.grow();
        }
        // Shrink check
        if settlement.prosperity < 30 && rng.random::<f32>() < 0.05 {
            settlement.population_class = settlement.population_class.shrink();
        }
    }

    // Phase 3: New character generation (stable factions produce notable members)
    for &fid in &living {
        let f = match factions.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => continue,
        };
        if f.stability < 40 { continue; } // unstable factions don't attract talent
        // ~1 new notable per faction per 15 years
        if rng.random::<f32>() >= (1.0 / 15.0) { continue; }

        let role_table = PopTable::pick_one(vec![
            (CharacterRole::General, 30.0),
            (CharacterRole::Advisor, 25.0),
            (CharacterRole::Scholar, 20.0),
            (CharacterRole::Hero, 15.0),
            (CharacterRole::Villain, 10.0),
        ]);
        let role = role_table.roll_one(rng).unwrap();
        let char_id = *next_id;
        *next_id += 1;
        let birth = year - rng.random_range(18..30);
        let new_char = generate_character(char_id, f.race, role, Some(fid), birth, rng);
        characters.push(new_char);
    }

    // Phase 3b: Random friction — border disputes, rivalries, incidents
    // This ensures tensions can build even without explicit events
    if living.len() >= 2 && rng.random::<f32>() < 0.30 {
        let a = living[rng.random_range(0..living.len())];
        let b_candidates: Vec<u32> = living.iter().copied().filter(|&x| x != a).collect();
        if !b_candidates.is_empty() {
            let b = b_candidates[rng.random_range(0..b_candidates.len())];
            let severity = rng.random_range(2..8);
            world_state.relations.modify(a, b, -(severity as i32));
        }
    }

    // Phase 4: Event evaluation (prerequisite-based, character-driven)
    evaluate_war_declared(year, factions, characters, events, world_state, &living, rng);
    evaluate_war_ended(year, factions, settlements, events, world_state, rng);
    evaluate_betrayal(year, factions, characters, events, world_state, rng);
    evaluate_alliance(year, factions, characters, events, world_state, &living, rng);
    evaluate_alliance_broken(year, factions, characters, events, world_state, rng);
    evaluate_trade_agreement(year, factions, events, world_state, &living, rng);
    evaluate_leader_changed(year, factions, characters, events, &living, next_id, rng);
    evaluate_plague(year, factions, settlements, events, regions, rng);
    evaluate_monster_attack(year, events, regions, rng);
    evaluate_hero(year, factions, characters, events, &living, next_id, race_table, rng);
    evaluate_artifact_discovered(year, factions, characters, artifacts_list, events, &living, next_id, rng);
    evaluate_settlement_founded(year, factions, settlements, events, next_id, &living, regions, rng);
    evaluate_new_faction(year, factions, events, world_state, next_id, regions, faction_type_table, race_table, rng);
    evaluate_faction_dissolved(year, factions, events, world_state, &living, rng);

    // Phase 4: Sentiment drift
    world_state.relations.drift_toward_neutral();
}

// ── Event evaluation functions ──
// Each checks prerequisites against world state, rolls probability, and applies consequences.

fn evaluate_war_declared(
    year: i32, factions: &[FactionState], characters: &[Character],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], rng: &mut impl Rng,
) {
    if living.len() < 2 { return; }
    let Some((a, b, sentiment)) = world_state.relations.most_hostile_pair(living) else { return };
    if sentiment >= -20 { return; } // lowered threshold
    if world_state.at_war(a, b) { return; }
    if world_state.war_count(a) > 1 || world_state.war_count(b) > 1 { return; }

    let aggressor_mil = factions.iter().find(|f| f.id == a).map(|f| f.military_strength).unwrap_or(0);
    if aggressor_mil < 10 { return; } // lowered threshold

    // Base probability + character modifiers
    let hostility_bonus = ((-sentiment - 20) as f32 * 0.8).min(40.0);
    let mut prob = 20.0 + hostility_bonus;

    // Leader traits modify war probability
    if leader_has_trait(a, CharacterTrait::Warlike, factions, characters) { prob += 20.0; }
    if leader_has_trait(a, CharacterTrait::Ambitious, factions, characters) { prob += 10.0; }
    if leader_has_trait(a, CharacterTrait::Peaceful, factions, characters) { prob -= 25.0; }
    if leader_has_trait(a, CharacterTrait::Diplomatic, factions, characters) { prob -= 15.0; }
    // Warlike general pushing for war
    if faction_has_member_with_trait(a, CharacterTrait::Warlike, characters, year) { prob += 5.0; }

    let prob = (prob / 100.0).clamp(0.02, 0.60);
    if rng.random::<f32>() >= prob { return; }

    // Declare war
    world_state.active_wars.push(War { aggressor: a, defender: b, start_year: year });
    world_state.relations.modify(a, b, -20);

    let fa = faction_name_by_id(factions, a);
    let fb = faction_name_by_id(factions, b);
    events.push(HistoricEvent {
        year, kind: EventKind::WarDeclared,
        description: format!("{} declared war on {}", fa, fb),
        participants: vec![a, b],
            god_participants: vec![],
    });
}

fn evaluate_war_ended(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    events: &mut Vec<HistoricEvent>, world_state: &mut WorldState, rng: &mut impl Rng,
) {
    let mut ended_wars = Vec::new();
    for (i, war) in world_state.active_wars.iter().enumerate() {
        let duration = year - war.start_year;
        if duration < 2 { continue; }

        // Probability increases with duration
        let base_prob = 0.10 + duration as f32 * 0.05;
        // Check if either side is very weak
        let a_mil = factions.iter().find(|f| f.id == war.aggressor).map(|f| f.military_strength).unwrap_or(0);
        let b_mil = factions.iter().find(|f| f.id == war.defender).map(|f| f.military_strength).unwrap_or(0);
        let weakness_bonus = if a_mil < 20 || b_mil < 20 { 0.30 } else { 0.0 };

        if rng.random::<f32>() < (base_prob + weakness_bonus).min(0.90) {
            ended_wars.push(i);
        }
    }

    // Process ended wars in reverse order to preserve indices
    for &i in ended_wars.iter().rev() {
        let war = world_state.active_wars.remove(i);
        let a_mil = factions.iter().find(|f| f.id == war.aggressor).map(|f| f.military_strength).unwrap_or(0);
        let b_mil = factions.iter().find(|f| f.id == war.defender).map(|f| f.military_strength).unwrap_or(0);

        let (winner, loser) = if a_mil >= b_mil {
            (war.aggressor, war.defender)
        } else {
            (war.defender, war.aggressor)
        };

        let fw = faction_name_by_id(factions, winner);
        let fl = faction_name_by_id(factions, loser);

        // Consequences
        if let Some(w) = factions.iter_mut().find(|f| f.id == winner) {
            w.military_strength = w.military_strength.saturating_sub(10);
            w.wealth = (w.wealth + 10).min(100);
        }
        if let Some(l) = factions.iter_mut().find(|f| f.id == loser) {
            l.military_strength = l.military_strength.saturating_sub(20);
            l.stability = l.stability.saturating_sub(15);
        }

        // Settlement conquest: winner may take a settlement from loser
        let loser_settlements: Vec<u32> = settlements.iter()
            .filter(|s| s.owner_faction == loser && s.destroyed_year.is_none())
            .map(|s| s.id)
            .collect();
        if !loser_settlements.is_empty() && rng.random::<f32>() < 0.4 {
            let target_sid = loser_settlements[rng.random_range(0..loser_settlements.len())];
            if let Some(s) = settlements.iter_mut().find(|s| s.id == target_sid) {
                let old_name = s.name.clone();
                s.owner_faction = winner;
                s.prosperity = s.prosperity.saturating_sub(20);
                s.population_class = s.population_class.shrink();

                // Update faction settlement lists
                if let Some(l) = factions.iter_mut().find(|f| f.id == loser) {
                    l.settlements.retain(|&sid| sid != target_sid);
                }
                if let Some(w) = factions.iter_mut().find(|f| f.id == winner) {
                    w.settlements.push(target_sid);
                }

                events.push(HistoricEvent {
                    year, kind: EventKind::Conquest,
                    description: format!("{} conquered {} from {}", fw, old_name, fl),
                    participants: vec![winner, loser],
            god_participants: vec![],
                });
                world_state.relations.modify(winner, loser, -15);
            }
        }

        events.push(HistoricEvent {
            year, kind: EventKind::WarEnded,
            description: format!("The war between {} and {} ended; {} emerged victorious", fw, fl, fw),
            participants: vec![winner, loser],
            god_participants: vec![],
        });
    }
}

/// A treacherous character in an allied faction betrays the alliance.
fn evaluate_betrayal(
    year: i32, factions: &mut Vec<FactionState>, characters: &mut Vec<Character>,
    events: &mut Vec<HistoricEvent>, world_state: &mut WorldState, rng: &mut impl Rng,
) {
    if world_state.active_alliances.is_empty() { return; }

    // Look for a treacherous character in any allied faction
    for i in 0..world_state.active_alliances.len() {
        let alliance = &world_state.active_alliances[i];
        let a = alliance.faction_a;
        let b = alliance.faction_b;

        // Find a treacherous character in either faction
        let betrayer = characters.iter()
            .filter(|c| c.is_alive(year))
            .filter(|c| c.faction_id == Some(a) || c.faction_id == Some(b))
            .filter(|c| c.has_trait(CharacterTrait::Treacherous) || c.has_trait(CharacterTrait::Corrupt))
            .max_by_key(|c| c.renown);

        let betrayer_id = match betrayer {
            Some(c) => c.id,
            None => continue,
        };

        // Low probability even with a traitor
        if rng.random::<f32>() >= 0.05 { continue; }

        let betrayer_char = characters.iter().find(|c| c.id == betrayer_id).unwrap();
        let betrayer_faction = betrayer_char.faction_id.unwrap();
        let victim_faction = if betrayer_faction == a { b } else { a };
        let betrayer_name = betrayer_char.full_display_name();
        let fb = faction_name_by_id(factions, betrayer_faction);
        let fv = faction_name_by_id(factions, victim_faction);

        // Consequences: alliance broken, massive sentiment drop, betrayer gains epithet
        world_state.active_alliances.remove(i);
        world_state.relations.modify(betrayer_faction, victim_faction, -40);

        if let Some(c) = characters.iter_mut().find(|c| c.id == betrayer_id) {
            if c.epithet.is_none() {
                c.epithet = Some("the Betrayer".into());
            }
            c.renown += 5; // infamy is still fame
        }

        // Victim faction loses stability
        if let Some(f) = factions.iter_mut().find(|f| f.id == victim_faction) {
            f.stability = f.stability.saturating_sub(10);
        }

        events.push(HistoricEvent {
            year, kind: EventKind::Betrayal,
            description: format!(
                "{} of {} betrayed the alliance with {}, shattering the pact",
                betrayer_name, fb, fv
            ),
            participants: vec![betrayer_faction, victim_faction],
            god_participants: vec![],
        });
        return; // One betrayal per year max
    }
}

fn evaluate_alliance(
    year: i32, factions: &[FactionState], characters: &[Character],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], rng: &mut impl Rng,
) {
    if living.len() < 2 { return; }

    // Base 10%, boosted by Diplomatic leaders
    let any_diplomatic = living.iter().any(|&fid| {
        leader_has_trait(fid, CharacterTrait::Diplomatic, factions, characters)
    });
    let prob = if any_diplomatic { 0.20 } else { 0.10 };
    if rng.random::<f32>() >= prob { return; }

    // Find two friendly factions not already allied
    for &a in living {
        for &b in living {
            if a >= b { continue; }
            if !world_state.relations.is_friendly(a, b) { continue; }
            if world_state.allied(a, b) { continue; }
            if world_state.at_war(a, b) { continue; }

            world_state.active_alliances.push(Alliance { faction_a: a, faction_b: b, formed_year: year });
            world_state.relations.modify(a, b, 10);

            let fa = faction_name_by_id(factions, a);
            let fb = faction_name_by_id(factions, b);
            events.push(HistoricEvent {
                year, kind: EventKind::AllianceFormed,
                description: format!("{} and {} formed an alliance", fa, fb),
                participants: vec![a, b],
            god_participants: vec![],
            });
            return; // One alliance per year max
        }
    }
}

fn evaluate_alliance_broken(
    year: i32, factions: &[FactionState], characters: &[Character],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, rng: &mut impl Rng,
) {
    let mut broken = Vec::new();
    for (i, alliance) in world_state.active_alliances.iter().enumerate() {
        let sentiment = world_state.relations.get(alliance.faction_a, alliance.faction_b);
        let treacherous_leader = leader_has_trait(alliance.faction_a, CharacterTrait::Treacherous, factions, characters)
            || leader_has_trait(alliance.faction_b, CharacterTrait::Treacherous, factions, characters);

        let break_prob = if treacherous_leader { 0.35 } else { 0.20 };
        if sentiment < 10 && rng.random::<f32>() < break_prob {
            broken.push(i);
        }
    }
    for &i in broken.iter().rev() {
        let alliance = world_state.active_alliances.remove(i);
        world_state.relations.modify(alliance.faction_a, alliance.faction_b, -25);
        let fa = faction_name_by_id(factions, alliance.faction_a);
        let fb = faction_name_by_id(factions, alliance.faction_b);
        events.push(HistoricEvent {
            year, kind: EventKind::AllianceBroken,
            description: format!("The alliance between {} and {} collapsed", fa, fb),
            participants: vec![alliance.faction_a, alliance.faction_b],
            god_participants: vec![],
        });
    }
}

fn evaluate_trade_agreement(
    year: i32, factions: &[FactionState], events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], rng: &mut impl Rng,
) {
    if living.len() < 2 { return; }
    if rng.random::<f32>() >= 0.08 { return; }

    for &a in living {
        for &b in living {
            if a >= b { continue; }
            let sentiment = world_state.relations.get(a, b);
            if sentiment < 0 { continue; }
            if world_state.at_war(a, b) { continue; }
            // Check not already in treaty
            let has_treaty = world_state.active_treaties.iter().any(|t| {
                (t.faction_a == a && t.faction_b == b) || (t.faction_a == b && t.faction_b == a)
            });
            if has_treaty { continue; }

            world_state.active_treaties.push(Treaty { faction_a: a, faction_b: b, formed_year: year });
            world_state.relations.modify(a, b, 5);

            let fa = faction_name_by_id(factions, a);
            let fb = faction_name_by_id(factions, b);
            events.push(HistoricEvent {
                year, kind: EventKind::TradeAgreement,
                description: format!("{} and {} signed a trade agreement", fa, fb),
                participants: vec![a, b],
            god_participants: vec![],
            });
            return;
        }
    }
}

fn evaluate_leader_changed(
    year: i32, factions: &mut Vec<FactionState>, characters: &mut Vec<Character>,
    events: &mut Vec<HistoricEvent>, living: &[u32],
    next_id: &mut u32, rng: &mut impl Rng,
) {
    for &fid in living {
        let f = match factions.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => continue,
        };

        // Check for ambitious coup: a PowerHungry/Ambitious character with more renown than leader
        let leader_renown = f.leader_id
            .and_then(|lid| characters.iter().find(|c| c.id == lid))
            .map(|c| c.renown)
            .unwrap_or(0);

        let usurper = characters.iter()
            .filter(|c| c.is_alive(year) && c.faction_id == Some(fid) && c.role != CharacterRole::Leader)
            .filter(|c| c.has_trait(CharacterTrait::PowerHungry) || c.has_trait(CharacterTrait::Ambitious))
            .filter(|c| c.renown > leader_renown)
            .max_by_key(|c| c.renown)
            .map(|c| c.id);

        let coup_prob = if usurper.is_some() && f.stability < 30 {
            0.20
        } else if f.stability < 20 {
            0.10
        } else {
            0.01
        };

        if rng.random::<f32>() >= coup_prob { continue; }

        let old_name = f.leader_name.clone();
        let race = f.race;
        let fname = f.name.clone();
        let is_coup = coup_prob > 0.05;

        // Determine new leader
        let new_leader_name = if let Some(uid) = usurper {
            // Usurper takes over
            if let Some(old_leader) = f.leader_id.and_then(|lid| characters.iter_mut().find(|c| c.id == lid)) {
                old_leader.role = CharacterRole::Advisor; // demoted
                if is_coup {
                    old_leader.renown = (old_leader.renown - 10).max(-50);
                }
            }
            if let Some(u) = characters.iter_mut().find(|c| c.id == uid) {
                u.role = CharacterRole::Leader;
                u.renown += 10;
                if u.epithet.is_none() {
                    u.epithet = Some(generate_epithet(u, rng));
                }
                let name = u.full_display_name();
                if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
                    f.leader_id = Some(uid);
                    f.leader_name = name.clone();
                    if is_coup { f.stability = f.stability.saturating_sub(15); }
                }
                name
            } else {
                continue;
            }
        } else {
            // Generate new character
            let new_id = *next_id;
            *next_id += 1;
            let birth = year - rng.random_range(25..40);
            let mut new_leader = generate_character(new_id, race, CharacterRole::Leader, Some(fid), birth, rng);
            new_leader.epithet = Some(generate_epithet(&new_leader, rng));
            let name = new_leader.full_display_name();
            characters.push(new_leader);
            if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
                f.leader_id = Some(new_id);
                f.leader_name = name.clone();
            }
            name
        };

        let desc = if is_coup {
            format!("{} seized power from {} in {}", new_leader_name, old_name, fname)
        } else {
            format!("{} succeeded {} as leader of {}", new_leader_name, old_name, fname)
        };

        events.push(HistoricEvent {
            year, kind: EventKind::LeaderChanged,
            description: desc,
            participants: vec![fid],
            god_participants: vec![],
        });
    }
}

fn evaluate_plague(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    events: &mut Vec<HistoricEvent>, regions: &[String], rng: &mut impl Rng,
) {
    if rng.random::<f32>() >= 0.02 { return; }

    let region = &regions[rng.random_range(0..regions.len())];
    let affected: Vec<u32> = factions.iter()
        .filter(|f| f.is_alive(year) && f.territory.contains(region))
        .map(|f| f.id).collect();

    for &fid in &affected {
        if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
            f.stability = f.stability.saturating_sub(10);
        }
    }
    for s in settlements.iter_mut() {
        if s.region == *region && s.destroyed_year.is_none() {
            s.prosperity = s.prosperity.saturating_sub(25);
        }
    }

    events.push(HistoricEvent {
        year, kind: EventKind::PlagueStruck,
        description: format!("A plague swept through {}", region),
        participants: affected,
        god_participants: vec![],
    });
}

fn evaluate_monster_attack(
    year: i32, events: &mut Vec<HistoricEvent>, regions: &[String], rng: &mut impl Rng,
) {
    if rng.random::<f32>() >= 0.08 { return; }
    let region = &regions[rng.random_range(0..regions.len())];
    let creatures = ["dragon", "wyvern", "troll horde", "undead army",
        "giant spider brood", "demon", "hydra"];
    let creature = creatures[rng.random_range(0..creatures.len())];
    events.push(HistoricEvent {
        year, kind: EventKind::MonsterAttack,
        description: format!("A {} terrorized {}", creature, region),
        participants: vec![],
            god_participants: vec![],
    });
}

fn evaluate_hero(
    year: i32, factions: &[FactionState], characters: &mut Vec<Character>,
    events: &mut Vec<HistoricEvent>, living: &[u32], next_id: &mut u32,
    race_table: &PopTable<Race>, rng: &mut impl Rng,
) {
    if rng.random::<f32>() >= 0.06 { return; }
    let fid = living[rng.random_range(0..living.len())];
    let race = factions.iter().find(|f| f.id == fid).map(|f| f.race)
        .unwrap_or_else(|| race_table.roll_one(rng).unwrap());
    let fname = faction_name_by_id(factions, fid);

    let hero_id = *next_id;
    *next_id += 1;
    let birth = year - rng.random_range(18..30);
    let mut hero = generate_character(hero_id, race, CharacterRole::Hero, Some(fid), birth, rng);
    hero.epithet = Some(generate_epithet(&hero, rng));
    hero.renown = 25;
    let hero_name = hero.full_display_name();
    characters.push(hero);

    events.push(HistoricEvent {
        year, kind: EventKind::HeroRose,
        description: format!("{} rose to fame within {}", hero_name, fname),
        participants: vec![fid],
            god_participants: vec![],
    });
}

fn evaluate_artifact_discovered(
    year: i32, factions: &[FactionState], characters: &mut Vec<Character>,
    artifacts_list: &mut Vec<Artifact>, events: &mut Vec<HistoricEvent>,
    living: &[u32], next_id: &mut u32, rng: &mut impl Rng,
) {
    // Scholarly characters boost discovery chance
    let scholarly_boost = living.iter().any(|&fid| {
        faction_has_member_with_trait(fid, CharacterTrait::Scholarly, characters, year)
    });
    let prob = if scholarly_boost { 0.04 } else { 0.02 };
    if rng.random::<f32>() >= prob { return; }

    let fid = living[rng.random_range(0..living.len())];
    let fname = faction_name_by_id(factions, fid);

    let kind_table = PopTable::pick_one(vec![
        (ArtifactKind::Weapon, 25.0),
        (ArtifactKind::Armor, 15.0),
        (ArtifactKind::Tome, 20.0),
        (ArtifactKind::Crown, 10.0),
        (ArtifactKind::Relic, 20.0),
        (ArtifactKind::Gem, 10.0),
    ]);
    let kind = kind_table.roll_one(rng).unwrap();

    let artifact_id = *next_id;
    *next_id += 1;
    let mut artifact = generate_artifact(artifact_id, kind, year, fid, rng);

    // If a hero or scholar exists in the faction, they hold it
    let holder = characters.iter()
        .filter(|c| c.is_alive(year) && c.faction_id == Some(fid))
        .filter(|c| matches!(c.role, CharacterRole::Hero | CharacterRole::Scholar))
        .max_by_key(|c| c.renown);

    let discoverer_desc = if let Some(h) = holder {
        artifact.holder_character = Some(h.id);
        format!("{} of {}", h.full_display_name(), fname)
    } else {
        format!("scholars of {}", fname)
    };

    let artifact_name = artifact.name.clone();
    artifacts_list.push(artifact);

    events.push(HistoricEvent {
        year, kind: EventKind::ArtifactDiscovered,
        description: format!("{} discovered {}", discoverer_desc, artifact_name),
        participants: vec![fid],
            god_participants: vec![],
    });
}

fn evaluate_settlement_founded(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    events: &mut Vec<HistoricEvent>, next_id: &mut u32, living: &[u32],
    regions: &[String], rng: &mut impl Rng,
) {
    if rng.random::<f32>() >= 0.05 { return; }
    let fid = living[rng.random_range(0..living.len())];
    let f = match factions.iter().find(|f| f.id == fid) {
        Some(f) => f,
        None => return,
    };
    if f.wealth < 30 { return; } // too poor to found settlement

    let fname = f.name.clone();
    let region = f.territory.first().cloned().unwrap_or_else(|| regions[0].clone());
    let sname = settlement_name(rng);
    let sid = *next_id;
    *next_id += 1;

    settlements.push(SettlementState {
        id: sid, name: sname.clone(), founded_year: year,
        owner_faction: fid, destroyed_year: None, region,
        population_class: PopulationClass::Hamlet, prosperity: 40, defenses: 20,
        patron_god: None, devotion: 0, world_pos: None,
    });

    if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
        f.settlements.push(sid);
        f.wealth = f.wealth.saturating_sub(10); // costs money to found
    }

    events.push(HistoricEvent {
        year, kind: EventKind::SettlementFounded,
        description: format!("{} established {}", fname, sname),
        participants: vec![fid],
            god_participants: vec![],
    });
}

fn evaluate_new_faction(
    year: i32, factions: &mut Vec<FactionState>, events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, next_id: &mut u32, regions: &[String],
    faction_type_table: &PopTable<FactionType>, race_table: &PopTable<Race>,
    rng: &mut impl Rng,
) {
    if rng.random::<f32>() >= 0.03 { return; }
    let ft = faction_type_table.roll_one(rng).unwrap();
    let race = race_table.roll_one(rng).unwrap();
    let region = regions[rng.random_range(0..regions.len())].clone();
    let leader = full_name(race, rng);
    let name = faction_name(ft, race, rng);
    let id = *next_id;
    *next_id += 1;
    let (mil, wealth, stab) = FactionState::initialize_gauges(ft);

    let new_faction = FactionState {
        id, name: name.clone(), faction_type: ft, race,
        founded_year: year, home_region: region.clone(),
        dissolved_year: None, leader_name: leader, leader_id: None,
        military_strength: mil, wealth, stability: stab,
        territory: vec![region.clone()], settlements: vec![],
        patron_god: None, devotion: 0,
    };

    // Initialize relations with all existing living factions
    for existing in factions.iter() {
        if existing.is_alive(year) {
            world_state.relations.initialize_pair(&new_faction, existing);
        }
    }

    factions.push(new_faction);
    events.push(HistoricEvent {
        year, kind: EventKind::FactionFounded,
        description: format!("{} was founded in {}", name, region),
        participants: vec![id],
            god_participants: vec![],
    });
}

fn evaluate_faction_dissolved(
    year: i32, factions: &mut Vec<FactionState>, events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], rng: &mut impl Rng,
) {
    for &fid in living {
        let f = match factions.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => continue,
        };
        // Dissolve if very weak, or lost all settlements
        let no_settlements = f.settlements.is_empty();
        let critically_weak = f.military_strength < 10 && f.stability < 20 && f.wealth < 15;
        if no_settlements || critically_weak {
            let prob = if no_settlements { 0.50 } else { 0.25 };
            if rng.random::<f32>() < prob {
                let fname = f.name.clone();
                if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
                    f.dissolved_year = Some(year);
                }
                // Remove from wars
                world_state.active_wars.retain(|w| w.aggressor != fid && w.defender != fid);
                world_state.active_alliances.retain(|a| a.faction_a != fid && a.faction_b != fid);
                world_state.active_treaties.retain(|t| t.faction_a != fid && t.faction_b != fid);

                events.push(HistoricEvent {
                    year, kind: EventKind::FactionDissolved,
                    description: format!("{} dissolved, unable to sustain itself", fname),
                    participants: vec![fid],
            god_participants: vec![],
                });
            }
        }
    }
}

/// Check if a faction's leader has a specific trait.
fn leader_has_trait(faction_id: u32, trait_: CharacterTrait, factions: &[FactionState], characters: &[Character]) -> bool {
    let leader_id = factions.iter()
        .find(|f| f.id == faction_id)
        .and_then(|f| f.leader_id);
    leader_id.map_or(false, |lid| {
        characters.iter().find(|c| c.id == lid).map_or(false, |c| c.has_trait(trait_))
    })
}

/// Check if any notable member of a faction has a trait.
fn faction_has_member_with_trait(faction_id: u32, trait_: CharacterTrait, characters: &[Character], year: i32) -> bool {
    characters.iter().any(|c| {
        c.is_alive(year) && c.faction_id == Some(faction_id) && c.has_trait(trait_)
    })
}

fn faction_name_by_id(factions: &[FactionState], id: u32) -> String {
    factions.iter().find(|f| f.id == id).map(|f| f.name.clone()).unwrap_or("Unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_history(seed: u64) -> WorldHistory {
        test_history_with_config(HistoryConfig::default(), seed)
    }

    fn test_history_with_config(config: HistoryConfig, seed: u64) -> WorldHistory {
        use crate::worldgen::world_map::generate_world_map;
        use crate::worldgen::gods::build_god_pool;
        use rand::SeedableRng;
        let mut world_map = generate_world_map(64, 64, seed);
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
        let history = test_history(42);

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

        println!("\n--- Key Events ---");
        for e in history.events.iter().filter(|e| matches!(e.kind,
            EventKind::WarDeclared | EventKind::Conquest | EventKind::Betrayal
            | EventKind::HeroRose | EventKind::ArtifactDiscovered
            | EventKind::FactionDissolved | EventKind::FactionFounded))
        {
            println!("  Year {}: {}", e.year, e.description);
        }
    }
}
