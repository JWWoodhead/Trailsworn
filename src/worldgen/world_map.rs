use rand::{Rng, RngExt, SeedableRng};

use super::zone::ZoneType;

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
            WorldPos::new(self.x, self.y + 1),
            WorldPos::new(self.x, self.y - 1),
            WorldPos::new(self.x + 1, self.y),
            WorldPos::new(self.x - 1, self.y),
        ]
    }
}

/// A cell on the world map grid.
#[derive(Clone, Debug)]
pub struct WorldCell {
    pub zone_type: ZoneType,
    pub has_cave: bool,
    pub explored: bool,
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
}

/// Generate a world map with the given dimensions.
pub fn generate_world_map(width: u32, height: u32, seed: u64) -> WorldMap {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let n = (width * height) as usize;

    let mut cells = Vec::with_capacity(n);

    // Assign zone types
    for y in 0..height {
        for x in 0..width {
            let zone_type = pick_zone_type(x, y, width, height, &mut rng);
            cells.push(WorldCell {
                zone_type,
                has_cave: false,
                explored: false,
            });
        }
    }

    // Place settlement near center
    let center = WorldPos::new(width as i32 / 2, height as i32 / 2);
    let center_idx = (center.y as u32 * width + center.x as u32) as usize;
    cells[center_idx].zone_type = ZoneType::Settlement;

    // Scatter caves in non-settlement zones
    for cell in &mut cells {
        if cell.zone_type != ZoneType::Settlement {
            if rng.random::<f32>() < 0.3 {
                cell.has_cave = true;
            }
        }
    }

    // Player spawns near the settlement
    let spawn_pos = WorldPos::new(center.x - 1, center.y);

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

fn pick_zone_type(x: u32, y: u32, width: u32, height: u32, rng: &mut impl Rng) -> ZoneType {
    // Simple noise-like distribution based on position
    let fx = x as f32 / width as f32;
    let fy = y as f32 / height as f32;

    // Mountains tend toward edges and corners
    let edge_dist = fx.min(1.0 - fx).min(fy).min(1.0 - fy);
    let mountain_chance = (1.0 - edge_dist * 4.0).max(0.0) * 0.6;

    // Forest tends toward one side
    let forest_chance = ((fx - 0.3).abs() * 2.0).min(0.5);

    let roll: f32 = rng.random();
    if roll < mountain_chance {
        ZoneType::Mountain
    } else if roll < mountain_chance + forest_chance * 0.3 {
        ZoneType::Forest
    } else {
        ZoneType::Grassland
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_map_has_correct_size() {
        let map = generate_world_map(5, 5, 42);
        assert_eq!(map.cells.len(), 25);
        assert_eq!(map.width, 5);
        assert_eq!(map.height, 5);
    }

    #[test]
    fn world_map_has_settlement() {
        let map = generate_world_map(5, 5, 42);
        let has_settlement = map.cells.iter().any(|c| c.zone_type == ZoneType::Settlement);
        assert!(has_settlement);
    }

    #[test]
    fn world_map_has_caves() {
        let map = generate_world_map(5, 5, 42);
        let caves = map.cells.iter().filter(|c| c.has_cave).count();
        assert!(caves > 0);
    }

    #[test]
    fn settlement_has_no_cave() {
        let map = generate_world_map(5, 5, 42);
        for cell in &map.cells {
            if cell.zone_type == ZoneType::Settlement {
                assert!(!cell.has_cave);
            }
        }
    }

    #[test]
    fn spawn_is_in_bounds() {
        let map = generate_world_map(5, 5, 42);
        assert!(map.in_bounds(map.spawn_pos));
    }

    #[test]
    fn spawn_zone_is_explored() {
        let map = generate_world_map(5, 5, 42);
        let cell = map.get(map.spawn_pos).unwrap();
        assert!(cell.explored);
    }
}
