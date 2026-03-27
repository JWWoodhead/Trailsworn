//! God behavior functions — callable subroutines for the unified simulation.
//! These operate on the unified WorldState rather than the old DivineWorldState.

use std::collections::BTreeSet;

use rand::{Rng, RngExt};

use crate::worldgen::gods::{DrawnPantheon, GodId, GodPool};
use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::history::state::{SettlementState, WorldState};
use crate::worldgen::history::{EventKind, HistoricEvent};
use crate::worldgen::names::{full_name, Race};
use crate::worldgen::population_table::PopTable;
use crate::worldgen::world_map::{WorldMap, WorldPos};
use crate::worldgen::zone::ZoneType;

use super::artifacts::{divine_artifact_name, ArtifactLocation, DivineArtifact, DivineArtifactKind};
use super::personality::DivineDrive;
use super::races::CreatedRace;
use super::sites::{divine_site_name, DivineSite, DivineSiteKind};
use super::state::{DivineWar, GodState};
use super::terrain_scars::{DivineTerrainType, TerrainScar, TerrainScarCause};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map a TerrainType to the most fitting ZoneType for seat selection.
pub fn terrain_to_zone(terrain: crate::terrain::TerrainType) -> ZoneType {
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

fn make_event(year: i32, kind: EventKind, description: String, god_ids: Vec<GodId>) -> HistoricEvent {
    HistoricEvent {
        year,
        kind,
        description,
        participants: vec![],
        god_participants: god_ids,
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Assign each god a starting seat of power based on terrain affinity.
pub fn assign_seats_of_power(
    gods: &mut [GodState],
    frontiers: &mut [BTreeSet<WorldPos>],
    world_state: &mut WorldState,
    world_map: &WorldMap,
    god_pool: &GodPool,
    rng: &mut impl Rng,
) {
    let mut used_positions: Vec<WorldPos> = Vec::new();
    let min_distance = 40i32;

    for (gi, god) in gods.iter_mut().enumerate() {
        let god_def = match god_pool.get(god.god_id) {
            Some(d) => d,
            None => continue,
        };

        let preferred_zone = terrain_to_zone(god_def.terrain_influence.primary_terrain);

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
                    (pos.x - used.x).abs() + (pos.y - used.y).abs() >= min_distance
                });
                if !far_enough { continue; }

                if cell.zone_type == preferred_zone {
                    candidates.push(pos);
                    candidates.push(pos);
                    candidates.push(pos);
                } else if cell.zone_type != ZoneType::Mountain {
                    candidates.push(pos);
                }
            }
        }

        if candidates.is_empty() {
            for y in 0..world_map.height as i32 {
                for x in 0..world_map.width as i32 {
                    let pos = WorldPos::new(x, y);
                    if let Some(cell) = world_map.get(pos) {
                        if cell.zone_type != ZoneType::Ocean {
                            let far_enough = used_positions.iter().all(|&used| {
                                (pos.x - used.x).abs() + (pos.y - used.y).abs() >= min_distance / 2
                            });
                            if far_enough { candidates.push(pos); }
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

        if let Some(idx) = world_map.idx(seat) {
            world_state.territory_map[idx] = Some(god.god_id);
        }

        for neighbor in seat.neighbors() {
            if world_map.is_passable(neighbor) {
                frontiers[gi].insert(neighbor);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Territory Expansion
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn evaluate_territory_expansion(
    _year: i32,
    expansion_rate: u32,
    gods: &mut [GodState],
    frontiers: &mut [BTreeSet<WorldPos>],
    _events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    world_map: &WorldMap,
    god_pool: &GodPool,
    _pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    let mut god_indices: Vec<usize> = (0..gods.len()).filter(|&i| gods[i].is_active()).collect();
    for i in (1..god_indices.len()).rev() {
        let j = rng.random_range(0..=i);
        god_indices.swap(i, j);
    }

    for gi in god_indices {
        let god = &gods[gi];
        let god_id = god.god_id;
        let god_def = match god_pool.get(god_id) { Some(d) => d, None => continue };
        let power_mod = god.power as f32 / 80.0;
        let cells_to_claim = ((expansion_rate as f32 * power_mod) as u32).max(1);

        let preferred_zone = terrain_to_zone(god_def.terrain_influence.primary_terrain);
        let secondary_zone = god_def.terrain_influence.secondary_terrain.map(|t| terrain_to_zone(t));

        let frontier = &frontiers[gi];
        let mut candidates: Vec<(WorldPos, u32)> = Vec::new();
        for &pos in frontier.iter() {
            if let Some(idx) = world_map.idx(pos) {
                let cell = &world_map.cells[idx];
                if cell.zone_type == ZoneType::Ocean { continue; }

                if let Some(owner) = world_state.territory_map[idx] {
                    if owner != god_id {
                        world_state.divine_relations.modify(god_id, owner, -5);
                    }
                    continue;
                }

                let weight = if cell.zone_type == preferred_zone { 3 }
                    else if secondary_zone == Some(cell.zone_type) { 2 }
                    else { 1 };
                candidates.push((pos, weight));
            }
        }

        if candidates.is_empty() { continue; }

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
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

        let claimed_this_year: Vec<WorldPos> = candidates.iter()
            .take(cells_to_claim as usize)
            .map(|(pos, _)| *pos)
            .collect();

        let god = &mut gods[gi];
        for pos in &claimed_this_year {
            if let Some(idx) = world_map.idx(*pos) {
                world_state.territory_map[idx] = Some(god_id);
            }
            god.territory.push(*pos);
            if god.core_territory.len() < 20 {
                god.core_territory.push(*pos);
            }
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
// Terrain Shaping
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn evaluate_terrain_shaping(
    year: i32,
    gods: &[GodState],
    _events: &mut Vec<HistoricEvent>,
    terrain_scars: &mut Vec<TerrainScar>,
    _world_state: &WorldState,
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
    }
}

// ---------------------------------------------------------------------------
// Worship Competition
// ---------------------------------------------------------------------------

pub fn evaluate_worship(
    year: i32,
    gods: &mut [GodState],
    events: &mut Vec<HistoricEvent>,
    settlements: &mut [SettlementState],
    world_state: &mut WorldState,
    world_map: &WorldMap,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    for si in 0..settlements.len() {
        let settlement_pos = match settlements[si].world_pos {
            Some(pos) => pos,
            None => continue,
        };
        let current_patron = settlements[si].patron_god;
        let current_devotion = settlements[si].devotion;

        let territory_owner = world_map.idx(settlement_pos)
            .and_then(|idx| world_state.territory_map[idx]);

        let owner_god_id = match territory_owner {
            Some(id) => id,
            None => {
                if current_devotion > 0 {
                    settlements[si].devotion = current_devotion.saturating_sub(2);
                }
                continue;
            }
        };

        let owner_active = gods.iter().any(|g| g.god_id == owner_god_id && g.is_active());
        if !owner_active { continue; }

        if current_patron == Some(owner_god_id) {
            let drive = gods.iter().find(|g| g.god_id == owner_god_id).map(|g| g.drive());
            let growth = match drive {
                Some(DivineDrive::Worship) => 5,
                Some(DivineDrive::Love) => 4,
                Some(DivineDrive::Dominion) => 3,
                _ => 2,
            };
            settlements[si].devotion = (current_devotion + growth).min(100);
        } else if current_patron.is_none() {
            let drive = gods.iter().find(|g| g.god_id == owner_god_id).map(|g| g.drive());
            let claim_prob = match drive {
                Some(DivineDrive::Worship) => 0.40,
                Some(DivineDrive::Love) => 0.30,
                Some(DivineDrive::Dominion) => 0.35,
                Some(DivineDrive::Legacy) => 0.25,
                _ => 0.15,
            };
            if rng.random::<f32>() < claim_prob {
                let sname = settlements[si].name.clone();
                settlements[si].patron_god = Some(owner_god_id);
                settlements[si].devotion = 20;

                if let Some(g) = gods.iter_mut().find(|g| g.god_id == owner_god_id) {
                    g.worshipper_settlements.push(settlement_pos);
                }

                let god_name = pantheon.name(owner_god_id).unwrap_or("A god");
                events.push(make_event(year, EventKind::WorshipEstablished,
                    format!("The people of {} began worshipping {}", sname, god_name),
                    vec![owner_god_id],
                ));
            }
        } else {
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
                let sname = settlements[si].name.clone();
                settlements[si].patron_god = Some(owner_god_id);
                settlements[si].devotion = 15;

                if let Some(g) = gods.iter_mut().find(|g| g.god_id == old_patron) {
                    g.worshipper_settlements.retain(|p| *p != settlement_pos);
                }
                if let Some(g) = gods.iter_mut().find(|g| g.god_id == owner_god_id) {
                    g.worshipper_settlements.push(settlement_pos);
                }

                world_state.divine_relations.modify(owner_god_id, old_patron, -10);

                let god_name = pantheon.name(owner_god_id).unwrap_or("A god");
                let old_name = pantheon.name(old_patron).unwrap_or("another god");
                events.push(make_event(year, EventKind::WorshipConverted,
                    format!("The people of {} abandoned {} and turned to {}", sname, old_name, god_name),
                    vec![owner_god_id, old_patron],
                ));
            } else {
                settlements[si].devotion = current_devotion.saturating_sub(1);
            }
        }
    }

    // Gods without worshippers slowly weaken
    for god in gods.iter_mut() {
        if !god.is_active() { continue; }
        if god.worshipper_settlements.is_empty() {
            god.years_without_worship += 1;
            if god.years_without_worship >= 20 {
                god.faded = true;
            }
        } else {
            god.years_without_worship = 0;
            // Un-fade if they regain worshippers
            if god.faded {
                god.faded = false;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// God Power Update
// ---------------------------------------------------------------------------

pub fn update_god_power(gods: &mut [GodState]) {
    for god in gods.iter_mut() {
        if !god.is_active() { continue; }
        // Power scales directly with worshipper count
        let worshipper_count = god.worshipper_settlements.len() as u32;
        god.power = (worshipper_count * 15).min(100);
    }
}

// ---------------------------------------------------------------------------
// Drive-Based Actions
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn evaluate_drive_actions(
    year: i32,
    race_window: (i32, i32),
    gods: &mut Vec<GodState>,
    events: &mut Vec<HistoricEvent>,
    sites: &mut Vec<DivineSite>,
    artifacts: &mut Vec<DivineArtifact>,
    created_races: &mut Vec<CreatedRace>,
    _world_state: &WorldState,
    _world_map: &WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    next_id: &mut u32,
    rng: &mut impl Rng,
) {
    let god_count = gods.len();
    for gi in 0..god_count {
        if !gods[gi].is_active() { continue; }
        if gods[gi].power < 10 { continue; } // too weak to act
        let drive = gods[gi].drive();

        match drive {
            DivineDrive::Knowledge => {
                eval_sacred_site(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.12);
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.10);
            }
            DivineDrive::Dominion => {
                eval_champion(year, gi, gods, events, pantheon, rng, 0.08);
                eval_temple(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.12);
            }
            DivineDrive::Worship => {
                eval_temple(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.15);
                eval_champion(year, gi, gods, events, pantheon, rng, 0.08);
            }
            DivineDrive::Perfection => {
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.15);
                eval_sacred_site(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.10);
            }
            DivineDrive::Justice => {
                eval_temple(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.10);
                eval_champion(year, gi, gods, events, pantheon, rng, 0.06);
            }
            DivineDrive::Love => {
                eval_champion(year, gi, gods, events, pantheon, rng, 0.06);
                if year >= race_window.0 && year <= race_window.1 {
                    eval_race(year, gi, gods, events, created_races, god_pool, pantheon, next_id, rng, 0.10);
                }
            }
            DivineDrive::Freedom => {
                eval_sacred_site(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.08);
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.06);
            }
            DivineDrive::Legacy => {
                if year >= race_window.0 && year <= race_window.1 {
                    eval_race(year, gi, gods, events, created_races, god_pool, pantheon, next_id, rng, 0.12);
                }
                eval_sacred_site(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.10);
            }
            DivineDrive::Vindication => {
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.12);
                eval_champion(year, gi, gods, events, pantheon, rng, 0.08);
            }
            DivineDrive::Supremacy => {
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.10);
                eval_champion(year, gi, gods, events, pantheon, rng, 0.10);
            }
        }
    }
}

// --- Sub-actions ---

fn eval_temple(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    sites: &mut Vec<DivineSite>, _god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].power < 40 || gods[gi].territory.is_empty() { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let pos = gods[gi].territory[rng.random_range(0..gods[gi].territory.len())];
    let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();
    let site_id = *next_id; *next_id += 1;
    let name = divine_site_name(DivineSiteKind::Temple, &god_name, rng);
    sites.push(DivineSite {
        id: site_id, name: name.clone(), kind: DivineSiteKind::Temple,
        world_pos: pos, creator_god: god_id, created_year: year,
        persists: true, description: format!("{} established {}", god_name, name),
        terrain_effect: None,
    });
    gods[gi].sites_created += 1;
    events.push(make_event(year, EventKind::TempleEstablished,
        format!("{} founded {}", god_name, name), vec![god_id]));
}

fn eval_champion(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    pantheon: &DrawnPantheon, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].power < 50 || gods[gi].champion_name.is_some() { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let race_table = PopTable::pick_one(vec![
        (Race::Human, 40.0), (Race::Dwarf, 20.0), (Race::Elf, 15.0),
        (Race::Orc, 15.0), (Race::Goblin, 10.0),
    ]);
    let race = race_table.roll_one(rng).unwrap();
    let name = full_name(race, rng);
    let god_name = pantheon.name(god_id).unwrap_or("A god");

    gods[gi].champion_name = Some(name.clone());
    gods[gi].champion_race = Some(race);
    events.push(make_event(year, EventKind::ChampionChosen,
        format!("{} chose {} as their mortal champion", god_name, name), vec![god_id]));
}

#[allow(clippy::too_many_arguments)]
fn eval_race(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    created_races: &mut Vec<CreatedRace>, god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].created_race_id.is_some() || gods[gi].power < 60 { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let god_def = match god_pool.get(god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();
    let race_id = *next_id; *next_id += 1;
    let core_territory = gods[gi].core_territory.clone();
    let race = super::races::race_template(race_id, god_def, &god_name, year, &core_territory, rng);

    gods[gi].created_race_id = Some(race_id);
    events.push(make_event(year, EventKind::RaceCreated,
        format!("{} created the {}", god_name, race.name), vec![god_id]));
    created_races.push(race);
}

#[allow(clippy::too_many_arguments)]
fn eval_artifact(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    artifacts: &mut Vec<DivineArtifact>, god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].power < 30 { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let power = gods[gi].power;
    let has_champion = gods[gi].champion_name.is_some();
    let god_def = match god_pool.get(god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();

    let kind_table = PopTable::pick_one(vec![
        (DivineArtifactKind::Weapon, 30.0), (DivineArtifactKind::Armor, 25.0),
        (DivineArtifactKind::Implement, 20.0), (DivineArtifactKind::Key, 10.0),
        (DivineArtifactKind::Vessel, 15.0),
    ]);
    let kind = kind_table.roll_one(rng).unwrap();
    let power_level = (power / 20).clamp(1, 5);
    let name = divine_artifact_name(kind, god_def.domain, rng);
    let artifact_id = *next_id; *next_id += 1;

    let location = if has_champion && rng.random::<f32>() < 0.3 {
        ArtifactLocation::HeldByChampion(god_id)
    } else { ArtifactLocation::Lost };

    artifacts.push(DivineArtifact {
        id: artifact_id, name: name.clone(), kind, creator_god: god_id,
        created_year: year, magic_school: god_def.domain, power_level, location,
        description: format!("{} forged {}", god_name, name),
        lore: format!("Created by {} in the age of gods", god_name),
    });
    gods[gi].artifacts_created += 1;
    events.push(make_event(year, EventKind::DivineArtifactForged,
        format!("{} forged the {}", god_name, name), vec![god_id]));
}

#[allow(clippy::too_many_arguments)]
fn eval_sacred_site(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    sites: &mut Vec<DivineSite>, god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].territory.is_empty() { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let god_def = match god_pool.get(god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();
    let kind = DivineSiteKind::for_domain(god_def.domain);
    let pos = gods[gi].territory[rng.random_range(0..gods[gi].territory.len())];
    let site_id = *next_id; *next_id += 1;
    let divine_terrain = god_def.terrain_influence.future_terrain.as_deref()
        .and_then(DivineTerrainType::from_future_terrain);
    let name = divine_site_name(kind, &god_name, rng);

    sites.push(DivineSite {
        id: site_id, name: name.clone(), kind, world_pos: pos,
        creator_god: god_id, created_year: year, persists: true,
        description: format!("{} created {}", god_name, name),
        terrain_effect: divine_terrain,
    });
    gods[gi].sites_created += 1;
    events.push(make_event(year, EventKind::SacredSiteCreated,
        format!("{} created {}", god_name, name), vec![god_id]));
}

// ---------------------------------------------------------------------------
// Divine Conflict
// ---------------------------------------------------------------------------

pub fn evaluate_divine_war_declared(
    year: i32,
    gods: &[GodState],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    active_ids: &[GodId],
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    if active_ids.len() < 2 { return; }
    let Some((a, b, sentiment)) = world_state.divine_relations.most_hostile_pair(active_ids) else { return };
    if sentiment >= -30 { return; }
    if world_state.gods_at_war(a, b) { return; }
    if world_state.god_war_count(a) > 0 || world_state.god_war_count(b) > 0 { return; }

    let hostility_bonus = ((-sentiment - 30) as f32 * 0.8).min(40.0);
    let mut prob = 25.0 + hostility_bonus;

    let a_traits = pantheon.traits(a);
    if a_traits.contains(&CharacterTrait::Warlike) { prob += 20.0; }
    if a_traits.contains(&CharacterTrait::Ambitious) { prob += 10.0; }
    if a_traits.contains(&CharacterTrait::Peaceful) { prob -= 25.0; }
    if a_traits.contains(&CharacterTrait::Diplomatic) { prob -= 15.0; }

    let a_power = gods.iter().find(|g| g.god_id == a).map(|g| g.power).unwrap_or(0);
    let b_power = gods.iter().find(|g| g.god_id == b).map(|g| g.power).unwrap_or(0);
    if a_power > b_power + 20 { prob += 15.0; }

    let prob = (prob / 100.0).clamp(0.05, 0.60);
    if rng.random::<f32>() >= prob { return; }

    let a_territory: std::collections::HashSet<WorldPos> = gods.iter()
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

    world_state.divine_wars.push(DivineWar {
        aggressor: a, defender: b, start_year: year, contested_cells: contested,
    });
    world_state.divine_relations.modify(a, b, -20);

    let na = pantheon.name(a).unwrap_or("Unknown");
    let nb = pantheon.name(b).unwrap_or("Unknown");
    events.push(make_event(year, EventKind::DivineWarDeclared,
        format!("{} declared war upon {}", na, nb), vec![a, b]));
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_divine_war_resolution(
    year: i32,
    gods: &mut Vec<GodState>,
    events: &mut Vec<HistoricEvent>,
    terrain_scars: &mut Vec<TerrainScar>,
    world_state: &mut WorldState,
    world_map: &mut WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    next_id: &mut u32,
    rng: &mut impl Rng,
) {
    let mut ended_wars: Vec<usize> = Vec::new();

    for (i, war) in world_state.divine_wars.iter().enumerate() {
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
        let war = world_state.divine_wars.remove(i);
        let a_power = gods.iter().find(|g| g.god_id == war.aggressor).map(|g| g.power).unwrap_or(0);
        let b_power = gods.iter().find(|g| g.god_id == war.defender).map(|g| g.power).unwrap_or(0);

        let (winner, loser) = if a_power >= b_power {
            (war.aggressor, war.defender)
        } else {
            (war.defender, war.aggressor)
        };

        let nw = pantheon.name(winner).unwrap_or("Unknown").to_string();
        let nl = pantheon.name(loser).unwrap_or("Unknown").to_string();

        if let Some(g) = gods.iter_mut().find(|g| g.god_id == winner) {
            g.wars_fought += 1; g.wars_won += 1;
        }
        if let Some(g) = gods.iter_mut().find(|g| g.god_id == loser) {
            g.wars_fought += 1;
        }

        // War zone terrain scars
        let aggressor_def = god_pool.get(war.aggressor);
        let scar_type = aggressor_def
            .and_then(|d| d.terrain_influence.future_terrain.as_deref())
            .and_then(DivineTerrainType::from_future_terrain);

        if let Some(dt) = scar_type {
            for &pos in &war.contested_cells {
                if rng.random::<f32>() < 0.45 {
                    if let Some(cell) = world_map.get_mut(pos) {
                        cell.divine_terrain = Some(dt);
                        let scar_id = *next_id; *next_id += 1;
                        terrain_scars.push(TerrainScar {
                            id: scar_id, world_pos: pos, terrain_type: dt,
                            cause: TerrainScarCause::DivineWarBattle, caused_year: year,
                            caused_by: vec![war.aggressor, war.defender],
                            description: format!("Scarred by the war between {} and {}", nw, nl),
                        });
                    }
                }
            }
        }

        world_state.divine_relations.modify(winner, loser, -40);
        events.push(make_event(year, EventKind::DivineWarEnded,
            format!("The divine war ended; {} prevailed over {}", nw, nl),
            vec![winner, loser]));
    }
}

pub fn evaluate_divine_pact(
    year: i32,
    _gods: &[GodState],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    active_ids: &[GodId],
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    use super::state::PactKind;
    if active_ids.len() < 2 { return; }
    if rng.random::<f32>() >= 0.10 { return; }

    let pact_kinds = [PactKind::NonAggression, PactKind::SharedDomain, PactKind::MutualDefense];

    for &a in active_ids {
        for &b in active_ids {
            if a >= b { continue; }
            if !world_state.divine_relations.is_friendly(a, b) { continue; }
            if world_state.gods_have_pact(a, b) { continue; }
            if world_state.gods_at_war(a, b) { continue; }

            let kind = pact_kinds[rng.random_range(0..pact_kinds.len())];
            world_state.divine_pacts.push(super::state::DivinePact {
                god_a: a, god_b: b, formed_year: year, kind,
            });
            world_state.divine_relations.modify(a, b, 10);

            let na = pantheon.name(a).unwrap_or("Unknown");
            let nb = pantheon.name(b).unwrap_or("Unknown");
            let kind_str = match kind {
                PactKind::NonAggression => "a pact of non-aggression",
                PactKind::SharedDomain => "a pact to share their domains",
                PactKind::MutualDefense => "a pact of mutual defense",
            };
            events.push(make_event(year, EventKind::PactFormed,
                format!("{} and {} formed {}", na, nb, kind_str), vec![a, b]));
            return;
        }
    }
}

pub fn evaluate_pact_broken(
    year: i32,
    _gods: &[GodState],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    let mut broken: Vec<usize> = Vec::new();
    for (i, pact) in world_state.divine_pacts.iter().enumerate() {
        let sentiment = world_state.divine_relations.get(pact.god_a, pact.god_b);
        let a_treacherous = pantheon.traits(pact.god_a).contains(&CharacterTrait::Treacherous);
        let b_treacherous = pantheon.traits(pact.god_b).contains(&CharacterTrait::Treacherous);
        let break_prob = if a_treacherous || b_treacherous { 0.35 } else { 0.20 };

        if sentiment < 10 && rng.random::<f32>() < break_prob {
            broken.push(i);
        }
    }

    for &i in broken.iter().rev() {
        let pact = world_state.divine_pacts.remove(i);
        world_state.divine_relations.modify(pact.god_a, pact.god_b, -25);
        let na = pantheon.name(pact.god_a).unwrap_or("Unknown");
        let nb = pantheon.name(pact.god_b).unwrap_or("Unknown");
        events.push(make_event(year, EventKind::PactBroken,
            format!("The pact between {} and {} shattered", na, nb),
            vec![pact.god_a, pact.god_b]));
    }
}

// ---------------------------------------------------------------------------
// Flaw Pressure & Triggers
// ---------------------------------------------------------------------------

pub fn accumulate_flaw_pressure(
    gods: &mut [GodState],
    new_events: &[HistoricEvent],
    world_state: &WorldState,
) {
    use super::personality::DivineFlaw::*;

    let active_god_ids: Vec<GodId> = gods.iter().filter(|g| g.is_active()).map(|g| g.god_id).collect();

    let pressure_gains: Vec<(usize, u32)> = gods.iter().enumerate()
        .filter(|(_, g)| g.is_active())
        .map(|(gi, god)| {
            let flaw = god.flaw();
            let god_id = god.god_id;

            let gain: u32 = match flaw {
                Hubris => {
                    new_events.iter().filter(|e| {
                        e.god_participants.contains(&god_id)
                            && matches!(e.kind, EventKind::DivineWarEnded | EventKind::DivineArtifactForged)
                    }).count() as u32 * 8
                }
                Jealousy => {
                    new_events.iter().filter(|e| {
                        !e.god_participants.contains(&god_id)
                            && matches!(e.kind, EventKind::WorshipEstablished | EventKind::DivineArtifactForged | EventKind::RaceCreated)
                    }).count() as u32 * 5
                }
                Obsession => 3,
                Cruelty => {
                    new_events.iter().filter(|e| {
                        e.god_participants.contains(&god_id)
                            && matches!(e.kind, EventKind::TerritoryContested | EventKind::PactBroken)
                    }).count() as u32 * 8
                }
                Blindness => 2,
                Isolation => {
                    let others: Vec<GodId> = active_god_ids.iter().copied().filter(|&id| id != god_id).collect();
                    let avg_sentiment: i32 = if others.is_empty() { 0 } else {
                        others.iter().map(|&id| world_state.divine_relations.get(god_id, id)).sum::<i32>() / others.len() as i32
                    };
                    if avg_sentiment < 0 { (-avg_sentiment / 10) as u32 } else { 1 }
                }
                Betrayal => {
                    let has_pact = world_state.divine_pacts.iter().any(|p| p.god_a == god_id || p.god_b == god_id);
                    if has_pact { 5 } else { 1 }
                }
                Sacrifice => if god.power < 40 { 5 } else { 1 },
                Rigidity => {
                    let disruptions = new_events.iter().filter(|e| {
                        matches!(e.kind, EventKind::DivineWarDeclared | EventKind::PactBroken)
                    }).count() as u32;
                    2 + disruptions * 3
                }
                Hollowness => {
                    new_events.iter().filter(|e| {
                        e.god_participants.contains(&god_id)
                            && matches!(e.kind, EventKind::DivineArtifactForged | EventKind::SacredSiteCreated)
                    }).count() as u32 * 6
                }
            };
            (gi, gain)
        })
        .collect();

    for (gi, gain) in pressure_gains {
        gods[gi].flaw_pressure = (gods[gi].flaw_pressure + gain).min(100);
    }
}

pub fn evaluate_flaw_triggers(
    year: i32,
    gods: &mut [GodState],
    events: &mut Vec<HistoricEvent>,
    settlements: &mut [SettlementState],
    world_state: &mut WorldState,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    use super::personality::DivineFlaw::*;

    let god_count = gods.len();
    for gi in 0..god_count {
        if !gods[gi].is_active() { continue; }
        if gods[gi].flaw_pressure < 80 { continue; }
        let trigger_prob = (gods[gi].flaw_pressure as f32 - 70.0) / 100.0;
        if rng.random::<f32>() >= trigger_prob { continue; }

        let flaw = gods[gi].flaw();
        let god_id = gods[gi].god_id;
        let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();

        match flaw {
            Hubris => {
                gods[gi].power = gods[gi].power.saturating_sub(15);
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{}, drunk on their own power, overreached and was diminished", god_name),
                    vec![god_id]));
            }
            Jealousy => {
                let target = gods.iter()
                    .filter(|g| g.is_active() && g.god_id != god_id)
                    .max_by_key(|g| g.worshipper_settlements.len());
                if let Some(t) = target {
                    let target_id = t.god_id;
                    let target_name = pantheon.name(target_id).unwrap_or("another god").to_string();
                    world_state.divine_relations.modify(god_id, target_id, -20);
                    events.push(make_event(year, EventKind::NarrativeAdvanced,
                        format!("{}, consumed by jealousy, turned against {} for having what they could not", god_name, target_name),
                        vec![god_id, target_id]));
                }
            }
            Obsession => {
                for s in settlements.iter_mut() {
                    if s.patron_god == Some(god_id) {
                        s.devotion = s.devotion.saturating_sub(10);
                    }
                }
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{}, lost in obsession, neglected those who worshipped them", god_name),
                    vec![god_id]));
            }
            Cruelty => {
                for s in settlements.iter_mut() {
                    if s.patron_god == Some(god_id) {
                        s.devotion = s.devotion.saturating_sub(15);
                    }
                }
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} lashed out in fury, and their own followers suffered for it", god_name),
                    vec![god_id]));
            }
            Blindness => {
                let other = gods.iter()
                    .filter(|g| g.is_active() && g.god_id != god_id)
                    .nth(rng.random_range(0..gods.iter().filter(|g| g.is_active() && g.god_id != god_id).count().max(1)));
                if let Some(t) = other {
                    let tid = t.god_id;
                    let tname = pantheon.name(tid).unwrap_or("another god").to_string();
                    world_state.divine_relations.modify(god_id, tid, -15);
                    events.push(make_event(year, EventKind::NarrativeAdvanced,
                        format!("{}, blind to the consequences, unknowingly trespassed against {}", god_name, tname),
                        vec![god_id, tid]));
                }
            }
            Isolation => {
                let lost: Vec<WorldPos> = gods[gi].worshipper_settlements.clone();
                for pos in &lost {
                    if let Some(s) = settlements.iter_mut().find(|s| s.world_pos == Some(*pos) && s.patron_god == Some(god_id)) {
                        s.devotion = s.devotion.saturating_sub(20);
                    }
                }
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} withdrew from the world, becoming distant and unreachable", god_name),
                    vec![god_id]));
            }
            Betrayal => {
                let pact_idx = world_state.divine_pacts.iter().position(|p| p.god_a == god_id || p.god_b == god_id);
                if let Some(idx) = pact_idx {
                    let pact = world_state.divine_pacts.remove(idx);
                    let other_id = if pact.god_a == god_id { pact.god_b } else { pact.god_a };
                    let other_name = pantheon.name(other_id).unwrap_or("another god").to_string();
                    world_state.divine_relations.modify(god_id, other_id, -30);
                    events.push(make_event(year, EventKind::PactBroken,
                        format!("{} betrayed {}, shattering the trust between them", god_name, other_name),
                        vec![god_id, other_id]));
                }
            }
            Sacrifice => {
                gods[gi].power = gods[gi].power.saturating_sub(20);
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} sacrificed a piece of themselves in pursuit of their deepest desire", god_name),
                    vec![god_id]));
            }
            Rigidity => {
                for other in gods.iter().filter(|g| g.is_active() && g.god_id != god_id) {
                    world_state.divine_relations.modify(god_id, other.god_id, -5);
                }
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} refused to bend, and the other gods grew weary of their inflexibility", god_name),
                    vec![god_id]));
            }
            Hollowness => {
                gods[gi].power = gods[gi].power.saturating_sub(10);
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} achieved what they sought, and found it meant nothing", god_name),
                    vec![god_id]));
            }
        }

        gods[gi].flaw_pressure = 20;
    }
}
