use super::WorldCell;
use crate::worldgen::zone::ZoneType;

/// Classify a cell's biome from its elevation, moisture, and temperature.
pub(super) fn classify_biome(elevation: f32, moisture: f32, temperature: f32, ocean_thresh: f32) -> ZoneType {
    if elevation < ocean_thresh {
        return ZoneType::Ocean;
    }
    if elevation > 0.78 {
        return ZoneType::Mountain;
    }
    if temperature < 0.25 {
        return ZoneType::Tundra;
    }
    if temperature > 0.60 && moisture < 0.35 {
        return ZoneType::Desert;
    }
    if moisture > 0.70 && elevation < 0.55 {
        return ZoneType::Swamp;
    }
    if moisture > 0.55 && temperature > 0.25 {
        return ZoneType::Forest;
    }
    ZoneType::Grassland
}

/// Fill gaps in mountain ranges: non-ocean cells adjacent to >=2 mountains become mountains.
/// This connects the domain-warped ridges into cohesive linear ranges.
pub(super) fn smooth_mountain_ranges(cells: &mut [WorldCell], width: u32, height: u32) {
    let offsets: [(i32, i32); 8] = [
        (0, 1), (0, -1), (1, 0), (-1, 0),
        (1, 1), (1, -1), (-1, 1), (-1, -1),
    ];
    let promote: Vec<usize> = (0..cells.len())
        .filter(|&i| {
            let cell = &cells[i];
            // Only promote high-elevation non-ocean land cells
            if cell.zone_type == ZoneType::Ocean || cell.zone_type == ZoneType::Mountain {
                return false;
            }
            if cell.elevation < 0.70 {
                return false;
            }
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            let mountain_neighbors = offsets.iter().filter(|(dx, dy)| {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    return false;
                }
                let ni = (ny as u32 * width + nx as u32) as usize;
                cells[ni].zone_type == ZoneType::Mountain
            }).count();
            mountain_neighbors >= 2
        })
        .collect();

    for i in promote {
        cells[i].zone_type = ZoneType::Mountain;
    }
}

/// Turn land cells adjacent to ocean into Coast (except Mountain).
pub(super) fn apply_coast_zones(cells: &mut [WorldCell], width: u32, height: u32) {
    let offsets: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
    // Collect indices to avoid borrow issues
    let coast_indices: Vec<usize> = (0..cells.len())
        .filter(|&i| {
            let cell = &cells[i];
            if cell.zone_type == ZoneType::Ocean || cell.zone_type == ZoneType::Mountain {
                return false;
            }
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            offsets.iter().any(|(dx, dy)| {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    return false;
                }
                let ni = (ny as u32 * width + nx as u32) as usize;
                cells[ni].zone_type == ZoneType::Ocean
            })
        })
        .collect();

    for i in coast_indices {
        cells[i].zone_type = ZoneType::Coast;
    }
}

/// Boost moisture for land cells near ocean using BFS distance falloff.
pub(super) fn boost_moisture_near_ocean(
    elevation: &[f32],
    moisture: &mut [f32],
    width: u32,
    height: u32,
    ocean_threshold: f32,
) {
    let n = (width * height) as usize;
    let max_dist = 20u32; // influence range in cells

    // BFS from all ocean cells
    let mut dist = vec![u32::MAX; n];
    let mut queue = std::collections::VecDeque::new();

    for i in 0..n {
        if elevation[i] < ocean_threshold {
            dist[i] = 0;
            queue.push_back(i);
        }
    }

    let offsets: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
    while let Some(ci) = queue.pop_front() {
        let cd = dist[ci];
        if cd >= max_dist {
            continue;
        }
        let cx = (ci as u32) % width;
        let cy = (ci as u32) / width;
        for (dx, dy) in &offsets {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let ni = (ny as u32 * width + nx as u32) as usize;
            let nd = cd + 1;
            if nd < dist[ni] {
                dist[ni] = nd;
                queue.push_back(ni);
            }
        }
    }

    // Apply moisture boost based on proximity to ocean
    for i in 0..n {
        if dist[i] > 0 && dist[i] < max_dist {
            let factor = 1.0 - (dist[i] as f32 / max_dist as f32);
            moisture[i] = (moisture[i] + factor * 0.20).min(1.0);
        }
    }
}
