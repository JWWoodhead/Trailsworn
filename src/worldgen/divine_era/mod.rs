pub mod artifacts;
pub mod creatures;
pub mod events;
pub mod personality;
pub mod races;
pub mod sites;
pub mod state;
pub mod terrain_scars;
pub mod the_fall;

pub use state::DivineHistory;
pub use terrain_scars::DivineTerrainType;

use std::collections::BTreeSet;

use rand::{Rng, RngExt, SeedableRng};

use crate::worldgen::gods::{DrawnPantheon, GodId, GodPool};
use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::names::{full_name, Race};
use crate::worldgen::population_table::PopTable;
use crate::worldgen::world_map::{WorldMap, WorldPos};
use crate::worldgen::zone::ZoneType;

use artifacts::{divine_artifact_name, ArtifactLocation, DivineArtifact, DivineArtifactKind};
use events::{DivineEvent, DivineEventKind};
use races::CreatedRace;
use sites::{divine_site_name, DivineSite, DivineSiteKind};
use state::{DivineRelationMatrix, DivinePact, DivineWar, DivineWorldState, GodState, PactKind};
use terrain_scars::{TerrainScar, TerrainScarCause};


/// Configuration for the divine era simulation.
pub struct DivineEraConfig {
    pub num_years: i32,
    pub start_year: i32,
    /// Base cells claimed per god per year.
    pub territory_expansion_rate: u32,
    /// Relationship below this triggers potential war.
    pub conflict_threshold: i32,
    /// Window (start_year_offset, end_year_offset) during which gods can create races.
    pub race_creation_window: (i32, i32),
}

impl Default for DivineEraConfig {
    fn default() -> Self {
        Self {
            num_years: 100,
            start_year: -100,
            territory_expansion_rate: 80,
            conflict_threshold: -30,
            race_creation_window: (10, 60),
        }
    }
}

/// Generate the divine era history, modifying the world map in place.
pub fn generate_divine_era(
    config: &DivineEraConfig,
    world_map: &mut WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    seed: u64,
) -> DivineHistory {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut next_id = 1u32;

    // Build initial god states with personalities rolled from domain + traits
    let mut gods: Vec<GodState> = pantheon
        .god_ids
        .iter()
        .map(|&id| {
            let domain = god_pool.get(id).map(|d| d.domain).unwrap_or(crate::resources::magic::MagicSchool::Arcane);
            let traits = pantheon.traits(id);
            let p = personality::roll_personality(domain, traits, &mut rng);
            GodState::new(id, p)
        })
        .collect();

    // Build divine settlements from world map settlement cells
    let divine_settlements: Vec<state::DivineSettlement> = world_map.cells.iter().enumerate()
        .filter(|(_, c)| c.settlement_name.is_some())
        .map(|(i, c)| {
            let x = (i as u32) % world_map.width;
            let y = (i as u32) / world_map.width;
            state::DivineSettlement {
                pos: WorldPos::new(x as i32, y as i32),
                name: c.settlement_name.clone().unwrap(),
                patron_god: None,
                devotion: 0,
                patron_since: None,
            }
        })
        .collect();

    // Initialize territory map (parallel to world_map.cells)
    let map_size = world_map.cells.len();
    let mut world_state = DivineWorldState {
        relations: DivineRelationMatrix::from_relationships(&pantheon.relationships),
        active_wars: Vec::new(),
        active_pacts: Vec::new(),
        territory_map: vec![None; map_size],
        settlements: divine_settlements,
    };

    // Build frontiers for BFS territory expansion
    let mut frontiers: Vec<BTreeSet<WorldPos>> = vec![BTreeSet::new(); gods.len()];

    // Assign starting seats of power
    assign_seats_of_power(&mut gods, &mut frontiers, &mut world_state, world_map, god_pool, pantheon, &mut rng);

    let mut events: Vec<DivineEvent> = Vec::new();
    let mut sites_list: Vec<DivineSite> = Vec::new();
    let mut artifacts_list: Vec<DivineArtifact> = Vec::new();
    let mut created_races: Vec<CreatedRace> = Vec::new();
    let mut terrain_scars: Vec<TerrainScar> = Vec::new();

    // Year-by-year simulation
    for year in config.start_year..config.start_year + config.num_years {
        simulate_divine_year(
            year,
            config,
            &mut gods,
            &mut frontiers,
            &mut events,
            &mut sites_list,
            &mut artifacts_list,
            &mut created_races,
            &mut terrain_scars,
            &mut world_state,
            world_map,
            god_pool,
            pantheon,
            &mut next_id,
            &mut rng,
        );
    }

    // Derive the Fall from final state
    let the_fall = the_fall::derive_the_fall(
        &gods,
        &world_state,
        &events,
        &artifacts_list,
        &mut terrain_scars,
        world_map,
        pantheon,
        config.start_year + config.num_years,
        &mut rng,
    );

    // Write final divine_owner to world map cells
    for (i, owner) in world_state.territory_map.iter().enumerate() {
        if let Some(god_id) = owner {
            world_map.cells[i].divine_owner = Some(*god_id);
        }
    }

    DivineHistory {
        gods,
        events,
        sites: sites_list,
        artifacts: artifacts_list,
        created_races,
        terrain_scars,
        current_year: config.start_year + config.num_years,
        the_fall: Some(the_fall),
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Assign each god a starting seat of power based on terrain affinity.
fn assign_seats_of_power(
    gods: &mut [GodState],
    frontiers: &mut [BTreeSet<WorldPos>],
    world_state: &mut DivineWorldState,
    world_map: &WorldMap,
    god_pool: &GodPool,
    _pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    let mut used_positions: Vec<WorldPos> = Vec::new();
    let min_distance = 40i32; // minimum distance between seats

    for (gi, god) in gods.iter_mut().enumerate() {
        let god_def = match god_pool.get(god.god_id) {
            Some(d) => d,
            None => continue,
        };

        // Find the ZoneType that matches this god's primary terrain
        let preferred_zone = terrain_to_zone(god_def.terrain_influence.primary_terrain);

        // Collect candidate cells: matching zone type, on land, far from other seats
        let mut candidates: Vec<WorldPos> = Vec::new();
        for y in 0..world_map.height as i32 {
            for x in 0..world_map.width as i32 {
                let pos = WorldPos::new(x, y);
                let cell = match world_map.get(pos) {
                    Some(c) => c,
                    None => continue,
                };
                if cell.zone_type == ZoneType::Ocean { continue; }

                let far_enough = used_positions.iter().all(|&used| {
                    let dx = (pos.x - used.x).abs();
                    let dy = (pos.y - used.y).abs();
                    dx + dy >= min_distance
                });
                if !far_enough { continue; }

                if cell.zone_type == preferred_zone {
                    // Primary match — add multiple times for weighting
                    candidates.push(pos);
                    candidates.push(pos);
                    candidates.push(pos);
                } else if cell.zone_type != ZoneType::Mountain {
                    candidates.push(pos);
                }
            }
        }

        if candidates.is_empty() {
            // Fallback: any non-ocean cell far enough
            for y in 0..world_map.height as i32 {
                for x in 0..world_map.width as i32 {
                    let pos = WorldPos::new(x, y);
                    if let Some(cell) = world_map.get(pos) {
                        if cell.zone_type != ZoneType::Ocean {
                            let far_enough = used_positions.iter().all(|&used| {
                                (pos.x - used.x).abs() + (pos.y - used.y).abs() >= min_distance / 2
                            });
                            if far_enough {
                                candidates.push(pos);
                            }
                        }
                    }
                }
            }
        }

        if candidates.is_empty() { continue; }

        let seat = candidates[rng.random_range(0..candidates.len())];
        god.seat_of_power = Some(seat);
        god.territory.push(seat);
        god.core_territory.push(seat);
        used_positions.push(seat);

        // Mark in territory map
        if let Some(idx) = world_map.idx(seat) {
            world_state.territory_map[idx] = Some(god.god_id);
        }

        // Initialize frontier with neighbors of the seat
        for neighbor in seat.neighbors() {
            if world_map.is_passable(neighbor) {
                frontiers[gi].insert(neighbor);
            }
        }
    }
}

/// Map a TerrainType to the most fitting ZoneType for seat selection.
fn terrain_to_zone(terrain: crate::terrain::TerrainType) -> ZoneType {
    use crate::terrain::TerrainType;
    match terrain {
        TerrainType::Grass => ZoneType::Grassland,
        TerrainType::Forest => ZoneType::Forest,
        TerrainType::Sand => ZoneType::Desert,
        TerrainType::Snow => ZoneType::Tundra,
        TerrainType::Swamp => ZoneType::Swamp,
        TerrainType::Stone | TerrainType::Mountain => ZoneType::Mountain,
        TerrainType::Water => ZoneType::Coast,
        TerrainType::Dirt => ZoneType::Grassland,
    }
}

// ---------------------------------------------------------------------------
// Year simulation
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn simulate_divine_year(
    year: i32,
    config: &DivineEraConfig,
    gods: &mut Vec<GodState>,
    frontiers: &mut Vec<BTreeSet<WorldPos>>,
    events: &mut Vec<DivineEvent>,
    sites: &mut Vec<DivineSite>,
    artifacts: &mut Vec<DivineArtifact>,
    created_races: &mut Vec<CreatedRace>,
    terrain_scars: &mut Vec<TerrainScar>,
    world_state: &mut DivineWorldState,
    world_map: &mut WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    next_id: &mut u32,
    rng: &mut impl Rng,
) {
    let active_ids: Vec<GodId> = gods.iter().filter(|g| g.is_active()).map(|g| g.god_id).collect();
    if active_ids.is_empty() { return; }

    let event_count_before = events.len();

    // Phase 0: Power recovery — scales with worshippers
    for god in gods.iter_mut() {
        if !god.is_active() { continue; }
        let worship_bonus = god.worshipper_settlements.len() as u32;
        god.power = (god.power + 2 + worship_bonus).min(100);
    }

    // Phase 1: Territory expansion (all gods do this, drive affects aggression)
    evaluate_territory_expansion(
        year, config, gods, frontiers, events, world_state, world_map, god_pool, pantheon, rng,
    );

    // Phase 2: Terrain shaping
    evaluate_terrain_shaping(
        year, gods, events, terrain_scars, world_state, world_map, god_pool, pantheon, next_id, rng,
    );

    // Phase 3: Worship competition (core resource — all gods participate)
    evaluate_worship(year, gods, events, world_state, world_map, pantheon, rng);

    // Phase 4: Drive-motivated actions — each god pursues what they want most
    evaluate_drive_actions(
        year, config, gods, events, sites, artifacts, created_races,
        world_state, world_map, god_pool, pantheon, next_id, rng,
    );

    // Phase 5: Divine conflict (driven by relationships + drives like Supremacy/Dominion/Vindication)
    evaluate_divine_war_declared(year, gods, events, world_state, &active_ids, pantheon, rng);
    evaluate_divine_war_resolution(
        year, gods, events, terrain_scars, world_state, world_map, god_pool, pantheon, next_id, rng,
    );
    evaluate_divine_pact(year, gods, events, world_state, &active_ids, pantheon, rng);
    evaluate_pact_broken(year, gods, events, world_state, pantheon, rng);

    // Phase 6: Flaw pressure — builds from events, triggers reactive flaw behavior
    evaluate_flaw_triggers(year, gods, events, world_state, pantheon, rng);

    // Phase 7: Upkeep & drift
    for i in 0..gods.len() {
        if !gods[i].is_active() { continue; }
        for j in (i + 1)..gods.len() {
            if !gods[j].is_active() { continue; }
            let a_id = gods[i].god_id;
            let b_id = gods[j].god_id;
            let shares_border = frontiers[i].iter().any(|pos| {
                if let Some(idx) = world_map.idx(*pos) {
                    world_state.territory_map[idx] == Some(b_id)
                } else {
                    false
                }
            });
            if shares_border {
                world_state.relations.modify(a_id, b_id, -1);
            }
        }
    }

    for war in &world_state.active_wars {
        if let Some(g) = gods.iter_mut().find(|g| g.god_id == war.aggressor) {
            g.power = g.power.saturating_sub(5);
        }
        if let Some(g) = gods.iter_mut().find(|g| g.god_id == war.defender) {
            g.power = g.power.saturating_sub(5);
        }
    }

    world_state.relations.drift_toward_neutral();

    // Build flaw pressure from this year's events
    let new_events = &events[event_count_before..];
    accumulate_flaw_pressure(gods, new_events, world_state, pantheon);
}

// ---------------------------------------------------------------------------
// Phase 1: Territory Expansion
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn evaluate_territory_expansion(
    year: i32,
    config: &DivineEraConfig,
    gods: &mut [GodState],
    frontiers: &mut [BTreeSet<WorldPos>],
    events: &mut Vec<DivineEvent>,
    world_state: &mut DivineWorldState,
    world_map: &WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    // Process gods in sorted order for determinism
    let mut god_indices: Vec<usize> = (0..gods.len()).filter(|&i| gods[i].is_active()).collect();
    // Shuffle to avoid first-mover advantage
    for i in (1..god_indices.len()).rev() {
        let j = rng.random_range(0..=i);
        god_indices.swap(i, j);
    }

    for gi in god_indices {
        let god = &gods[gi];
        let god_id = god.god_id;
        let god_def = match god_pool.get(god_id) { Some(d) => d, None => continue };
        let power_mod = god.power as f32 / 80.0; // 1.0 at starting power
        let cells_to_claim = ((config.territory_expansion_rate as f32 * power_mod) as u32).max(1);

        let preferred_zone = terrain_to_zone(god_def.terrain_influence.primary_terrain);
        let secondary_zone = god_def.terrain_influence.secondary_terrain.map(|t| terrain_to_zone(t));

        // Build weighted candidates from frontier
        let frontier = &frontiers[gi];
        let mut candidates: Vec<(WorldPos, u32)> = Vec::new();
        for &pos in frontier.iter() {
            if let Some(idx) = world_map.idx(pos) {
                let cell = &world_map.cells[idx];
                if cell.zone_type == ZoneType::Ocean { continue; }

                // Check if already claimed
                if let Some(owner) = world_state.territory_map[idx] {
                    if owner != god_id {
                        // Contested — don't claim, but register friction
                        world_state.relations.modify(god_id, owner, -5);
                        if rng.random::<f32>() < 0.1 {
                            let god_name = pantheon.name(god_id).unwrap_or("Unknown");
                            let other_name = pantheon.name(owner).unwrap_or("Unknown");
                            events.push(DivineEvent {
                                year,
                                kind: DivineEventKind::TerritoryContested,
                                description: format!(
                                    "{} and {} clashed over territory",
                                    god_name, other_name
                                ),
                                participants: vec![god_id, owner],
                            });
                        }
                    }
                    continue;
                }

                let weight = if cell.zone_type == preferred_zone {
                    3
                } else if secondary_zone == Some(cell.zone_type) {
                    2
                } else {
                    1
                };
                candidates.push((pos, weight));
            }
        }

        if candidates.is_empty() { continue; }

        // Sort by weight descending (preferred terrain first), then shuffle within weight groups
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        // Shuffle within each weight group for variety
        let mut start = 0;
        while start < candidates.len() {
            let w = candidates[start].1;
            let end = candidates[start..].iter().position(|(_, cw)| *cw != w)
                .map(|p| start + p).unwrap_or(candidates.len());
            for i in (start + 1..end).rev() {
                let j = rng.random_range(start..=i);
                candidates.swap(i, j);
            }
            start = end;
        }

        // Take up to cells_to_claim from sorted candidates
        let claimed_this_year: Vec<WorldPos> = candidates.iter()
            .take(cells_to_claim as usize)
            .map(|(pos, _)| *pos)
            .collect();

        // Apply claims
        let god = &mut gods[gi];
        for pos in &claimed_this_year {
            if let Some(idx) = world_map.idx(*pos) {
                world_state.territory_map[idx] = Some(god_id);
            }
            god.territory.push(*pos);
            if god.core_territory.len() < 20 {
                god.core_territory.push(*pos);
            }

            // Update frontier: remove claimed cell, add its unclaimed neighbors
            frontiers[gi].remove(pos);
            for neighbor in pos.neighbors() {
                if !world_map.in_bounds(neighbor) { continue; }
                if let Some(idx) = world_map.idx(neighbor) {
                    if world_state.territory_map[idx].is_none()
                        && world_map.cells[idx].zone_type != ZoneType::Ocean
                    {
                        frontiers[gi].insert(neighbor);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 2: Terrain Shaping
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn evaluate_terrain_shaping(
    year: i32,
    gods: &[GodState],
    events: &mut Vec<DivineEvent>,
    terrain_scars: &mut Vec<TerrainScar>,
    _world_state: &DivineWorldState,
    world_map: &mut WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    next_id: &mut u32,
    rng: &mut impl Rng,
) {
    for god in gods.iter() {
        if !god.is_active() { continue; }
        let god_def = match god_pool.get(god.god_id) { Some(d) => d, None => continue };
        let preferred_zone = terrain_to_zone(god_def.terrain_influence.primary_terrain);
        let divine_terrain = god_def.terrain_influence.future_terrain.as_deref()
            .and_then(DivineTerrainType::from_future_terrain);

        let mut shapings = 0u32;
        let max_shapings = 20;

        for &pos in &god.territory {
            if shapings >= max_shapings { break; }
            if rng.random::<f32>() >= 0.03 { continue; }

            if let Some(cell) = world_map.get_mut(pos) {
                if cell.zone_type != preferred_zone && cell.zone_type != ZoneType::Ocean {
                    cell.zone_type = preferred_zone;
                    shapings += 1;

                    // 10% chance of divine terrain scar
                    if let Some(dt) = divine_terrain {
                        if rng.random::<f32>() < 0.10 {
                            cell.divine_terrain = Some(dt);
                            let scar_id = *next_id;
                            *next_id += 1;
                            terrain_scars.push(TerrainScar {
                                id: scar_id,
                                world_pos: pos,
                                terrain_type: dt,
                                cause: TerrainScarCause::TerritoryShaping,
                                caused_year: year,
                                caused_by: vec![god.god_id],
                                description: format!(
                                    "{} shaped the land with divine power",
                                    pantheon.name(god.god_id).unwrap_or("A god")
                                ),
                            });
                        }
                    }
                }
            }
        }

        if shapings > 0 {
            events.push(DivineEvent {
                year,
                kind: DivineEventKind::TerrainShaped,
                description: format!(
                    "{} reshaped {} cells of their domain",
                    pantheon.name(god.god_id).unwrap_or("A god"),
                    shapings
                ),
                participants: vec![god.god_id],
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 3: Mortal Interaction
// ---------------------------------------------------------------------------

/// Gods compete for mortal worship. Settlements in a god's territory may begin
/// worshipping them; gods with Worship/Love/Dominion drives are more aggressive
/// about seeking worshippers. Settlements can also be converted away.
fn evaluate_worship(
    year: i32,
    gods: &mut [GodState],
    events: &mut Vec<DivineEvent>,
    world_state: &mut DivineWorldState,
    world_map: &WorldMap,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    use personality::DivineDrive;

    // For each settlement, check if a god can claim or convert it
    for si in 0..world_state.settlements.len() {
        let settlement_pos = world_state.settlements[si].pos;
        let current_patron = world_state.settlements[si].patron_god;
        let current_devotion = world_state.settlements[si].devotion;

        // Find which god's territory this settlement is in
        let territory_owner = world_map.idx(settlement_pos)
            .and_then(|idx| world_state.territory_map[idx]);

        let owner_god_id = match territory_owner {
            Some(id) => id,
            None => {
                // No god controls this area — devotion decays
                if current_devotion > 0 {
                    world_state.settlements[si].devotion = current_devotion.saturating_sub(2);
                }
                continue;
            }
        };

        // Is the territory owner active?
        let owner_active = gods.iter().any(|g| g.god_id == owner_god_id && g.is_active());
        if !owner_active { continue; }

        if current_patron == Some(owner_god_id) {
            // Already worshipping the territory owner — devotion grows
            let drive = gods.iter().find(|g| g.god_id == owner_god_id).map(|g| g.drive());
            let growth = match drive {
                Some(DivineDrive::Worship) => 5,
                Some(DivineDrive::Love) => 4,
                Some(DivineDrive::Dominion) => 3,
                _ => 2,
            };
            world_state.settlements[si].devotion = (current_devotion + growth).min(100);
        } else if current_patron.is_none() {
            // Unclaimed settlement in this god's territory — claim it
            let drive = gods.iter().find(|g| g.god_id == owner_god_id).map(|g| g.drive());
            let claim_prob = match drive {
                Some(DivineDrive::Worship) => 0.40,
                Some(DivineDrive::Love) => 0.30,
                Some(DivineDrive::Dominion) => 0.35,
                Some(DivineDrive::Legacy) => 0.25,
                _ => 0.15,
            };
            if rng.random::<f32>() < claim_prob {
                let sname = world_state.settlements[si].name.clone();
                world_state.settlements[si].patron_god = Some(owner_god_id);
                world_state.settlements[si].devotion = 20;
                world_state.settlements[si].patron_since = Some(year);

                // Update god's worshipper list
                if let Some(g) = gods.iter_mut().find(|g| g.god_id == owner_god_id) {
                    g.worshipper_settlements.push(settlement_pos);
                }

                let god_name = pantheon.name(owner_god_id).unwrap_or("A god");
                events.push(DivineEvent {
                    year,
                    kind: DivineEventKind::GiftBestowed,
                    description: format!("The people of {} began worshipping {}", sname, god_name),
                    participants: vec![owner_god_id],
                });
            }
        } else {
            // Settlement worships a different god — potential conversion
            // Only happens if devotion is low and territory owner is aggressive
            if current_devotion > 30 { continue; }

            let drive = gods.iter().find(|g| g.god_id == owner_god_id).map(|g| g.drive());
            let convert_prob = match drive {
                Some(DivineDrive::Dominion) => 0.20,
                Some(DivineDrive::Worship) => 0.15,
                Some(DivineDrive::Supremacy) => 0.15,
                Some(DivineDrive::Vindication) => 0.10,
                _ => 0.05,
            };

            if rng.random::<f32>() < convert_prob {
                let old_patron = current_patron.unwrap();
                let sname = world_state.settlements[si].name.clone();
                world_state.settlements[si].patron_god = Some(owner_god_id);
                world_state.settlements[si].devotion = 15;
                world_state.settlements[si].patron_since = Some(year);

                // Update worship lists
                if let Some(g) = gods.iter_mut().find(|g| g.god_id == old_patron) {
                    g.worshipper_settlements.retain(|p| *p != settlement_pos);
                }
                if let Some(g) = gods.iter_mut().find(|g| g.god_id == owner_god_id) {
                    g.worshipper_settlements.push(settlement_pos);
                }

                // This creates tension between gods
                world_state.relations.modify(owner_god_id, old_patron, -10);

                let god_name = pantheon.name(owner_god_id).unwrap_or("A god");
                let old_name = pantheon.name(old_patron).unwrap_or("another god");
                events.push(DivineEvent {
                    year,
                    kind: DivineEventKind::GiftBestowed,
                    description: format!(
                        "The people of {} abandoned {} and turned to {}",
                        sname, old_name, god_name
                    ),
                    participants: vec![owner_god_id, old_patron],
                });
            } else {
                // Devotion decays when under another god's influence
                world_state.settlements[si].devotion = current_devotion.saturating_sub(1);
            }
        }
    }

    // Update power based on worship — gods without worshippers fade
    for god in gods.iter_mut() {
        if !god.is_active() { continue; }
        if god.worshipper_settlements.is_empty() && god.power > 20 {
            // A god without worshippers slowly weakens
            god.power = god.power.saturating_sub(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 4: Drive-Motivated Actions
// ---------------------------------------------------------------------------

/// Each god chooses actions based on what they want most (their drive).
/// Instead of all gods rolling on every possible action, drive determines priority.
#[allow(clippy::too_many_arguments)]
fn evaluate_drive_actions(
    year: i32,
    config: &DivineEraConfig,
    gods: &mut Vec<GodState>,
    events: &mut Vec<DivineEvent>,
    sites: &mut Vec<DivineSite>,
    artifacts: &mut Vec<DivineArtifact>,
    created_races: &mut Vec<CreatedRace>,
    world_state: &mut DivineWorldState,
    world_map: &WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    next_id: &mut u32,
    rng: &mut impl Rng,
) {
    use personality::DivineDrive::*;

    // Each god gets a primary action based on drive + a chance at a secondary
    let god_count = gods.len();
    for gi in 0..god_count {
        if !gods[gi].is_active() { continue; }
        let drive = gods[gi].drive();
        let _god_id = gods[gi].god_id;

        match drive {
            // Knowledge gods seek to create observatories, study, forge implements
            Knowledge => {
                evaluate_sacred_site_for_god(year, gi, gods, events, sites, world_state, world_map, god_pool, pantheon, next_id, rng, 0.12);
                evaluate_artifact_for_god(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.10);
            }

            // Dominion gods demand submission, expand aggressively, build fortifications
            Dominion => {
                evaluate_champion_for_god(year, gi, gods, events, pantheon, rng, 0.08);
                evaluate_temple_for_god(year, gi, gods, events, sites, world_state, world_map, god_pool, pantheon, next_id, rng, 0.12);
            }

            // Worship gods build temples, gift mortals, choose champions
            Worship => {
                evaluate_temple_for_god(year, gi, gods, events, sites, world_state, world_map, god_pool, pantheon, next_id, rng, 0.15);
                evaluate_gift_for_god(year, gi, gods, events, world_state, world_map, pantheon, rng, 0.25);
                evaluate_champion_for_god(year, gi, gods, events, pantheon, rng, 0.08);
            }

            // Perfection gods forge artifacts and create masterwork sites
            Perfection => {
                evaluate_artifact_for_god(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.15);
                evaluate_sacred_site_for_god(year, gi, gods, events, sites, world_state, world_map, god_pool, pantheon, next_id, rng, 0.10);
            }

            // Justice gods establish temples (courts), choose champions (judges)
            Justice => {
                evaluate_temple_for_god(year, gi, gods, events, sites, world_state, world_map, god_pool, pantheon, next_id, rng, 0.10);
                evaluate_champion_for_god(year, gi, gods, events, pantheon, rng, 0.06);
            }

            // Love gods protect their people, choose champions, create races
            Love => {
                evaluate_gift_for_god(year, gi, gods, events, world_state, world_map, pantheon, rng, 0.20);
                evaluate_champion_for_god(year, gi, gods, events, pantheon, rng, 0.06);
                let year_offset = year - config.start_year;
                if year_offset >= config.race_creation_window.0 && year_offset <= config.race_creation_window.1 {
                    evaluate_race_for_god(year, gi, gods, events, created_races, god_pool, pantheon, next_id, rng, 0.10);
                }
            }

            // Freedom gods create hidden sites, forge keys/implements
            Freedom => {
                evaluate_sacred_site_for_god(year, gi, gods, events, sites, world_state, world_map, god_pool, pantheon, next_id, rng, 0.08);
                evaluate_artifact_for_god(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.06);
            }

            // Legacy gods create races, build monuments, teach mortals
            Legacy => {
                let year_offset = year - config.start_year;
                if year_offset >= config.race_creation_window.0 && year_offset <= config.race_creation_window.1 {
                    evaluate_race_for_god(year, gi, gods, events, created_races, god_pool, pantheon, next_id, rng, 0.12);
                }
                evaluate_sacred_site_for_god(year, gi, gods, events, sites, world_state, world_map, god_pool, pantheon, next_id, rng, 0.10);
                evaluate_gift_for_god(year, gi, gods, events, world_state, world_map, pantheon, rng, 0.15);
            }

            // Vindication gods forge weapons, choose champions, build monuments
            Vindication => {
                evaluate_artifact_for_god(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.12);
                evaluate_champion_for_god(year, gi, gods, events, pantheon, rng, 0.08);
            }

            // Supremacy gods forge weapons, choose champions, build war-sites
            Supremacy => {
                evaluate_artifact_for_god(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.10);
                evaluate_champion_for_god(year, gi, gods, events, pantheon, rng, 0.10);
            }
        }
    }
}

// --- Per-god action helpers (called by evaluate_drive_actions) ---

fn evaluate_gift_for_god(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<DivineEvent>,
    _world_state: &DivineWorldState, world_map: &WorldMap, pantheon: &DrawnPantheon,
    rng: &mut impl Rng, prob: f32,
) {
    let god = &gods[gi];
    if rng.random::<f32>() >= prob { return; }
    let has_settlement = god.territory.iter().any(|pos| {
        world_map.get(*pos).is_some_and(|c| c.settlement_name.is_some())
    });
    if has_settlement {
        let god_name = pantheon.name(god.god_id).unwrap_or("A god");
        events.push(DivineEvent {
            year,
            kind: DivineEventKind::GiftBestowed,
            description: format!("{} bestowed gifts upon the mortals of their domain", god_name),
            participants: vec![god.god_id],
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_temple_for_god(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<DivineEvent>,
    sites: &mut Vec<DivineSite>, _world_state: &DivineWorldState, _world_map: &WorldMap,
    _god_pool: &GodPool, pantheon: &DrawnPantheon, next_id: &mut u32,
    rng: &mut impl Rng, prob: f32,
) {
    let god = &gods[gi];
    if god.power < 40 { return; }
    if rng.random::<f32>() >= prob { return; }
    if god.territory.is_empty() { return; }

    let pos = god.territory[rng.random_range(0..god.territory.len())];
    let god_name = pantheon.name(god.god_id).unwrap_or("A god").to_string();
    let site_id = *next_id;
    *next_id += 1;
    let name = divine_site_name(DivineSiteKind::Temple, &god_name, rng);
    sites.push(DivineSite {
        id: site_id, name: name.clone(), kind: DivineSiteKind::Temple,
        world_pos: pos, creator_god: god.god_id, created_year: year,
        persists: true,
        description: format!("{} established {} as a center of worship", god_name, name),
        terrain_effect: None,
    });
    let god = &mut gods[gi];
    god.sites_created += 1;
    events.push(DivineEvent {
        year, kind: DivineEventKind::TempleEstablished,
        description: format!("{} founded {}", god_name, name),
        participants: vec![god.god_id],
    });
}

fn evaluate_champion_for_god(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<DivineEvent>,
    pantheon: &DrawnPantheon, rng: &mut impl Rng, prob: f32,
) {
    let god = &gods[gi];
    if god.power < 50 { return; }
    if god.champion_name.is_some() { return; }
    if rng.random::<f32>() >= prob { return; }

    let race_table = PopTable::pick_one(vec![
        (Race::Human, 40.0), (Race::Dwarf, 20.0), (Race::Elf, 15.0),
        (Race::Orc, 15.0), (Race::Goblin, 10.0),
    ]);
    let race = race_table.roll_one(rng).unwrap();
    let name = full_name(race, rng);
    let god_name = pantheon.name(god.god_id).unwrap_or("A god");

    let god = &mut gods[gi];
    god.champion_name = Some(name.clone());
    god.champion_race = Some(race);

    events.push(DivineEvent {
        year, kind: DivineEventKind::ChampionChosen,
        description: format!("{} chose {} as their mortal champion", god_name, name),
        participants: vec![god.god_id],
    });
}

#[allow(clippy::too_many_arguments)]
fn evaluate_race_for_god(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<DivineEvent>,
    created_races: &mut Vec<CreatedRace>, god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    let god = &gods[gi];
    if god.created_race_id.is_some() { return; }
    if god.power < 60 { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_def = match god_pool.get(god.god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god.god_id).unwrap_or("A god").to_string();
    let race_id = *next_id;
    *next_id += 1;
    let race = races::race_template(race_id, god_def, &god_name, year, &god.core_territory, rng);

    let god = &mut gods[gi];
    god.created_race_id = Some(race_id);
    events.push(DivineEvent {
        year, kind: DivineEventKind::RaceCreated,
        description: format!("{} created the {}", god_name, race.name),
        participants: vec![god.god_id],
    });
    created_races.push(race);
}

#[allow(clippy::too_many_arguments)]
fn evaluate_artifact_for_god(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<DivineEvent>,
    artifacts: &mut Vec<DivineArtifact>, god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    let god = &gods[gi];
    if god.power < 30 { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_def = match god_pool.get(god.god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god.god_id).unwrap_or("A god").to_string();

    let kind_table = PopTable::pick_one(vec![
        (DivineArtifactKind::Weapon, 30.0), (DivineArtifactKind::Armor, 25.0),
        (DivineArtifactKind::Implement, 20.0), (DivineArtifactKind::Key, 10.0),
        (DivineArtifactKind::Vessel, 15.0),
    ]);
    let kind = kind_table.roll_one(rng).unwrap();
    let power_level = (god.power / 20).clamp(1, 5);
    let name = divine_artifact_name(kind, god_def.domain, rng);
    let artifact_id = *next_id;
    *next_id += 1;

    let location = if god.champion_name.is_some() && rng.random::<f32>() < 0.3 {
        ArtifactLocation::HeldByChampion(god.god_id)
    } else {
        ArtifactLocation::Lost
    };

    artifacts.push(DivineArtifact {
        id: artifact_id, name: name.clone(), kind, creator_god: god.god_id,
        created_year: year, magic_school: god_def.domain, power_level, location,
        description: format!("{} forged {}", god_name, name),
        lore: format!("Created by {} in the divine era", god_name),
    });

    let god = &mut gods[gi];
    god.artifacts_created += 1;
    events.push(DivineEvent {
        year, kind: DivineEventKind::ArtifactForged,
        description: format!("{} forged the {}", god_name, name),
        participants: vec![god.god_id],
    });
}

#[allow(clippy::too_many_arguments)]
fn evaluate_sacred_site_for_god(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<DivineEvent>,
    sites: &mut Vec<DivineSite>, _world_state: &DivineWorldState, _world_map: &WorldMap,
    god_pool: &GodPool, pantheon: &DrawnPantheon, next_id: &mut u32,
    rng: &mut impl Rng, prob: f32,
) {
    let god = &gods[gi];
    if rng.random::<f32>() >= prob { return; }
    if god.territory.is_empty() { return; }

    let god_def = match god_pool.get(god.god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god.god_id).unwrap_or("A god").to_string();
    let kind = DivineSiteKind::for_domain(god_def.domain);
    let pos = god.territory[rng.random_range(0..god.territory.len())];
    let site_id = *next_id;
    *next_id += 1;
    let divine_terrain = god_def.terrain_influence.future_terrain.as_deref()
        .and_then(DivineTerrainType::from_future_terrain);
    let name = divine_site_name(kind, &god_name, rng);

    sites.push(DivineSite {
        id: site_id, name: name.clone(), kind, world_pos: pos,
        creator_god: god.god_id, created_year: year, persists: true,
        description: format!("{} created {}", god_name, name),
        terrain_effect: divine_terrain,
    });

    let god = &mut gods[gi];
    god.sites_created += 1;
    events.push(DivineEvent {
        year, kind: DivineEventKind::SacredSiteCreated,
        description: format!("{} created {}", god_name, name),
        participants: vec![god.god_id],
    });
}

// ---------------------------------------------------------------------------
// Phase 6: Flaw Pressure & Triggers
// ---------------------------------------------------------------------------

/// Build flaw pressure from events that happened this year.
/// Each god's flaw is sensitive to different kinds of events.
fn accumulate_flaw_pressure(
    gods: &mut [GodState],
    new_events: &[DivineEvent],
    world_state: &DivineWorldState,
    _pantheon: &DrawnPantheon,
) {
    use personality::DivineFlaw::*;

    // Collect active god IDs for Isolation calculation (avoids borrow conflict)
    let active_god_ids: Vec<GodId> = gods.iter().filter(|g| g.is_active()).map(|g| g.god_id).collect();

    // Compute pressure gains per god index
    let pressure_gains: Vec<(usize, u32)> = gods.iter().enumerate()
        .filter(|(_, g)| g.is_active())
        .map(|(gi, god)| {
            let flaw = god.flaw();
            let god_id = god.god_id;

            let gain: u32 = match flaw {
                Hubris => {
                    let victories = new_events.iter().filter(|e| {
                        e.participants.contains(&god_id)
                            && matches!(e.kind, DivineEventKind::DivineWarEnded | DivineEventKind::ArtifactForged | DivineEventKind::DomainAbsorbed)
                    }).count() as u32;
                    victories * 8
                }
                Jealousy => {
                    let others_gaining = new_events.iter().filter(|e| {
                        !e.participants.contains(&god_id)
                            && matches!(e.kind, DivineEventKind::GiftBestowed | DivineEventKind::ArtifactForged | DivineEventKind::RaceCreated)
                    }).count() as u32;
                    others_gaining * 5
                }
                Obsession => 3,
                Cruelty => {
                    let frustrations = new_events.iter().filter(|e| {
                        e.participants.contains(&god_id)
                            && matches!(e.kind, DivineEventKind::TerritoryContested | DivineEventKind::PactBroken)
                    }).count() as u32;
                    frustrations * 8
                }
                Blindness => 2,
                Isolation => {
                    let others: Vec<GodId> = active_god_ids.iter().copied().filter(|&id| id != god_id).collect();
                    let avg_sentiment: i32 = if others.is_empty() { 0 } else {
                        others.iter().map(|&id| world_state.relations.get(god_id, id)).sum::<i32>() / others.len() as i32
                    };
                    if avg_sentiment < 0 { (-avg_sentiment / 10) as u32 } else { 1 }
                }
                Betrayal => {
                    let has_pact = world_state.active_pacts.iter().any(|p| p.god_a == god_id || p.god_b == god_id);
                    if has_pact { 5 } else { 1 }
                }
                Sacrifice => {
                    if god.power < 40 { 5 } else { 1 }
                }
                Rigidity => {
                    let disruptions = new_events.iter().filter(|e| {
                        matches!(e.kind, DivineEventKind::DivineWarDeclared | DivineEventKind::GodVanquished | DivineEventKind::PactBroken)
                    }).count() as u32;
                    2 + disruptions * 3
                }
                Hollowness => {
                    let achievements = new_events.iter().filter(|e| {
                        e.participants.contains(&god_id)
                            && matches!(e.kind, DivineEventKind::ArtifactForged | DivineEventKind::SacredSiteCreated | DivineEventKind::DomainAbsorbed)
                    }).count() as u32;
                    achievements * 6
                }
            };
            (gi, gain)
        })
        .collect();

    // Apply pressure gains
    for (gi, gain) in pressure_gains {
        gods[gi].flaw_pressure = (gods[gi].flaw_pressure + gain).min(100);
    }
}

/// When flaw pressure exceeds 80, the flaw triggers — creating a dramatic event
/// that reflects the god's tragic nature.
fn evaluate_flaw_triggers(
    year: i32,
    gods: &mut [GodState],
    events: &mut Vec<DivineEvent>,
    world_state: &mut DivineWorldState,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    use personality::DivineFlaw::*;

    let god_count = gods.len();
    for gi in 0..god_count {
        if !gods[gi].is_active() { continue; }
        if gods[gi].flaw_pressure < 80 { continue; }
        // Flaw triggers — probability scales with pressure
        let trigger_prob = (gods[gi].flaw_pressure as f32 - 70.0) / 100.0;
        if rng.random::<f32>() >= trigger_prob { continue; }

        let flaw = gods[gi].flaw();
        let god_id = gods[gi].god_id;
        let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();

        match flaw {
            Hubris => {
                // Overreach — god loses power from trying too much
                gods[gi].power = gods[gi].power.saturating_sub(15);
                events.push(DivineEvent {
                    year, kind: DivineEventKind::NarrativeAdvanced,
                    description: format!("{}, drunk on their own power, overreached and was diminished", god_name),
                    participants: vec![god_id],
                });
            }
            Jealousy => {
                // Lash out at the most successful god — damage relationship
                let target = gods.iter()
                    .filter(|g| g.is_active() && g.god_id != god_id)
                    .max_by_key(|g| g.worshipper_settlements.len());
                if let Some(t) = target {
                    let target_id = t.god_id;
                    let target_name = pantheon.name(target_id).unwrap_or("another god").to_string();
                    world_state.relations.modify(god_id, target_id, -20);
                    events.push(DivineEvent {
                        year, kind: DivineEventKind::NarrativeAdvanced,
                        description: format!(
                            "{}, consumed by jealousy, turned against {} for having what they could not",
                            god_name, target_name
                        ),
                        participants: vec![god_id, target_id],
                    });
                }
            }
            Obsession => {
                // Neglect worshippers — devotion drops across all settlements
                for s in world_state.settlements.iter_mut() {
                    if s.patron_god == Some(god_id) {
                        s.devotion = s.devotion.saturating_sub(10);
                    }
                }
                events.push(DivineEvent {
                    year, kind: DivineEventKind::NarrativeAdvanced,
                    description: format!("{}, lost in obsession, neglected those who worshipped them", god_name),
                    participants: vec![god_id],
                });
            }
            Cruelty => {
                // Punish the wrong target — harm own worshippers
                for s in world_state.settlements.iter_mut() {
                    if s.patron_god == Some(god_id) {
                        s.devotion = s.devotion.saturating_sub(15);
                    }
                }
                events.push(DivineEvent {
                    year, kind: DivineEventKind::NarrativeAdvanced,
                    description: format!("{} lashed out in fury, and their own followers suffered for it", god_name),
                    participants: vec![god_id],
                });
            }
            Blindness => {
                // Accidentally harm another god's domain — damage relationship with random god
                let other = gods.iter()
                    .filter(|g| g.is_active() && g.god_id != god_id)
                    .nth(rng.random_range(0..gods.iter().filter(|g| g.is_active() && g.god_id != god_id).count().max(1)));
                if let Some(t) = other {
                    let target_id = t.god_id;
                    let target_name = pantheon.name(target_id).unwrap_or("another god").to_string();
                    world_state.relations.modify(god_id, target_id, -15);
                    events.push(DivineEvent {
                        year, kind: DivineEventKind::NarrativeAdvanced,
                        description: format!(
                            "{}, blind to the consequences, unknowingly trespassed against {}",
                            god_name, target_name
                        ),
                        participants: vec![god_id, target_id],
                    });
                }
            }
            Isolation => {
                // Withdraw — lose worshipper settlements as god becomes unreachable
                let lost: Vec<WorldPos> = gods[gi].worshipper_settlements.clone();
                for pos in &lost {
                    if let Some(s) = world_state.settlements.iter_mut().find(|s| s.pos == *pos && s.patron_god == Some(god_id)) {
                        s.devotion = s.devotion.saturating_sub(20);
                    }
                }
                events.push(DivineEvent {
                    year, kind: DivineEventKind::NarrativeAdvanced,
                    description: format!("{} withdrew from the world, becoming distant and unreachable", god_name),
                    participants: vec![god_id],
                });
            }
            Betrayal => {
                // Break a pact for personal gain
                let pact_idx = world_state.active_pacts.iter().position(|p| p.god_a == god_id || p.god_b == god_id);
                if let Some(idx) = pact_idx {
                    let pact = world_state.active_pacts.remove(idx);
                    let other_id = if pact.god_a == god_id { pact.god_b } else { pact.god_a };
                    let other_name = pantheon.name(other_id).unwrap_or("another god").to_string();
                    world_state.relations.modify(god_id, other_id, -30);
                    events.push(DivineEvent {
                        year, kind: DivineEventKind::PactBroken,
                        description: format!(
                            "{} betrayed {}, shattering the trust between them",
                            god_name, other_name
                        ),
                        participants: vec![god_id, other_id],
                    });
                }
            }
            Sacrifice => {
                // Give up territory or power for the drive
                gods[gi].power = gods[gi].power.saturating_sub(20);
                events.push(DivineEvent {
                    year, kind: DivineEventKind::NarrativeAdvanced,
                    description: format!("{} sacrificed a piece of themselves in pursuit of their deepest desire", god_name),
                    participants: vec![god_id],
                });
            }
            Rigidity => {
                // Refuse to adapt — alienate allies
                for other in gods.iter().filter(|g| g.is_active() && g.god_id != god_id) {
                    world_state.relations.modify(god_id, other.god_id, -5);
                }
                events.push(DivineEvent {
                    year, kind: DivineEventKind::NarrativeAdvanced,
                    description: format!("{} refused to bend, and the other gods grew weary of their inflexibility", god_name),
                    participants: vec![god_id],
                });
            }
            Hollowness => {
                // Achievement feels empty — god becomes listless, power drains
                gods[gi].power = gods[gi].power.saturating_sub(10);
                events.push(DivineEvent {
                    year, kind: DivineEventKind::NarrativeAdvanced,
                    description: format!("{} achieved what they sought, and found it meant nothing", god_name),
                    participants: vec![god_id],
                });
            }
        }

        // Reset pressure after trigger (but not to zero — flaws recur)
        gods[gi].flaw_pressure = 20;
    }
}


// ---------------------------------------------------------------------------
// Phase 4: Divine Conflict
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn evaluate_divine_war_declared(
    year: i32,
    gods: &[GodState],
    events: &mut Vec<DivineEvent>,
    world_state: &mut DivineWorldState,
    active_ids: &[GodId],
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    if active_ids.len() < 2 { return; }
    let Some((a, b, sentiment)) = world_state.relations.most_hostile_pair(active_ids) else { return };
    if sentiment >= -30 { return; }
    if world_state.at_war(a, b) { return; }
    if world_state.war_count(a) > 0 || world_state.war_count(b) > 0 { return; }

    // Base probability + trait modifiers
    let hostility_bonus = ((-sentiment - 30) as f32 * 0.8).min(40.0);
    let mut prob = 25.0 + hostility_bonus;

    let a_traits = pantheon.traits(a);
    if a_traits.contains(&CharacterTrait::Warlike) { prob += 20.0; }
    if a_traits.contains(&CharacterTrait::Ambitious) { prob += 10.0; }
    if a_traits.contains(&CharacterTrait::Peaceful) { prob -= 25.0; }
    if a_traits.contains(&CharacterTrait::Diplomatic) { prob -= 15.0; }

    // Power differential
    let a_power = gods.iter().find(|g| g.god_id == a).map(|g| g.power).unwrap_or(0);
    let b_power = gods.iter().find(|g| g.god_id == b).map(|g| g.power).unwrap_or(0);
    if a_power > b_power + 20 { prob += 15.0; }

    let prob = (prob / 100.0).clamp(0.05, 0.60);
    if rng.random::<f32>() >= prob { return; }

    // Declare war — contested cells are the border cells between their territories
    let a_territory: BTreeSet<WorldPos> = gods.iter()
        .find(|g| g.god_id == a)
        .map(|g| g.territory.iter().copied().collect())
        .unwrap_or_default();

    let contested: Vec<WorldPos> = a_territory.iter()
        .flat_map(|pos| pos.neighbors())
        .filter(|pos| {
            gods.iter().find(|g| g.god_id == b)
                .is_some_and(|g| g.territory.contains(pos))
        })
        .collect();

    world_state.active_wars.push(DivineWar {
        aggressor: a,
        defender: b,
        start_year: year,
        contested_cells: contested,
    });
    world_state.relations.modify(a, b, -20);

    let na = pantheon.name(a).unwrap_or("Unknown");
    let nb = pantheon.name(b).unwrap_or("Unknown");
    events.push(DivineEvent {
        year,
        kind: DivineEventKind::DivineWarDeclared,
        description: format!("{} declared war upon {}", na, nb),
        participants: vec![a, b],
    });

    // Update war counters
    if let Some(_g) = gods.iter().find(|g| g.god_id == a) {
        // Can't mutate through find since gods is borrowed — handled after
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_divine_war_resolution(
    year: i32,
    gods: &mut Vec<GodState>,
    events: &mut Vec<DivineEvent>,
    terrain_scars: &mut Vec<TerrainScar>,
    world_state: &mut DivineWorldState,
    world_map: &mut WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    next_id: &mut u32,
    rng: &mut impl Rng,
) {
    let mut ended_wars: Vec<usize> = Vec::new();

    for (i, war) in world_state.active_wars.iter().enumerate() {
        let duration = year - war.start_year;
        if duration < 2 { continue; }

        let a_power = gods.iter().find(|g| g.god_id == war.aggressor).map(|g| g.power).unwrap_or(0);
        let b_power = gods.iter().find(|g| g.god_id == war.defender).map(|g| g.power).unwrap_or(0);
        let weakness_bonus = if a_power < 20 || b_power < 20 { 0.30 } else { 0.0 };

        let prob = (0.15 + duration as f32 * 0.05 + weakness_bonus).min(0.90);
        if rng.random::<f32>() < prob {
            ended_wars.push(i);
        }
    }

    for &i in ended_wars.iter().rev() {
        let war = world_state.active_wars.remove(i);
        let a_power = gods.iter().find(|g| g.god_id == war.aggressor).map(|g| g.power).unwrap_or(0);
        let b_power = gods.iter().find(|g| g.god_id == war.defender).map(|g| g.power).unwrap_or(0);

        let (winner, loser) = if a_power >= b_power {
            (war.aggressor, war.defender)
        } else {
            (war.defender, war.aggressor)
        };

        let nw = pantheon.name(winner).unwrap_or("Unknown").to_string();
        let nl = pantheon.name(loser).unwrap_or("Unknown").to_string();

        // Power consequences
        if let Some(g) = gods.iter_mut().find(|g| g.god_id == winner) {
            g.power = g.power.saturating_sub(10);
            g.wars_fought += 1;
            g.wars_won += 1;
        }
        if let Some(g) = gods.iter_mut().find(|g| g.god_id == loser) {
            g.power = g.power.saturating_sub(20);
            g.wars_fought += 1;
        }

        // Check if loser is vanquished
        let loser_power = gods.iter().find(|g| g.god_id == loser).map(|g| g.power).unwrap_or(0);
        let vanquished = loser_power < 15;

        if vanquished {
            if let Some(g) = gods.iter_mut().find(|g| g.god_id == loser) {
                g.active = false;
                g.vanquished_year = Some(year);
            }

            events.push(DivineEvent {
                year,
                kind: DivineEventKind::GodVanquished,
                description: format!("{} was vanquished by {}", nl, nw),
                participants: vec![loser, winner],
            });

            // Transfer 60% of loser's territory to winner, 40% becomes unclaimed
            let loser_territory: Vec<WorldPos> = gods.iter()
                .find(|g| g.god_id == loser)
                .map(|g| g.territory.clone())
                .unwrap_or_default();

            for (ti, &pos) in loser_territory.iter().enumerate() {
                if let Some(idx) = world_map.idx(pos) {
                    if ti % 5 < 3 { // ~60%
                        world_state.territory_map[idx] = Some(winner);
                        if let Some(g) = gods.iter_mut().find(|g| g.god_id == winner) {
                            g.territory.push(pos);
                        }
                    } else {
                        world_state.territory_map[idx] = None;
                    }
                }
            }

            events.push(DivineEvent {
                year,
                kind: DivineEventKind::DomainAbsorbed,
                description: format!("{} absorbed the domain of the fallen {}", nw, nl),
                participants: vec![winner, loser],
            });

            // Heavy terrain scars around vanquished god's seat
            if let Some(seat) = gods.iter().find(|g| g.god_id == loser).and_then(|g| g.seat_of_power) {
                let aggressor_def = god_pool.get(winner);
                let scar_type = aggressor_def
                    .and_then(|d| d.terrain_influence.future_terrain.as_deref())
                    .and_then(DivineTerrainType::from_future_terrain);

                if let Some(dt) = scar_type {
                    for dy in -5..=5i32 {
                        for dx in -5..=5i32 {
                            let scar_pos = WorldPos::new(seat.x + dx, seat.y + dy);
                            if (dx.abs() + dy.abs()) > 7 { continue; }
                            if let Some(cell) = world_map.get_mut(scar_pos) {
                                cell.divine_terrain = Some(dt);
                                let scar_id = *next_id;
                                *next_id += 1;
                                terrain_scars.push(TerrainScar {
                                    id: scar_id,
                                    world_pos: scar_pos,
                                    terrain_type: dt,
                                    cause: TerrainScarCause::GodVanquished,
                                    caused_year: year,
                                    caused_by: vec![winner, loser],
                                    description: format!(
                                        "The land was scarred when {} fell",
                                        nl
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        // War zone terrain scars (30-60% of contested cells)
        let aggressor_def = god_pool.get(war.aggressor);
        let scar_type = aggressor_def
            .and_then(|d| d.terrain_influence.future_terrain.as_deref())
            .and_then(DivineTerrainType::from_future_terrain);

        if let Some(dt) = scar_type {
            for &pos in &war.contested_cells {
                if rng.random::<f32>() < 0.45 {
                    if let Some(cell) = world_map.get_mut(pos) {
                        cell.divine_terrain = Some(dt);
                        let scar_id = *next_id;
                        *next_id += 1;
                        terrain_scars.push(TerrainScar {
                            id: scar_id,
                            world_pos: pos,
                            terrain_type: dt,
                            cause: TerrainScarCause::DivineWarBattle,
                            caused_year: year,
                            caused_by: vec![war.aggressor, war.defender],
                            description: format!("Scarred by the war between {} and {}", nw, nl),
                        });
                    }
                }
            }
        }

        world_state.relations.modify(winner, loser, -40);

        events.push(DivineEvent {
            year,
            kind: DivineEventKind::DivineWarEnded,
            description: format!("The divine war ended; {} prevailed over {}", nw, nl),
            participants: vec![winner, loser],
        });
    }
}

fn evaluate_divine_pact(
    year: i32,
    _gods: &[GodState],
    events: &mut Vec<DivineEvent>,
    world_state: &mut DivineWorldState,
    active_ids: &[GodId],
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    if active_ids.len() < 2 { return; }
    if rng.random::<f32>() >= 0.10 { return; }

    let pact_kinds = [PactKind::NonAggression, PactKind::SharedDomain, PactKind::MutualDefense];

    for &a in active_ids {
        for &b in active_ids {
            if a >= b { continue; }
            if !world_state.relations.is_friendly(a, b) { continue; }
            if world_state.have_pact(a, b) { continue; }
            if world_state.at_war(a, b) { continue; }

            let kind = pact_kinds[rng.random_range(0..pact_kinds.len())];
            world_state.active_pacts.push(DivinePact {
                god_a: a, god_b: b, formed_year: year, kind,
            });
            world_state.relations.modify(a, b, 10);

            let na = pantheon.name(a).unwrap_or("Unknown");
            let nb = pantheon.name(b).unwrap_or("Unknown");
            let kind_str = match kind {
                PactKind::NonAggression => "a pact of non-aggression",
                PactKind::SharedDomain => "a pact to share their domains",
                PactKind::MutualDefense => "a pact of mutual defense",
            };
            events.push(DivineEvent {
                year,
                kind: DivineEventKind::PactFormed,
                description: format!("{} and {} formed {}", na, nb, kind_str),
                participants: vec![a, b],
            });
            return; // One pact per year max
        }
    }
}

fn evaluate_pact_broken(
    year: i32,
    _gods: &[GodState],
    events: &mut Vec<DivineEvent>,
    world_state: &mut DivineWorldState,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    let mut broken: Vec<usize> = Vec::new();
    for (i, pact) in world_state.active_pacts.iter().enumerate() {
        let sentiment = world_state.relations.get(pact.god_a, pact.god_b);
        let a_treacherous = pantheon.traits(pact.god_a).contains(&CharacterTrait::Treacherous);
        let b_treacherous = pantheon.traits(pact.god_b).contains(&CharacterTrait::Treacherous);
        let break_prob = if a_treacherous || b_treacherous { 0.35 } else { 0.20 };

        if sentiment < 10 && rng.random::<f32>() < break_prob {
            broken.push(i);
        }
    }

    for &i in broken.iter().rev() {
        let pact = world_state.active_pacts.remove(i);
        world_state.relations.modify(pact.god_a, pact.god_b, -25);
        let na = pantheon.name(pact.god_a).unwrap_or("Unknown");
        let nb = pantheon.name(pact.god_b).unwrap_or("Unknown");
        events.push(DivineEvent {
            year,
            kind: DivineEventKind::PactBroken,
            description: format!("The pact between {} and {} shattered", na, nb),
            participants: vec![pact.god_a, pact.god_b],
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world_map(width: u32, height: u32) -> WorldMap {
        use crate::worldgen::world_map::WorldCell;
        let cells: Vec<WorldCell> = (0..width * height)
            .map(|i| {
                let x = i % width;
                let y = i / width;
                let mut cell = WorldCell {
                    zone_type: if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                        ZoneType::Ocean
                    } else if (x + y) % 7 == 0 {
                        ZoneType::Forest
                    } else if (x + y) % 5 == 0 {
                        ZoneType::Mountain
                    } else {
                        ZoneType::Grassland
                    },
                    has_cave: false,
                    explored: false,
                    elevation: y as f32 / height as f32,
                    moisture: x as f32 / width as f32,
                    temperature: 0.5,
                    river: false,
                    river_entry: [false; 4],
                    river_width: 0.0,
                    region_id: None,
                    settlement_name: None,
                    settlement_size: None,
                    divine_terrain: None,
                    divine_owner: None,
                };
                // Add settlements for worship testing
                if x == 10 && y == 10 {
                    cell.settlement_name = Some("Testville".into());
                    cell.settlement_size = Some(crate::worldgen::world_map::SettlementSize::Town);
                } else if x == 20 && y == 20 {
                    cell.settlement_name = Some("Hamlet's Rest".into());
                    cell.settlement_size = Some(crate::worldgen::world_map::SettlementSize::Hamlet);
                } else if x == 30 && y == 30 {
                    cell.settlement_name = Some("Oakford".into());
                    cell.settlement_size = Some(crate::worldgen::world_map::SettlementSize::Village);
                } else if x == 40 && y == 40 {
                    cell.settlement_name = Some("Ironhaven".into());
                    cell.settlement_size = Some(crate::worldgen::world_map::SettlementSize::City);
                }
                cell
            })
            .collect();
        WorldMap {
            width,
            height,
            cells,
            spawn_pos: WorldPos::new(10, 10),
        }
    }

    fn test_god_pool_and_pantheon() -> (GodPool, DrawnPantheon) {
        let god_pool = crate::worldgen::gods::build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = god_pool.draw_pantheon(6, &mut rng);
        (god_pool, pantheon)
    }

    #[test]
    fn divine_era_runs_without_panic() {
        let mut world_map = test_world_map(64, 64);
        let (god_pool, pantheon) = test_god_pool_and_pantheon();
        let config = DivineEraConfig::default();
        let history = generate_divine_era(&config, &mut world_map, &god_pool, &pantheon, 42);

        assert!(!history.events.is_empty());
        assert!(history.the_fall.is_some());
        assert!(!history.gods.is_empty());
    }

    #[test]
    fn divine_era_deterministic() {
        let (god_pool, pantheon) = test_god_pool_and_pantheon();
        let config = DivineEraConfig::default();

        let mut map1 = test_world_map(64, 64);
        let h1 = generate_divine_era(&config, &mut map1, &god_pool, &pantheon, 42);

        let mut map2 = test_world_map(64, 64);
        let h2 = generate_divine_era(&config, &mut map2, &god_pool, &pantheon, 42);

        assert_eq!(h1.events.len(), h2.events.len());
        assert_eq!(h1.sites.len(), h2.sites.len());
        assert_eq!(h1.artifacts.len(), h2.artifacts.len());
        assert_eq!(h1.terrain_scars.len(), h2.terrain_scars.len());
        assert_eq!(h1.created_races.len(), h2.created_races.len());

        // Same event descriptions
        for (a, b) in h1.events.iter().zip(h2.events.iter()) {
            assert_eq!(a.description, b.description);
            assert_eq!(a.year, b.year);
        }
    }

    #[test]
    fn gods_claim_territory() {
        let mut world_map = test_world_map(64, 64);
        let (god_pool, pantheon) = test_god_pool_and_pantheon();
        let config = DivineEraConfig::default();
        let history = generate_divine_era(&config, &mut world_map, &god_pool, &pantheon, 42);

        // Every active god should have some territory
        for god in &history.gods {
            if god.is_active() {
                assert!(!god.territory.is_empty(), "Active god {} has no territory", god.god_id);
                assert!(god.seat_of_power.is_some(), "Active god {} has no seat", god.god_id);
            }
        }
    }

    #[test]
    fn world_map_modified_by_divine_era() {
        let mut world_map = test_world_map(64, 64);
        let (god_pool, pantheon) = test_god_pool_and_pantheon();
        let config = DivineEraConfig::default();
        let _history = generate_divine_era(&config, &mut world_map, &god_pool, &pantheon, 42);

        // Some cells should have divine_owner set
        let owned_count = world_map.cells.iter().filter(|c| c.divine_owner.is_some()).count();
        assert!(owned_count > 0, "No cells have divine_owner set");
    }

    #[test]
    fn the_fall_always_generated() {
        let mut world_map = test_world_map(64, 64);
        let (god_pool, pantheon) = test_god_pool_and_pantheon();
        let config = DivineEraConfig::default();
        let history = generate_divine_era(&config, &mut world_map, &god_pool, &pantheon, 42);

        let fall = history.the_fall.as_ref().unwrap();
        assert!(!fall.description.is_empty());
        assert!(!fall.consequences.is_empty());
    }

    #[test]
    fn terrain_to_zone_coverage() {
        use crate::terrain::TerrainType;
        // Every terrain type should map to a zone type
        for tt in [
            TerrainType::Grass, TerrainType::Dirt, TerrainType::Sand,
            TerrainType::Snow, TerrainType::Swamp, TerrainType::Stone,
            TerrainType::Forest, TerrainType::Water, TerrainType::Mountain,
        ] {
            let _zone = terrain_to_zone(tt);
        }
    }

    #[test]
    fn divine_era_full_map_summary() {
        use crate::worldgen::world_map::generate_world_map;
        let mut world_map = generate_world_map(256, 256, 42);
        let settlements_before = world_map.cells.iter().filter(|c| c.settlement_name.is_some()).count();
        let god_pool = crate::worldgen::gods::build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = god_pool.draw_pantheon(6, &mut rng);
        let config = DivineEraConfig::default();
        let history = generate_divine_era(&config, &mut world_map, &god_pool, &pantheon, 1042);

        let total_worship = history.gods.iter().map(|g| g.worshipper_settlements.len()).sum::<usize>();
        let _worship_events = history.events.iter()
            .filter(|e| e.description.contains("worshipping") || e.description.contains("abandoned"))
            .count();
        let narrative_events = history.events.iter()
            .filter(|e| e.kind == events::DivineEventKind::NarrativeAdvanced)
            .count();
        let _wars = history.events.iter()
            .filter(|e| e.kind == events::DivineEventKind::DivineWarDeclared)
            .count();
        let _vanquished = history.gods.iter().filter(|g| !g.is_active()).count();

        // These are informational — the test just ensures the simulation produces meaningful output
        assert!(settlements_before > 30, "Expected >30 settlements, got {}", settlements_before);
        assert!(total_worship > 0, "No worship happened — gods never claimed settlements");
        assert!(narrative_events > 0, "No narrative events — flaws never triggered");
        assert!(history.events.len() > 100, "Too few events: {}", history.events.len());
    }
}
