use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy_ecs_tilemap::prelude::*;

/// Shader parameters for terrain rendering.
#[derive(Clone, Copy, Debug, Default, bevy::render::render_resource::ShaderType)]
pub struct TerrainParams {
    /// World tiles per texture repeat (e.g., 4.0 = texture repeats every 4 tiles).
    pub texture_scale: f32,
    /// Edge blend depth as fraction of tile (0.0-1.0).
    pub blend_depth: f32,
    /// Corner blend radius as fraction of tile (0.0-1.0).
    pub corner_radius: f32,
    /// Map width in tiles (for bounds checking in shader).
    pub map_width: f32,
}

/// Custom terrain material that renders terrain with world-space UV tiling
/// and dynamic per-pixel blending between terrain types.
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
}

impl MaterialTilemap for TerrainMaterial {
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
