use bevy::prelude::*;
use bevy::image::ImageSampler;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy_ecs_tilemap::prelude::*;
use rand::RngExt;

use crate::resources::map::{MapSettings, TileWorld};
use crate::resources::terrain_material::{
    TerrainMaterial, TerrainMapHandle, TerrainMaterialHandle, TerrainParams,
};

/// Spawns the terrain tilemap using our custom TerrainMaterial shader.
pub fn spawn_tilemap(
    mut commands: Commands,
    tile_world: Res<TileWorld>,
    map_settings: Res<MapSettings>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    // Load the terrain texture array (stacked 512×N → 2D array, one layer per terrain type)
    let terrain_tex_handle: Handle<Image> = asset_server.load_with_settings(
        "terrain_array.png",
        |settings: &mut bevy::image::ImageLoaderSettings| {
            settings.is_srgb = true;
            // Linear filtering with Repeat addressing for seamless world-space UV tiling
            settings.sampler = ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                address_mode_w: bevy::image::ImageAddressMode::Repeat,
                ..bevy::image::ImageSamplerDescriptor::linear()
            });
            // Split the stacked image into layers (each 512px tall)
            settings.array_layout = Some(bevy::image::ImageArrayLayout::RowHeight { pixels: 512 });
        },
    );

    // Create the terrain map texture (Rgba8Uint, one pixel per tile)
    let terrain_map_image = create_terrain_map_image(&tile_world);
    let terrain_map_handle = images.add(terrain_map_image);

    // Create the material
    let material = TerrainMaterial {
        terrain_textures: terrain_tex_handle,
        terrain_map: terrain_map_handle.clone(),
        params: TerrainParams {
            texture_scale: 4.0,
            blend_depth: 0.5,
            corner_radius: 0.5,
            map_width: tile_world.width as f32,
        },
    };
    let material_handle = materials.add(material);

    // Store handles as resources
    commands.insert_resource(TerrainMapHandle(terrain_map_handle));
    commands.insert_resource(TerrainMaterialHandle(material_handle.clone()));

    // Spawn the tilemap
    let map_size = TilemapSize {
        x: tile_world.width,
        y: tile_world.height,
    };
    let tile_size = TilemapTileSize {
        x: map_settings.tile_size,
        y: map_settings.tile_size,
    };
    let grid_size = tile_size.into();

    let mut tile_storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    for y in 0..tile_world.height {
        for x in 0..tile_world.width {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(0),
                    ..default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    commands.entity(tilemap_entity).insert(MaterialTilemapBundle {
        grid_size,
        map_type: TilemapType::Square,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(asset_server.load("terrain.png")),
        tile_size,
        transform: Transform::from_translation(Vec3::ZERO),
        material: MaterialTilemapHandle::from(material_handle),
        ..default()
    });
}

/// Update the terrain map texture when TileWorld changes (zone transitions).
pub fn update_terrain_map(
    tile_world: Res<TileWorld>,
    terrain_map: Res<TerrainMapHandle>,
    mut images: ResMut<Assets<Image>>,
) {
    if !tile_world.is_changed() {
        return;
    }

    let Some(image) = images.get_mut(&terrain_map.0) else { return };
    let n = (tile_world.width * tile_world.height) as usize;

    let Some(ref mut data) = image.data else { return };
    if data.len() < n * 4 {
        return;
    }

    let mut rng = rand::rng();
    for i in 0..n {
        let base = i * 4;
        data[base] = tile_world.terrain[i].tile_texture_index() as u8;
        data[base + 1] = rng.random::<u8>();
        data[base + 2] = rng.random::<u8>();
        data[base + 3] = 255;
    }
}

/// Create an RGBA8Uint image holding terrain type indices in the R channel.
fn create_terrain_map_image(tile_world: &TileWorld) -> Image {
    let w = tile_world.width;
    let h = tile_world.height;
    let n = (w * h) as usize;

    let mut rng = rand::rng();
    let mut data = vec![0u8; n * 4];
    for i in 0..n {
        let base = i * 4;
        data[base] = tile_world.terrain[i].tile_texture_index() as u8;
        // G, B: random UV offset per tile (0-255 → 0.0-1.0 in shader)
        data[base + 1] = rng.random::<u8>();
        data[base + 2] = rng.random::<u8>();
        data[base + 3] = 255;
    }

    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Uint,
        default(), // RenderAssetUsages::MAIN_WORLD | RENDER_WORLD
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;

    image
}
