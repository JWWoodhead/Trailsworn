use std::collections::HashMap;

use bevy::prelude::*;

use super::damage::DamageType;
use super::status_effects::StatusId;
use super::stats::AttributeChoice;

pub type AbilityId = u32;

/// Reasons a cast attempt can fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastError {
    InvalidSlot,
    OnCooldown,
    NotEnoughMana,
    NotEnoughStamina,
    OutOfRange,
    Silenced,
    AbilityNotFound,
}

/// How an ability selects its target(s).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetType {
    SelfOnly,
    SingleEnemy,
    SingleAlly,
    /// Circle centered on a target point.
    CircleAoE,
    /// Cone from caster in a direction.
    ConeAoE,
    /// Line from caster in a direction.
    LineAoE,
}

/// How an ability's damage/healing scales with a stat.
#[derive(Clone, Debug)]
pub struct StatScaling {
    pub attribute: AttributeChoice,
    /// Fraction of attribute value added. 0.5 = +50% of attribute.
    pub factor: f32,
}

/// A single effect that an ability produces.
#[derive(Clone, Debug)]
pub enum AbilityEffect {
    Damage {
        damage_type: DamageType,
        base_amount: f32,
        scaling: Option<StatScaling>,
    },
    Heal {
        base_amount: f32,
        scaling: Option<StatScaling>,
    },
    ApplyStatus {
        status_id: StatusId,
        duration_ticks: u32,
        /// Chance to apply (0.0 to 1.0).
        chance: f32,
    },
    Knockback {
        distance_tiles: f32,
    },
    GenerateThreat {
        amount: f32,
    },
}

/// Definition of an ability. Loaded from data, shared across characters.
#[derive(Clone, Debug)]
pub struct AbilityDef {
    pub id: AbilityId,
    pub name: String,
    /// 0 = instant cast.
    pub cast_time_ticks: u32,
    pub cooldown_ticks: u32,
    pub mana_cost: u32,
    pub stamina_cost: u32,
    /// Range in tiles.
    pub range: f32,
    pub target_type: TargetType,
    /// For CircleAoE.
    pub aoe_radius: f32,
    /// For ConeAoE: half-angle in degrees.
    pub cone_half_angle: f32,
    /// For LineAoE / ConeAoE: length in tiles.
    pub aoe_length: f32,
    /// For LineAoE: width in tiles.
    pub aoe_width: f32,
    pub effects: Vec<AbilityEffect>,
    /// Can be interrupted by taking damage while casting.
    pub interruptible: bool,
}

/// Registry of all ability definitions.
#[derive(Resource, Default)]
pub struct AbilityRegistry {
    pub abilities: HashMap<AbilityId, AbilityDef>,
}

impl AbilityRegistry {
    pub fn register(&mut self, ability: AbilityDef) {
        self.abilities.insert(ability.id, ability);
    }

    pub fn get(&self, id: AbilityId) -> Option<&AbilityDef> {
        self.abilities.get(&id)
    }
}

/// Character's known abilities and their cooldown state.
#[derive(Component, Clone, Debug)]
pub struct AbilitySlots {
    pub abilities: Vec<AbilityId>,
    /// Per-ability cooldown remaining in ticks. Parallel to `abilities`.
    pub cooldowns: Vec<u32>,
}

impl AbilitySlots {
    pub fn new(abilities: Vec<AbilityId>) -> Self {
        let len = abilities.len();
        Self {
            abilities,
            cooldowns: vec![0; len],
        }
    }

    pub fn is_ready(&self, slot: usize) -> bool {
        slot < self.cooldowns.len() && self.cooldowns[slot] == 0
    }

    pub fn start_cooldown(&mut self, slot: usize, ticks: u32) {
        if slot < self.cooldowns.len() {
            self.cooldowns[slot] = ticks;
        }
    }

    pub fn tick_cooldowns(&mut self) {
        for cd in &mut self.cooldowns {
            if *cd > 0 {
                *cd -= 1;
            }
        }
    }
}

/// Mana pool for magic abilities.
#[derive(Component, Clone, Debug)]
pub struct Mana {
    pub current: f32,
    pub max: f32,
    /// Base max before equipment bonuses. Set at spawn, never changed by equipment.
    pub base_max: f32,
}

impl Mana {
    pub fn new(max: f32) -> Self {
        Self { current: max, max, base_max: max }
    }

    pub fn spend(&mut self, amount: f32) -> bool {
        if self.current >= amount {
            self.current -= amount;
            true
        } else {
            false
        }
    }

    pub fn regenerate(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// Stamina pool for physical abilities.
#[derive(Component, Clone, Debug)]
pub struct Stamina {
    pub current: f32,
    pub max: f32,
    /// Base max before equipment bonuses. Set at spawn, never changed by equipment.
    pub base_max: f32,
}

impl Stamina {
    pub fn new(max: f32) -> Self {
        Self { current: max, max, base_max: max }
    }

    pub fn spend(&mut self, amount: f32) -> bool {
        if self.current >= amount {
            self.current -= amount;
            true
        } else {
            false
        }
    }

    pub fn regenerate(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// Active casting state. Present while a character is casting an ability.
#[derive(Component, Clone, Debug)]
pub struct CastingState {
    pub ability_id: AbilityId,
    pub slot_index: usize,
    pub remaining_ticks: u32,
    pub target: CastTarget,
}

/// What an ability is being cast at.
#[derive(Clone, Debug)]
pub enum CastTarget {
    SelfCast,
    Entity(Entity),
    Position { x: f32, y: f32 },
    Direction { dx: f32, dy: f32 },
}
