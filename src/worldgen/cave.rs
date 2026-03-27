use rand::{Rng, RngExt, SeedableRng};

use crate::resources::map::TileWorld;
use crate::terrain::TerrainType;

use super::zone::{PoiKind, PointOfInterest, ZoneData};

/// Generate a cave interior zone using a simple cellular automata approach.
pub fn generate_cave(width: u32, height: u32, seed: u64) -> ZoneData {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // Start with random fill: ~45% stone (walls), 55% dirt (floor)
    let n = (width * height) as usize;
    let mut is_wall = vec![false; n];

    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            // Edges are always walls
            if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                is_wall[i] = true;
            } else {
                is_wall[i] = rng.random::<f32>() < 0.45;
            }
        }
    }

    // Cellular automata smoothing (4-5 rule)
    for _ in 0..5 {
        let mut next = is_wall.clone();
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let i = (y * width + x) as usize;
                let neighbors = count_wall_neighbors(&is_wall, x, y, width, height);
                next[i] = neighbors >= 5 || neighbors <= 1;
            }
        }
        is_wall = next;
    }

    // Ensure entrance area is clear (bottom center)
    let entrance_x = width / 2;
    let entrance_y = 5;
    clear_area(&mut is_wall, entrance_x, entrance_y, 4, width, height);

    // Build tile world
    let mut tile_world = TileWorld::filled(width, height, TerrainType::Stone);
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            if !is_wall[i] {
                tile_world.set_terrain(x, y, TerrainType::Dirt);
            } else {
                // Stone walls are impassable and block LOS
                let idx = tile_world.idx(x, y);
                tile_world.walk_cost[idx] = 0.0;
                tile_world.blocks_los[idx] = true;
            }
        }
    }

    // Place enemy spawns in open areas
    let mut pois = Vec::new();

    // Cave exit (the way back out)
    pois.push(PointOfInterest {
        x: entrance_x,
        y: entrance_y,
        kind: PoiKind::CaveEntrance, // reuse as exit marker
    });

    // Enemy groups deeper in the cave
    let group_count = rng.random_range(2..=4);
    for _ in 0..group_count {
        if let Some((x, y)) = find_open_cave_spot(&is_wall, width, height, &pois, &mut rng) {
            let enemy_count = rng.random_range(2..=4);
            pois.push(PointOfInterest {
                x,
                y,
                kind: PoiKind::EnemyCamp { enemy_count },
            });
        }
    }

    ZoneData { tile_world, pois, features: Vec::new() }
}

fn count_wall_neighbors(is_wall: &[bool], x: u32, y: u32, width: u32, height: u32) -> u32 {
    let mut count = 0;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                count += 1; // Out of bounds counts as wall
            } else {
                let ni = (ny as u32 * width + nx as u32) as usize;
                if is_wall[ni] {
                    count += 1;
                }
            }
        }
    }
    count
}

fn clear_area(is_wall: &mut [bool], cx: u32, cy: u32, radius: u32, width: u32, height: u32) {
    let r2 = (radius * radius) as i32;
    for dy in -(radius as i32)..=(radius as i32) {
        for dx in -(radius as i32)..=(radius as i32) {
            if dx * dx + dy * dy > r2 {
                continue;
            }
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx > 0 && ny > 0 && nx < width as i32 - 1 && ny < height as i32 - 1 {
                let ni = (ny as u32 * width + nx as u32) as usize;
                is_wall[ni] = false;
            }
        }
    }
}

fn find_open_cave_spot(
    is_wall: &[bool],
    width: u32,
    height: u32,
    existing_pois: &[PointOfInterest],
    rng: &mut impl Rng,
) -> Option<(u32, u32)> {
    let margin = 15;
    for _ in 0..200 {
        let x = rng.random_range(margin..width - margin);
        let y = rng.random_range(margin..height - margin);
        let i = (y * width + x) as usize;

        if is_wall[i] {
            continue;
        }

        // Check surrounding area is mostly open (3x3)
        let mut open = 0;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let ni = ((y as i32 + dy) as u32 * width + (x as i32 + dx) as u32) as usize;
                if !is_wall[ni] {
                    open += 1;
                }
            }
        }
        if open < 7 {
            continue;
        }

        // Distance from existing POIs
        let too_close = existing_pois.iter().any(|poi| {
            let dx = x.abs_diff(poi.x);
            let dy = y.abs_diff(poi.y);
            dx * dx + dy * dy < 400
        });
        if too_close {
            continue;
        }

        return Some((x, y));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cave_generates_walkable_areas() {
        let data = generate_cave(250, 250, 42);
        let walkable = data.tile_world.walk_cost.iter().filter(|c| **c > 0.0).count();
        // Should have significant open space (at least 20%)
        assert!(walkable > 12000, "only {walkable} walkable tiles");
    }

    #[test]
    fn cave_has_entrance() {
        let data = generate_cave(250, 250, 42);
        let entrances: Vec<_> = data
            .pois
            .iter()
            .filter(|p| matches!(p.kind, PoiKind::CaveEntrance))
            .collect();
        assert_eq!(entrances.len(), 1);
    }

    #[test]
    fn cave_has_enemies() {
        let data = generate_cave(250, 250, 42);
        let camps: Vec<_> = data
            .pois
            .iter()
            .filter(|p| matches!(p.kind, PoiKind::EnemyCamp { .. }))
            .collect();
        assert!(!camps.is_empty());
    }

    #[test]
    fn cave_entrance_is_walkable() {
        let data = generate_cave(250, 250, 42);
        let entrance = data
            .pois
            .iter()
            .find(|p| matches!(p.kind, PoiKind::CaveEntrance))
            .unwrap();
        let i = data.tile_world.idx(entrance.x, entrance.y);
        assert!(data.tile_world.walk_cost[i] > 0.0);
    }
}
