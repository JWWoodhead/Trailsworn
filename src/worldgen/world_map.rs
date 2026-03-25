use rand::{Rng, RngExt, SeedableRng};

use super::noise_util::NoiseLayer;
use super::zone::{ZoneGenContext, ZoneType};

/// Position on the world grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    pub region_id: Option<u32>,
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
            region_id: None,
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
            neighbors,
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
    let elev_noise = NoiseLayer::new(elev_seed, 0.008, 6);
    let ridge_noise = NoiseLayer::new(elev_seed.wrapping_add(1000), 0.025, 4);
    let mut elevation = vec![0.0f32; n];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let base = elev_noise.sample_normalized(x as f64, y as f64) as f32;
            // Ridge noise: invert so ridges (near 0 in raw noise) become peaks
            let ridge_raw = ridge_noise.sample(x as f64, y as f64) as f32;
            let ridge = (1.0 - ridge_raw.abs()) * 0.15; // subtle ridge contribution
            // Only apply ridges on land (where base is already above ocean level)
            let ridge_masked = if base > 0.45 { ridge } else { 0.0 };
            // Apply contrast curve: push values away from the midpoint for steeper slopes
            let combined = base * 0.85 + ridge_masked;
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
    let ocean_threshold = 0.42f32;
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
    generate_rivers(
        &elevation,
        &mut river,
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
            cells.push(cell);
        }
    }

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
    if moisture > 0.50 && temperature > 0.25 {
        return ZoneType::Forest;
    }
    ZoneType::Grassland
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
            moisture[i] = (moisture[i] + factor * 0.3).min(1.0);
        }
    }
}

/// Generate rivers by walking downhill from high-elevation sources to ocean.
fn generate_rivers(
    elevation: &[f32],
    river: &mut [bool],
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
        walk_river(c, elevation, river, moisture, width, height, ocean_threshold);
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

    // Rasterize the smooth path with increasing width
    let path_len = path.len() as f32;
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
                }
            }
        }

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

/// Place settlements on suitable land cells, spaced apart.
fn place_settlements(cells: &mut [WorldCell], width: u32, height: u32, rng: &mut impl Rng) {
    let target_count = (width as usize * height as usize / 10000).clamp(2, 12);
    let min_dist_sq = (width.min(height) / 8).pow(2) as u32;

    // Candidates: Grassland or Forest, prefer river adjacency
    let mut candidates: Vec<(usize, bool)> = cells
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c.zone_type, ZoneType::Grassland | ZoneType::Forest))
        .map(|(i, c)| (i, c.river))
        .collect();

    // Sort river-adjacent first, then shuffle within each group
    candidates.sort_by_key(|(_, has_river)| if *has_river { 0 } else { 1 });
    // Shuffle within river group
    let river_end = candidates.iter().position(|(_, r)| !r).unwrap_or(candidates.len());
    for i in (1..river_end).rev() {
        let j = rng.random_range(0..=i);
        candidates.swap(i, j);
    }
    // Shuffle non-river group
    for i in (river_end + 1..candidates.len()).rev() {
        let j = rng.random_range(river_end..=i);
        candidates.swap(i, j);
    }

    let mut placed: Vec<(u32, u32)> = Vec::new();

    for (i, _) in &candidates {
        if placed.len() >= target_count {
            break;
        }
        let x = (*i as u32) % width;
        let y = (*i as u32) / width;

        // Check minimum distance from existing settlements
        let too_close = placed
            .iter()
            .any(|(sx, sy)| x.abs_diff(*sx).pow(2) + y.abs_diff(*sy).pow(2) < min_dist_sq);
        if too_close {
            continue;
        }

        cells[*i].zone_type = ZoneType::Settlement;
        cells[*i].has_cave = false;
        placed.push((x, y));
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
        let has_settlement = map.cells.iter().any(|c| c.zone_type == ZoneType::Settlement);
        assert!(has_settlement);
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
}
