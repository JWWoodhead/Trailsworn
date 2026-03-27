use rand::{Rng, RngExt, SeedableRng};

use super::noise_util::NoiseLayer;
use crate::resources::map::TileWorld;
use crate::resources::feature_defs::{FeatureId, FeatureRegistry};
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

/// A terrain feature to be spawned as a y-sorted sprite entity.
#[derive(Clone, Debug)]
pub struct TerrainFeature {
    pub x: u32,
    pub y: u32,
    pub kind: FeatureId,
}

/// The generated output for a zone.
pub struct ZoneData {
    pub tile_world: TileWorld,
    pub pois: Vec<PointOfInterest>,
    pub features: Vec<TerrainFeature>,
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
    /// Which edges the river enters/exits: [N, E, S, W].
    pub river_entry: [bool; 4],
    /// Normalized river width (0.0 = thin source, 1.0 = wide mouth).
    pub river_width: f32,
    /// Neighbor zone types: [N, E, S, W]. None = ocean or map edge.
    pub neighbors: [Option<ZoneType>; 4],
    /// Which edges border ocean: [N, E, S, W]. True = neighbor is Ocean.
    pub ocean_edges: [bool; 4],
}

/// Generate a zone's tile map and points of interest (legacy API, no world context).
pub fn generate_zone(
    zone_type: ZoneType,
    has_cave: bool,
    width: u32,
    height: u32,
    seed: u64,
) -> ZoneData {
    let registry = crate::resources::feature_defs::default_feature_registry();
    generate_zone_with_context(
        &ZoneGenContext {
            zone_type,
            has_cave,
            elevation: 0.5,
            moisture: 0.5,
            temperature: 0.5,
            river: false,
            river_entry: [false; 4],
            river_width: 0.0,
            neighbors: [None; 4],
            ocean_edges: [false; 4],
        },
        width,
        height,
        seed,
        &registry,
    )
}

/// Generate a zone using world-level context for coherent terrain.
pub fn generate_zone_with_context(
    ctx: &ZoneGenContext,
    width: u32,
    height: u32,
    seed: u64,
    feature_registry: &FeatureRegistry,
) -> ZoneData {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    let mut tile_world = match ctx.zone_type {
        ZoneType::Settlement => generate_settlement(ctx, width, height, &mut rng),
        ZoneType::Ocean => TileWorld::filled(width, height, TerrainType::Water),
        _ => generate_biome_terrain(ctx, width, height, seed, &mut rng),
    };

    // Apply directional coast water if this zone borders ocean
    if ctx.zone_type == ZoneType::Coast && ctx.ocean_edges.iter().any(|e| *e) {
        let coast_seed = (seed & 0xFFFFFFFF) as u32 ^ 0xC0A57;
        let coast_noise = NoiseLayer::new(coast_seed, 0.04, 4);
        apply_coast_water(&mut tile_world, ctx, &coast_noise);
    }

    // Carve river if this zone has one
    if ctx.river {
        carve_river(&mut tile_world, ctx, &mut rng);
    }

    // Scatter terrain features (modifies walk_cost/blocks_los, must happen before POI placement)
    let features = if matches!(ctx.zone_type, ZoneType::Settlement | ZoneType::Ocean) {
        Vec::new()
    } else {
        scatter_features(&mut tile_world, ctx, seed, feature_registry)
    };

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

    // Clear terrain around enemy camps and cull nearby features
    let camp_radius = 4i32;
    let feature_cull_dist = 6u32;
    for poi in &pois {
        if let PoiKind::EnemyCamp { .. } = &poi.kind {
            // Clear a dirt patch around the camp
            for dx in -camp_radius..=camp_radius {
                for dy in -camp_radius..=camp_radius {
                    if dx * dx + dy * dy > camp_radius * camp_radius {
                        continue;
                    }
                    let tx = poi.x as i32 + dx;
                    let ty = poi.y as i32 + dy;
                    if tx >= 0 && ty >= 0 && tx < width as i32 && ty < height as i32 {
                        tile_world.set_terrain(tx as u32, ty as u32, TerrainType::Dirt);
                    }
                }
            }
        }
    }

    // Remove features that overlap with camp areas
    let mut features = features;
    features.retain(|f| {
        !pois.iter().any(|poi| {
            matches!(poi.kind, PoiKind::EnemyCamp { .. })
                && f.x.abs_diff(poi.x) < feature_cull_dist
                && f.y.abs_diff(poi.y) < feature_cull_dist
        })
    });

    ZoneData { tile_world, pois, features }
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
            if wetness > wt + 0.3 { TerrainType::Dirt } // wet areas become muddy, not swampy
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

/// Apply directional water for coast zones based on which edges face ocean.
///
/// Creates a noise-modulated shoreline: water near the ocean edge, sand transition,
/// then the biome interior. Replaces the generic wetness-based water placement.
fn apply_coast_water(world: &mut TileWorld, ctx: &ZoneGenContext, noise: &NoiseLayer) {
    let w = world.width;
    let h = world.height;
    let water_depth = 35.0f32; // tiles of water from ocean edge
    let sand_depth = 50.0f32;  // tiles of sand/transition band

    for y in 0..h {
        for x in 0..w {
            // Find minimum distance to any ocean edge
            let distances = [
                if ctx.ocean_edges[0] { Some((h - 1 - y) as f32) } else { None }, // N
                if ctx.ocean_edges[1] { Some((w - 1 - x) as f32) } else { None }, // E
                if ctx.ocean_edges[2] { Some(y as f32) } else { None },             // S
                if ctx.ocean_edges[3] { Some(x as f32) } else { None },             // W
            ];
            let min_dist = distances.iter().filter_map(|d| *d).reduce(f32::min);

            let Some(dist) = min_dist else { continue };

            // Noise modulation: vary the shoreline
            let n = noise.sample_normalized(x as f64, y as f64) as f32;
            let noise_offset = (n - 0.5) * 15.0; // +/- 7.5 tiles of shoreline variation

            let effective_dist = dist - noise_offset;

            if effective_dist < water_depth {
                world.set_terrain(x, y, TerrainType::Water);
            } else if effective_dist < sand_depth {
                world.set_terrain(x, y, TerrainType::Sand);
            }
            // Beyond sand_depth: keep existing biome terrain
        }
    }
}

// ---------------------------------------------------------------------------
// Terrain feature scattering
// ---------------------------------------------------------------------------

/// Scatter terrain features across the zone using noise-driven placement.
///
/// Reads feature definitions and biome tables from the registry.
/// Modifies `tile_world.walk_cost` and `blocks_los` for blocking features.
/// Must be called before POI placement so `find_open_spot` respects blocked tiles.
fn scatter_features(
    tile_world: &mut TileWorld,
    ctx: &ZoneGenContext,
    seed: u64,
    registry: &FeatureRegistry,
) -> Vec<TerrainFeature> {
    let table = registry.biome_table(ctx.zone_type);
    let target_density = registry.biome_density(ctx.zone_type);
    if table.is_empty() || target_density == 0 {
        return Vec::new();
    }

    let feature_seed = seed ^ 0xFEA7;
    let mut rng = rand::rngs::StdRng::seed_from_u64(feature_seed);
    let noise = NoiseLayer::new((feature_seed & 0xFFFFFFFF) as u32, 0.06, 3);

    let w = tile_world.width;
    let h = tile_world.height;
    let margin = 5u32;
    let stride = 4u32;

    // Calculate placement probability to hit target density
    let candidates = ((w - 2 * margin) / stride) * ((h - 2 * margin) / stride);
    let place_prob = (target_density as f64 / candidates as f64).min(1.0);

    let mut features = Vec::with_capacity(target_density as usize);
    let mut blocking_count = 0u32;
    let max_blocking = target_density / 10; // Cap blocking features at ~10%

    for gy in (margin..(h - margin)).step_by(stride as usize) {
        for gx in (margin..(w - margin)).step_by(stride as usize) {
            // RNG-driven placement with noise modulation for spatial clustering
            let n = noise.sample_normalized(gx as f64, gy as f64);

            // Small random offset within the stride cell to avoid grid patterns
            let x = gx + rng.random_range(0..stride.min(w - margin - gx));
            let y = gy + rng.random_range(0..stride.min(h - margin - gy));

            let idx = tile_world.idx(x, y);

            // Only place on walkable terrain
            if tile_world.walk_cost[idx] <= 0.0 {
                continue;
            }

            // Terrain-aware density: scale placement probability by terrain type
            let terrain = tile_world.terrain[idx];
            let density_mult = terrain.scatter_density();
            if density_mult <= 0.0 {
                continue;
            }

            let adjusted_prob = place_prob * (0.5 + n) * density_mult as f64;
            if rng.random::<f64>() > adjusted_prob {
                continue;
            }

            // Filter biome table to features compatible with this terrain,
            // adjusting weights by the terrain multiplier
            let mut filtered: Vec<(FeatureId, f32)> = Vec::new();
            let mut total_w = 0.0f32;
            for &(id, base_weight) in table {
                if let Some(def) = registry.get(id) {
                    if let Some(&(_, tw)) = def.terrain_weights.iter().find(|(t, _)| *t == terrain) {
                        let w = base_weight as f32 * tw;
                        filtered.push((id, w));
                        total_w += w;
                    }
                }
            }
            if filtered.is_empty() || total_w <= 0.0 {
                continue;
            }

            // Weighted random selection from filtered table
            let roll = rng.random::<f32>() * total_w;
            let mut accum = 0.0f32;
            let mut selected_id = filtered[0].0;
            for &(id, fw) in &filtered {
                accum += fw;
                if roll < accum {
                    selected_id = id;
                    break;
                }
            }

            let Some(def) = registry.get(selected_id) else { continue };

            // Cap blocking features
            if def.blocks_movement && blocking_count >= max_blocking {
                continue;
            }

            // Don't block a tile if it would isolate neighbors
            if def.blocks_movement {
                let mut walkable_neighbors = 0u32;
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && ny >= 0 && nx < w as i32 && ny < h as i32 {
                        let ni = tile_world.idx(nx as u32, ny as u32);
                        if tile_world.walk_cost[ni] > 0.0 {
                            walkable_neighbors += 1;
                        }
                    }
                }
                if walkable_neighbors < 3 {
                    continue;
                }
                blocking_count += 1;
            }

            // Apply blocking effects to tile_world
            if def.blocks_movement {
                tile_world.walk_cost[idx] = 0.0;
            }
            if def.blocks_los {
                tile_world.blocks_los[idx] = true;
            }

            features.push(TerrainFeature {
                x,
                y,
                kind: selected_id,
            });
        }
    }

    features
}

/// Carve a river across the zone using world-level entry/exit metadata.
///
/// Uses `ctx.river_entry` to determine which edges the river enters/exits,
/// connects them with a noise-driven curved path, and scales width by `ctx.river_width`.
fn carve_river(world: &mut TileWorld, ctx: &ZoneGenContext, rng: &mut impl Rng) {
    let w = world.width as f32;
    let h = world.height as f32;
    let meander = NoiseLayer::new(rng.random::<u32>(), 0.02, 3);

    // Collect entry/exit points from the edge metadata [N, E, S, W]
    let mut endpoints: Vec<(f32, f32)> = Vec::new();
    let mid_offset = rng.random_range(-0.15f32..0.15); // slight offset from center
    if ctx.river_entry[0] { // N edge
        endpoints.push((w * (0.5 + mid_offset), h - 1.0));
    }
    if ctx.river_entry[1] { // E edge
        endpoints.push((w - 1.0, h * (0.5 + mid_offset)));
    }
    if ctx.river_entry[2] { // S edge
        endpoints.push((w * (0.5 + mid_offset), 0.0));
    }
    if ctx.river_entry[3] { // W edge
        endpoints.push((0.0, h * (0.5 + mid_offset)));
    }

    // Fallback: if no edge data, pick random horizontal/vertical traversal
    if endpoints.len() < 2 {
        endpoints.clear();
        let horizontal = rng.random::<bool>();
        if horizontal {
            let y_pos = h * (0.3 + rng.random::<f32>() * 0.4);
            endpoints.push((0.0, y_pos));
            endpoints.push((w - 1.0, y_pos));
        } else {
            let x_pos = w * (0.3 + rng.random::<f32>() * 0.4);
            endpoints.push((x_pos, 0.0));
            endpoints.push((x_pos, h - 1.0));
        }
    }

    // River width in tiles: thin (2-4) at source, wide (6-10) at mouth
    let base_half_width = 1.0 + ctx.river_width * 4.0; // 1-5 half-width

    // Connect each pair of consecutive endpoints with a curved path
    let start = endpoints[0];
    for target in endpoints.iter().skip(1) {
        // Interpolate from start to target with noise-driven curvature
        let steps = (((target.0 - start.0).abs() + (target.1 - start.1).abs()) * 1.5) as usize;
        let steps = steps.max(50);

        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            // Linear interpolation + perpendicular noise offset for curvature
            let base_x = start.0 + (target.0 - start.0) * t;
            let base_y = start.1 + (target.1 - start.1) * t;

            // Perpendicular direction for meander offset
            let dx = target.0 - start.0;
            let dy = target.1 - start.1;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let perp_x = -dy / len;
            let perp_y = dx / len;

            // Noise-driven curvature: stronger in the middle of the path
            let curve_strength = (t * (1.0 - t) * 4.0) * 25.0; // max ~25 tiles offset at midpoint
            let noise_val = meander.sample(base_x as f64 * 0.03, base_y as f64 * 0.03) as f32;
            let px = base_x + perp_x * noise_val * curve_strength;
            let py = base_y + perp_y * noise_val * curve_strength;

            // Width varies slightly along the path
            let width_here = base_half_width + meander.sample(s as f64 * 0.1, 0.0) as f32 * 0.5;
            let hw = width_here as i32;

            // Paint water
            let cx = px as i32;
            let cy = py as i32;
            for ddx in -hw..=hw {
                for ddy in -hw..=hw {
                    if ddx * ddx + ddy * ddy > hw * hw + 1 {
                        continue;
                    }
                    let rx = cx + ddx;
                    let ry = cy + ddy;
                    if rx >= 0 && ry >= 0 && rx < w as i32 && ry < h as i32 {
                        world.set_terrain(rx as u32, ry as u32, TerrainType::Water);
                    }
                }
            }

            // Riverbank: dirt strip alongside water
            let bank_hw = hw + 2;
            for ddx in -bank_hw..=bank_hw {
                for ddy in -bank_hw..=bank_hw {
                    let dist_sq = ddx * ddx + ddy * ddy;
                    if dist_sq <= hw * hw + 1 || dist_sq > bank_hw * bank_hw + 1 {
                        continue;
                    }
                    let rx = cx + ddx;
                    let ry = cy + ddy;
                    if rx >= 0 && ry >= 0 && rx < w as i32 && ry < h as i32 {
                        let ru = rx as u32;
                        let rv = ry as u32;
                        // Only place bank on non-water tiles
                        let idx = world.idx(ru, rv);
                        if world.terrain[idx] != TerrainType::Water {
                            world.set_terrain(ru, rv, TerrainType::Dirt);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Settlement (mostly hand-crafted, not noise-based)
// ---------------------------------------------------------------------------

fn generate_settlement(ctx: &ZoneGenContext, width: u32, height: u32, rng: &mut impl Rng) -> TileWorld {
    // Determine surrounding biome from neighbors (most common non-settlement type)
    let surrounding = ctx.neighbors.iter()
        .filter_map(|n| *n)
        .filter(|n| *n != ZoneType::Settlement)
        .next()
        .unwrap_or(ZoneType::Grassland);

    // Biome-specific materials
    let (fill_terrain, building_terrain, road_terrain, building_count) = match surrounding {
        ZoneType::Desert => (TerrainType::Sand, TerrainType::Sand, TerrainType::Dirt, 5),
        ZoneType::Tundra => (TerrainType::Snow, TerrainType::Stone, TerrainType::Dirt, 4),
        ZoneType::Swamp => (TerrainType::Dirt, TerrainType::Stone, TerrainType::Stone, 5),
        ZoneType::Coast => (TerrainType::Sand, TerrainType::Stone, TerrainType::Dirt, 5),
        ZoneType::Forest => (TerrainType::Grass, TerrainType::Stone, TerrainType::Dirt, 6),
        _ => (TerrainType::Grass, TerrainType::Stone, TerrainType::Dirt, 6),
    };

    let mut world = TileWorld::filled(width, height, fill_terrain);

    let cx = width / 2;
    let cy = height / 2;

    // Central area — the town square
    let square_radius = if building_count <= 4 { 12 } else { 15 };
    for x in (cx - square_radius)..(cx + square_radius) {
        for y in (cy - square_radius)..(cy + square_radius) {
            world.set_terrain(x, y, road_terrain);
        }
    }

    // Buildings (impassable blocks)
    for _ in 0..building_count {
        let spread = square_radius - 3;
        let bx = rng.random_range(cx - spread..cx + spread);
        let by = rng.random_range(cy - spread..cy + spread);
        let bw = rng.random_range(3..6);
        let bh = rng.random_range(3..6);
        for x in bx..(bx + bw).min(width) {
            for y in by..(by + bh).min(height) {
                world.set_terrain(x, y, building_terrain);
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
        world.set_terrain(x, cy, road_terrain);
    }
    for y in 0..height {
        world.set_terrain(cx, y, road_terrain);
    }

    // Biome-specific features
    match surrounding {
        ZoneType::Desert => {
            // Small oasis near the center
            let ox = cx + rng.random_range(5..10);
            let oy = cy + rng.random_range(5..10);
            for dx in -2i32..=2 {
                for dy in -2i32..=2 {
                    if dx * dx + dy * dy <= 4 {
                        let wx = (ox as i32 + dx).clamp(0, width as i32 - 1) as u32;
                        let wy = (oy as i32 + dy).clamp(0, height as i32 - 1) as u32;
                        world.set_terrain(wx, wy, TerrainType::Water);
                    }
                }
            }
        }
        ZoneType::Coast => {
            // Water along ocean-facing edges
            let depth = 8;
            if ctx.ocean_edges[0] { // N
                for x in 0..width { for y in (height - depth)..height { world.set_terrain(x, y, TerrainType::Water); } }
            }
            if ctx.ocean_edges[1] { // E
                for x in (width - depth)..width { for y in 0..height { world.set_terrain(x, y, TerrainType::Water); } }
            }
            if ctx.ocean_edges[2] { // S
                for x in 0..width { for y in 0..depth { world.set_terrain(x, y, TerrainType::Water); } }
            }
            if ctx.ocean_edges[3] { // W
                for x in 0..depth { for y in 0..height { world.set_terrain(x, y, TerrainType::Water); } }
            }
            // Fallback: no ocean edges known, default south
            if !ctx.ocean_edges.iter().any(|e| *e) {
                for x in 0..width { for y in 0..depth { world.set_terrain(x, y, TerrainType::Water); } }
            }
        }
        ZoneType::Swamp => {
            // Scattered water puddles in the outskirts
            for _ in 0..8 {
                let px = rng.random_range(10..width - 10);
                let py = rng.random_range(10..height - 10);
                // Don't place in the town square
                if px.abs_diff(cx) < square_radius && py.abs_diff(cy) < square_radius {
                    continue;
                }
                for dx in -1i32..=1 {
                    for dy in -1i32..=1 {
                        let wx = (px as i32 + dx).clamp(0, width as i32 - 1) as u32;
                        let wy = (py as i32 + dy).clamp(0, height as i32 - 1) as u32;
                        world.set_terrain(wx, wy, TerrainType::Water);
                    }
                }
            }
        }
        _ => {}
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
            river_entry: [false, true, false, true], // W to E
            river_width: 0.5,
            neighbors: [None; 4],
            ocean_edges: [false; 4],
        };
        let registry = crate::resources::feature_defs::default_feature_registry();
        let data = generate_zone_with_context(&ctx, 250, 250, 42, &registry);
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
            river_entry: [false; 4],
            river_width: 0.0,
            neighbors: [Some(ZoneType::Desert), None, None, None], // Desert to the north
            ocean_edges: [false; 4],
        };
        let registry = crate::resources::feature_defs::default_feature_registry();
        let data = generate_zone_with_context(&ctx, 250, 250, 42, &registry);
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

    #[test]
    fn terrain_composition_per_biome() {
        let total = 250 * 250;
        let min_base_pct = 0.30; // base terrain should be >= 30% of tiles

        // (zone_type, expected_base_terrain, label, min_percentage)
        let cases: Vec<(ZoneType, TerrainType, &str, f64)> = vec![
            (ZoneType::Grassland, TerrainType::Grass, "grass", min_base_pct),
            (ZoneType::Forest, TerrainType::Forest, "forest", min_base_pct),
            (ZoneType::Mountain, TerrainType::Stone, "stone", min_base_pct),
            (ZoneType::Desert, TerrainType::Sand, "sand", min_base_pct),
            (ZoneType::Tundra, TerrainType::Snow, "snow", min_base_pct),
            (ZoneType::Swamp, TerrainType::Swamp, "swamp", 0.20), // swamp has lots of water mixed in
            (ZoneType::Coast, TerrainType::Sand, "sand", 0.20), // coast has water/grass/dirt mixed in
        ];

        for (zone_type, expected_base, label, threshold) in &cases {
            // Test across a few seeds for robustness
            for seed in [42u64, 123, 999] {
                let data = generate_zone(*zone_type, false, 250, 250, seed);
                let base_count = data
                    .tile_world
                    .terrain
                    .iter()
                    .filter(|t| *t == expected_base)
                    .count();
                let pct = base_count as f64 / total as f64;
                assert!(
                    pct >= *threshold,
                    "{:?} (seed {seed}): {label} at {:.1}%, expected >= {:.0}%",
                    zone_type,
                    pct * 100.0,
                    threshold * 100.0
                );
            }
        }
    }

    #[test]
    fn coast_has_water() {
        let data = generate_zone(ZoneType::Coast, false, 250, 250, 42);
        let water_count = data
            .tile_world
            .terrain
            .iter()
            .filter(|t| **t == TerrainType::Water)
            .count();
        assert!(water_count > 500, "Coast zone should have water tiles, got {water_count}");
    }

    #[test]
    fn swamp_has_water() {
        let data = generate_zone(ZoneType::Swamp, false, 250, 250, 42);
        let water_count = data
            .tile_world
            .terrain
            .iter()
            .filter(|t| **t == TerrainType::Water)
            .count();
        assert!(water_count > 200, "Swamp zone should have water features, got {water_count}");
    }

    #[test]
    fn coast_south_ocean_has_water_at_bottom() {
        let ctx = ZoneGenContext {
            zone_type: ZoneType::Coast,
            has_cave: false,
            elevation: 0.5,
            moisture: 0.5,
            temperature: 0.5,
            river: false,
            river_entry: [false; 4],
            river_width: 0.0,
            neighbors: [Some(ZoneType::Grassland), None, None, None],
            ocean_edges: [false, false, true, false], // S ocean
        };
        let registry = crate::resources::feature_defs::default_feature_registry();
        let data = generate_zone_with_context(&ctx, 250, 250, 42, &registry);
        // Bottom 20 rows should have mostly water
        let bottom_water = (0..250)
            .flat_map(|x| (0..20).map(move |y| (x, y)))
            .filter(|&(x, y)| data.tile_world.terrain[data.tile_world.idx(x, y)] == TerrainType::Water)
            .count();
        // Top 20 rows should have very little water
        let top_water = (0..250)
            .flat_map(|x| (230..250).map(move |y| (x, y)))
            .filter(|&(x, y)| data.tile_world.terrain[data.tile_world.idx(x, y)] == TerrainType::Water)
            .count();
        assert!(bottom_water > 3000, "South coast should have water at bottom, got {bottom_water}");
        assert!(top_water < 500, "South coast should NOT have water at top, got {top_water}");
    }

    #[test]
    fn coast_north_ocean_has_water_at_top() {
        let ctx = ZoneGenContext {
            zone_type: ZoneType::Coast,
            has_cave: false,
            elevation: 0.5,
            moisture: 0.5,
            temperature: 0.5,
            river: false,
            river_entry: [false; 4],
            river_width: 0.0,
            neighbors: [None, None, Some(ZoneType::Grassland), None],
            ocean_edges: [true, false, false, false], // N ocean
        };
        let registry = crate::resources::feature_defs::default_feature_registry();
        let data = generate_zone_with_context(&ctx, 250, 250, 42, &registry);
        // Top 20 rows should have mostly water
        let top_water = (0..250)
            .flat_map(|x| (230..250).map(move |y| (x, y)))
            .filter(|&(x, y)| data.tile_world.terrain[data.tile_world.idx(x, y)] == TerrainType::Water)
            .count();
        // Bottom 20 rows should have very little water
        let bottom_water = (0..250)
            .flat_map(|x| (0..20).map(move |y| (x, y)))
            .filter(|&(x, y)| data.tile_world.terrain[data.tile_world.idx(x, y)] == TerrainType::Water)
            .count();
        assert!(top_water > 3000, "North coast should have water at top, got {top_water}");
        assert!(bottom_water < 500, "North coast should NOT have water at bottom, got {bottom_water}");
    }

    #[test]
    fn features_scattered_per_biome() {
        let biomes = [
            (ZoneType::Grassland, 150, 800),
            (ZoneType::Forest, 200, 2500),
            (ZoneType::Mountain, 100, 700),
            (ZoneType::Desert, 80, 500),
            (ZoneType::Tundra, 100, 600),
            (ZoneType::Swamp, 150, 800),
            (ZoneType::Coast, 100, 600),
        ];
        for (zone_type, min, max) in biomes {
            let data = generate_zone(zone_type, false, 250, 250, 42);
            let count = data.features.len();
            assert!(
                count >= min && count <= max,
                "{zone_type:?}: expected {min}-{max} features, got {count}"
            );
        }
    }

    #[test]
    fn features_on_walkable_terrain() {
        let data = generate_zone(ZoneType::Forest, false, 250, 250, 42);
        for feature in &data.features {
            assert!(feature.x < 250 && feature.y < 250, "Feature out of bounds");
            let idx = data.tile_world.idx(feature.x, feature.y);
            let terrain = data.tile_world.terrain[idx];
            assert!(
                terrain != TerrainType::Water && terrain != TerrainType::Mountain,
                "Feature at ({},{}) on {:?}", feature.x, feature.y, terrain
            );
        }
    }

    #[test]
    fn settlement_has_no_features() {
        let data = generate_zone(ZoneType::Settlement, false, 250, 250, 42);
        assert!(data.features.is_empty(), "Settlement should have no features");
    }

    #[test]
    fn forest_terrain_breakdown() {
        let data = generate_zone(ZoneType::Forest, false, 250, 250, 42);
        let total = data.tile_world.terrain.len();
        let mut counts = std::collections::HashMap::new();
        for t in &data.tile_world.terrain {
            *counts.entry(format!("{:?}", t)).or_insert(0usize) += 1;
        }
        let mut sorted: Vec<_> = counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (name, count) in &sorted {
            let pct = **count as f64 / total as f64 * 100.0;
            eprintln!("  {name}: {count} ({pct:.1}%)");
        }
    }

    #[test]
    fn blocking_features_update_walk_cost() {
        let registry = crate::resources::feature_defs::default_feature_registry();
        let data = generate_zone(ZoneType::Mountain, false, 250, 250, 42);
        for feature in &data.features {
            let def = registry.get(feature.kind).unwrap();
            if def.blocks_movement {
                let idx = data.tile_world.idx(feature.x, feature.y);
                assert_eq!(
                    data.tile_world.walk_cost[idx], 0.0,
                    "Blocking feature at ({},{}) should set walk_cost to 0",
                    feature.x, feature.y
                );
            }
        }
    }
}
