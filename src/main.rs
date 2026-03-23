use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use trailsworn::generation;
use trailsworn::resources::abilities::AbilityRegistry;
use trailsworn::resources::body::{humanoid_template, BodyTemplates};
use trailsworn::resources::events::{AttackMissedEvent, DamageDealtEvent};
use trailsworn::resources::faction::{Disposition, FactionRelations};
use trailsworn::resources::game_state::{GameSet, GameState};
use trailsworn::resources::game_time::GameTime;
use trailsworn::resources::identity::{
    cleanup_stable_ids, register_stable_ids, StableIdRegistry,
};
use trailsworn::resources::input::{self, ActionState, InputMap};
use trailsworn::resources::map::MapSettings;
use trailsworn::resources::status_effects::StatusEffectRegistry;
use trailsworn::resources::theme::Theme;
use trailsworn::systems::{
    ai, camera, combat, debug, floating_text, game_time, health_bars, hover_info, hud, movement,
    rendering, selection, spawning,
};

fn main() {
    let debug_flags = debug::DebugFlags::from_args();

    let settings = MapSettings::default();
    let tile_world = generation::generate_test_map(settings.width, settings.height);

    let mut body_templates = BodyTemplates::default();
    body_templates.register(humanoid_template());

    let mut faction_relations = FactionRelations::default();
    faction_relations.set(1, 2, Disposition::Hostile);

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Trailsworn".into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(TilemapPlugin)
    // State
    .init_state::<GameState>()
    // Resources
    .insert_resource(settings)
    .insert_resource(tile_world)
    .insert_resource(GameTime::default())
    .insert_resource(faction_relations)
    .insert_resource(body_templates)
    .insert_resource(AbilityRegistry::default())
    .insert_resource(StatusEffectRegistry::default())
    .insert_resource(Theme::default())
    .insert_resource(StableIdRegistry::default())
    .insert_resource(trailsworn::resources::selection::DragSelection::default())
    .insert_resource(InputMap::default())
    .insert_resource(ActionState::default())
    // Messages
    .add_message::<DamageDealtEvent>()
    .add_message::<AttackMissedEvent>()
    // System set ordering
    .configure_sets(
        Update,
        (
            GameSet::Input,
            GameSet::Tick,
            GameSet::Ai,
            GameSet::Combat,
            GameSet::Movement,
            GameSet::Ui,
            GameSet::Render,
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    // Startup: transition to Playing after setup
    .add_systems(
        Startup,
        (
            camera::setup_camera,
            rendering::spawn_tilemap,
            spawning::spawn_test_scene,
            hover_info::setup_hover_tooltip,
            hud::setup_hud,
            transition_to_playing,
        )
            .chain(),
    )
    // Update systems organized by GameSet
    .add_systems(
        Update,
        (
            // Input
            (
                input::process_input,
                game_time::game_speed_input.after(input::process_input),
                camera::camera_pan.after(input::process_input),
                camera::camera_zoom,
                selection::selection_input.after(input::process_input),
                selection::right_click_command.after(input::process_input),
            )
                .in_set(GameSet::Input),
            // Tick
            game_time::advance_game_time.in_set(GameSet::Tick),
            // AI
            (ai::ai_decision, ai::resolve_movement_intent.after(ai::ai_decision))
                .in_set(GameSet::Ai),
            // Combat
            (
                combat::tick_weapon_cooldowns,
                combat::auto_attack,
                combat::tick_status_effects,
                combat::cleanup_dead.after(combat::auto_attack),
                ai::cleanup_commands.after(combat::auto_attack),
            )
                .in_set(GameSet::Combat),
            // Movement
            movement::movement.in_set(GameSet::Movement),
            // UI
            (
                health_bars::spawn_health_bars,
                health_bars::update_health_bars,
                health_bars::cleanup_orphaned_health_bars,
                floating_text::spawn_damage_numbers,
                floating_text::animate_floating_text,
                hover_info::update_hover_tooltip,
                selection::update_selection_visuals,
                selection::draw_drag_box,
                hud::update_speed_indicator,
                hud::combat_log_damage,
            )
                .in_set(GameSet::Ui),
            // Render
            movement::sync_transforms.in_set(GameSet::Render),
            // Identity (runs always, not state-gated)
            register_stable_ids,
            cleanup_stable_ids,
        ),
    );

    if let Some(flags) = debug_flags {
        app.insert_resource(flags);
        app.add_systems(
            Update,
            (
                debug::debug_key_toggles,
                debug::draw_grid,
                debug::draw_pathing,
                debug::draw_aggro_radius,
                debug::draw_ai_state,
            )
                .in_set(GameSet::Render),
        );
    }

    app.run();
}

fn transition_to_playing(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}
