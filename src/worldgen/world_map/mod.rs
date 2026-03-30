mod regions;
mod rivers;
mod roads;
mod settlements;
mod terrain;

#[cfg(test)]
mod tests;

use rand::{RngExt, SeedableRng};

use super::divine::DivineTerrainType;
use super::divine::gods::GodId;
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

    pub fn manhattan_distance(self, other: WorldPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
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
    /// Whether a road passes through this cell.
    pub road: bool,
    /// Which edges the road enters/exits: [N, E, S, W].
    pub road_entry: [bool; 4],
    /// Road importance: 0 = no road, 1 = minor (hamlet trail), 2 = major (town/city road).
    pub road_class: u8,
    /// Divine terrain overlay — what kind of divine scar exists here, if any.
    pub divine_terrain: Option<DivineTerrainType>,
    /// Which god last owned this cell during the divine era.
    pub divine_owner: Option<GodId>,
}

impl WorldCell {
    pub(super) fn new(zone_type: ZoneType) -> Self {
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
            road: false,
            road_entry: [false; 4],
            road_class: 0,
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
            road: cell.road,
            road_entry: cell.road_entry,
            road_class: cell.road_class,
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

    terrain::boost_moisture_near_ocean(&elevation, &mut moisture, width, height, ocean_threshold);

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
    rivers::generate_rivers(
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

            let zone_type = terrain::classify_biome(e, m, t, ocean_threshold);
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
    terrain::smooth_mountain_ranges(&mut cells, width, height);

    // --- Coast detection: land cells adjacent to ocean ---
    terrain::apply_coast_zones(&mut cells, width, height);

    // --- Region identification via flood-fill ---
    regions::assign_regions(&mut cells, width, height);

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
    settlements::place_settlements(&mut cells, width, height, &mut rng);

    // --- Road network connecting settlements ---
    roads::generate_roads(&mut cells, width, height);

    // --- Find spawn position ---
    let spawn_pos = settlements::find_spawn_pos(&cells, width, height);

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
