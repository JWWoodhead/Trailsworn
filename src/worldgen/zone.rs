use rand::{Rng, RngExt, SeedableRng};

use crate::resources::map::TileWorld;
use crate::terrain::TerrainType;

/// What kind of zone this world map cell is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZoneType {
    Grassland,
    Forest,
    Mountain,
    Settlement,
}

/// Points of interest within a generated zone.
#[derive(Clone, Debug)]
pub struct PointOfInterest {
    pub x: u32,
    pub y: u32,
    pub kind: PoiKind,
}

#[derive(Clone, Debug)]
pub enum PoiKind {
    CaveEntrance,
    EnemyCamp { enemy_count: u32 },
    WildlifeSpawn { creature_count: u32 },
}

/// The generated output for a zone.
pub struct ZoneData {
    pub tile_world: TileWorld,
    pub pois: Vec<PointOfInterest>,
}

/// Generate a zone's tile map and points of interest.
pub fn generate_zone(
    zone_type: ZoneType,
    has_cave: bool,
    width: u32,
    height: u32,
    seed: u64,
) -> ZoneData {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    let mut tile_world = match zone_type {
        ZoneType::Grassland => generate_grassland(width, height, &mut rng),
        ZoneType::Forest => generate_forest(width, height, &mut rng),
        ZoneType::Mountain => generate_mountain(width, height, &mut rng),
        ZoneType::Settlement => generate_settlement(width, height, &mut rng),
    };

    let mut pois = Vec::new();

    // Place cave entrance
    if has_cave {
        let (cx, cy) = find_open_spot(width, height, &tile_world, &pois, &mut rng);
        // Mark a small area as stone around the entrance
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                let tx = cx as i32 + dx;
                let ty = cy as i32 + dy;
                if tx >= 0 && ty >= 0 && tx < width as i32 && ty < height as i32 {
                    tile_world.set_terrain(tx as u32, ty as u32, TerrainType::Stone);
                }
            }
        }
        pois.push(PointOfInterest {
            x: cx,
            y: cy,
            kind: PoiKind::CaveEntrance,
        });
    }

    // Place enemy camps (1-3 per non-settlement zone)
    if zone_type != ZoneType::Settlement {
        let camp_count = rng.random_range(1..=3);
        for _ in 0..camp_count {
            let (cx, cy) = find_open_spot(width, height, &tile_world, &pois, &mut rng);
            let enemy_count = rng.random_range(2..=5);
            pois.push(PointOfInterest {
                x: cx,
                y: cy,
                kind: PoiKind::EnemyCamp { enemy_count },
            });
        }
    }

    // Place wildlife spawns in grassland/forest
    if matches!(zone_type, ZoneType::Grassland | ZoneType::Forest) {
        let wildlife_count = rng.random_range(1..=2);
        for _ in 0..wildlife_count {
            let (cx, cy) = find_open_spot(width, height, &tile_world, &pois, &mut rng);
            let creature_count = rng.random_range(2..=4);
            pois.push(PointOfInterest {
                x: cx,
                y: cy,
                kind: PoiKind::WildlifeSpawn { creature_count },
            });
        }
    }

    ZoneData { tile_world, pois }
}

/// Find a walkable tile far from existing POIs.
fn find_open_spot(
    width: u32,
    height: u32,
    tile_world: &TileWorld,
    existing_pois: &[PointOfInterest],
    rng: &mut impl Rng,
) -> (u32, u32) {
    let margin = 20;
    for _ in 0..100 {
        let x = rng.random_range(margin..width - margin);
        let y = rng.random_range(margin..height - margin);

        // Must be walkable
        if tile_world.walk_cost[tile_world.idx(x, y)] <= 0.0 {
            continue;
        }

        // Must be far from other POIs (at least 30 tiles)
        let too_close = existing_pois.iter().any(|poi| {
            let dx = x.abs_diff(poi.x);
            let dy = y.abs_diff(poi.y);
            dx * dx + dy * dy < 900
        });
        if too_close {
            continue;
        }

        return (x, y);
    }

    // Fallback
    (width / 2, height / 2)
}

fn generate_grassland(width: u32, height: u32, rng: &mut impl Rng) -> TileWorld {
    let mut world = TileWorld::filled(width, height, TerrainType::Grass);

    // Scatter dirt patches
    scatter_patches(&mut world, TerrainType::Dirt, 3, 8, 15, rng);

    // Scatter stone outcrops
    scatter_patches(&mut world, TerrainType::Stone, 2, 4, 8, rng);

    // A few tree clusters
    scatter_patches(&mut world, TerrainType::Forest, 2, 6, 12, rng);

    // Maybe a small pond
    if rng.random::<f32>() < 0.4 {
        place_pond(&mut world, rng);
    }

    world
}

fn generate_forest(width: u32, height: u32, rng: &mut impl Rng) -> TileWorld {
    let mut world = TileWorld::filled(width, height, TerrainType::Forest);

    // Clearings of grass
    scatter_patches(&mut world, TerrainType::Grass, 5, 10, 20, rng);

    // Dirt paths between clearings
    scatter_patches(&mut world, TerrainType::Dirt, 3, 3, 6, rng);

    // Streams
    if rng.random::<f32>() < 0.5 {
        place_stream(&mut world, rng);
    }

    world
}

fn generate_mountain(width: u32, height: u32, rng: &mut impl Rng) -> TileWorld {
    let mut world = TileWorld::filled(width, height, TerrainType::Stone);

    // Grassy valleys
    scatter_patches(&mut world, TerrainType::Grass, 4, 12, 25, rng);

    // Mountain peaks (impassable)
    scatter_patches(&mut world, TerrainType::Mountain, 3, 8, 15, rng);

    // Dirt patches
    scatter_patches(&mut world, TerrainType::Dirt, 2, 5, 10, rng);

    world
}

fn generate_settlement(width: u32, height: u32, rng: &mut impl Rng) -> TileWorld {
    let mut world = TileWorld::filled(width, height, TerrainType::Grass);

    // Central area is dirt (the town square)
    let cx = width / 2;
    let cy = height / 2;
    for x in (cx - 15)..(cx + 15) {
        for y in (cy - 15)..(cy + 15) {
            world.set_terrain(x, y, TerrainType::Dirt);
        }
    }

    // Stone buildings (just impassable blocks for now)
    for _ in 0..6 {
        let bx = rng.random_range(cx - 12..cx + 12);
        let by = rng.random_range(cy - 12..cy + 12);
        let bw = rng.random_range(3..6);
        let bh = rng.random_range(3..6);
        for x in bx..(bx + bw).min(width) {
            for y in by..(by + bh).min(height) {
                world.set_terrain(x, y, TerrainType::Stone);
                // Make walls impassable but leave a door
                if x == bx || x == bx + bw - 1 || y == by || y == by + bh - 1 {
                    let i = world.idx(x, y);
                    world.walk_cost[i] = 0.0;
                }
            }
        }
        // Door on one side
        let door_x = bx + bw / 2;
        let door_i = world.idx(door_x, by);
        world.walk_cost[door_i] = 1.0;
    }

    // Roads leading out in cardinal directions
    for x in 0..width {
        world.set_terrain(x, cy, TerrainType::Dirt);
    }
    for y in 0..height {
        world.set_terrain(cx, y, TerrainType::Dirt);
    }

    world
}

fn scatter_patches(
    world: &mut TileWorld,
    terrain: TerrainType,
    count: u32,
    min_radius: u32,
    max_radius: u32,
    rng: &mut impl Rng,
) {
    let margin = max_radius + 5;
    for _ in 0..count {
        let cx = rng.random_range(margin..world.width - margin);
        let cy = rng.random_range(margin..world.height - margin);
        let radius = rng.random_range(min_radius..=max_radius);
        let r2 = (radius * radius) as f32;

        for dx in -(radius as i32)..=(radius as i32) {
            for dy in -(radius as i32)..=(radius as i32) {
                let dist2 = (dx * dx + dy * dy) as f32;
                // Organic shape: jitter the boundary
                let jitter = rng.random::<f32>() * 0.4 + 0.8;
                if dist2 < r2 * jitter {
                    let tx = cx as i32 + dx;
                    let ty = cy as i32 + dy;
                    if tx >= 0
                        && ty >= 0
                        && tx < world.width as i32
                        && ty < world.height as i32
                    {
                        world.set_terrain(tx as u32, ty as u32, terrain);
                    }
                }
            }
        }
    }
}

fn place_pond(world: &mut TileWorld, rng: &mut impl Rng) {
    let margin = 30;
    let cx = rng.random_range(margin..world.width - margin);
    let cy = rng.random_range(margin..world.height - margin);
    let radius = rng.random_range(5..12);
    let r2 = (radius * radius) as f32;

    for dx in -(radius as i32)..=(radius as i32) {
        for dy in -(radius as i32)..=(radius as i32) {
            let dist2 = (dx * dx + dy * dy) as f32;
            let jitter = rng.random::<f32>() * 0.3 + 0.85;
            if dist2 < r2 * jitter {
                let tx = cx as i32 + dx;
                let ty = cy as i32 + dy;
                if tx >= 0 && ty >= 0 && tx < world.width as i32 && ty < world.height as i32 {
                    world.set_terrain(tx as u32, ty as u32, TerrainType::Water);
                }
            }
        }
    }
}

fn place_stream(world: &mut TileWorld, rng: &mut impl Rng) {
    // Meandering stream across the map
    let horizontal = rng.random::<bool>();
    let mut pos = if horizontal {
        (0i32, rng.random_range(50..world.height as i32 - 50))
    } else {
        (rng.random_range(50..world.width as i32 - 50), 0i32)
    };

    let length = if horizontal { world.width } else { world.height };

    for _ in 0..length {
        // Place water in a 2-wide strip
        for d in -1..=1 {
            let (wx, wy) = if horizontal {
                (pos.0, pos.1 + d)
            } else {
                (pos.0 + d, pos.1)
            };
            if wx >= 0 && wy >= 0 && wx < world.width as i32 && wy < world.height as i32 {
                world.set_terrain(wx as u32, wy as u32, TerrainType::Water);
            }
        }

        // Advance and meander
        if horizontal {
            pos.0 += 1;
            pos.1 += rng.random_range(-1..=1);
        } else {
            pos.1 += 1;
            pos.0 += rng.random_range(-1..=1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grassland_is_walkable() {
        let data = generate_zone(ZoneType::Grassland, false, 250, 250, 42);
        // Most tiles should be walkable
        let walkable = data.tile_world.walk_cost.iter().filter(|c| **c > 0.0).count();
        assert!(walkable > 50000); // more than 80% of 62500
    }

    #[test]
    fn cave_entrance_placed() {
        let data = generate_zone(ZoneType::Grassland, true, 250, 250, 42);
        let caves: Vec<_> = data
            .pois
            .iter()
            .filter(|p| matches!(p.kind, PoiKind::CaveEntrance))
            .collect();
        assert_eq!(caves.len(), 1);
    }

    #[test]
    fn enemy_camps_placed() {
        let data = generate_zone(ZoneType::Grassland, false, 250, 250, 42);
        let camps: Vec<_> = data
            .pois
            .iter()
            .filter(|p| matches!(p.kind, PoiKind::EnemyCamp { .. }))
            .collect();
        assert!(!camps.is_empty());
    }

    #[test]
    fn settlement_has_no_camps() {
        let data = generate_zone(ZoneType::Settlement, false, 250, 250, 42);
        let camps: Vec<_> = data
            .pois
            .iter()
            .filter(|p| matches!(p.kind, PoiKind::EnemyCamp { .. }))
            .collect();
        assert!(camps.is_empty());
    }

    #[test]
    fn mountain_has_impassable_peaks() {
        let data = generate_zone(ZoneType::Mountain, false, 250, 250, 42);
        let impassable = data.tile_world.walk_cost.iter().filter(|c| **c <= 0.0).count();
        assert!(impassable > 100);
    }
}
