use bevy::prelude::*;

use super::abilities::AbilityId;

/// Explicit player-issued command. While present, AI does not act.
/// Removed when the command completes or is cancelled.
#[derive(Component, Clone, Debug)]
pub enum PlayerCommand {
    /// Move to a specific tile.
    MoveTo { x: u32, y: u32 },
    /// Attack a specific entity.
    Attack(Entity),
    /// Cast a specific ability at a target.
    CastAbility {
        ability_id: AbilityId,
        slot_index: usize,
        target: AbilityTarget,
    },
    /// Hold position — don't move, but still auto-attack if in range.
    HoldPosition,
}

/// Target for an ability command.
#[derive(Clone, Debug)]
pub enum AbilityTarget {
    SelfCast,
    Entity(Entity),
    Position { x: f32, y: f32 },
    Direction { dx: f32, dy: f32 },
}

/// How a party member behaves when not given explicit commands.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PartyMode {
    /// Attack nearest enemy in range.
    #[default]
    Aggressive,
    /// Only fight back if attacked.
    Defensive,
    /// Don't fight at all.
    Passive,
    /// Stay near the party leader, don't initiate.
    Follow,
}

/// Combat role — drives AI target selection and positioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRole {
    /// Close distance, taunt, absorb damage.
    Tank,
    /// Stay at range, deal damage.
    RangedDps,
    /// Close distance, deal damage.
    MeleeDps,
    /// Stay back, heal allies.
    Healer,
    /// Stay at range, cast spells (CC, damage).
    Caster,
}

/// When an ability should be used by the AI.
#[derive(Clone, Debug)]
pub enum UseCondition {
    /// Always use when off cooldown and in range.
    Always,
    /// Use when own HP is below this fraction (0.0 to 1.0).
    SelfHpBelow(f32),
    /// Use when target HP is below this fraction.
    TargetHpBelow(f32),
    /// Use when an ally's HP is below this fraction (for healers).
    AllyHpBelow(f32),
    /// Use when number of enemies in range exceeds this count (for AoE).
    EnemiesInRange(u32),
}

/// A prioritized ability for AI to consider using.
#[derive(Clone, Debug)]
pub struct AbilityPriority {
    pub ability_id: AbilityId,
    pub slot_index: usize,
    pub condition: UseCondition,
    /// Higher = considered first.
    pub priority: u32,
}

/// How an entity behaves in combat. Works for both enemies and party members.
#[derive(Component, Clone, Debug)]
pub struct CombatBehavior {
    pub role: CombatRole,
    /// How far away this entity can detect hostiles (in tiles).
    pub aggro_range: f32,
    /// How close to get to a target for attacking (in tiles). Will be replaced by weapon/spell range.
    pub attack_range: f32,
    /// Flee when HP fraction drops below this (0.0 = never flee).
    pub flee_hp_threshold: f32,
    /// Whether AI should automatically use abilities (true for enemies, false for party).
    pub auto_use_abilities: bool,
    /// Prioritized list of abilities for AI to use. Sorted by priority descending.
    pub ability_priorities: Vec<AbilityPriority>,
}

impl CombatBehavior {
    /// Create a basic melee enemy behavior.
    pub fn melee_enemy(ability_priorities: Vec<AbilityPriority>) -> Self {
        Self {
            role: CombatRole::MeleeDps,
            aggro_range: 25.0,
            attack_range: 1.5,
            flee_hp_threshold: 0.0,
            auto_use_abilities: true,
            ability_priorities,
        }
    }

    /// Create a basic ranged enemy behavior.
    pub fn ranged_enemy(attack_range: f32, ability_priorities: Vec<AbilityPriority>) -> Self {
        Self {
            role: CombatRole::RangedDps,
            aggro_range: 30.0,
            attack_range,
            flee_hp_threshold: 0.0,
            auto_use_abilities: true,
            ability_priorities,
        }
    }

    /// Create a party member behavior (no auto abilities).
    pub fn party_member(role: CombatRole, attack_range: f32) -> Self {
        Self {
            role,
            aggro_range: 25.0,
            attack_range,
            flee_hp_threshold: 0.0,
            auto_use_abilities: false,
            ability_priorities: Vec::new(),
        }
    }
}

/// Current AI combat state for an entity.
#[derive(Component, Clone, Debug, Default)]
pub enum AiState {
    /// Not in combat, idle.
    #[default]
    Idle,
    /// Engaging a target.
    Engaging { target: Entity },
    /// Fleeing from combat.
    Fleeing,
    /// Following party leader (out of combat).
    Following { leader: Entity },
}

/// What the entity wants in terms of positioning.
/// Set by AI, consumed by the movement intent system.
#[derive(Component, Clone, Debug, Default)]
pub enum MovementIntent {
    #[default]
    None,
    /// Move within range of a target entity.
    MoveToEntity { target: Entity, desired_range: f32 },
    /// Move to a specific tile.
    MoveToPosition { x: u32, y: u32 },
    /// Move away from a threat.
    FleeFrom { threat: Entity },
    /// Stay near another entity.
    FollowEntity { leader: Entity, follow_distance: f32 },
}

/// Tracks when the entity last repathed, to avoid repathing every tick.
#[derive(Component, Debug)]
pub struct RepathTimer {
    pub ticks_since_repath: u32,
    pub repath_interval: u32,
}

impl Default for RepathTimer {
    fn default() -> Self {
        Self {
            ticks_since_repath: 0,
            repath_interval: 30, // every 0.5 seconds
        }
    }
}

impl RepathTimer {
    pub fn tick(&mut self) {
        self.ticks_since_repath += 1;
    }

    pub fn should_repath(&self) -> bool {
        self.ticks_since_repath >= self.repath_interval
    }

    pub fn reset(&mut self) {
        self.ticks_since_repath = 0;
    }
}
