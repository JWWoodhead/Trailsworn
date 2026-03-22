use bevy::prelude::*;

use crate::resources::faction::{Faction, FACTION_PLAYER};
use crate::resources::map::{render_layers, GridPosition, MapSettings};
use crate::resources::movement::{FacingDirection, MovementSpeed};

/// Marker for player-controlled entities.
#[derive(Component)]
pub struct PlayerControlled;

/// Spawn a test pawn at the given tile position.
pub fn spawn_test_pawn(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_settings: Res<MapSettings>,
) {
    let grid_pos = GridPosition::new(125, 125); // center of map
    let world_pos = grid_pos.to_world(map_settings.tile_size);

    info!("Spawning pawn: grid={grid_pos:?} world={world_pos:?} tile_size={}", map_settings.tile_size);

    commands.spawn((
        Sprite {
            image: asset_server.load("pawn.png"),
            ..default()
        },
        Transform::from_translation(Vec3::new(
            world_pos.x,
            world_pos.y,
            render_layers::ENTITIES,
        )),
        grid_pos,
        MovementSpeed::default(),
        FacingDirection::default(),
        Faction(FACTION_PLAYER),
        PlayerControlled,
    ));
}
