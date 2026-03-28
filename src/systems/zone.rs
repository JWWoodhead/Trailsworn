use bevy::prelude::*;

use crate::resources::combat_behavior::CombatBehavior;
use crate::resources::equipment_bonuses::EquipmentBonuses;
use crate::resources::movement::RepathTimer;
use crate::resources::task::AiBrain;
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::combat::InCombat;
use crate::resources::damage::{EquippedArmor, EquippedWeapon};
use crate::resources::faction::Faction;
use crate::resources::feature_defs::FeatureRegistry;
use crate::resources::game_state::GameState;
use crate::resources::game_time::GameTime;
use crate::resources::identity::StableId;
use crate::resources::item_defs::{
    ITEM_BENT_STAVE, ITEM_GNARLED_BRANCH, ITEM_KNOBWOOD_CLUB,
};
use crate::resources::pack::PackId;
use crate::resources::items::{Equipment, EquipSlot, ItemInstanceRegistry, ItemRegistry};
use crate::resources::map::{GridPosition, MapSettings, TerrainFeatureEntity, render_layers};
use crate::resources::movement::{FacingDirection, MovementSpeed, PathOffset};
use crate::resources::stats::{Attributes, CharacterLevel};
use crate::resources::status_effects::ActiveStatusEffects;
use crate::resources::threat::ThreatTable;
use crate::resources::world::{CurrentZone, EntryEdge, ZoneTransitionEvent};
use crate::resources::abilities::{Mana, Stamina};
use crate::resources::zone_persistence::ZoneSpawnIndex;
use crate::systems::spawning::{create_item_instance, EntityName, PlayerControlled, placeholder_weapon};
use crate::worldgen::zone::{PoiKind, ZoneType};
use crate::worldgen::{WorldMap, WorldPos};

/// Marker for entities that belong to the current zone and should be despawned on transition.
#[derive(Component)]
pub struct ZoneEntity;

/// Holds the starting zone's generated data so the startup system can spawn features + POIs.
#[derive(Resource)]
pub struct StartingZoneData {
    pub pois: Vec<crate::worldgen::zone::PointOfInterest>,
    pub features: Vec<crate::worldgen::zone::TerrainFeature>,
    pub zone_type: ZoneType,
}

/// Spawn features and POI entities for the starting zone (runs once on startup).
pub fn spawn_starting_zone(
    mut commands: Commands,
    starting_data: Option<Res<StartingZoneData>>,
    map_settings: Res<MapSettings>,
    body_templates: Res<BodyTemplates>,
    item_registry: Res<ItemRegistry>,
    mut instance_registry: ResMut<ItemInstanceRegistry>,
    _zone_cache: Res<crate::resources::zone_persistence::ZoneStateCache>,
    asset_server: Res<AssetServer>,
    feature_registry: Res<FeatureRegistry>,
) {
    let Some(data) = starting_data else { return };

    let pawn_texture: Handle<Image> = asset_server.load("pawn.png");
    let map_height_px = map_settings.height as f32 * map_settings.tile_size;

    // Spawn features
    for feature in &data.features {
        let Some(def) = feature_registry.get(feature.kind) else { continue };
        let world_x = feature.x as f32 * map_settings.tile_size + map_settings.tile_size * 0.5;
        let world_y = feature.y as f32 * map_settings.tile_size + map_settings.tile_size * 0.5;
        let z = render_layers::y_sorted_z(world_y, map_height_px, render_layers::TERRAIN_FEATURES);

        let (image, color, custom_size, y_offset) = if let Some(sprite_path) = def.sprite {
            let size = map_settings.tile_size * def.scale;
            (asset_server.load(sprite_path), Color::WHITE, Some(Vec2::new(size, size)), size * 0.5)
        } else {
            let c = def.placeholder_color;
            (pawn_texture.clone(), Color::srgba(c[0], c[1], c[2], c[3]), None, 0.0)
        };

        commands.spawn((
            ZoneEntity,
            TerrainFeatureEntity,
            DespawnOnExit(GameState::Playing),
            Sprite { image, color, custom_size, ..default() },
            Transform::from_translation(Vec3::new(world_x, world_y + y_offset, z)),
        ));
    }

    // Spawn POIs
    let template = match body_templates.get("humanoid") {
        Some(t) => t,
        None => {
            commands.remove_resource::<StartingZoneData>();
            return;
        }
    };

    let mut spawn_index = 0u32;
    let snapshot = None; // No snapshot for fresh starting zone
    for poi in &data.pois {
        match &poi.kind {
            PoiKind::EnemyCamp { enemy_count } => {
                spawn_enemy_camp(
                    &mut commands, &pawn_texture, &map_settings, template,
                    &item_registry, &mut instance_registry,
                    poi.x, poi.y, *enemy_count, spawn_index, snapshot,
                );
                spawn_index += enemy_count;
            }
            PoiKind::WildlifeSpawn { creature_count } => {
                spawn_wildlife_group(
                    &mut commands, &pawn_texture, &map_settings, template,
                    &item_registry, &mut instance_registry,
                    poi.x, poi.y, *creature_count, spawn_index, snapshot,
                    data.zone_type,
                );
                spawn_index += creature_count;
            }
            PoiKind::CaveEntrance => {
                let world_x = poi.x as f32 * map_settings.tile_size + map_settings.tile_size * 0.5;
                let world_y = poi.y as f32 * map_settings.tile_size + map_settings.tile_size * 0.5;
                commands.spawn((
                    Name::new("Cave Entrance"), ZoneEntity, DespawnOnExit(GameState::Playing),
                    Sprite {
                        image: pawn_texture.clone(), color: Color::srgb(0.3, 0.3, 0.3),
                        custom_size: Some(Vec2::new(map_settings.tile_size * 2.0, map_settings.tile_size * 2.0)),
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(world_x, world_y, render_layers::TERRAIN_FEATURES)),
                    GridPosition::new(poi.x, poi.y),
                ));
            }
        }
    }

    commands.remove_resource::<StartingZoneData>();
}

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
            if (*world_map).is_passable(new_pos) {
                transition_events.write(ZoneTransitionEvent {
                    new_pos,
                    entry_edge,
                });
            }
        }
    }
}

/// Handle zone transitions: snapshot alive entities, despawn, generate new zone, respawn.
pub fn handle_zone_transition(
    mut commands: Commands,
    mut transition_events: MessageReader<ZoneTransitionEvent>,
    mut current_zone: ResMut<CurrentZone>,
    world_map: Res<WorldMap>,
    map_settings: Res<MapSettings>,
    body_templates: Res<BodyTemplates>,
    item_registry: Res<ItemRegistry>,
    mut instance_registry: ResMut<ItemInstanceRegistry>,
    mut zone_cache: ResMut<crate::resources::zone_persistence::ZoneStateCache>,
    asset_server: Res<AssetServer>,
    zone_entities: Query<Entity, With<ZoneEntity>>,
    snapshot_query: Query<
        (&ZoneSpawnIndex, &GridPosition, &Body, &Mana, &Stamina, &Equipment),
        With<ZoneEntity>,
    >,
    mut player_query: Query<&mut GridPosition, (With<PlayerControlled>, Without<ZoneEntity>)>,
    feature_registry: Res<FeatureRegistry>,
) {
    let Some(event) = transition_events.read().last() else {
        return;
    };

    let new_pos = event.new_pos;
    let entry_edge = event.entry_edge;

    // --- Snapshot alive entities in the zone we are leaving ---
    let old_pos = current_zone.world_pos;
    let mut snapshot = zone_cache.remove(&old_pos).unwrap_or_default();
    // dead_indices already populated by cleanup_dead; now capture alive overrides
    snapshot.alive_overrides.clear();
    snapshot.preserved_item_instances.clear();

    for (spawn_idx, grid_pos, body, mana, stamina, equipment) in &snapshot_query {
        if snapshot.dead_indices.contains(&spawn_idx.0) {
            continue;
        }
        let entity_snap = crate::resources::zone_persistence::EntitySnapshot {
            position: (grid_pos.x, grid_pos.y),
            body_part_hp: body.parts.iter().map(|p| p.current_hp).collect(),
            body_part_destroyed: body.parts.iter().map(|p| p.destroyed).collect(),
            mana_current: mana.current,
            stamina_current: stamina.current,
            equipment_instance_ids: equipment.slots.iter().map(|(&s, &id)| (s, id)).collect(),
        };
        for (_, &id) in &equipment.slots {
            snapshot.preserved_item_instances.insert(id);
        }
        snapshot.alive_overrides.insert(spawn_idx.0, entity_snap);
    }
    zone_cache.insert(old_pos, snapshot);

    // Update current zone
    current_zone.move_to(new_pos);

    // Despawn all zone entities (enemies, etc.)
    // ItemInstances for alive entities are preserved in the snapshot;
    // dead entities' instances were already cleaned up by cleanup_dead.
    for entity in &zone_entities {
        commands.entity(entity).despawn();
    }

    // Get zone context from world map
    let ctx = match (*world_map).zone_context(new_pos) {
        Some(c) => c,
        None => return,
    };

    // Generate zone
    let zone_data = crate::worldgen::zone::generate_zone_with_context(
        &ctx,
        map_settings.width,
        map_settings.height,
        current_zone.zone_seed,
        &feature_registry,
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

    // Spread party members so they don't all stack on one tile
    let offsets: [(i32, i32); 4] = [(0, 0), (1, 0), (0, 1), (1, 1)];
    let w = map_settings.width as i32;
    let h = map_settings.height as i32;
    for (i, mut grid_pos) in player_query.iter_mut().enumerate() {
        let (dx, dy) = offsets[i % offsets.len()];
        grid_pos.x = (spawn_x as i32 + dx).clamp(0, w - 1) as u32;
        grid_pos.y = (spawn_y as i32 + dy).clamp(0, h - 1) as u32;
    }

    // Spawn terrain features (lightweight entities, no persistence)
    let pawn_texture: Handle<Image> = asset_server.load("pawn.png");
    let map_height_px = map_settings.height as f32 * map_settings.tile_size;
    for feature in &zone_data.features {
        let Some(def) = feature_registry.get(feature.kind) else { continue };
        let world_x = feature.x as f32 * map_settings.tile_size + map_settings.tile_size * 0.5;
        let world_y = feature.y as f32 * map_settings.tile_size + map_settings.tile_size * 0.5;
        let z = render_layers::y_sorted_z(world_y, map_height_px, render_layers::TERRAIN_FEATURES);

        let (image, color, custom_size, y_offset) = if let Some(sprite_path) = def.sprite {
            let size = map_settings.tile_size * def.scale;
            // Offset Y up by half sprite height so the bottom sits on the tile
            (asset_server.load(sprite_path), Color::WHITE, Some(Vec2::new(size, size)), size * 0.5)
        } else {
            let c = def.placeholder_color;
            (pawn_texture.clone(), Color::srgba(c[0], c[1], c[2], c[3]), None, 0.0)
        };

        commands.spawn((
            ZoneEntity,
            TerrainFeatureEntity,
            DespawnOnExit(GameState::Playing),
            Sprite {
                image,
                color,
                custom_size,
                ..default()
            },
            Transform::from_translation(Vec3::new(world_x, world_y + y_offset, z)),
        ));
    }

    // Spawn POI entities
    let template = match body_templates.get("humanoid") {
        Some(t) => t,
        None => return,
    };

    let new_zone_snapshot = zone_cache.get(&new_pos);
    let mut spawn_index = 0u32;
    for poi in &zone_data.pois {
        match &poi.kind {
            PoiKind::EnemyCamp { enemy_count } => {
                spawn_enemy_camp(
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
                    new_zone_snapshot,
                );
                spawn_index += enemy_count;
            }
            PoiKind::WildlifeSpawn { creature_count } => {
                spawn_wildlife_group(
                    &mut commands,
                    &pawn_texture,
                    &map_settings,
                    template,
                    &item_registry,
                    &mut instance_registry,
                    poi.x,
                    poi.y,
                    *creature_count,
                    spawn_index,
                    new_zone_snapshot,
                    ctx.zone_type,
                );
                spawn_index += creature_count;
            }
            PoiKind::CaveEntrance => {
                let world_x = poi.x as f32 * map_settings.tile_size + map_settings.tile_size * 0.5;
                let world_y = poi.y as f32 * map_settings.tile_size + map_settings.tile_size * 0.5;
                commands.spawn((
                    Name::new("Cave Entrance"),
                    ZoneEntity,
                    DespawnOnExit(GameState::Playing),
                    Sprite {
                        image: pawn_texture.clone(),
                        color: Color::srgb(0.3, 0.3, 0.3),
                        custom_size: Some(Vec2::new(
                            map_settings.tile_size * 2.0,
                            map_settings.tile_size * 2.0,
                        )),
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(
                        world_x,
                        world_y,
                        render_layers::TERRAIN_FEATURES,
                    )),
                    GridPosition::new(poi.x, poi.y),
                ));
            }
        }
    }

    // Mark zone as explored
    // Note: world_map is Res (immutable). We'd need ResMut to mark explored.
    // TODO: track explored state separately or use ResMut.
}

const FACTION_BANDITS: u32 = 2;

// ---------------------------------------------------------------------------
// Enemy roles & pack layout
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum EnemyRole {
    Melee,
    Ranged,
    Caster,
}

/// Determine the role for enemy `index` within a camp of `count`.
/// Camps of 4+ split into two packs; roles are assigned per-pack.
fn camp_role(index: u32, count: u32) -> EnemyRole {
    match count {
        // 1 pack: melee-heavy with a ranged
        2 => if index == 0 { EnemyRole::Melee } else { EnemyRole::Ranged },
        3 => if index < 2 { EnemyRole::Melee } else { EnemyRole::Ranged },
        // 2 packs: Pack A (indices 0..pack_a_size), Pack B (rest)
        // Camp 4 → Pack A: melee+ranged, Pack B: melee+caster
        4 => match index {
            0 => EnemyRole::Melee,
            1 => EnemyRole::Ranged,
            2 => EnemyRole::Melee,
            _ => EnemyRole::Caster,
        },
        // Camp 5 → Pack A: melee+melee+ranged, Pack B: ranged+caster
        _ => match index {
            0 | 1 => EnemyRole::Melee,
            2 => EnemyRole::Ranged,
            3 => EnemyRole::Ranged,
            _ => EnemyRole::Caster,
        },
    }
}

/// Number of enemies in Pack A. Pack B gets the rest.
fn pack_a_size(count: u32) -> u32 {
    match count {
        4 => 2,
        5 => 3,
        _ => count, // single pack
    }
}

/// Compute the sub-center for an enemy given pack splitting.
/// Single-pack camps use the camp center. Multi-pack camps offset ±15 tiles.
fn pack_center(index: u32, count: u32, cx: u32, cy: u32) -> (u32, u32) {
    if count < 4 {
        return (cx, cy);
    }
    let in_pack_a = index < pack_a_size(count);
    if in_pack_a {
        (cx.saturating_sub(15), cy)
    } else {
        (cx + 15, cy)
    }
}

/// Compute the PackId for an enemy.
fn enemy_pack_id(index: u32, count: u32, spawn_index_start: u32) -> PackId {
    if count < 4 || index < pack_a_size(count) {
        PackId(spawn_index_start)
    } else {
        PackId(spawn_index_start + 100)
    }
}

pub fn spawn_enemy_camp(
    commands: &mut Commands,
    texture: &Handle<Image>,
    map_settings: &MapSettings,
    body_template: &crate::resources::body::BodyTemplate,
    item_registry: &ItemRegistry,
    instance_registry: &mut ItemInstanceRegistry,
    cx: u32,
    cy: u32,
    count: u32,
    spawn_index_start: u32,
    snapshot: Option<&crate::resources::zone_persistence::ZoneSnapshot>,
) {
    use crate::resources::ability_defs::*;
    use crate::resources::combat_behavior::{AbilityPriority, UseCondition};

    for i in 0..count {
        let spawn_idx = spawn_index_start + i;

        // Skip dead entities from snapshot
        if let Some(snap) = snapshot
            && snap.dead_indices.contains(&spawn_idx)
        {
            continue;
        }

        let alive_override = snapshot.and_then(|s| s.alive_overrides.get(&spawn_idx));

        // --- Role-specific configuration ---
        let role = camp_role(i, count);

        let (weapon_item_id, role_name, sprite_color, base_mana, abilities, ability_priorities, attributes, behavior) =
            match role {
                EnemyRole::Melee => (
                    ITEM_KNOBWOOD_CLUB,
                    "Bandit Brute",
                    Color::srgb(1.0, 0.4, 0.4),
                    50.0,
                    vec![ABILITY_CLEAVE, ABILITY_WAR_CRY],
                    vec![
                        AbilityPriority { ability_id: ABILITY_WAR_CRY, slot_index: 1, condition: UseCondition::Always, priority: 20 },
                        AbilityPriority { ability_id: ABILITY_CLEAVE, slot_index: 0, condition: UseCondition::Always, priority: 10 },
                    ],
                    Attributes { strength: 6, agility: 3, toughness: 5, ..Default::default() },
                    {
                        let mut b = CombatBehavior::melee_enemy(vec![]); // priorities set below
                        b.flee_hp_threshold = 0.15;
                        b
                    },
                ),
                EnemyRole::Ranged => (
                    ITEM_BENT_STAVE,
                    "Bandit Archer",
                    Color::srgb(0.4, 0.8, 0.4),
                    50.0,
                    vec![ABILITY_AIMED_SHOT],
                    vec![
                        AbilityPriority { ability_id: ABILITY_AIMED_SHOT, slot_index: 0, condition: UseCondition::Always, priority: 10 },
                    ],
                    Attributes { strength: 3, agility: 6, toughness: 4, ..Default::default() },
                    {
                        let mut b = CombatBehavior::ranged_enemy(8.0, vec![]);
                        b.flee_hp_threshold = 0.25;
                        b
                    },
                ),
                EnemyRole::Caster => (
                    ITEM_GNARLED_BRANCH,
                    "Bandit Hexer",
                    Color::srgb(0.6, 0.4, 1.0),
                    80.0,
                    vec![ABILITY_FROST_BOLT, ABILITY_FIREBALL],
                    vec![
                        AbilityPriority { ability_id: ABILITY_FROST_BOLT, slot_index: 0, condition: UseCondition::Always, priority: 20 },
                        AbilityPriority { ability_id: ABILITY_FIREBALL, slot_index: 1, condition: UseCondition::Always, priority: 10 },
                    ],
                    Attributes { strength: 3, agility: 3, toughness: 3, intellect: 7, willpower: 5 },
                    {
                        let mut b = CombatBehavior::ranged_enemy(8.0, vec![]);
                        b.role = crate::resources::combat_behavior::CombatRole::Caster;
                        b.flee_hp_threshold = 0.30;
                        b.preferred_min_range = Some(5.0);
                        b
                    },
                ),
            };

        // Set ability_priorities on the behavior (couldn't pass through constructor due to borrow)
        let mut behavior = behavior;
        behavior.ability_priorities = ability_priorities;

        // Weapon instance: reuse from snapshot or create fresh
        let weapon_instance_id = if let Some(entity_snap) = alive_override {
            entity_snap.equipment_instance_ids
                .iter()
                .find(|(slot, _)| *slot == EquipSlot::MainHand)
                .map(|(_, id)| *id)
                .unwrap_or_else(|| create_item_instance(weapon_item_id, instance_registry))
        } else {
            create_item_instance(weapon_item_id, instance_registry)
        };

        let placeholder = placeholder_weapon(weapon_item_id, item_registry);

        let mut equipment = Equipment::default();
        equipment.equip(EquipSlot::MainHand, weapon_instance_id);

        // Position: snapshot override or deterministic pack offset
        let (pcx, pcy) = pack_center(i, count, cx, cy);
        let index_in_pack = if count >= 4 && i >= pack_a_size(count) {
            i - pack_a_size(count)
        } else {
            i
        };
        let (ex, ey) = if let Some(entity_snap) = alive_override {
            entity_snap.position
        } else {
            let offset_x = (index_in_pack % 3) as i32 - 1;
            let offset_y = (index_in_pack / 3) as i32 - 1;
            (
                (pcx as i32 + offset_x * 2).max(0) as u32,
                (pcy as i32 + offset_y * 2).max(0) as u32,
            )
        };
        let grid_pos = GridPosition::new(ex, ey);
        let world_pos = grid_pos.to_world(map_settings.tile_size);

        // Body: apply snapshot HP/destroyed overrides
        let mut body = Body::from_template(body_template);
        if let Some(entity_snap) = alive_override {
            for (j, part) in body.parts.iter_mut().enumerate() {
                if j < entity_snap.body_part_hp.len() {
                    part.current_hp = entity_snap.body_part_hp[j];
                    part.destroyed = entity_snap.body_part_destroyed[j];
                }
            }
        }

        // Resources: apply snapshot overrides
        let mut mana = Mana::new(base_mana);
        let mut stamina = Stamina::new(50.0);
        if let Some(entity_snap) = alive_override {
            mana.current = entity_snap.mana_current;
            stamina.current = entity_snap.stamina_current;
        }

        let name = format!("{} {}", role_name, i + 1);
        let pack_id = enemy_pack_id(i, count, spawn_index_start);

        let mut entity_commands = commands.spawn((
            Name::new(name.clone()),
            StableId::next(),
            DespawnOnExit(GameState::Playing),
            ZoneEntity,
            Sprite {
                image: texture.clone(),
                color: sprite_color,
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
            PathOffset::random(&mut rand::rng()),
            Faction(FACTION_BANDITS),
            EntityName(name),
        ));

        entity_commands.insert((
            body,
            attributes,
            CharacterLevel::default(),
            EquippedWeapon::new(placeholder),
            EquippedArmor::default(),
            EquipmentBonuses::default(),
            mana,
            stamina,
            ActiveStatusEffects::default(),
            ThreatTable::default(),
            crate::resources::abilities::AbilitySlots::new(abilities),
        ));
        entity_commands.insert((
            behavior,
            RepathTimer::default(),
            AiBrain::enemy(),
            InCombat,
            equipment,
            ZoneSpawnIndex(spawn_idx),
            pack_id,
        ));
    }
}

// ---------------------------------------------------------------------------
// Wildlife spawning
// ---------------------------------------------------------------------------

const FACTION_WILDLIFE: u32 = 3;

/// Pick a wildlife name appropriate for the biome.
fn wildlife_name(zone_type: ZoneType, index: u32, rng: &mut impl rand::RngExt) -> String {
    let pool: &[&str] = match zone_type {
        ZoneType::Forest => &["Wolf", "Boar", "Stag", "Bear"],
        ZoneType::Grassland => &["Boar", "Stag", "Hare", "Bison"],
        ZoneType::Swamp => &["Croc", "Serpent", "Toad", "Leech"],
        ZoneType::Coast => &["Crab", "Gull", "Seal"],
        ZoneType::Mountain => &["Goat", "Eagle", "Bear"],
        ZoneType::Desert => &["Hyena", "Scorpion", "Vulture"],
        ZoneType::Tundra => &["Wolf", "Bear", "Elk"],
        _ => &["Beast"],
    };
    let name = pool[rng.random_range(0..pool.len())];
    format!("{} {}", name, index + 1)
}

/// Spawn a group of wildlife entities. Similar to enemy camps but with neutral
/// faction, brown tint, simpler combat, and biome-appropriate names.
fn spawn_wildlife_group(
    commands: &mut Commands,
    texture: &Handle<Image>,
    map_settings: &MapSettings,
    body_template: &crate::resources::body::BodyTemplate,
    item_registry: &ItemRegistry,
    instance_registry: &mut ItemInstanceRegistry,
    cx: u32,
    cy: u32,
    count: u32,
    spawn_index_start: u32,
    snapshot: Option<&crate::resources::zone_persistence::ZoneSnapshot>,
    zone_type: ZoneType,
) {
    let mut rng = rand::rng();

    for i in 0..count {
        let spawn_idx = spawn_index_start + i;

        // Skip dead entities from snapshot
        if let Some(snap) = snapshot
            && snap.dead_indices.contains(&spawn_idx)
        {
            continue;
        }

        let alive_override = snapshot.and_then(|s| s.alive_overrides.get(&spawn_idx));

        let name = wildlife_name(zone_type, i, &mut rng);
        let sprite_color = Color::srgb(0.7, 0.5, 0.3); // Brown/tan

        // Wildlife: basic melee, no special abilities, high flee threshold
        let weapon_item_id = ITEM_KNOBWOOD_CLUB; // placeholder "claws/bite"
        let attributes = Attributes { strength: 5, agility: 5, toughness: 4, ..Default::default() };
        let mut behavior = CombatBehavior::melee_enemy(vec![]);
        behavior.flee_hp_threshold = 0.40; // Wildlife flees sooner

        // Weapon instance
        let weapon_instance_id = if let Some(entity_snap) = alive_override {
            entity_snap.equipment_instance_ids
                .iter()
                .find(|(slot, _)| *slot == EquipSlot::MainHand)
                .map(|(_, id)| *id)
                .unwrap_or_else(|| create_item_instance(weapon_item_id, instance_registry))
        } else {
            create_item_instance(weapon_item_id, instance_registry)
        };

        let placeholder = placeholder_weapon(weapon_item_id, item_registry);

        let mut equipment = Equipment::default();
        equipment.equip(EquipSlot::MainHand, weapon_instance_id);

        // Position: snapshot or offset from center
        let (ex, ey) = if let Some(entity_snap) = alive_override {
            entity_snap.position
        } else {
            let offset_x = (i % 3) as i32 - 1;
            let offset_y = (i / 3) as i32 - 1;
            (
                (cx as i32 + offset_x * 2).max(0) as u32,
                (cy as i32 + offset_y * 2).max(0) as u32,
            )
        };
        let grid_pos = GridPosition::new(ex, ey);
        let world_pos = grid_pos.to_world(map_settings.tile_size);

        // Body
        let mut body = Body::from_template(body_template);
        if let Some(entity_snap) = alive_override {
            for (j, part) in body.parts.iter_mut().enumerate() {
                if j < entity_snap.body_part_hp.len() {
                    part.current_hp = entity_snap.body_part_hp[j];
                    part.destroyed = entity_snap.body_part_destroyed[j];
                }
            }
        }

        // Resources
        let mut mana = Mana::new(0.0); // Wildlife has no mana
        let mut stamina = Stamina::new(40.0);
        if let Some(entity_snap) = alive_override {
            mana.current = entity_snap.mana_current;
            stamina.current = entity_snap.stamina_current;
        }

        let pack_id = PackId(spawn_index_start);

        let mut entity_commands = commands.spawn((
            Name::new(name.clone()),
            StableId::next(),
            DespawnOnExit(GameState::Playing),
            ZoneEntity,
            Sprite {
                image: texture.clone(),
                color: sprite_color,
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
            PathOffset::random(&mut rng),
            Faction(FACTION_WILDLIFE),
            EntityName(name),
        ));

        entity_commands.insert((
            body,
            attributes,
            CharacterLevel::default(),
            EquippedWeapon::new(placeholder),
            EquippedArmor::default(),
            EquipmentBonuses::default(),
            mana,
            stamina,
            ActiveStatusEffects::default(),
            ThreatTable::default(),
            crate::resources::abilities::AbilitySlots::new(vec![]),
        ));
        entity_commands.insert((
            behavior,
            RepathTimer::default(),
            AiBrain::enemy(),
            InCombat,
            equipment,
            ZoneSpawnIndex(spawn_idx),
            pack_id,
        ));
    }
}
