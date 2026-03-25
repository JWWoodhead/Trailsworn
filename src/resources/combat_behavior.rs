use super::abilities::AbilityId;

/// Combat role — drives target selection and positioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRole {
    Tank,
    RangedDps,
    MeleeDps,
    Healer,
    Caster,
}

/// When an ability should be used automatically.
#[derive(Clone, Debug)]
pub enum UseCondition {
    Always,
    SelfHpBelow(f32),
    TargetHpBelow(f32),
    AllyHpBelow(f32),
    EnemiesInRange(u32),
}

/// A prioritized ability for automatic use.
#[derive(Clone, Debug)]
pub struct AbilityPriority {
    pub ability_id: AbilityId,
    pub slot_index: usize,
    pub condition: UseCondition,
    /// Higher = considered first.
    pub priority: u32,
}

/// How an entity behaves in combat. Works for both enemies and party members.
#[derive(bevy::prelude::Component, Clone, Debug)]
pub struct CombatBehavior {
    pub role: CombatRole,
    pub aggro_range: f32,
    pub attack_range: f32,
    pub flee_hp_threshold: f32,
    pub auto_use_abilities: bool,
    pub ability_priorities: Vec<AbilityPriority>,
    /// If set, entity tries to stay at least this many tiles from its target.
    /// Used by the MaintainRange evaluator for caster/ranged kiting.
    pub preferred_min_range: Option<f32>,
}

impl CombatBehavior {
    pub fn melee_enemy(ability_priorities: Vec<AbilityPriority>) -> Self {
        Self {
            role: CombatRole::MeleeDps,
            aggro_range: 25.0,
            attack_range: 1.5,
            flee_hp_threshold: 0.0,
            auto_use_abilities: true,
            ability_priorities,
            preferred_min_range: None,
        }
    }

    pub fn ranged_enemy(attack_range: f32, ability_priorities: Vec<AbilityPriority>) -> Self {
        Self {
            role: CombatRole::RangedDps,
            aggro_range: 30.0,
            attack_range,
            flee_hp_threshold: 0.0,
            auto_use_abilities: true,
            ability_priorities,
            preferred_min_range: None,
        }
    }

    pub fn party_member(role: CombatRole, attack_range: f32) -> Self {
        Self {
            role,
            aggro_range: 25.0,
            attack_range,
            flee_hp_threshold: 0.0,
            auto_use_abilities: false,
            ability_priorities: Vec::new(),
            preferred_min_range: None,
        }
    }
}
