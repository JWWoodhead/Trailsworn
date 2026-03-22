use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use gold_and_glory::generation;
use gold_and_glory::resources::abilities::AbilityRegistry;
use gold_and_glory::resources::body::{humanoid_template, BodyTemplates};
use gold_and_glory::resources::faction::{Disposition, FactionRelations};
use gold_and_glory::resources::game_time::GameTime;
use gold_and_glory::resources::map::MapSettings;
use gold_and_glory::resources::status_effects::StatusEffectRegistry;
use gold_and_glory::systems::{
    ai, camera, combat, debug, game_time, input, movement, rendering, spawning,
};

fn main() {
    let debug_mode = std::env::args().any(|a| a == "--debug");

    let settings = MapSettings::default();
    let tile_world = generation::generate_test_map(settings.width, settings.height);

    // Register body templates
    let mut body_templates = BodyTemplates::default();
    body_templates.register(humanoid_template());

    // Set up faction relations
    let mut faction_relations = FactionRelations::default();
    faction_relations.set(1, 2, Disposition::Hostile); // player vs bandits

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
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
    .insert_resource(faction_relations)
    .insert_resource(body_templates)
    .insert_resource(AbilityRegistry::default())
    .insert_resource(StatusEffectRegistry::default())
    .add_systems(
        Startup,
        (
            camera::setup_camera,
            rendering::spawn_tilemap,
            spawning::spawn_test_scene,
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
            // AI + Combat
            ai::ai_decision.after(game_time::advance_game_time),
            ai::resolve_movement_intent.after(ai::ai_decision),
            combat::tick_weapon_cooldowns.after(game_time::advance_game_time),
            combat::auto_attack.after(ai::ai_decision),
            combat::tick_status_effects.after(game_time::advance_game_time),
            combat::cleanup_dead.after(combat::auto_attack),
            ai::cleanup_completed_commands.after(combat::auto_attack),
            // Movement
            movement::movement.after(ai::resolve_movement_intent),
            // Rendering (every frame)
            movement::sync_transforms.after(movement::movement),
        ),
    );

    if debug_mode {
        app.add_systems(Update, debug::draw_grid);
    }

    app.run();
}
