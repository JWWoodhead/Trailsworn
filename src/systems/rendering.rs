use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::resources::map::{MapSettings, TileWorld};

/// Spawns the tilemap entities from the current TileWorld data.
pub fn spawn_tilemap(
    mut commands: Commands,
    tile_world: Res<TileWorld>,
    map_settings: Res<MapSettings>,
    asset_server: Res<AssetServer>,
) {
    let texture_handle: Handle<Image> = asset_server.load("terrain.png");

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
            let i = tile_world.idx(x, y);
            let texture_index = TileTextureIndex(tile_world.terrain[i].tile_texture_index());

            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index,
                    ..default()
                })
                .id();

            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let map_type = TilemapType::Square;

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: Transform::from_translation(Vec3::ZERO),
        ..default()
    });
}

/// Update tilemap visuals when TileWorld changes (e.g., zone transition).
/// bevy_ecs_tilemap detects Changed<TileTextureIndex> and re-renders.
pub fn sync_tilemap(
    tile_world: Res<TileWorld>,
    mut tiles: Query<(&TilePos, &mut TileTextureIndex)>,
) {
    if !tile_world.is_changed() {
        return;
    }

    for (pos, mut tex_idx) in &mut tiles {
        let i = tile_world.idx(pos.x, pos.y);
        tex_idx.0 = tile_world.terrain[i].tile_texture_index();
    }
}
