use rand::{Rng, RngExt, SeedableRng};

use super::noise_util::NoiseLayer;
use crate::resources::map::TileWorld;
use crate::terrain::TerrainType;

/// What kind of zone this world map cell is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZoneType {
    Grassland,
    Forest,
    Mountain,
    Settlement,
    Desert,
    Tundra,
    Swamp,
    Coast,
    Ocean,
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

/// World-level context passed to zone generation for coherent terrain.
#[derive(Clone, Debug)]
pub struct ZoneGenContext {
    pub zone_type: ZoneType,
    pub has_cave: bool,
    pub elevation: f32,
    pub moisture: f32,
    pub temperature: f32,
    pub river: bool,
    /// Neighbor zone types: [N, E, S, W]. None = ocean or map edge.
    pub neighbors: [Option<ZoneType>; 4],
}

/// Generate a zone's tile map and points of interest (legacy API, no world context).
pub fn generate_zone(
    zone_type: ZoneType,
    has_cave: bool,
    width: u32,
    height: u32,
    seed: u64,
) -> ZoneData {
    generate_zone_with_context(
        &ZoneGenContext {
            zone_type,
            has_cave,
            elevation: 0.5,
            moisture: 0.5,
            temperature: 0.5,
            river: false,
            neighbors: [None; 4],
        },
        width,
        height,
        seed,
    )
}

/// Generate a zone using world-level context for coherent terrain.
pub fn generate_zone_with_context(
    ctx: &ZoneGenContext,
    width: u32,
    height: u32,
    seed: u64,
) -> ZoneData {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    let mut tile_world = match ctx.zone_type {
        ZoneType::Settlement => generate_settlement(width, height, &mut rng),
        ZoneType::Ocean => TileWorld::filled(width, height, TerrainType::Water),
        _ => generate_biome_terrain(ctx, width, height, seed, &mut rng),
    };

    // Carve river if this zone has one
    if ctx.river {
        carve_river(&mut tile_world, ctx, &mut rng);
    }

    let mut pois = Vec::new();

    // Place cave entrance
    if ctx.has_cave {
        let (cx, cy) = find_open_spot(width, height, &tile_world, &pois, &mut rng);
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
    if ctx.zone_type != ZoneType::Settlement {
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

    // Place wildlife spawns
    if matches!(
        ctx.zone_type,
        ZoneType::Grassland | ZoneType::Forest | ZoneType::Swamp | ZoneType::Coast
    ) {
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

// ---------------------------------------------------------------------------
// Noise-based biome terrain generation
// ---------------------------------------------------------------------------

/// Primary terrain for each zone type.
fn base_terrain(zone_type: ZoneType) -> TerrainType {
    match zone_type {
        ZoneType::Grassland => TerrainType::Grass,
        ZoneType::Forest => TerrainType::Forest,
        ZoneType::Mountain => TerrainType::Stone,
        ZoneType::Desert => TerrainType::Sand,
        ZoneType::Tundra => TerrainType::Snow,
        ZoneType::Swamp => TerrainType::Swamp,
        ZoneType::Coast => TerrainType::Sand,
        ZoneType::Settlement => TerrainType::Grass,
        ZoneType::Ocean => TerrainType::Water,
    }
}

/// Generate terrain for a non-settlement, non-ocean zone using noise layers.
fn generate_biome_terrain(
    ctx: &ZoneGenContext,
    width: u32,
    height: u32,
    seed: u64,
    rng: &mut impl Rng,
) -> TileWorld {
    let base = base_terrain(ctx.zone_type);
    let mut world = TileWorld::filled(width, height, base);

    // Derive noise seeds from zone seed
    let detail_seed = (seed & 0xFFFFFFFF) as u32;
    let wet_seed = ((seed >> 16) & 0xFFFFFFFF) as u32;
    let rocky_seed = ((seed >> 32) & 0xFFFFFFFF) as u32;

    // Detail noise: drives secondary terrain placement
    let detail = NoiseLayer::new(detail_seed, 0.03, 5);
    // Wetness noise: drives water/swamp placement
    let wetness = NoiseLayer::new(wet_seed, 0.025, 4);
    // Rocky noise: drives stone/mountain placement
    let rocky = NoiseLayer::new(rocky_seed, 0.02, 4);

    // Context-driven thresholds: wetter world cells get more water, etc.
    let wet_threshold = 0.7 - ctx.moisture * 0.3; // range 0.40-0.70
    let rocky_threshold = 0.7 - ctx.elevation * 0.25; // range 0.45-0.70

    for y in 0..height {
        for x in 0..width {
            let d = detail.sample_normalized(x as f64, y as f64);
            let w = wetness.sample_normalized(x as f64, y as f64);
            let r = rocky.sample_normalized(x as f64, y as f64);

            let terrain = pick_biome_terrain(ctx.zone_type, d, w, r, wet_threshold, rocky_threshold);
            if terrain != base {
                world.set_terrain(x, y, terrain);
            }
        }
    }

    // Edge blending with neighbors
    apply_edge_blending(&mut world, ctx, &detail, rng);

    world
}

/// Pick terrain for a single tile based on biome recipe and noise values.
fn pick_biome_terrain(
    zone_type: ZoneType,
    detail: f64,
    wetness: f64,
    rocky: f64,
    wet_thresh: f32,
    rocky_thresh: f32,
) -> TerrainType {
    let wt = wet_thresh as f64;
    let rt = rocky_thresh as f64;

    match zone_type {
        ZoneType::Grassland => {
            if wetness > wt + 0.15 { TerrainType::Water }
            else if rocky > rt + 0.1 { TerrainType::Stone }
            else if detail > 0.65 { TerrainType::Forest }
            else if detail > 0.55 { TerrainType::Dirt }
            else { TerrainType::Grass }
        }
        ZoneType::Forest => {
            if wetness > wt + 0.1 { TerrainType::Swamp }
            else if rocky > rt + 0.15 { TerrainType::Stone }
            else if detail > 0.7 { TerrainType::Grass }
            else if detail < 0.2 { TerrainType::Dirt }
            else { TerrainType::Forest }
        }
        ZoneType::Mountain => {
            if wetness > wt + 0.2 { TerrainType::Water }
            else if rocky > rt { TerrainType::Mountain }
            else if detail > 0.65 { TerrainType::Grass }
            else if detail > 0.5 { TerrainType::Dirt }
            else { TerrainType::Stone }
        }
        ZoneType::Desert => {
            if wetness > wt + 0.25 { TerrainType::Water } // rare oasis
            else if rocky > rt + 0.1 { TerrainType::Stone }
            else if detail > 0.7 { TerrainType::Dirt }
            else { TerrainType::Sand }
        }
        ZoneType::Tundra => {
            if wetness > wt + 0.15 { TerrainType::Water }
            else if rocky > rt { TerrainType::Mountain }
            else if detail > 0.65 { TerrainType::Stone }
            else if detail > 0.5 { TerrainType::Dirt }
            else { TerrainType::Snow }
        }
        ZoneType::Swamp => {
            if wetness > wt { TerrainType::Water }
            else if detail > 0.75 { TerrainType::Dirt }
            else if detail > 0.6 { TerrainType::Grass }
            else { TerrainType::Swamp }
        }
        ZoneType::Coast => {
            if wetness > wt + 0.05 { TerrainType::Water }
            else if detail > 0.65 { TerrainType::Grass }
            else if detail > 0.45 { TerrainType::Dirt }
            else { TerrainType::Sand }
        }
        _ => base_terrain(zone_type),
    }
}

/// Blend terrain near zone edges toward the neighbor's primary terrain.
fn apply_edge_blending(
    world: &mut TileWorld,
    ctx: &ZoneGenContext,
    detail: &NoiseLayer,
    _rng: &mut impl Rng,
) {
    let w = world.width;
    let h = world.height;
    let blend_width = 30.0f32;

    // [N, E, S, W] — each neighbor blends into the corresponding edge
    for y in 0..h {
        for x in 0..w {
            // Calculate distance to each edge
            let dist_n = (h - 1 - y) as f32; // North = top = high y
            let dist_s = y as f32;            // South = bottom = low y
            let dist_e = (w - 1 - x) as f32;
            let dist_w = x as f32;

            let edges = [
                (dist_n, ctx.neighbors[0]), // N
                (dist_e, ctx.neighbors[1]), // E
                (dist_s, ctx.neighbors[2]), // S
                (dist_w, ctx.neighbors[3]), // W
            ];

            for (dist, neighbor) in &edges {
                if *dist >= blend_width {
                    continue;
                }
                let Some(neighbor_type) = neighbor else {
                    continue;
                };

                let blend_factor = 1.0 - dist / blend_width;
                let noise_val = detail.sample_normalized(x as f64 * 1.5, y as f64 * 1.5);

                // Higher blend_factor near edge = more likely to use neighbor terrain
                if blend_factor as f64 > noise_val {
                    let neighbor_terrain = base_terrain(*neighbor_type);
                    world.set_terrain(x, y, neighbor_terrain);
                }
            }
        }
    }
}

/// Carve a river across the zone when ctx.river is true.
fn carve_river(world: &mut TileWorld, _ctx: &ZoneGenContext, rng: &mut impl Rng) {
    let w = world.width as i32;
    let h = world.height as i32;

    // Determine entry/exit edges based on which neighbors also have context
    // (simplified: pick a random traversal direction)
    let horizontal = rng.random::<bool>();

    let (mut x, mut y) = if horizontal {
        (0i32, rng.random_range(h / 4..h * 3 / 4))
    } else {
        (rng.random_range(w / 4..w * 3 / 4), 0i32)
    };

    let length = if horizontal { w } else { h };

    for _ in 0..length {
        // Place water in a 2-3 wide strip
        let river_width = 1i32;
        for dx in -river_width..=river_width {
            for dy in -river_width..=river_width {
                let rx = x + if horizontal { 0 } else { dx };
                let ry = y + if horizontal { dy } else { 0 };
                if rx >= 0 && ry >= 0 && rx < w && ry < h {
                    world.set_terrain(rx as u32, ry as u32, TerrainType::Water);
                }
            }
        }

        // Advance with meander
        if horizontal {
            x += 1;
            y += rng.random_range(-1..=1);
            y = y.clamp(2, h - 3);
        } else {
            y += 1;
            x += rng.random_range(-1..=1);
            x = x.clamp(2, w - 3);
        }
    }
}

// ---------------------------------------------------------------------------
// Settlement (mostly hand-crafted, not noise-based)
// ---------------------------------------------------------------------------

fn generate_settlement(width: u32, height: u32, rng: &mut impl Rng) -> TileWorld {
    let mut world = TileWorld::filled(width, height, TerrainType::Grass);

    let cx = width / 2;
    let cy = height / 2;

    // Central area is dirt (the town square)
    for x in (cx - 15)..(cx + 15) {
        for y in (cy - 15)..(cy + 15) {
            world.set_terrain(x, y, TerrainType::Dirt);
        }
    }

    // Stone buildings (impassable blocks)
    for _ in 0..6 {
        let bx = rng.random_range(cx - 12..cx + 12);
        let by = rng.random_range(cy - 12..cy + 12);
        let bw = rng.random_range(3..6);
        let bh = rng.random_range(3..6);
        for x in bx..(bx + bw).min(width) {
            for y in by..(by + bh).min(height) {
                world.set_terrain(x, y, TerrainType::Stone);
                if x == bx || x == bx + bw - 1 || y == by || y == by + bh - 1 {
                    let i = world.idx(x, y);
                    world.walk_cost[i] = 0.0;
                }
            }
        }
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

// ---------------------------------------------------------------------------
// POI helpers
// ---------------------------------------------------------------------------

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

        if tile_world.walk_cost[tile_world.idx(x, y)] <= 0.0 {
            continue;
        }

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

    (width / 2, height / 2)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grassland_is_walkable() {
        let data = generate_zone(ZoneType::Grassland, false, 250, 250, 42);
        let walkable = data.tile_world.walk_cost.iter().filter(|c| **c > 0.0).count();
        assert!(walkable > 50000);
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

    #[test]
    fn desert_uses_sand() {
        let data = generate_zone(ZoneType::Desert, false, 250, 250, 42);
        let sand_count = data
            .tile_world
            .terrain
            .iter()
            .filter(|t| **t == TerrainType::Sand)
            .count();
        assert!(sand_count > 30000, "Desert should be mostly sand, got {sand_count}");
    }

    #[test]
    fn tundra_uses_snow() {
        let data = generate_zone(ZoneType::Tundra, false, 250, 250, 42);
        let snow_count = data
            .tile_world
            .terrain
            .iter()
            .filter(|t| **t == TerrainType::Snow)
            .count();
        assert!(snow_count > 20000, "Tundra should be mostly snow, got {snow_count}");
    }

    #[test]
    fn swamp_uses_swamp_terrain() {
        let data = generate_zone(ZoneType::Swamp, false, 250, 250, 42);
        let swamp_count = data
            .tile_world
            .terrain
            .iter()
            .filter(|t| **t == TerrainType::Swamp)
            .count();
        assert!(swamp_count > 15000, "Swamp zone should have swamp terrain, got {swamp_count}");
    }

    #[test]
    fn river_zone_has_water() {
        let ctx = ZoneGenContext {
            zone_type: ZoneType::Grassland,
            has_cave: false,
            elevation: 0.5,
            moisture: 0.5,
            temperature: 0.5,
            river: true,
            neighbors: [None; 4],
        };
        let data = generate_zone_with_context(&ctx, 250, 250, 42);
        let water_count = data
            .tile_world
            .terrain
            .iter()
            .filter(|t| **t == TerrainType::Water)
            .count();
        assert!(water_count > 200, "River zone should have water tiles, got {water_count}");
    }

    #[test]
    fn edge_blending_with_neighbor() {
        let ctx = ZoneGenContext {
            zone_type: ZoneType::Grassland,
            has_cave: false,
            elevation: 0.5,
            moisture: 0.5,
            temperature: 0.5,
            river: false,
            neighbors: [Some(ZoneType::Desert), None, None, None], // Desert to the north
        };
        let data = generate_zone_with_context(&ctx, 250, 250, 42);
        // Check that the north edge (high y) has some sand tiles
        let north_sand = (220..250)
            .flat_map(|y| (0..250).map(move |x| (x, y)))
            .filter(|&(x, y)| data.tile_world.terrain[data.tile_world.idx(x, y)] == TerrainType::Sand)
            .count();
        assert!(north_sand > 100, "North edge should blend toward desert (sand), got {north_sand}");
    }

    #[test]
    fn all_biomes_generate_without_panic() {
        let biomes = [
            ZoneType::Grassland,
            ZoneType::Forest,
            ZoneType::Mountain,
            ZoneType::Desert,
            ZoneType::Tundra,
            ZoneType::Swamp,
            ZoneType::Coast,
            ZoneType::Settlement,
        ];
        for biome in biomes {
            let _ = generate_zone(biome, false, 250, 250, 42);
        }
    }
}
