use bevy::prelude::*;

use crate::resources::combat_behavior::CombatBehavior;
use crate::resources::equipment_bonuses::EquipmentBonuses;
use crate::resources::movement::RepathTimer;
use crate::resources::task::AiBrain;
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::combat::InCombat;
use crate::resources::damage::{EquippedArmor, EquippedWeapon};
use crate::resources::faction::Faction;
use crate::resources::game_state::GameState;
use crate::resources::game_time::GameTime;
use crate::resources::identity::StableId;
use crate::resources::item_defs::{
    ITEM_BENT_STAVE, ITEM_GNARLED_BRANCH, ITEM_KNOBWOOD_CLUB,
};
use crate::resources::pack::PackId;
use crate::resources::items::{Equipment, EquipSlot, ItemInstanceRegistry, ItemRegistry};
use crate::resources::map::{GridPosition, MapSettings, render_layers};
use crate::resources::movement::{FacingDirection, MovementSpeed, PathOffset};
use crate::resources::stats::{Attributes, CharacterLevel};
use crate::resources::status_effects::ActiveStatusEffects;
use crate::resources::threat::ThreatTable;
use crate::resources::world::{CurrentZone, EntryEdge, ZoneTransitionEvent};
use crate::resources::abilities::{Mana, Stamina};
use crate::resources::zone_persistence::ZoneSpawnIndex;
use crate::systems::spawning::{create_item_instance, EntityName, PlayerControlled, placeholder_weapon};
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
                // TODO: wildlife entities — use enemy camp for now
                spawn_enemy_camp(
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
                );
                spawn_index += creature_count;
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
