use super::abilities::{AbilityDef, AbilityEffect, AbilityRegistry, StatScaling, TargetType};
use super::audio::SfxKind;
use super::damage::DamageType;
use super::particles::VfxKind;
use super::stats::AttributeChoice;
use super::status_effects::{CcFlags, StatModifier, StatusEffectDef, StatusEffectRegistry, TickEffect};

// --- Status Effect IDs ---
pub const STATUS_STUNNED: u32 = 1;
pub const STATUS_SLOW: u32 = 2;
pub const STATUS_REGEN: u32 = 3;
pub const STATUS_ATTACK_SPEED_BUFF: u32 = 4;
pub const STATUS_BURNING: u32 = 5;
pub const STATUS_FROSTBITTEN: u32 = 6;

// --- Ability IDs ---
pub const ABILITY_CLEAVE: u32 = 1;
pub const ABILITY_SHIELD_BASH: u32 = 2;
pub const ABILITY_FIREBALL: u32 = 3;
pub const ABILITY_FROST_BOLT: u32 = 4;
pub const ABILITY_HEAL: u32 = 5;
pub const ABILITY_BANDAGE: u32 = 6;
pub const ABILITY_WAR_CRY: u32 = 7;
pub const ABILITY_AIMED_SHOT: u32 = 8;

fn register_status_effects(registry: &mut StatusEffectRegistry) {
    registry.register(StatusEffectDef {
        id: STATUS_STUNNED,
        name: "Stunned".into(),
        max_stacks: 1,
        tick_interval_ticks: 0,
        tick_effect: None,
        stat_modifiers: vec![],
        cc_flags: CcFlags { stunned: true, ..Default::default() },
        is_buff: false,
    });

    registry.register(StatusEffectDef {
        id: STATUS_SLOW,
        name: "Slow".into(),
        max_stacks: 1,
        tick_interval_ticks: 0,
        tick_effect: None,
        stat_modifiers: vec![StatModifier::MoveSpeedMul(0.5)],
        cc_flags: CcFlags::default(),
        is_buff: false,
    });

    registry.register(StatusEffectDef {
        id: STATUS_REGEN,
        name: "Regen".into(),
        max_stacks: 1,
        tick_interval_ticks: 60, // heals every 1 second
        tick_effect: Some(TickEffect {
            damage_type: None,
            amount: 2.0,
            is_heal: true,
        }),
        stat_modifiers: vec![],
        cc_flags: CcFlags::default(),
        is_buff: true,
    });

    registry.register(StatusEffectDef {
        id: STATUS_ATTACK_SPEED_BUFF,
        name: "Battle Fury".into(),
        max_stacks: 1,
        tick_interval_ticks: 0,
        tick_effect: None,
        stat_modifiers: vec![StatModifier::AttackSpeedMul(1.3)],
        cc_flags: CcFlags::default(),
        is_buff: true,
    });

    registry.register(StatusEffectDef {
        id: STATUS_BURNING,
        name: "Burning".into(),
        max_stacks: 3,
        tick_interval_ticks: 60,
        tick_effect: Some(TickEffect {
            damage_type: Some(DamageType::Fire),
            amount: 3.0,
            is_heal: false,
        }),
        stat_modifiers: vec![],
        cc_flags: CcFlags::default(),
        is_buff: false,
    });

    registry.register(StatusEffectDef {
        id: STATUS_FROSTBITTEN,
        name: "Frostbitten".into(),
        max_stacks: 1,
        tick_interval_ticks: 0,
        tick_effect: None,
        stat_modifiers: vec![
            StatModifier::MoveSpeedMul(0.7),
            StatModifier::AttackSpeedMul(0.8),
        ],
        cc_flags: CcFlags::default(),
        is_buff: false,
    });
}

fn register_abilities(registry: &mut AbilityRegistry) {
    // 1. Cleave — instant melee AoE-ish (single target for MVP), threat gen
    registry.register(AbilityDef {
        id: ABILITY_CLEAVE,
        name: "Cleave".into(),
        cast_time_ticks: 0,
        cooldown_ticks: 180, // 3 seconds
        mana_cost: 0,
        stamina_cost: 15,
        range: 1.5,
        target_type: TargetType::SingleEnemy,
        aoe_radius: 0.0,
        cone_half_angle: 0.0,
        aoe_length: 0.0,
        aoe_width: 0.0,
        effects: vec![
            AbilityEffect::Damage {
                damage_type: DamageType::Slashing,
                base_amount: 12.0,
                scaling: Some(StatScaling {
                    attribute: AttributeChoice::Strength,
                    factor: 0.3,
                }),
            },
            AbilityEffect::GenerateThreat { amount: 15.0 },
        ],
        interruptible: false,
        cast_sfx: None,
        impact_sfx: Some(SfxKind::CleaveImpact),
        impact_vfx: Some(VfxKind::CleaveImpact),
        impact_vfx_scale: 1.5,
        cast_vfx: None,
    });

    // 2. Shield Bash — instant stun + threat
    registry.register(AbilityDef {
        id: ABILITY_SHIELD_BASH,
        name: "Shield Bash".into(),
        cast_time_ticks: 0,
        cooldown_ticks: 360, // 6 seconds
        mana_cost: 0,
        stamina_cost: 20,
        range: 1.5,
        target_type: TargetType::SingleEnemy,
        aoe_radius: 0.0,
        cone_half_angle: 0.0,
        aoe_length: 0.0,
        aoe_width: 0.0,
        effects: vec![
            AbilityEffect::Damage {
                damage_type: DamageType::Blunt,
                base_amount: 6.0,
                scaling: None,
            },
            AbilityEffect::ApplyStatus {
                status_id: STATUS_STUNNED,
                duration_ticks: 120, // 2 seconds
                chance: 0.7,
            },
            AbilityEffect::GenerateThreat { amount: 25.0 },
        ],
        interruptible: false,
        cast_sfx: None,
        impact_sfx: Some(SfxKind::ShieldBashImpact),
        impact_vfx: Some(VfxKind::ShieldBashImpact),
        impact_vfx_scale: 1.0,
        cast_vfx: None,
    });

    // 3. Fireball — cast-time AoE fire damage
    registry.register(AbilityDef {
        id: ABILITY_FIREBALL,
        name: "Fireball".into(),
        cast_time_ticks: 90, // 1.5 seconds
        cooldown_ticks: 300, // 5 seconds
        mana_cost: 25,
        stamina_cost: 0,
        range: 8.0,
        target_type: TargetType::CircleAoE,
        aoe_radius: 2.0,
        cone_half_angle: 0.0,
        aoe_length: 0.0,
        aoe_width: 0.0,
        effects: vec![
            AbilityEffect::Damage {
                damage_type: DamageType::Fire,
                base_amount: 15.0,
                scaling: Some(StatScaling {
                    attribute: AttributeChoice::Intellect,
                    factor: 0.5,
                }),
            },
        ],
        interruptible: true,
        cast_sfx: Some(SfxKind::SpellCast),
        impact_sfx: Some(SfxKind::FireImpact),
        impact_vfx: Some(VfxKind::FireballImpact),
        impact_vfx_scale: 6.0,
        cast_vfx: None,
    });

    // 4. Frost Bolt — cast-time single target + slow
    registry.register(AbilityDef {
        id: ABILITY_FROST_BOLT,
        name: "Frost Bolt".into(),
        cast_time_ticks: 60, // 1 second
        cooldown_ticks: 180, // 3 seconds
        mana_cost: 15,
        stamina_cost: 0,
        range: 10.0,
        target_type: TargetType::SingleEnemy,
        aoe_radius: 0.0,
        cone_half_angle: 0.0,
        aoe_length: 0.0,
        aoe_width: 0.0,
        effects: vec![
            AbilityEffect::Damage {
                damage_type: DamageType::Frost,
                base_amount: 10.0,
                scaling: Some(StatScaling {
                    attribute: AttributeChoice::Intellect,
                    factor: 0.4,
                }),
            },
            AbilityEffect::ApplyStatus {
                status_id: STATUS_FROSTBITTEN,
                duration_ticks: 180, // 3 seconds
                chance: 0.9,
            },
        ],
        interruptible: true,
        cast_sfx: Some(SfxKind::SpellCast),
        impact_sfx: Some(SfxKind::FrostImpact),
        impact_vfx: Some(VfxKind::FrostBoltImpact),
        impact_vfx_scale: 1.5,
        cast_vfx: None,
    });

    // 5. Heal — cast-time single ally heal
    registry.register(AbilityDef {
        id: ABILITY_HEAL,
        name: "Heal".into(),
        cast_time_ticks: 90, // 1.5 seconds
        cooldown_ticks: 240, // 4 seconds
        mana_cost: 20,
        stamina_cost: 0,
        range: 8.0,
        target_type: TargetType::SingleAlly,
        aoe_radius: 0.0,
        cone_half_angle: 0.0,
        aoe_length: 0.0,
        aoe_width: 0.0,
        effects: vec![
            AbilityEffect::Heal {
                base_amount: 20.0,
                scaling: Some(StatScaling {
                    attribute: AttributeChoice::Willpower,
                    factor: 0.4,
                }),
            },
        ],
        interruptible: true,
        cast_sfx: Some(SfxKind::HealCast),
        impact_sfx: Some(SfxKind::HealLand),
        impact_vfx: Some(VfxKind::HealLand),
        impact_vfx_scale: 2.0,
        cast_vfx: None,
    });

    // 6. Bandage — cast-time self heal + regen
    registry.register(AbilityDef {
        id: ABILITY_BANDAGE,
        name: "Bandage".into(),
        cast_time_ticks: 120, // 2 seconds
        cooldown_ticks: 600, // 10 seconds
        mana_cost: 0,
        stamina_cost: 10,
        range: 0.0,
        target_type: TargetType::SelfOnly,
        aoe_radius: 0.0,
        cone_half_angle: 0.0,
        aoe_length: 0.0,
        aoe_width: 0.0,
        effects: vec![
            AbilityEffect::Heal {
                base_amount: 15.0,
                scaling: None,
            },
            AbilityEffect::ApplyStatus {
                status_id: STATUS_REGEN,
                duration_ticks: 300, // 5 seconds
                chance: 1.0,
            },
        ],
        interruptible: true,
        cast_sfx: Some(SfxKind::BandageRip),
        impact_sfx: None,
        impact_vfx: None,
        impact_vfx_scale: 1.0,
        cast_vfx: None,
    });

    // 7. War Cry — instant self buff + massive threat
    registry.register(AbilityDef {
        id: ABILITY_WAR_CRY,
        name: "War Cry".into(),
        cast_time_ticks: 0,
        cooldown_ticks: 600, // 10 seconds
        mana_cost: 0,
        stamina_cost: 20,
        range: 0.0,
        target_type: TargetType::SelfOnly,
        aoe_radius: 0.0,
        cone_half_angle: 0.0,
        aoe_length: 0.0,
        aoe_width: 0.0,
        effects: vec![
            AbilityEffect::GenerateThreat { amount: 50.0 },
            AbilityEffect::ApplyStatus {
                status_id: STATUS_ATTACK_SPEED_BUFF,
                duration_ticks: 300, // 5 seconds
                chance: 1.0,
            },
        ],
        interruptible: false,
        cast_sfx: Some(SfxKind::WarCry),
        impact_sfx: None,
        impact_vfx: None,
        impact_vfx_scale: 1.0,
        cast_vfx: None,
    });

    // 8. Aimed Shot — cast-time ranged physical, agility scaling
    registry.register(AbilityDef {
        id: ABILITY_AIMED_SHOT,
        name: "Aimed Shot".into(),
        cast_time_ticks: 60, // 1 second
        cooldown_ticks: 240, // 4 seconds
        mana_cost: 0,
        stamina_cost: 15,
        range: 12.0,
        target_type: TargetType::SingleEnemy,
        aoe_radius: 0.0,
        cone_half_angle: 0.0,
        aoe_length: 0.0,
        aoe_width: 0.0,
        effects: vec![
            AbilityEffect::Damage {
                damage_type: DamageType::Piercing,
                base_amount: 18.0,
                scaling: Some(StatScaling {
                    attribute: AttributeChoice::Agility,
                    factor: 0.5,
                }),
            },
        ],
        interruptible: true,
        cast_sfx: Some(SfxKind::BowDraw),
        impact_sfx: Some(SfxKind::ArrowImpact),
        impact_vfx: Some(VfxKind::AimedShotImpact),
        impact_vfx_scale: 1.0,
        cast_vfx: None,
    });
}

/// Register all starter abilities and status effects.
/// Called once at startup from main.rs.
pub fn register_starter_abilities(
    ability_registry: &mut AbilityRegistry,
    status_registry: &mut StatusEffectRegistry,
) {
    register_status_effects(status_registry);
    register_abilities(ability_registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_abilities_registered() {
        let mut ability_reg = AbilityRegistry::default();
        let mut status_reg = StatusEffectRegistry::default();
        register_starter_abilities(&mut ability_reg, &mut status_reg);
        assert_eq!(ability_reg.abilities.len(), 8);
        for id in 1..=8 {
            assert!(ability_reg.get(id).is_some(), "Ability {} not found", id);
        }
    }

    #[test]
    fn all_status_effects_registered() {
        let mut ability_reg = AbilityRegistry::default();
        let mut status_reg = StatusEffectRegistry::default();
        register_starter_abilities(&mut ability_reg, &mut status_reg);
        assert_eq!(status_reg.effects.len(), 6);
        for id in 1..=6 {
            assert!(status_reg.get(id).is_some(), "Status effect {} not found", id);
        }
    }

    #[test]
    fn abilities_have_positive_costs_or_zero() {
        let mut ability_reg = AbilityRegistry::default();
        let mut status_reg = StatusEffectRegistry::default();
        register_starter_abilities(&mut ability_reg, &mut status_reg);
        for ability in ability_reg.abilities.values() {
            assert!(
                ability.mana_cost > 0 || ability.stamina_cost > 0,
                "Ability '{}' has no resource cost",
                ability.name
            );
        }
    }

    #[test]
    fn ability_effects_reference_valid_statuses() {
        let mut ability_reg = AbilityRegistry::default();
        let mut status_reg = StatusEffectRegistry::default();
        register_starter_abilities(&mut ability_reg, &mut status_reg);
        for ability in ability_reg.abilities.values() {
            for effect in &ability.effects {
                if let AbilityEffect::ApplyStatus { status_id, .. } = effect {
                    assert!(
                        status_reg.get(*status_id).is_some(),
                        "Ability '{}' references missing status effect {}",
                        ability.name,
                        status_id
                    );
                }
            }
        }
    }

    #[test]
    fn abilities_have_valid_ranges() {
        let mut ability_reg = AbilityRegistry::default();
        let mut status_reg = StatusEffectRegistry::default();
        register_starter_abilities(&mut ability_reg, &mut status_reg);
        for ability in ability_reg.abilities.values() {
            match ability.target_type {
                TargetType::SelfOnly => {
                    // Self abilities can have range 0
                }
                _ => {
                    assert!(
                        ability.range > 0.0,
                        "Ability '{}' with target type {:?} has non-positive range",
                        ability.name,
                        ability.target_type
                    );
                }
            }
        }
    }
}
