use bevy::prelude::*;

use crate::resources::ai::{AiState, CombatBehavior, MovementIntent, RepathTimer};
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::combat::InCombat;
use crate::resources::damage::{DamageType, EquippedArmor, EquippedWeapon, WeaponDef};
use crate::resources::faction::Faction;
use crate::resources::game_state::GameState;
use crate::resources::game_time::GameTime;
use crate::resources::identity::StableId;
use crate::resources::map::{GridPosition, MapSettings, render_layers};
use crate::resources::movement::{FacingDirection, MovementSpeed};
use crate::resources::stats::{Attributes, CharacterLevel};
use crate::resources::status_effects::ActiveStatusEffects;
use crate::resources::threat::ThreatTable;
use crate::resources::world::{CurrentZone, EntryEdge, ZoneTransitionEvent};
use crate::resources::abilities::{Mana, Stamina};
use crate::systems::spawning::{EntityName, PlayerControlled};
use crate::worldgen::zone::PoiKind;
use crate::worldgen::{WorldMap, WorldPos};

/// Marker for entities that belong to the current zone and should be despawned on transition.
#[derive(Component)]
pub struct ZoneEntity;

/// Detect when player walks to the edge of the map and fire a transition event.
pub fn detect_zone_edge(
    game_time: Res<GameTime>,
    map_settings: Res<MapSettings>,
    world_map: Res<WorldMap>,
    current_zone: Res<CurrentZone>,
    player_query: Query<&GridPosition, With<PlayerControlled>>,
    mut transition_events: MessageWriter<ZoneTransitionEvent>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    for grid_pos in &player_query {
        let edge = if grid_pos.y >= map_settings.height - 1 {
            Some((EntryEdge::South, WorldPos::new(current_zone.world_pos.x, current_zone.world_pos.y + 1)))
        } else if grid_pos.y == 0 {
            Some((EntryEdge::North, WorldPos::new(current_zone.world_pos.x, current_zone.world_pos.y - 1)))
        } else if grid_pos.x >= map_settings.width - 1 {
            Some((EntryEdge::West, WorldPos::new(current_zone.world_pos.x + 1, current_zone.world_pos.y)))
        } else if grid_pos.x == 0 {
            Some((EntryEdge::East, WorldPos::new(current_zone.world_pos.x - 1, current_zone.world_pos.y)))
        } else {
            None
        };

        if let Some((entry_edge, new_pos)) = edge {
            if (*world_map).in_bounds(new_pos) {
                transition_events.write(ZoneTransitionEvent {
                    new_pos,
                    entry_edge,
                });
            }
        }
    }
}

/// Handle zone transitions: despawn zone entities, generate new zone, respawn.
pub fn handle_zone_transition(
    mut commands: Commands,
    mut transition_events: MessageReader<ZoneTransitionEvent>,
    mut current_zone: ResMut<CurrentZone>,
    world_map: Res<WorldMap>,
    map_settings: Res<MapSettings>,
    body_templates: Res<BodyTemplates>,
    asset_server: Res<AssetServer>,
    zone_entities: Query<Entity, With<ZoneEntity>>,
    mut player_query: Query<&mut GridPosition, With<PlayerControlled>>,
) {
    let Some(event) = transition_events.read().last() else {
        return;
    };

    let new_pos = event.new_pos;
    let entry_edge = event.entry_edge;

    // Update current zone
    current_zone.move_to(new_pos);

    // Despawn all zone entities (enemies, etc.)
    for entity in &zone_entities {
        commands.entity(entity).despawn();
    }

    // Get zone info from world map
    let cell = match (*world_map).get(new_pos) {
        Some(c) => c,
        None => return,
    };
    let zone_type = cell.zone_type;
    let has_cave = cell.has_cave;

    // Generate zone
    let zone_data = crate::worldgen::zone::generate_zone(
        zone_type,
        has_cave,
        map_settings.width,
        map_settings.height,
        current_zone.zone_seed,
    );

    // Replace tile world
    commands.insert_resource(zone_data.tile_world);

    // Reposition player based on entry edge
    let (spawn_x, spawn_y) = match entry_edge {
        EntryEdge::North => (map_settings.width / 2, map_settings.height - 5),
        EntryEdge::South => (map_settings.width / 2, 5),
        EntryEdge::East => (map_settings.width - 5, map_settings.height / 2),
        EntryEdge::West => (5, map_settings.height / 2),
        EntryEdge::Center => (map_settings.width / 2, map_settings.height / 2),
    };

    for mut grid_pos in &mut player_query {
        grid_pos.x = spawn_x;
        grid_pos.y = spawn_y;
    }

    // Spawn POI entities
    let pawn_texture = asset_server.load("pawn.png");
    let template = match body_templates.get("humanoid") {
        Some(t) => t,
        None => return,
    };

    for poi in &zone_data.pois {
        match &poi.kind {
            PoiKind::EnemyCamp { enemy_count } => {
                spawn_enemy_camp(
                    &mut commands,
                    &pawn_texture,
                    &map_settings,
                    template,
                    poi.x,
                    poi.y,
                    *enemy_count,
                );
            }
            PoiKind::WildlifeSpawn { creature_count } => {
                // TODO: wildlife entities — use enemy camp for now
                spawn_enemy_camp(
                    &mut commands,
                    &pawn_texture,
                    &map_settings,
                    template,
                    poi.x,
                    poi.y,
                    *creature_count,
                );
            }
            PoiKind::CaveEntrance => {
                // TODO: cave entrance interactable
            }
        }
    }

    // Mark zone as explored
    // Note: world_map is Res (immutable). We'd need ResMut to mark explored.
    // TODO: track explored state separately or use ResMut.
}

const FACTION_BANDITS: u32 = 2;

fn spawn_enemy_camp(
    commands: &mut Commands,
    texture: &Handle<Image>,
    map_settings: &MapSettings,
    body_template: &crate::resources::body::BodyTemplate,
    cx: u32,
    cy: u32,
    count: u32,
) {
    let weapon = WeaponDef {
        name: "Rusty Sword".into(),
        damage_type: DamageType::Slashing,
        base_damage: 5.0,
        attack_speed_ticks: 120,
        range: 1.5,
        projectile_speed: 0.0,
        is_melee: true,
    };

    for i in 0..count {
        // Spread enemies around the camp center
        let offset_x = (i % 3) as i32 - 1;
        let offset_y = (i / 3) as i32 - 1;
        let ex = (cx as i32 + offset_x * 2).max(0) as u32;
        let ey = (cy as i32 + offset_y * 2).max(0) as u32;
        let grid_pos = GridPosition::new(ex, ey);
        let world_pos = grid_pos.to_world(map_settings.tile_size);

        let name = format!("Bandit {}", i + 1);

        let mut entity_commands = commands.spawn((
            Name::new(name.clone()),
            StableId::next(),
            DespawnOnExit(GameState::Playing),
            ZoneEntity,
            Sprite {
                image: texture.clone(),
                color: Color::srgb(1.0, 0.4, 0.4),
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
            Faction(FACTION_BANDITS),
            EntityName(name),
        ));

        entity_commands.insert((
            Body::from_template(body_template),
            Attributes { strength: 4, agility: 4, toughness: 4, ..Default::default() },
            CharacterLevel::default(),
            EquippedWeapon::new(weapon.clone()),
            EquippedArmor::default(),
            Mana::new(50.0),
            Stamina::new(50.0),
            ActiveStatusEffects::default(),
            ThreatTable::default(),
            CombatBehavior::melee_enemy(Vec::new()),
            AiState::default(),
            MovementIntent::default(),
            RepathTimer::default(),
            InCombat,
        ));
    }
}
