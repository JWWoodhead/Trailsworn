use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use gold_and_glory::generation;
use gold_and_glory::resources::faction::FactionRelations;
use gold_and_glory::resources::game_time::GameTime;
use gold_and_glory::resources::map::MapSettings;
use gold_and_glory::systems::{camera, game_time, input, movement, rendering, spawning};

fn main() {
    let settings = MapSettings::default();
    let tile_world = generation::generate_test_map(settings.width, settings.height);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gold and Glory".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TilemapPlugin)
        .insert_resource(settings)
        .insert_resource(tile_world)
        .insert_resource(GameTime::default())
        .insert_resource(FactionRelations::default())
        .add_systems(
            Startup,
            (
                camera::setup_camera,
                rendering::spawn_tilemap,
                spawning::spawn_test_pawn,
            ),
        )
        .add_systems(
            Update,
            (
                // Input (always runs, even when paused)
                game_time::game_speed_input,
                camera::camera_pan,
                camera::camera_zoom,
                input::click_to_move,
                // Simulation tick
                game_time::advance_game_time.after(game_time::game_speed_input),
                movement::movement.after(game_time::advance_game_time),
                // Rendering (every frame)
                movement::sync_transforms.after(movement::movement),
            ),
        )
        .run();
}
