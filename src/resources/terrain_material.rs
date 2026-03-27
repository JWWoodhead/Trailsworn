use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;

/// Shader parameters for terrain rendering.
#[derive(Clone, Copy, Debug, Default, bevy::render::render_resource::ShaderType)]
pub struct TerrainParams {
    /// World tiles per texture repeat (e.g., 4.0 = texture repeats every 4 tiles).
    pub texture_scale: f32,
    /// How many tiles the blend texture covers before repeating.
    pub blend_texture_tiles: f32,
    /// Map width in tiles (for bounds checking in shader).
    pub map_width: f32,
    /// Map height in tiles.
    pub map_height: f32,
    /// Tile size in world units.
    pub tile_size: f32,
    pub _padding1: f32,
    pub _padding2: f32,
    pub _padding3: f32,
}

/// Custom terrain material that renders terrain with world-space UV tiling
/// and Rimworld-style weighted-average blending between terrain types.
#[derive(AsBindGroup, TypePath, Debug, Clone, Default, Asset)]
pub struct TerrainMaterial {
    /// 2D array texture with one layer per terrain type (512×512 each).
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub terrain_textures: Handle<Image>,

    /// Rgba8Uint texture (map_w × map_h) encoding terrain type index per tile.
    #[texture(2, dimension = "2d", sample_type = "u_int")]
    pub terrain_map: Handle<Image>,

    /// Shader parameters.
    #[uniform(3)]
    pub params: TerrainParams,

    /// Blend weight texture (RGBA: R=horizontal, G=vertical, B=corner, A=self).
    #[texture(4, dimension = "2d")]
    #[sampler(5)]
    pub blend_texture: Handle<Image>,
}

impl bevy::sprite_render::Material2d for TerrainMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "terrain_shader.wgsl".into()
    }
}

/// Resource holding the handle to the terrain map image (updated on zone transitions).
#[derive(Resource)]
pub struct TerrainMapHandle(pub Handle<Image>);

/// Resource holding the handle to the terrain material.
#[derive(Resource)]
pub struct TerrainMaterialHandle(pub Handle<TerrainMaterial>);
