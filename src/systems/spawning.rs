use bevy::prelude::*;

use crate::resources::abilities::{AbilitySlots, Mana, Stamina};
use crate::resources::ability_defs;
use crate::resources::combat_behavior::{CombatBehavior, CombatRole};
use crate::resources::equipment_bonuses::EquipmentBonuses;
use crate::resources::movement::RepathTimer;
use crate::resources::task::PartyMode;
use crate::resources::body::{Body, BodyTemplates, Health};
use crate::resources::combat::InCombat;
use crate::resources::items::{
    Equipment, EquipSlot, Inventory, ItemId, ItemInstance, ItemInstanceId,
    ItemInstanceRegistry, ItemRegistry, Rarity,
};
use crate::resources::item_defs::{
    ITEM_GNARLED_BRANCH, ITEM_RANGERS_LONGBOW, ITEM_WANDERERS_STAFF, ITEM_WATCHMANS_SWORD,
};
use crate::resources::damage::{EquippedArmor, EquippedWeapon, WeaponDef};
use crate::resources::faction::{Faction, FACTION_PLAYER};
use crate::resources::game_state::GameState;
use crate::resources::identity::StableId;
use crate::resources::map::{render_layers, GridPosition, MapSettings, TileWorld};
use crate::resources::movement::{FacingDirection, MovementSpeed, PathOffset};
use crate::resources::stats::{Attributes, CharacterLevel};
use crate::resources::status_effects::ActiveStatusEffects;
use crate::resources::threat::ThreatTable;

/// Marker for player-controlled entities.
#[derive(Component)]
pub struct PlayerControlled;

/// Marker for entity name display.
#[derive(Component)]
pub struct EntityName(pub String);

/// Create a Normal-rarity, no-affix ItemInstance from a base item and register it.
pub fn create_item_instance(
    base_id: ItemId,
    instance_registry: &mut ItemInstanceRegistry,
) -> ItemInstanceId {
    let id = instance_registry.next_id();
    let instance = ItemInstance {
        id,
        base_item_id: base_id,
        rarity: Rarity::Normal,
        item_level: 1,
        prefixes: vec![],
        suffixes: vec![],
    };
    instance_registry.insert(instance);
    id
}

/// Build a placeholder WeaponDef from an item definition.
/// Used at spawn so the entity has a valid EquippedWeapon before sync_equipment runs.
pub fn placeholder_weapon(base_id: ItemId, item_registry: &ItemRegistry) -> WeaponDef {
    item_registry
        .get(base_id)
        .and_then(|def| {
            if let crate::resources::items::ItemProperties::Weapon(w) = &def.properties {
                Some(w.clone())
            } else {
                None
            }
        })
        .unwrap_or(WeaponDef {
            name: "Fists".into(),
            damage_type: crate::resources::damage::DamageType::Blunt,
            base_damage: 3.0,
            attack_speed_ticks: 60,
            range: 1.5,
            projectile_speed: 0.0,
            is_melee: true,
            attack_sfx: None,
        })
}

/// Spawn the player's party (4 members with different roles).
pub fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_settings: Res<MapSettings>,
    tile_world: Res<TileWorld>,
    body_templates: Res<BodyTemplates>,
    item_registry: Res<ItemRegistry>,
    mut instance_registry: ResMut<ItemInstanceRegistry>,
) {
    let pawn_texture: Handle<Image> = asset_server.load("pawn.png");
    let template = body_templates.get("humanoid").unwrap();

    // Find a walkable spawn center near the map center
    let center = find_walkable_near(
        map_settings.width / 2,
        map_settings.height / 2,
        &tile_world,
    );

    struct PartyDef {
        name: &'static str,
        weapon_id: ItemId,
        color: Color,
        role: CombatRole,
        attack_range: f32,
        attrs: Attributes,
        mana: f32,
        stamina: f32,
        abilities: Vec<u32>,
    }

    let party = [
        PartyDef {
            name: "Warrior",
            weapon_id: ITEM_WATCHMANS_SWORD,
            color: Color::srgb(0.5, 0.6, 0.8),
            role: CombatRole::Tank,
            attack_range: 1.5,
            attrs: Attributes { strength: 8, agility: 4, toughness: 8, intellect: 3, willpower: 4 },
            mana: 30.0,
            stamina: 120.0,
            abilities: vec![
                ability_defs::ABILITY_CLEAVE,
                ability_defs::ABILITY_SHIELD_BASH,
                ability_defs::ABILITY_WAR_CRY,
                ability_defs::ABILITY_BANDAGE,
                0, 0,
            ],
        },
        PartyDef {
            name: "Archer",
            weapon_id: ITEM_RANGERS_LONGBOW,
            color: Color::srgb(0.4, 0.7, 0.3),
            role: CombatRole::RangedDps,
            attack_range: 10.0,
            attrs: Attributes { strength: 4, agility: 8, toughness: 5, intellect: 3, willpower: 3 },
            mana: 20.0,
            stamina: 100.0,
            abilities: vec![
                ability_defs::ABILITY_AIMED_SHOT,
                ability_defs::ABILITY_BANDAGE,
                0, 0, 0, 0,
            ],
        },
        PartyDef {
            name: "Mage",
            weapon_id: ITEM_WANDERERS_STAFF,
            color: Color::srgb(0.6, 0.4, 0.9),
            role: CombatRole::Caster,
            attack_range: 1.5,
            attrs: Attributes { strength: 3, agility: 4, toughness: 4, intellect: 8, willpower: 6 },
            mana: 150.0,
            stamina: 50.0,
            abilities: vec![
                ability_defs::ABILITY_FIREBALL,
                ability_defs::ABILITY_FROST_BOLT,
                ability_defs::ABILITY_BANDAGE,
                0, 0, 0,
            ],
        },
        PartyDef {
            name: "Healer",
            weapon_id: ITEM_GNARLED_BRANCH,
            color: Color::srgb(0.9, 0.8, 0.3),
            role: CombatRole::Healer,
            attack_range: 1.5,
            attrs: Attributes { strength: 3, agility: 4, toughness: 5, intellect: 5, willpower: 8 },
            mana: 130.0,
            stamina: 60.0,
            abilities: vec![
                ability_defs::ABILITY_HEAL,
                ability_defs::ABILITY_BANDAGE,
                ability_defs::ABILITY_FROST_BOLT,
                0, 0, 0,
            ],
        },
    ];

    // Find walkable spawn positions for each member near center
    let spawn_positions = find_walkable_formation(center, party.len(), &tile_world);

    for (i, member) in party.into_iter().enumerate() {
        let weapon_instance_id = create_item_instance(member.weapon_id, &mut instance_registry);
        let placeholder = placeholder_weapon(member.weapon_id, &item_registry);
        let mut equipment = Equipment::default();
        equipment.equip(EquipSlot::MainHand, weapon_instance_id);

        let grid_pos = GridPosition::new(spawn_positions[i].0, spawn_positions[i].1);
        let world_pos = grid_pos.to_world(map_settings.tile_size);

        let mut entity_commands = commands.spawn((
            Name::new(member.name),
            StableId::next(),
            DespawnOnExit(GameState::Playing),
            Sprite {
                image: pawn_texture.clone(),
                color: member.color,
                ..default()
            },
            Transform::from_translation(Vec3::new(
                world_pos.x,
                world_pos.y,
                render_layers::WORLD_OBJECTS,
            )),
            grid_pos,
            MovementSpeed::default(),
            FacingDirection::default(),
            PathOffset::random(&mut rand::rng()),
            Faction(FACTION_PLAYER),
            EntityName(member.name.into()),
        ));

        let max_hp = 50.0 + member.attrs.toughness as f32 * 10.0;
        entity_commands.insert((
            Body::from_template(template),
            member.attrs,
            CharacterLevel::default(),
            EquippedWeapon::new(placeholder),
            EquippedArmor::default(),
            EquipmentBonuses::default(),
            Mana::new(member.mana),
            Stamina::new(member.stamina),
            ActiveStatusEffects::default(),
            ThreatTable::default(),
            Health::new(max_hp),
            CombatBehavior::party_member(member.role, member.attack_range),
            AbilitySlots::new(member.abilities),
        ));
        entity_commands.insert((
            RepathTimer::default(),
            InCombat,
            PlayerControlled,
            PartyMode::Passive,
            Inventory::new(24),
            equipment,
        ));
    }
}

/// Find a walkable tile near (cx, cy), spiraling outward. Falls back to (cx, cy).
fn find_walkable_near(cx: u32, cy: u32, tile_world: &TileWorld) -> (u32, u32) {
    let w = tile_world.width as i32;
    let h = tile_world.height as i32;
    let ix = cx as i32;
    let iy = cy as i32;

    if tile_world.walk_cost[tile_world.idx(cx, cy)] > 0.0 {
        return (cx, cy);
    }

    for ring in 1..30i32 {
        for dx in -ring..=ring {
            for dy in -ring..=ring {
                if dx.abs() != ring && dy.abs() != ring {
                    continue;
                }
                let nx = ix + dx;
                let ny = iy + dy;
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let (ux, uy) = (nx as u32, ny as u32);
                if tile_world.walk_cost[tile_world.idx(ux, uy)] > 0.0 {
                    return (ux, uy);
                }
            }
        }
    }

    (cx, cy)
}

/// Find `count` distinct walkable tiles near `center`. Each tile is validated individually.
fn find_walkable_formation(
    center: (u32, u32),
    count: usize,
    tile_world: &TileWorld,
) -> Vec<(u32, u32)> {
    const OFFSETS: [(i32, i32); 9] = [
        (0, 0), (1, 0), (-1, 0), (0, 1), (0, -1),
        (1, 1), (-1, 1), (1, -1), (-1, -1),
    ];

    let w = tile_world.width as i32;
    let h = tile_world.height as i32;
    let mut result = Vec::with_capacity(count);

    for &(dx, dy) in &OFFSETS {
        let nx = center.0 as i32 + dx;
        let ny = center.1 as i32 + dy;
        if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
        let (ux, uy) = (nx as u32, ny as u32);
        if tile_world.walk_cost[tile_world.idx(ux, uy)] <= 0.0 { continue; }
        result.push((ux, uy));
        if result.len() >= count { break; }
    }

    // If not enough walkable neighbours, search further out
    if result.len() < count {
        for ring in 2..10i32 {
            for dx in -ring..=ring {
                for dy in -ring..=ring {
                    if dx.abs() != ring && dy.abs() != ring { continue; }
                    let nx = center.0 as i32 + dx;
                    let ny = center.1 as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
                    let (ux, uy) = (nx as u32, ny as u32);
                    if tile_world.walk_cost[tile_world.idx(ux, uy)] <= 0.0 { continue; }
                    if result.contains(&(ux, uy)) { continue; }
                    result.push((ux, uy));
                    if result.len() >= count { break; }
                }
                if result.len() >= count { break; }
            }
            if result.len() >= count { break; }
        }
    }

    while result.len() < count {
        result.push(center);
    }
    result
}
