//! Territory expansion and terrain shaping for gods.

use std::collections::BTreeSet;

use rand::{Rng, RngExt};

use super::gods::{DrawnPantheon, GodPool};
use super::state::GodState;
use super::terrain_scars::{DivineTerrainType, TerrainScar, TerrainScarCause};
use crate::worldgen::history::state::WorldState;
use crate::worldgen::history::HistoricEvent;
use crate::worldgen::world_map::{WorldMap, WorldPos};
use crate::worldgen::zone::ZoneType;

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

        // First priority: cities and towns (gods center their power on civilization)
        let mut candidates: Vec<WorldPos> = Vec::new();
        for y in 0..world_map.height as i32 {
            for x in 0..world_map.width as i32 {
                let pos = WorldPos::new(x, y);
                let cell = match world_map.get(pos) {
                    Some(c) => c,
                    None => continue,
                };

                let far_enough = used_positions.iter().all(|&used| {
                    pos.manhattan_distance(used) >= min_distance
                });
                if !far_enough { continue; }

                let is_city = cell.settlement_size == Some(crate::worldgen::world_map::SettlementSize::City);
                let is_town = cell.settlement_size == Some(crate::worldgen::world_map::SettlementSize::Town);

                if is_city {
                    // Cities are the strongest candidates
                    for _ in 0..10 { candidates.push(pos); }
                } else if is_town {
                    for _ in 0..5 { candidates.push(pos); }
                }
            }
        }

        // Fallback: any non-ocean cell if no cities/towns available
        if candidates.is_empty() {
            for y in 0..world_map.height as i32 {
                for x in 0..world_map.width as i32 {
                    let pos = WorldPos::new(x, y);
                    if let Some(cell) = world_map.get(pos) {
                        if cell.zone_type == ZoneType::Ocean { continue; }
                        let far_enough = used_positions.iter().all(|&used| {
                            pos.manhattan_distance(used) >= min_distance / 2
                        });
                        if !far_enough { continue; }
                        if cell.zone_type == preferred_zone {
                            candidates.push(pos);
                            candidates.push(pos);
                            candidates.push(pos);
                        } else {
                            candidates.push(pos);
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
