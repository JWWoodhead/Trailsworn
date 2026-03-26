use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use trailsworn::resources::terrain_material::TerrainMaterial;
use trailsworn::resources::abilities::AbilityRegistry;
use trailsworn::resources::ability_defs::register_starter_abilities;
use trailsworn::resources::affix_defs::register_starter_affixes;
use trailsworn::resources::affixes::AffixRegistry;
use trailsworn::resources::item_defs::register_starter_items;
use trailsworn::resources::items::{ItemInstanceRegistry, ItemRegistry};
use trailsworn::resources::body::{humanoid_template, BodyTemplates};
use trailsworn::resources::events::{AbilityCastEvent, AttackMissedEvent, CastInterruptedEvent, DamageDealtEvent};
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
use trailsworn::resources::world::{CurrentZone, ZoneTransitionEvent};
use trailsworn::systems::{
    ability_bar, camera, cast_bars, casting, character_sheet, combat, debug, equipment, floating_text, game_time,
    health_bars, hover_info, hud, inventory, movement, profiling, rendering, selection, spawning, task, ui_panel, world_map_ui, zone,
};
use rand::SeedableRng;
use trailsworn::worldgen::world_map::generate_world_map;

fn main() {
    let debug_flags = debug::DebugFlags::from_args();
    let world_seed = 42u64; // TODO: randomize or accept from CLI

    let settings = MapSettings::default();

    let mut body_templates = BodyTemplates::default();
    body_templates.register(humanoid_template());

    let mut faction_relations = FactionRelations::default();
    faction_relations.set(1, 2, Disposition::Hostile);

    // Populate ability and status effect registries
    let mut ability_registry = AbilityRegistry::default();
    let mut status_registry = StatusEffectRegistry::default();
    register_starter_abilities(&mut ability_registry, &mut status_registry);

    let mut item_registry = ItemRegistry::default();
    register_starter_items(&mut item_registry);

    let mut affix_registry = AffixRegistry::default();
    register_starter_affixes(&mut affix_registry);

    // Generate world map
    let world_map = generate_world_map(256, 256, world_seed);
    let spawn_pos = world_map.spawn_pos;
    let current_zone = CurrentZone::new(world_seed, spawn_pos);

    // Build god pool and draw this run's pantheon
    let god_pool = trailsworn::worldgen::gods::build_god_pool();
    let mut pantheon_rng = rand::rngs::StdRng::seed_from_u64(world_seed);
    let drawn_pantheon = god_pool.draw_pantheon(6, &mut pantheon_rng);

    // Generate the starting zone's tile world
    let start_ctx = world_map.zone_context(spawn_pos).unwrap();
    let start_zone = trailsworn::worldgen::zone::generate_zone_with_context(
        &start_ctx,
        settings.width,
        settings.height,
        current_zone.zone_seed,
    );

    let mut app = App::new();

    app.add_plugins(DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Trailsworn".into(),
                ..default()
            }),
            ..default()
        })
        .set(ImagePlugin::default_nearest())
    )
    .add_plugins(TilemapPlugin)
    .add_plugins(MaterialTilemapPlugin::<TerrainMaterial>::default())
    // State
    .init_state::<GameState>()
    // Resources
    .insert_resource(settings)
    .insert_resource(start_zone.tile_world)
    .insert_resource(GameTime::default())
    .insert_resource(faction_relations)
    .insert_resource(body_templates)
    .insert_resource(ability_registry)
    .insert_resource(status_registry)
    .insert_resource(item_registry)
    .insert_resource(affix_registry)
    .insert_resource(ItemInstanceRegistry::default())
    .insert_resource(Theme::default())
    .insert_resource(StableIdRegistry::default())
    .insert_resource(trailsworn::resources::selection::DragSelection::default())
    .insert_resource(trailsworn::resources::selection::TargetingMode::default())
    .insert_resource(InputMap::default())
    .insert_resource(ActionState::default())
    .insert_resource(trailsworn::systems::profiling::FrameProfiler::default())
    .insert_resource(ui_panel::ActiveUiTab::default())
    .insert_resource(world_map_ui::WorldMapVisible::default())
    .insert_resource(world_map_ui::WorldMapViewState::default())
    .insert_resource(trailsworn::resources::map::CursorPosition::default())
    .insert_resource(world_map)
    .insert_resource(current_zone)
    .insert_resource(god_pool)
    .insert_resource(drawn_pantheon)
    .insert_resource(trailsworn::resources::zone_persistence::ZoneStateCache::default())
    // Messages
    .add_message::<DamageDealtEvent>()
    .add_message::<AttackMissedEvent>()
    .add_message::<CastInterruptedEvent>()
    .add_message::<AbilityCastEvent>()
    .add_message::<ZoneTransitionEvent>()
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
    // Startup
    .add_systems(
        Startup,
        (
            camera::setup_camera,
            rendering::spawn_tilemap,
            spawning::spawn_player,
            hover_info::setup_hover_tooltip,
            hud::setup_hud,
            ability_bar::setup_ability_bar,
            ui_panel::setup_ui_panel,
            world_map_ui::setup_world_map_ui,
            spawn_initial_zone_entities,
            transition_to_playing,
        )
            .chain(),
    )
    // Update systems
    .add_systems(
        Update,
        (
            // Input
            (
                input::process_input,
                camera::update_cursor_position.after(input::process_input),
                game_time::game_speed_input.after(input::process_input),
                camera::camera_pan.after(input::process_input),
                camera::camera_zoom,
                selection::selection_input.after(camera::update_cursor_position),
                selection::right_click_command.after(camera::update_cursor_position),
                selection::ability_input.after(input::process_input),
                ui_panel::toggle_ui_panel.after(input::process_input),
                world_map_ui::toggle_world_map.after(input::process_input),
                world_map_ui::world_map_zoom.after(world_map_ui::toggle_world_map),
                world_map_ui::world_map_pan.after(world_map_ui::toggle_world_map),
                world_map_ui::world_map_click.after(world_map_ui::toggle_world_map),
            )
                .in_set(GameSet::Input),
            // Tick
            game_time::advance_game_time.in_set(GameSet::Tick),
            // Task scheduling, evaluation, execution
            (
                task::advance_eval_timers,
                task::propagate_pack_aggro.after(task::advance_eval_timers),
                task::flee.after(task::propagate_pack_aggro),
                task::use_ability.after(task::propagate_pack_aggro),
                task::engage_combat.after(task::propagate_pack_aggro),
                task::defend_self.after(task::propagate_pack_aggro),
                task::maintain_range.after(task::propagate_pack_aggro),
                task::follow_leader.after(task::propagate_pack_aggro),
                task::assign_task
                    .after(task::flee)
                    .after(task::use_ability)
                    .after(task::engage_combat)
                    .after(task::defend_self)
                    .after(task::maintain_range)
                    .after(task::follow_leader),
                task::execute_actions.after(task::assign_task),
                movement::resolve_movement.after(task::execute_actions),
            )
                .in_set(GameSet::Ai),
            // Combat
            (
                equipment::sync_equipment,
                combat::tick_weapon_cooldowns.after(equipment::sync_equipment),
                casting::tick_ability_cooldowns,
                casting::regenerate_resources,
                combat::auto_attack,
                casting::begin_cast.after(combat::auto_attack),
                casting::tick_casting.after(casting::begin_cast),
                casting::interrupt_casting.after(combat::auto_attack),
                combat::tick_status_effects,
                combat::cleanup_dead.after(casting::tick_casting),
            )
                .in_set(GameSet::Combat),
            // Movement + Zone transitions
            (
                movement::movement,
                zone::detect_zone_edge.after(movement::movement),
                zone::handle_zone_transition.after(zone::detect_zone_edge),
            )
                .in_set(GameSet::Movement),
            // UI (split into two groups to stay within Bevy's tuple limit)
            (
                health_bars::spawn_health_bars,
                health_bars::update_health_bars,
                health_bars::cleanup_orphaned_health_bars,
                cast_bars::spawn_cast_bars,
                cast_bars::update_cast_bars,
                cast_bars::cleanup_cast_bars,
                floating_text::spawn_damage_numbers,
                floating_text::animate_floating_text,
                hover_info::update_hover_tooltip,
                selection::update_selection_visuals,
                selection::draw_drag_box,
            )
                .in_set(GameSet::Ui),
            (
                hud::update_speed_indicator,
                hud::combat_log_damage,
                ability_bar::update_ability_bar,
                ability_bar::update_cast_bar,
                ability_bar::update_resource_bars,
                ability_bar::draw_targeting_reticle,
                ui_panel::update_tab_visuals,
                ui_panel::update_ui_panel_overlay,
                character_sheet::update_character_sheet,
                inventory::update_inventory_panel,
                world_map_ui::update_world_map_marker,
            )
                .in_set(GameSet::Ui),
            // Render
            (
                movement::sync_transforms,
                rendering::update_terrain_map,
            ).in_set(GameSet::Render),
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
                profiling::frame_profiler,
                profiling::entity_counter,
            )
                .in_set(GameSet::Render),
        );
    }

    app.run();
}

/// Spawn enemies from the starting zone's POIs.
fn spawn_initial_zone_entities(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_settings: Res<MapSettings>,
    body_templates: Res<BodyTemplates>,
    item_registry: Res<ItemRegistry>,
    mut instance_registry: ResMut<ItemInstanceRegistry>,
    current_zone: Res<CurrentZone>,
    world_map: Res<trailsworn::worldgen::WorldMap>,
) {
    let cell = match world_map.get(current_zone.world_pos) {
        Some(c) => c.clone(),
        None => return,
    };

    let zone_data = trailsworn::worldgen::zone::generate_zone(
        cell.zone_type,
        cell.has_cave,
        map_settings.width,
        map_settings.height,
        current_zone.zone_seed,
    );

    let pawn_texture = asset_server.load("pawn.png");
    let template = match body_templates.get("humanoid") {
        Some(t) => t,
        None => return,
    };

    let mut spawn_index = 0u32;
    for poi in &zone_data.pois {
        match &poi.kind {
            trailsworn::worldgen::zone::PoiKind::EnemyCamp { enemy_count }
            | trailsworn::worldgen::zone::PoiKind::WildlifeSpawn { creature_count: enemy_count } => {
                zone::spawn_enemy_camp(
                    &mut commands,
                    &pawn_texture,
                    &map_settings,
                    template,
                    &item_registry,
                    &mut instance_registry,
                    poi.x,
                    poi.y,
                    *enemy_count,
                    spawn_index,
                    None, // No snapshot for initial spawn
                );
                spawn_index += enemy_count;
            }
            trailsworn::worldgen::zone::PoiKind::CaveEntrance => {
                // TODO: cave entrance interactable
            }
        }
    }
}

fn transition_to_playing(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}
