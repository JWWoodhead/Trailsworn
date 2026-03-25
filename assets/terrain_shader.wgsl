#import bevy_ecs_tilemap::common::{tilemap_data, sprite_texture, sprite_sampler}
#import bevy_ecs_tilemap::vertex_output::MeshVertexOutput

// Our custom material bindings (group 3)
@group(3) @binding(0) var terrain_textures: texture_2d_array<f32>;
@group(3) @binding(1) var terrain_sampler: sampler;
@group(3) @binding(2) var terrain_map: texture_2d<u32>;

struct TerrainParams {
    texture_scale: f32,
    blend_depth: f32,
    corner_radius: f32,
    map_width: f32,
};
@group(3) @binding(3) var<uniform> params: TerrainParams;

// Terrain blend priorities (must match TerrainType::blend_priority in Rust)
// tex_index == blend_priority for all types (both follow enum order)
fn priority(terrain_type: u32) -> u32 {
    switch terrain_type {
        case 0u: { return 0u; } // Grass
        case 1u: { return 1u; } // Dirt
        case 2u: { return 2u; } // Sand
        case 3u: { return 3u; } // Snow
        case 4u: { return 4u; } // Swamp
        case 5u: { return 5u; } // Stone
        case 6u: { return 6u; } // Forest
        case 7u: { return 7u; } // Water
        case 8u: { return 8u; } // Mountain
        default: { return 0u; }
    }
}

// Smooth edge gradient: 1.0 at t=0, fading to 0.0 at blend_depth
fn edge_gradient(t: f32, depth: f32) -> f32 {
    let border = 0.03; // ~2px solid border at 64px tiles
    if t <= border {
        return 1.0;
    }
    let adjusted = (t - border) / (depth - border);
    if adjusted >= 1.0 {
        return 0.0;
    }
    return 0.5 * (1.0 + cos(adjusted * 3.14159265));
}

// Smooth corner gradient: radial from corner
fn corner_gradient(dx: f32, dy: f32, radius: f32) -> f32 {
    let border = 0.03;
    let dist = sqrt(dx * dx + dy * dy);
    if dist <= border {
        return 1.0;
    }
    let adjusted = (dist - border) / (radius - border);
    if adjusted >= 1.0 {
        return 0.0;
    }
    return 0.5 * (1.0 + cos(adjusted * 3.14159265));
}

// Read terrain data at a tile coordinate, with bounds checking.
// Returns vec4<u32>: r=terrain_type, g=random_offset_x (0-255), b=random_offset_y (0-255), a=255
fn read_terrain_data(coord: vec2<i32>) -> vec4<u32> {
    let map_w = i32(params.map_width);
    let map_h = i32(params.map_width); // square map
    if coord.x < 0 || coord.y < 0 || coord.x >= map_w || coord.y >= map_h {
        return vec4<u32>(0u, 0u, 0u, 255u);
    }
    return textureLoad(terrain_map, coord, 0);
}

fn read_terrain(coord: vec2<i32>) -> u32 {
    return read_terrain_data(coord).r;
}

// Compute blend alpha from a specific neighbor direction
fn compute_neighbor_blend(local_uv: vec2<f32>, dir_idx: u32) -> f32 {
    let depth = params.blend_depth;
    let radius = params.corner_radius;

    // UV convention: local_uv.x = 0 at left, 1 at right
    //                local_uv.y (w) = 0 at NORTH/top, 1 at SOUTH/bottom
    // (confirmed from tilemap_vertex.wgsl: bot_left vertex gets w=1, top_left gets w=0)
    switch dir_idx {
        // Cardinals
        case 0u: { return edge_gradient(local_uv.y, depth); }              // North: max at y=0 (top)
        case 1u: { return edge_gradient(1.0 - local_uv.x, depth); }        // East: max at x=1 (right)
        case 2u: { return edge_gradient(1.0 - local_uv.y, depth); }        // South: max at y=1 (bottom)
        case 3u: { return edge_gradient(local_uv.x, depth); }              // West: max at x=0 (left)
        // Corners
        case 4u: { return corner_gradient(1.0 - local_uv.x, local_uv.y, radius); }        // NE: top-right
        case 5u: { return corner_gradient(1.0 - local_uv.x, 1.0 - local_uv.y, radius); }  // SE: bottom-right
        case 6u: { return corner_gradient(local_uv.x, 1.0 - local_uv.y, radius); }        // SW: bottom-left
        case 7u: { return corner_gradient(local_uv.x, local_uv.y, radius); }               // NW: top-left
        default: { return 0.0; }
    }
}

@fragment
fn fragment(in: MeshVertexOutput) -> @location(0) vec4<f32> {
    // World tile coordinate
    let world_tile = tilemap_data.chunk_pos + vec2<f32>(in.storage_position);
    let local_uv = in.uv.zw; // 0..1 within tile

    // Read this tile's terrain data (type + random UV offset)
    let tile_coord = vec2<i32>(world_tile);
    let tile_data = read_terrain_data(tile_coord);
    let my_type = tile_data.r;
    let my_pri = priority(my_type);

    // Per-tile random UV offset breaks visible repetition pattern
    let rand_offset = vec2<f32>(f32(tile_data.g), f32(tile_data.b)) / 255.0;

    // World-space UV with per-tile random offset for texture variation
    let world_uv = (world_tile + local_uv + rand_offset) / params.texture_scale;

    // Sample base terrain texture at world-space UV
    var color = textureSample(terrain_textures, terrain_sampler, world_uv, i32(my_type));

    // Neighbor offsets: N, E, S, W, NE, SE, SW, NW
    let offsets = array<vec2<i32>, 8>(
        vec2<i32>(0, 1), vec2<i32>(1, 0), vec2<i32>(0, -1), vec2<i32>(-1, 0),
        vec2<i32>(1, 1), vec2<i32>(1, -1), vec2<i32>(-1, -1), vec2<i32>(-1, 1)
    );

    // For each neighbor, blend higher-priority terrains on top
    // Process in priority order: first find the highest-priority neighbor,
    // then blend all directions that have that terrain
    var best_pri = 0u;
    var best_type = 0u;
    var blend_mask = 0u; // bitmask of directions with the best terrain

    for (var i = 0u; i < 8u; i++) {
        let n_type = read_terrain(tile_coord + offsets[i]);
        let n_pri = priority(n_type);

        if n_pri > my_pri {
            if n_pri > best_pri {
                best_pri = n_pri;
                best_type = n_type;
                blend_mask = 1u << i;
            } else if n_pri == best_pri {
                blend_mask |= 1u << i;
            }
        }
    }

    if best_pri > my_pri {
        // Suppress corners where adjacent cardinal already covers
        let has_n = (blend_mask & 1u) != 0u;
        let has_e = (blend_mask & 2u) != 0u;
        let has_s = (blend_mask & 4u) != 0u;
        let has_w = (blend_mask & 8u) != 0u;

        if has_n || has_e { blend_mask &= ~16u; } // suppress NE corner
        if has_e || has_s { blend_mask &= ~32u; } // suppress SE corner
        if has_s || has_w { blend_mask &= ~64u; } // suppress SW corner
        if has_w || has_n { blend_mask &= ~128u; } // suppress NW corner

        // Compute max alpha from all active directions
        var alpha = 0.0;
        for (var i = 0u; i < 8u; i++) {
            if (blend_mask & (1u << i)) != 0u {
                alpha = max(alpha, compute_neighbor_blend(local_uv, i));
            }
        }

        // Sample the higher-priority terrain at world-space UV and blend
        let blend_color = textureSample(terrain_textures, terrain_sampler, world_uv, i32(best_type));
        color = mix(color, blend_color, alpha);
    }

    return color * in.color;
}
