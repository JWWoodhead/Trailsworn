use bevy::prelude::*;

use crate::resources::abilities::{AbilitySlots, Mana, Stamina};
use crate::resources::ability_defs;
use crate::resources::combat_behavior::{CombatBehavior, CombatRole};
use crate::resources::movement::RepathTimer;
use crate::resources::task::PartyMode;
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::combat::InCombat;
use crate::resources::damage::{DamageType, EquippedArmor, EquippedWeapon, WeaponDef};
use crate::resources::faction::{Faction, FACTION_PLAYER};
use crate::resources::game_state::GameState;
use crate::resources::identity::StableId;
use crate::resources::map::{render_layers, GridPosition, MapSettings};
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

/// Spawn the player's party.
pub fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_settings: Res<MapSettings>,
    body_templates: Res<BodyTemplates>,
) {
    let pawn_texture = asset_server.load("pawn.png");
    let template = body_templates.get("humanoid").unwrap();

    let sword = WeaponDef {
        name: "Iron Sword".into(),
        damage_type: DamageType::Slashing,
        base_damage: 8.0,
        attack_speed_ticks: 90,
        range: 1.5,
        projectile_speed: 0.0,
        is_melee: true,
    };

    let grid_pos = GridPosition::new(125, 125);
    let world_pos = grid_pos.to_world(map_settings.tile_size);

    let mut entity_commands = commands.spawn((
        Name::new("Hero"),
        StableId::next(),
        DespawnOnExit(GameState::Playing),
        Sprite {
            image: pawn_texture,
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
        Faction(FACTION_PLAYER),
        EntityName("Hero".into()),
    ));

    entity_commands.insert((
        Body::from_template(template),
        Attributes { strength: 7, agility: 6, toughness: 6, ..Default::default() },
        CharacterLevel::default(),
        EquippedWeapon::new(sword),
        EquippedArmor::default(),
        Mana::new(100.0),
        Stamina::new(100.0),
        ActiveStatusEffects::default(),
        ThreatTable::default(),
        CombatBehavior::party_member(CombatRole::MeleeDps, 1.5),
        AbilitySlots::new(vec![
            ability_defs::ABILITY_CLEAVE,       // Q — SingleEnemy, instant
            ability_defs::ABILITY_SHIELD_BASH,   // E — SingleEnemy, instant stun
            ability_defs::ABILITY_BANDAGE,        // R — SelfOnly, cast time
            ability_defs::ABILITY_FIREBALL,       // T — CircleAoE, cast time
            ability_defs::ABILITY_FROST_BOLT,     // F — SingleEnemy, cast time ranged
            ability_defs::ABILITY_HEAL,           // G — SingleAlly, cast time
        ]),
    ));
    entity_commands.insert((
        RepathTimer::default(),
        InCombat,
        PlayerControlled,
        PartyMode::Passive,
    ));
}
