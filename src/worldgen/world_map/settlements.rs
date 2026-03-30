use rand::{Rng, RngExt};

use super::{SettlementSize, WorldCell, WorldPos};
use crate::worldgen::names;
use crate::worldgen::zone::ZoneType;

/// Place settlements of varying sizes on suitable land cells.
/// Distribution: ~40 hamlets, ~20 villages, ~8 towns, ~3 cities (scaled to map size).
/// Cities occupy 2x2 zones, towns 2x1, villages and hamlets 1x1.
/// Only towns and cities get ZoneType::Settlement; hamlets and villages keep natural terrain.
pub(super) fn place_settlements(cells: &mut [WorldCell], width: u32, height: u32, rng: &mut impl Rng) {
    let scale = (width as f32 * height as f32) / (256.0 * 256.0); // 1.0 for default map
    let targets = [
        (SettlementSize::City,   (3.0 * scale) as usize),
        (SettlementSize::Town,   (8.0 * scale) as usize),
        (SettlementSize::Village, (20.0 * scale) as usize),
        (SettlementSize::Hamlet, (40.0 * scale) as usize),
    ];

    // Minimum distance between settlements (squared), by size.
    // Cities are spaced far apart, hamlets can be close together.
    fn min_dist_sq(size: SettlementSize, map_side: u32) -> u32 {
        let base = map_side / 4;
        match size {
            SettlementSize::City   => (base).pow(2),
            SettlementSize::Town   => (base * 2 / 3).pow(2),
            SettlementSize::Village => (base / 3).pow(2),
            SettlementSize::Hamlet => (base / 5).pow(2),
        }
    }

    fn is_habitable(cell: &WorldCell) -> bool {
        if cell.river {
            return false; // don't place settlements directly on river cells
        }
        matches!(cell.zone_type,
            ZoneType::Grassland | ZoneType::Forest | ZoneType::Coast
            | ZoneType::Swamp | ZoneType::Desert | ZoneType::Tundra)
    }

    /// Cell offsets for each settlement size's footprint.
    /// City = 2x2, Town = 2x1 (horizontal), Village/Hamlet = 1x1.
    fn footprint(size: SettlementSize) -> &'static [(i32, i32)] {
        match size {
            SettlementSize::City   => &[(0, 0), (1, 0), (0, 1), (1, 1)],
            SettlementSize::Town   => &[(0, 0), (1, 0)],
            SettlementSize::Village | SettlementSize::Hamlet => &[(0, 0)],
        }
    }

    /// Check that all cells in a footprint are in-bounds, habitable, and unclaimed.
    fn footprint_valid(
        x: u32, y: u32, size: SettlementSize,
        cells: &[WorldCell], width: u32, height: u32,
    ) -> bool {
        for &(dx, dy) in footprint(size) {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                return false;
            }
            let idx = ny as u32 * width + nx as u32;
            let cell = &cells[idx as usize];
            if !is_habitable(cell) || cell.settlement_name.is_some() {
                return false;
            }
        }
        true
    }

    // Candidates: habitable land, prefer river adjacency and grassland/forest
    let offsets: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
    let mut candidates: Vec<(usize, u32)> = cells
        .iter()
        .enumerate()
        .filter(|(_, c)| is_habitable(c))
        .map(|(i, c)| {
            let mut weight = 1u32;
            if matches!(c.zone_type, ZoneType::Grassland | ZoneType::Forest) { weight += 2; }
            // Prefer cells adjacent to a river (not ON a river — those are excluded)
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            let near_river = offsets.iter().any(|(dx, dy)| {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    return false;
                }
                cells[(ny as u32 * width + nx as u32) as usize].river
            });
            if near_river { weight += 3; }
            (i, weight)
        })
        .collect();

    // Sort by weight descending, shuffle within weight groups
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

    let map_side = width.min(height);
    let mut placed: Vec<(u32, u32, SettlementSize)> = Vec::new();

    // Place each tier largest-first so cities get the best spots
    for &(size, target_count) in &targets {
        let mut count = 0;

        for (i, _) in &candidates {
            if count >= target_count { break; }
            let x = (*i as u32) % width;
            let y = (*i as u32) / width;

            // Already has a settlement?
            if cells[*i].settlement_name.is_some() { continue; }

            // Check that the full footprint is valid
            if !footprint_valid(x, y, size, cells, width, height) { continue; }

            // Distance check against same-or-larger settlements
            let too_close = placed.iter().any(|(sx, sy, existing_size)| {
                let d = x.abs_diff(*sx).pow(2) + y.abs_diff(*sy).pow(2);
                // Use the stricter (larger) distance of the two
                let required = min_dist_sq(size.max(*existing_size), map_side);
                d < required
            });
            if too_close { continue; }

            // Claim all cells in the footprint
            let name = names::settlement_name(rng);
            for &(dx, dy) in footprint(size) {
                let nx = (x as i32 + dx) as u32;
                let ny = (y as i32 + dy) as u32;
                let idx = (ny * width + nx) as usize;
                cells[idx].settlement_name = Some(name.clone());
                cells[idx].settlement_size = Some(size);
                match size {
                    SettlementSize::Town | SettlementSize::City => {
                        cells[idx].zone_type = ZoneType::Settlement;
                        cells[idx].has_cave = false;
                    }
                    _ => {}
                }
            }

            placed.push((x, y, size));
            count += 1;
        }
    }
}

/// Find a suitable spawn position: a settlement cell, preferring near center.
pub(super) fn find_spawn_pos(cells: &[WorldCell], width: u32, height: u32) -> WorldPos {
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    // Prefer settlement closest to center
    let mut best_pos = None;
    let mut best_dist = f32::MAX;

    for (i, cell) in cells.iter().enumerate() {
        if cell.zone_type == ZoneType::Settlement {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            let dist = (x as f32 - cx).powi(2) + (y as f32 - cy).powi(2);
            if dist < best_dist {
                best_dist = dist;
                best_pos = Some(WorldPos::new(x as i32, y as i32));
            }
        }
    }

    // Fallback: any walkable land cell near center
    best_pos.unwrap_or_else(|| {
        let mut fallback_pos = WorldPos::new(cx as i32, cy as i32);
        let mut fallback_dist = f32::MAX;
        for (i, cell) in cells.iter().enumerate() {
            if cell.zone_type != ZoneType::Ocean {
                let x = (i as u32) % width;
                let y = (i as u32) / width;
                let dist = (x as f32 - cx).powi(2) + (y as f32 - cy).powi(2);
                if dist < fallback_dist {
                    fallback_dist = dist;
                    fallback_pos = WorldPos::new(x as i32, y as i32);
                }
            }
        }
        fallback_pos
    })
}
