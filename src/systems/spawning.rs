use bevy::prelude::*;

use crate::resources::abilities::{Mana, Stamina};
use crate::resources::ai::{AiState, CombatBehavior, CombatRole, MovementIntent, PartyMode, RepathTimer};
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::combat::InCombat;
use crate::resources::damage::{DamageType, EquippedArmor, EquippedWeapon, WeaponDef};
use crate::resources::faction::{Faction, FACTION_PLAYER};
use crate::resources::map::{render_layers, GridPosition, MapSettings};
use crate::resources::movement::{FacingDirection, MovementSpeed};
use crate::resources::stats::{Attributes, CharacterLevel};
use crate::resources::status_effects::ActiveStatusEffects;
use crate::resources::threat::ThreatTable;

/// Marker for player-controlled entities.
#[derive(Component)]
pub struct PlayerControlled;

/// Marker for entity name display.
#[derive(Component)]
pub struct EntityName(pub String);

const FACTION_BANDITS: u32 = 2;

/// Spawn the player's party and some test enemies.
pub fn spawn_test_scene(
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
        attack_speed_ticks: 90, // 1.5 sec
        range: 1.5,
        projectile_speed: 0.0,
        is_melee: true,
    };

    // Player pawn
    spawn_character(
        &mut commands,
        &pawn_texture,
        &map_settings,
        template,
        GridPosition::new(120, 125),
        Faction(FACTION_PLAYER),
        "Hero".into(),
        sword.clone(),
        Attributes { strength: 7, agility: 6, toughness: 6, ..Default::default() },
        Some(PlayerControlled),
        Some(PartyMode::Passive),
        CombatBehavior::party_member(CombatRole::MeleeDps, 1.5),
        Color::WHITE,
    );

    // Enemy bandits
    let bandit_sword = WeaponDef {
        name: "Rusty Sword".into(),
        damage_type: DamageType::Slashing,
        base_damage: 5.0,
        attack_speed_ticks: 120, // 2 sec
        range: 1.5,
        projectile_speed: 0.0,
        is_melee: true,
    };

    for (i, (x, y)) in [(130, 125), (132, 127), (131, 123)].iter().enumerate() {
        spawn_character(
            &mut commands,
            &pawn_texture,
            &map_settings,
            template,
            GridPosition::new(*x, *y),
            Faction(FACTION_BANDITS),
            format!("Bandit {}", i + 1),
            bandit_sword.clone(),
            Attributes { strength: 4, agility: 4, toughness: 4, ..Default::default() },
            None,
            None,
            CombatBehavior::melee_enemy(Vec::new()),
            Color::srgb(1.0, 0.4, 0.4), // reddish tint
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_character(
    commands: &mut Commands,
    texture: &Handle<Image>,
    map_settings: &MapSettings,
    body_template: &crate::resources::body::BodyTemplate,
    grid_pos: GridPosition,
    faction: Faction,
    name: String,
    weapon: WeaponDef,
    attributes: Attributes,
    player_controlled: Option<PlayerControlled>,
    party_mode: Option<PartyMode>,
    combat_behavior: CombatBehavior,
    tint: Color,
) {
    let world_pos = grid_pos.to_world(map_settings.tile_size);

    let mut entity_commands = commands.spawn((
        Sprite {
            image: texture.clone(),
            color: tint,
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
        faction,
        EntityName(name),
        Body::from_template(body_template),
        attributes,
        CharacterLevel::default(),
    ));

    entity_commands.insert((
        EquippedWeapon::new(weapon),
        EquippedArmor::default(),
        Mana::new(100.0),
        Stamina::new(100.0),
        ActiveStatusEffects::default(),
        ThreatTable::default(),
        combat_behavior,
        AiState::default(),
        MovementIntent::default(),
        RepathTimer::default(),
        InCombat,
    ));

    if let Some(pc) = player_controlled {
        entity_commands.insert(pc);
    }
    if let Some(pm) = party_mode {
        entity_commands.insert(pm);
    }
}
