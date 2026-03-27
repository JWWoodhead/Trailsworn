#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Custom material bindings (group 2 for Material2d)
@group(2) @binding(0) var terrain_textures: texture_2d_array<f32>;
@group(2) @binding(1) var terrain_sampler: sampler;
@group(2) @binding(2) var terrain_map: texture_2d<u32>;

struct TerrainParams {
    texture_scale: f32,
    blend_texture_tiles: f32,
    map_width: f32,
    map_height: f32,
    tile_size: f32,
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
};
@group(2) @binding(3) var<uniform> params: TerrainParams;

@group(2) @binding(4) var blend_texture: texture_2d<f32>;
@group(2) @binding(5) var blend_sampler: sampler;

// Read terrain type at a tile coordinate, with bounds checking.
fn read_terrain(coord: vec2<i32>) -> u32 {
    let map_w = i32(params.map_width);
    let map_h = i32(params.map_height);
    if coord.x < 0 || coord.y < 0 || coord.x >= map_w || coord.y >= map_h {
        return 0u;
    }
    return textureLoad(terrain_map, coord, 0).r;
}

// Get the color for a terrain type at the current pixel.
// Uses world_tile position for megatexture-style tiling (texture repeats every texture_scale tiles).
fn get_terrain_color(world_tile: vec2<f32>, local_uv: vec2<f32>, terrain_type: u32) -> vec4<f32> {
    let world_uv = (world_tile + local_uv) / params.texture_scale;
    return textureSample(terrain_textures, terrain_sampler, world_uv, i32(terrain_type));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Compute tile coordinates from world position (avoids UV precision issues)
    let world_pos = in.world_position.xy / params.tile_size;
    let world_tile = floor(world_pos);
    let local_uv = fract(world_pos);
    let tile_coord = vec2<i32>(world_tile);

    // Current tile's terrain type
    let my_type = read_terrain(tile_coord);

    // Pixel position within tile (0 to tile_px-1)
    let blend_tex_size = textureDimensions(blend_texture, 0);
    let px_per_tile_f = f32(blend_tex_size.x) / params.blend_texture_tiles;
    let pixel_in_tile = vec2<f32>(
        floor(local_uv.x * px_per_tile_f),
        floor(local_uv.y * px_per_tile_f),
    );

    // Quadrant detection: clamp(-halfTile + pixelPos, -1, 1)
    let half_tile = px_per_tile_f * 0.5;
    let horizontal = clamp(-half_tile + pixel_in_tile.x, -1.0, 1.0);
    let vertical = clamp(-half_tile + pixel_in_tile.y, -1.0, 1.0);

    let h_dir = i32(sign(horizontal));
    let v_dir = i32(sign(vertical));

    // World space: Y increases upward, local_uv.y=0 is bottom of tile
    let h_offset = vec2<i32>(h_dir, 0);
    let v_offset = vec2<i32>(0, v_dir);
    let c_offset = vec2<i32>(h_dir, v_dir);

    // Get terrain types for neighbors
    let h_type = read_terrain(tile_coord + h_offset);
    let v_type = read_terrain(tile_coord + v_offset);
    let c_type = read_terrain(tile_coord + c_offset);

    // Sample terrain colors (all use current tile's world position for coherent UVs)
    let color_self = get_terrain_color(world_tile, local_uv, my_type);
    let color_h = get_terrain_color(world_tile, local_uv, h_type);
    let color_v = get_terrain_color(world_tile, local_uv, v_type);
    let color_c = get_terrain_color(world_tile, local_uv, c_type);

    // Sample blend texture using integer pixel coordinates (texelFetch equivalent)
    let blend_tile_x = i32(world_tile.x) % i32(params.blend_texture_tiles);
    let blend_tile_y = i32(world_tile.y) % i32(params.blend_texture_tiles);
    let blend_px = vec2<i32>(
        blend_tile_x * i32(px_per_tile_f) + i32(pixel_in_tile.x),
        blend_tile_y * i32(px_per_tile_f) + i32(pixel_in_tile.y),
    );
    // Use textureLoad (integer lookup, no filtering)
    let blend_raw = textureLoad(blend_texture, blend_px, 0);

    // Extract blend strengths: channel * 255 / 100
    let str_h = blend_raw.r * 255.0 / 100.0;
    let str_v = blend_raw.g * 255.0 / 100.0;
    let str_c = blend_raw.b * 255.0 / 100.0;
    let str_self = blend_raw.a * 255.0 / 100.0;

    // Weighted blend (additive)
    let color = color_self * str_self + color_h * str_h + color_v * str_v + color_c * str_c;

    return color;
}
