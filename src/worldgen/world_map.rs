use rand::{Rng, RngExt, SeedableRng};

use super::divine_era::DivineTerrainType;
use super::gods::GodId;
use super::names;
use super::noise_util::NoiseLayer;
use super::zone::{ZoneGenContext, ZoneType};

/// Position on the world grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorldPos {
    pub x: i32,
    pub y: i32,
}

impl WorldPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn neighbors(self) -> [WorldPos; 4] {
        [
            WorldPos::new(self.x, self.y + 1),  // N
            WorldPos::new(self.x + 1, self.y),  // E
            WorldPos::new(self.x, self.y - 1),  // S
            WorldPos::new(self.x - 1, self.y),  // W
        ]
    }
}

/// Size of a settlement on the world map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SettlementSize {
    /// Tiny community (10-50 people). Keeps natural zone type.
    Hamlet,
    /// Small community (50-200). Keeps natural zone type.
    Village,
    /// Medium settlement (200-1000). Gets ZoneType::Settlement.
    Town,
    /// Large settlement (1000+). Gets ZoneType::Settlement.
    City,
}

/// A cell on the world map grid.
#[derive(Clone, Debug)]
pub struct WorldCell {
    pub zone_type: ZoneType,
    pub has_cave: bool,
    pub explored: bool,
    pub elevation: f32,
    pub moisture: f32,
    pub temperature: f32,
    pub river: bool,
    /// Which edges the river enters/exits: [N, E, S, W].
    pub river_entry: [bool; 4],
    /// Normalized river width at this cell (0.0 = source, 1.0 = mouth).
    pub river_width: f32,
    pub region_id: Option<u32>,
    pub settlement_name: Option<String>,
    /// Size of settlement at this cell, if any.
    pub settlement_size: Option<SettlementSize>,
    /// Divine terrain overlay — what kind of divine scar exists here, if any.
    pub divine_terrain: Option<DivineTerrainType>,
    /// Which god last owned this cell during the divine era.
    pub divine_owner: Option<GodId>,
}

impl WorldCell {
    fn new(zone_type: ZoneType) -> Self {
        Self {
            zone_type,
            has_cave: false,
            explored: false,
            elevation: 0.0,
            moisture: 0.0,
            temperature: 0.0,
            river: false,
            river_entry: [false; 4],
            river_width: 0.0,
            region_id: None,
            settlement_name: None,
            settlement_size: None,
            divine_terrain: None,
            divine_owner: None,
        }
    }
}

/// The world map — a grid of zones.
#[derive(bevy::prelude::Resource)]
pub struct WorldMap {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<WorldCell>,
    /// Where the player starts.
    pub spawn_pos: WorldPos,
}

impl WorldMap {
    pub fn idx(&self, pos: WorldPos) -> Option<usize> {
        if pos.x < 0 || pos.y < 0 || pos.x >= self.width as i32 || pos.y >= self.height as i32 {
            return None;
        }
        Some((pos.y as u32 * self.width + pos.x as u32) as usize)
    }

    pub fn get(&self, pos: WorldPos) -> Option<&WorldCell> {
        self.idx(pos).map(|i| &self.cells[i])
    }

    pub fn get_mut(&mut self, pos: WorldPos) -> Option<&mut WorldCell> {
        self.idx(pos).map(|i| &mut self.cells[i])
    }

    pub fn in_bounds(&self, pos: WorldPos) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.width as i32 && pos.y < self.height as i32
    }

    /// Returns true if the position is in-bounds and the zone is enterable (not Ocean).
    pub fn is_passable(&self, pos: WorldPos) -> bool {
        self.get(pos).is_some_and(|c| c.zone_type != ZoneType::Ocean)
    }

    /// Build a [`ZoneGenContext`] for the cell at `pos`, including neighbor info.
    pub fn zone_context(&self, pos: WorldPos) -> Option<ZoneGenContext> {
        let cell = self.get(pos)?;
        let neighbors_pos = pos.neighbors(); // [N, E, S, W]
        let ocean_edges = [
            self.get(neighbors_pos[0]).is_some_and(|c| c.zone_type == ZoneType::Ocean),
            self.get(neighbors_pos[1]).is_some_and(|c| c.zone_type == ZoneType::Ocean),
            self.get(neighbors_pos[2]).is_some_and(|c| c.zone_type == ZoneType::Ocean),
            self.get(neighbors_pos[3]).is_some_and(|c| c.zone_type == ZoneType::Ocean),
        ];
        let neighbors = [
            self.get(neighbors_pos[0]).map(|c| c.zone_type).filter(|z| *z != ZoneType::Ocean),
            self.get(neighbors_pos[1]).map(|c| c.zone_type).filter(|z| *z != ZoneType::Ocean),
            self.get(neighbors_pos[2]).map(|c| c.zone_type).filter(|z| *z != ZoneType::Ocean),
            self.get(neighbors_pos[3]).map(|c| c.zone_type).filter(|z| *z != ZoneType::Ocean),
        ];
        Some(ZoneGenContext {
            zone_type: cell.zone_type,
            has_cave: cell.has_cave,
            elevation: cell.elevation,
            moisture: cell.moisture,
            temperature: cell.temperature,
            river: cell.river,
            river_entry: cell.river_entry,
            river_width: cell.river_width,
            neighbors,
            ocean_edges,
        })
    }
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

/// Generate a world map with noise-driven geography.
///
/// Produces multiple landmasses with ocean, mountains, forests, deserts, tundra,
/// swamps, and coast zones. Rivers flow from high elevation to ocean.
pub fn generate_world_map(width: u32, height: u32, seed: u64) -> WorldMap {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let n = (width * height) as usize;

    // Derive sub-seeds so noise layers are independent
    let elev_seed = rng.random::<u32>();
    let moist_seed = rng.random::<u32>();
    let temp_seed = rng.random::<u32>();

    // --- Layer 1: Elevation ---
    // Base continental noise (low freq, big landmasses) + ridge noise for mountain ranges
    // + continent-scale modulation for ocean variety across seeds
    let elev_noise = NoiseLayer::new(elev_seed, 0.008, 6);
    let ridge_noise = NoiseLayer::new(elev_seed.wrapping_add(1000), 0.025, 4);
    let continent_noise = NoiseLayer::new(elev_seed.wrapping_add(2000), 0.003, 2);
    // Domain warp layer: distorts ridge coordinates to create elongated mountain ranges
    let warp_noise = NoiseLayer::new(elev_seed.wrapping_add(3000), 0.005, 3);
    // Seed-derived directional bias for ridge stretch (varies range orientation per world)
    let ridge_angle = (elev_seed as f64 % 1000.0) / 1000.0 * std::f64::consts::PI;
    let cos_a = ridge_angle.cos();
    let sin_a = ridge_angle.sin();
    let mut elevation = vec![0.0f32; n];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let base = elev_noise.sample_normalized(x as f64, y as f64) as f32;
            // Continent-scale modulation: creates large-scale highs/lows so some regions
            // are structurally low (ocean) regardless of detail noise
            let continent = continent_noise.sample_normalized(x as f64, y as f64) as f32;
            let modulated = base * 0.75 + continent * 0.25;
            // Domain-warped ridge noise for elongated mountain ranges
            let warp = warp_noise.sample(x as f64, y as f64) * 40.0;
            // Rotate + stretch coordinates so ridges trend in a seed-specific direction
            let rx = x as f64 * cos_a - y as f64 * sin_a;
            let ry = x as f64 * sin_a + y as f64 * cos_a;
            let ridge_raw = ridge_noise.sample(rx * 0.7 + warp, ry * 1.3 + warp) as f32;
            // Power curve for sharper, narrower ridges
            let ridge = (1.0 - ridge_raw.abs()).powf(1.8) * 0.16;
            // Only apply ridges on land (where base is already above ocean level)
            let ridge_masked = if modulated > 0.45 { ridge } else { 0.0 };
            // Apply contrast curve: push values away from the midpoint for steeper slopes
            let combined = modulated * 0.85 + ridge_masked;
            let contrasted = ((combined - 0.5) * 1.3 + 0.5).clamp(0.0, 1.0);
            elevation[i] = contrasted;
        }
    }

    // --- Layer 2: Moisture ---
    let moist_noise = NoiseLayer::new(moist_seed, 0.015, 5);
    let mut moisture = vec![0.0f32; n];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            moisture[i] = moist_noise.sample_normalized(x as f64, y as f64) as f32;
        }
    }

    // Boost moisture near ocean (distance-based falloff)
    let mut ocean_threshold = 0.42f32;

    // Ocean fraction safety check: adjust threshold to keep ocean between 15-60%.
    // This ensures every seed has meaningful ocean coverage and land variety.
    loop {
        let ocean_count = elevation.iter().filter(|&&e| e < ocean_threshold).count();
        let ocean_frac = ocean_count as f32 / n as f32;
        if ocean_frac < 0.15 {
            ocean_threshold += 0.02;
        } else if ocean_frac > 0.60 {
            ocean_threshold -= 0.02;
        } else {
            break;
        }
    }

    boost_moisture_near_ocean(&elevation, &mut moisture, width, height, ocean_threshold);

    // --- Layer 3: Temperature ---
    let temp_noise = NoiseLayer::new(temp_seed, 0.02, 4);
    let mut temperature = vec![0.0f32; n];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            // Latitude gradient: warm at bottom (y=0), cold at top (y=max)
            let latitude = y as f32 / height as f32;
            let base_temp = 1.0 - latitude;
            // Noise variation
            let noise_var = (temp_noise.sample_normalized(x as f64, y as f64) as f32 - 0.5) * 0.3;
            // Elevation penalty (mountains are cold)
            let elev_penalty = (elevation[i] - 0.5).max(0.0) * 0.6;
            temperature[i] = (base_temp + noise_var - elev_penalty).clamp(0.0, 1.0);
        }
    }

    // --- River generation ---
    let mut river = vec![false; n];
    let mut river_progress = vec![0.0f32; n]; // 0.0 = source, 1.0 = mouth
    let mut river_edges: Vec<[bool; 4]> = vec![[false; 4]; n]; // [N, E, S, W]
    generate_rivers(
        &elevation,
        &mut river,
        &mut river_progress,
        &mut river_edges,
        &mut moisture,
        width,
        height,
        ocean_threshold,
        &mut rng,
    );

    // --- Build cells with biome classification ---
    let mut cells = Vec::with_capacity(n);
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let e = elevation[i];
            let m = moisture[i];
            let t = temperature[i];

            let zone_type = classify_biome(e, m, t, ocean_threshold);
            let mut cell = WorldCell::new(zone_type);
            cell.elevation = e;
            cell.moisture = m;
            cell.temperature = t;
            cell.river = river[i];
            cell.river_entry = river_edges[i];
            cell.river_width = river_progress[i];
            cells.push(cell);
        }
    }

    // --- Mountain range smoothing: connect isolated mountain cells to form ranges ---
    smooth_mountain_ranges(&mut cells, width, height);

    // --- Coast detection: land cells adjacent to ocean ---
    apply_coast_zones(&mut cells, width, height);

    // --- Region identification via flood-fill ---
    assign_regions(&mut cells, width, height);

    // --- Cave placement: 20% chance on eligible land zones ---
    for cell in &mut cells {
        if matches!(
            cell.zone_type,
            ZoneType::Grassland
                | ZoneType::Forest
                | ZoneType::Mountain
                | ZoneType::Desert
                | ZoneType::Tundra
                | ZoneType::Swamp
                | ZoneType::Coast
        ) {
            if rng.random::<f32>() < 0.2 {
                cell.has_cave = true;
            }
        }
    }

    // --- Settlement placement ---
    place_settlements(&mut cells, width, height, &mut rng);

    // --- Find spawn position ---
    let spawn_pos = find_spawn_pos(&cells, width, height);

    // Mark spawn zone as explored
    let spawn_idx = (spawn_pos.y as u32 * width + spawn_pos.x as u32) as usize;
    cells[spawn_idx].explored = true;

    WorldMap {
        width,
        height,
        cells,
        spawn_pos,
    }
}

/// Classify a cell's biome from its elevation, moisture, and temperature.
fn classify_biome(elevation: f32, moisture: f32, temperature: f32, ocean_thresh: f32) -> ZoneType {
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

/// Fill gaps in mountain ranges: non-ocean cells adjacent to ≥2 mountains become mountains.
/// This connects the domain-warped ridges into cohesive linear ranges.
fn smooth_mountain_ranges(cells: &mut [WorldCell], width: u32, height: u32) {
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
fn apply_coast_zones(cells: &mut [WorldCell], width: u32, height: u32) {
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
fn boost_moisture_near_ocean(
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

/// Generate rivers by walking downhill from high-elevation sources to ocean.
fn generate_rivers(
    elevation: &[f32],
    river: &mut [bool],
    river_progress: &mut [f32],
    river_edges: &mut [[bool; 4]],
    moisture: &mut [f32],
    width: u32,
    height: u32,
    ocean_threshold: f32,
    rng: &mut impl Rng,
) {
    let n = (width * height) as usize;
    let num_rivers = (width as usize * height as usize / 4000).clamp(5, 40);

    // Collect candidate source cells (elevated land — mountains and hills)
    let mut candidates: Vec<usize> = (0..n)
        .filter(|&i| elevation[i] > 0.52 && elevation[i] < 0.90)
        .collect();

    // Shuffle and pick sources, ensuring minimum spacing between river starts
    for i in (1..candidates.len()).rev() {
        let j = rng.random_range(0..=i);
        candidates.swap(i, j);
    }

    let min_source_dist_sq = (width.min(height) / 10).pow(2) as u32;
    let mut sources: Vec<(u32, u32)> = Vec::new();

    for &c in &candidates {
        if sources.len() >= num_rivers {
            break;
        }
        let cx = (c as u32) % width;
        let cy = (c as u32) / width;
        let too_close = sources
            .iter()
            .any(|(sx, sy)| cx.abs_diff(*sx).pow(2) + cy.abs_diff(*sy).pow(2) < min_source_dist_sq);
        if too_close {
            continue;
        }
        sources.push((cx, cy));
        walk_river(c, elevation, river, river_progress, river_edges, moisture, width, height, ocean_threshold);
    }
}

/// Walk a single river downhill from source until reaching ocean or map edge.
///
/// Traces a smooth float-coordinate path guided by elevation gradient + noise meander,
/// then rasterizes it onto the grid with increasing width. This avoids the staircase
/// artifacts of grid-cell-by-cell walking.
fn walk_river(
    start: usize,
    elevation: &[f32],
    river: &mut [bool],
    river_progress: &mut [f32],
    river_edges: &mut [[bool; 4]],
    moisture: &mut [f32],
    width: u32,
    height: u32,
    ocean_threshold: f32,
) {
    let meander = NoiseLayer::new(start as u32, 0.05, 3);

    // Start at the center of the source cell
    let mut fx = ((start as u32) % width) as f64 + 0.5;
    let mut fy = ((start as u32) / width) as f64 + 0.5;
    let max_steps = (width + height) as usize * 4;
    let step_size = 0.8;

    // Track previous direction for momentum when gradient is flat
    let mut prev_dx = 0.0f64;
    let mut prev_dy = -1.0; // default: flow "south" (toward y=0)

    let mut path: Vec<(f64, f64)> = Vec::new();

    for step in 0..max_steps {
        let ix = (fx as i32).clamp(0, width as i32 - 1) as u32;
        let iy = (fy as i32).clamp(0, height as i32 - 1) as u32;
        let idx = (iy * width + ix) as usize;

        path.push((fx, fy));

        // Reached ocean — done
        if elevation[idx] < ocean_threshold {
            break;
        }

        // Compute downhill gradient from neighboring cells (wide radius to see past flat areas)
        let mut grad_x = 0.0f64;
        let mut grad_y = 0.0f64;
        let sample_r = 4.0;
        for &(dx, dy) in &[(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0),
                           (0.7, 0.7), (0.7, -0.7), (-0.7, 0.7), (-0.7, -0.7)] {
            let sx = (fx + dx * sample_r).clamp(0.0, width as f64 - 1.0);
            let sy = (fy + dy * sample_r).clamp(0.0, height as f64 - 1.0);
            let si = (sy as u32 * width + sx as u32) as usize;
            let diff = elevation[idx] as f64 - elevation[si] as f64;
            grad_x += dx * diff;
            grad_y += dy * diff;
        }

        // Normalize gradient; if flat, rely more on momentum
        let grad_len = (grad_x * grad_x + grad_y * grad_y).sqrt();
        if grad_len > 1e-10 {
            grad_x /= grad_len;
            grad_y /= grad_len;
        } else {
            grad_x = prev_dx;
            grad_y = prev_dy;
        }

        // Blend with previous direction: heavy momentum keeps rivers flowing through flats
        let momentum = 0.65;
        grad_x = grad_x * (1.0 - momentum) + prev_dx * momentum;
        grad_y = grad_y * (1.0 - momentum) + prev_dy * momentum;
        let blend_len = (grad_x * grad_x + grad_y * grad_y).sqrt();
        if blend_len > 1e-8 {
            grad_x /= blend_len;
            grad_y /= blend_len;
        }

        // Add meander: perpendicular oscillation via noise
        let perp_x = -grad_y;
        let perp_y = grad_x;
        let meander_strength = meander.sample(step as f64 * 0.05, fx * 0.01) * 0.6;
        let dir_x = grad_x + perp_x * meander_strength;
        let dir_y = grad_y + perp_y * meander_strength;

        // Normalize and step
        let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
        let ndx = dir_x / dir_len;
        let ndy = dir_y / dir_len;
        fx += ndx * step_size;
        fy += ndy * step_size;
        prev_dx = ndx;
        prev_dy = ndy;

        // Out of bounds — done
        if fx < 0.0 || fy < 0.0 || fx >= width as f64 || fy >= height as f64 {
            break;
        }
    }

    // Rasterize the smooth path with increasing width + track edge crossings
    let path_len = path.len() as f32;
    let mut prev_cell: Option<(u32, u32)> = None;
    // Subsample path to avoid over-painting (take every ~2 points)
    let paint_step = 2.max(1);
    for (step, &(px, py)) in path.iter().enumerate().step_by(paint_step) {
        let progress = step as f32 / path_len;
        let half_width = (0.8 + progress * 2.2) as i32; // 1 at source, ~3 at mouth

        let cx = px as i32;
        let cy = py as i32;
        for dx in -half_width..=half_width {
            for dy in -half_width..=half_width {
                if dx * dx + dy * dy > half_width * half_width + 1 {
                    continue;
                }
                let rx = cx + dx;
                let ry = cy + dy;
                if rx >= 0 && ry >= 0 && rx < width as i32 && ry < height as i32 {
                    let ri = (ry as u32 * width + rx as u32) as usize;
                    river[ri] = true;
                    // Track maximum progress (width) for each cell
                    if progress > river_progress[ri] {
                        river_progress[ri] = progress;
                    }
                }
            }
        }

        // Track edge crossings between world cells
        let cur_cell = (cx.max(0) as u32, cy.max(0) as u32);
        if let Some(prev) = prev_cell {
            if cur_cell != prev {
                let pi = (prev.1 * width + prev.0) as usize;
                let ci = (cur_cell.1.min(height - 1) * width + cur_cell.0.min(width - 1)) as usize;
                // Determine which edge was crossed: N=+y, S=-y, E=+x, W=-x
                if cur_cell.1 > prev.1 {
                    // Moved north: prev exits N, cur enters S
                    if pi < river_edges.len() { river_edges[pi][0] = true; }
                    if ci < river_edges.len() { river_edges[ci][2] = true; }
                } else if cur_cell.1 < prev.1 {
                    if pi < river_edges.len() { river_edges[pi][2] = true; }
                    if ci < river_edges.len() { river_edges[ci][0] = true; }
                }
                if cur_cell.0 > prev.0 {
                    if pi < river_edges.len() { river_edges[pi][1] = true; }
                    if ci < river_edges.len() { river_edges[ci][3] = true; }
                } else if cur_cell.0 < prev.0 {
                    if pi < river_edges.len() { river_edges[pi][3] = true; }
                    if ci < river_edges.len() { river_edges[ci][1] = true; }
                }
            }
        }
        prev_cell = Some(cur_cell);

        // Moisture boost
        let moist_r = half_width + 2;
        for dx in -moist_r..=moist_r {
            for dy in -moist_r..=moist_r {
                let mx = cx + dx;
                let my = cy + dy;
                if mx >= 0 && my >= 0 && mx < width as i32 && my < height as i32 {
                    let mi = (my as u32 * width + mx as u32) as usize;
                    moisture[mi] = (moisture[mi] + 0.15).min(1.0);
                }
            }
        }
    }
}

/// Flood-fill contiguous same-type land zones to assign region_id.
fn assign_regions(cells: &mut [WorldCell], width: u32, height: u32) {
    let n = cells.len();
    let mut region_counter = 0u32;
    let offsets: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    for start in 0..n {
        if cells[start].zone_type == ZoneType::Ocean || cells[start].region_id.is_some() {
            continue;
        }

        let zone_type = cells[start].zone_type;
        let rid = region_counter;
        region_counter += 1;

        let mut stack = vec![start];
        while let Some(ci) = stack.pop() {
            if cells[ci].region_id.is_some() {
                continue;
            }
            if cells[ci].zone_type != zone_type {
                continue;
            }
            cells[ci].region_id = Some(rid);

            let cx = (ci as u32) % width;
            let cy = (ci as u32) / width;
            for (dx, dy) in &offsets {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;
                if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                    let ni = (ny as u32 * width + nx as u32) as usize;
                    if cells[ni].region_id.is_none() && cells[ni].zone_type == zone_type {
                        stack.push(ni);
                    }
                }
            }
        }
    }
}

/// Place settlements of varying sizes on suitable land cells.
/// Distribution: ~40 hamlets, ~20 villages, ~8 towns, ~3 cities (scaled to map size).
/// Cities occupy 2x2 zones, towns 2x1, villages and hamlets 1x1.
/// Only towns and cities get ZoneType::Settlement; hamlets and villages keep natural terrain.
fn place_settlements(cells: &mut [WorldCell], width: u32, height: u32, rng: &mut impl Rng) {
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
    let mut candidates: Vec<(usize, u32)> = cells
        .iter()
        .enumerate()
        .filter(|(_, c)| is_habitable(c))
        .map(|(i, c)| {
            let mut weight = 1u32;
            if matches!(c.zone_type, ZoneType::Grassland | ZoneType::Forest) { weight += 2; }
            if c.river { weight += 3; }
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
fn find_spawn_pos(cells: &[WorldCell], width: u32, height: u32) -> WorldPos {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_map_has_correct_size() {
        let map = generate_world_map(64, 64, 42);
        assert_eq!(map.cells.len(), 64 * 64);
        assert_eq!(map.width, 64);
        assert_eq!(map.height, 64);
    }

    #[test]
    fn world_map_has_settlement() {
        let map = generate_world_map(64, 64, 42);
        let has_settlement = map.cells.iter().any(|c| c.settlement_name.is_some());
        assert!(has_settlement);
    }

    #[test]
    fn large_map_has_settlement_variety() {
        let map = generate_world_map(256, 256, 42);
        let hamlets = map.cells.iter().filter(|c| c.settlement_size == Some(SettlementSize::Hamlet)).count();
        let villages = map.cells.iter().filter(|c| c.settlement_size == Some(SettlementSize::Village)).count();
        let towns = map.cells.iter().filter(|c| c.settlement_size == Some(SettlementSize::Town)).count();
        let cities = map.cells.iter().filter(|c| c.settlement_size == Some(SettlementSize::City)).count();
        assert!(hamlets > 10, "Expected >10 hamlets, got {}", hamlets);
        assert!(villages > 5, "Expected >5 villages, got {}", villages);
        assert!(towns > 2, "Expected >2 towns, got {}", towns);
        assert!(cities >= 1, "Expected >=1 city, got {}", cities);
        // Towns and cities should have ZoneType::Settlement
        let zone_settlements = map.cells.iter().filter(|c| c.zone_type == ZoneType::Settlement).count();
        assert_eq!(zone_settlements, towns + cities);
    }

    #[test]
    fn world_map_has_caves() {
        let map = generate_world_map(64, 64, 42);
        let caves = map.cells.iter().filter(|c| c.has_cave).count();
        assert!(caves > 0);
    }

    #[test]
    fn settlement_has_no_cave() {
        let map = generate_world_map(64, 64, 42);
        for cell in &map.cells {
            if cell.zone_type == ZoneType::Settlement {
                assert!(!cell.has_cave);
            }
        }
    }

    #[test]
    fn spawn_is_in_bounds() {
        let map = generate_world_map(64, 64, 42);
        assert!(map.in_bounds(map.spawn_pos));
    }

    #[test]
    fn spawn_zone_is_explored() {
        let map = generate_world_map(64, 64, 42);
        let cell = map.get(map.spawn_pos).unwrap();
        assert!(cell.explored);
    }

    #[test]
    fn spawn_is_not_ocean() {
        let map = generate_world_map(64, 64, 42);
        let cell = map.get(map.spawn_pos).unwrap();
        assert_ne!(cell.zone_type, ZoneType::Ocean);
    }

    #[test]
    fn has_multiple_biomes() {
        let map = generate_world_map(128, 128, 42);
        let mut types = std::collections::HashSet::new();
        for cell in &map.cells {
            types.insert(cell.zone_type);
        }
        // Should have at least Ocean + 3 land biomes
        assert!(types.len() >= 4, "Only found {} biome types: {:?}", types.len(), types);
    }

    #[test]
    fn has_ocean_cells() {
        let map = generate_world_map(128, 128, 42);
        let ocean_count = map.cells.iter().filter(|c| c.zone_type == ZoneType::Ocean).count();
        assert!(ocean_count > 0, "No ocean cells found");
    }

    #[test]
    fn has_land_cells() {
        let map = generate_world_map(128, 128, 42);
        let land_count = map.cells.iter().filter(|c| c.zone_type != ZoneType::Ocean).count();
        assert!(land_count > 100, "Too few land cells: {land_count}");
    }

    #[test]
    fn has_rivers() {
        let map = generate_world_map(128, 128, 42);
        let river_count = map.cells.iter().filter(|c| c.river).count();
        assert!(river_count > 0, "No river cells found");
    }

    #[test]
    fn regions_are_assigned() {
        let map = generate_world_map(64, 64, 42);
        let assigned = map
            .cells
            .iter()
            .filter(|c| c.zone_type != ZoneType::Ocean && c.region_id.is_some())
            .count();
        let land = map
            .cells
            .iter()
            .filter(|c| c.zone_type != ZoneType::Ocean)
            .count();
        assert_eq!(assigned, land, "All land cells should have a region_id");
    }

    #[test]
    fn deterministic() {
        let a = generate_world_map(64, 64, 123);
        let b = generate_world_map(64, 64, 123);
        for (ca, cb) in a.cells.iter().zip(b.cells.iter()) {
            assert_eq!(ca.zone_type, cb.zone_type);
            assert_eq!(ca.elevation, cb.elevation);
            assert_eq!(ca.river, cb.river);
        }
    }

    #[test]
    fn is_passable_checks_ocean() {
        let map = generate_world_map(64, 64, 42);
        // Find an ocean cell
        for (i, cell) in map.cells.iter().enumerate() {
            let pos = WorldPos::new((i as u32 % map.width) as i32, (i as u32 / map.width) as i32);
            if cell.zone_type == ZoneType::Ocean {
                assert!(!map.is_passable(pos));
            } else {
                assert!(map.is_passable(pos));
            }
        }
    }

    #[test]
    fn biome_balance_across_seeds() {
        // Verify biome distribution is reasonable across multiple seeds
        for seed in [1, 42, 123, 999, 2025, 7777, 31415, 54321, 100_000, 999_999] {
            let map = generate_world_map(128, 128, seed);
            let total = map.cells.len() as f32;

            let mut counts = std::collections::HashMap::new();
            for cell in &map.cells {
                *counts.entry(cell.zone_type).or_insert(0u32) += 1;
            }

            let ocean_pct = *counts.get(&ZoneType::Ocean).unwrap_or(&0) as f32 / total;
            let forest_pct = *counts.get(&ZoneType::Forest).unwrap_or(&0) as f32 / total;

            // Ocean should be between 15% and 65%
            assert!(
                ocean_pct > 0.15 && ocean_pct < 0.65,
                "Seed {seed}: ocean {:.1}% out of range (expected 15-65%)",
                ocean_pct * 100.0
            );

            // Forest should not dominate (< 30%)
            assert!(
                forest_pct < 0.30,
                "Seed {seed}: forest {:.1}% too high (expected < 30%)",
                forest_pct * 100.0
            );

            // Should have at least 5 distinct biome types
            assert!(
                counts.len() >= 5,
                "Seed {seed}: only {} biome types: {:?}",
                counts.len(),
                counts.keys().collect::<Vec<_>>()
            );

            // No single land biome should exceed 35%
            for (biome, count) in &counts {
                if *biome == ZoneType::Ocean {
                    continue;
                }
                let pct = *count as f32 / total;
                assert!(
                    pct < 0.35,
                    "Seed {seed}: {:?} at {:.1}% exceeds 35% cap",
                    biome,
                    pct * 100.0
                );
            }
        }
    }
}
