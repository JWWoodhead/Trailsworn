use std::collections::HashMap;

use bevy::prelude::*;

use super::damage::DamageType;

pub type StatusId = u32;

/// Crowd control flags. Multiple can be active at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CcFlags {
    pub stunned: bool,
    pub rooted: bool,
    pub silenced: bool,
    pub feared: bool,
    pub sleeping: bool,
}

impl CcFlags {
    pub fn is_incapacitated(&self) -> bool {
        self.stunned || self.sleeping
    }

    pub fn can_move(&self) -> bool {
        !self.stunned && !self.rooted && !self.sleeping
    }

    pub fn can_cast(&self) -> bool {
        !self.stunned && !self.silenced && !self.sleeping
    }

    pub fn can_attack(&self) -> bool {
        !self.stunned && !self.sleeping
    }
}

/// What a status effect does each tick (if anything).
#[derive(Clone, Debug)]
pub struct TickEffect {
    pub damage_type: Option<DamageType>,
    pub amount: f32,
    pub is_heal: bool,
}

/// Modifier to a character's stats while the effect is active.
#[derive(Clone, Debug)]
pub enum StatModifier {
    /// Multiply movement speed (0.5 = 50% slow).
    MoveSpeedMul(f32),
    /// Multiply attack speed.
    AttackSpeedMul(f32),
    /// Flat bonus/penalty to an attribute.
    AttributeFlat { attribute: super::stats::AttributeChoice, amount: i32 },
}

/// Definition of a status effect. Shared template.
#[derive(Clone, Debug)]
pub struct StatusEffectDef {
    pub id: StatusId,
    pub name: String,
    pub max_stacks: u32,
    /// Ticks between each tick effect. 0 = no ticking (pure duration buff/debuff).
    pub tick_interval_ticks: u32,
    pub tick_effect: Option<TickEffect>,
    pub stat_modifiers: Vec<StatModifier>,
    pub cc_flags: CcFlags,
    /// If true, this is beneficial (buff). If false, harmful (debuff).
    pub is_buff: bool,
}

/// Registry of all status effect definitions.
#[derive(Resource, Default)]
pub struct StatusEffectRegistry {
    pub effects: HashMap<StatusId, StatusEffectDef>,
}

impl StatusEffectRegistry {
    pub fn register(&mut self, effect: StatusEffectDef) {
        self.effects.insert(effect.id, effect);
    }

    pub fn get(&self, id: StatusId) -> Option<&StatusEffectDef> {
        self.effects.get(&id)
    }
}

/// A single active status effect instance on a character.
#[derive(Clone, Debug)]
pub struct ActiveEffect {
    pub status_id: StatusId,
    pub remaining_ticks: u32,
    pub stacks: u32,
    /// Ticks until next tick effect fires.
    pub tick_timer: u32,
    pub source: Option<Entity>,
}

/// All active status effects on a character.
#[derive(Component, Clone, Debug, Default)]
pub struct ActiveStatusEffects {
    pub effects: Vec<ActiveEffect>,
}

impl ActiveStatusEffects {
    /// Apply a new status effect or refresh/stack existing one.
    pub fn apply(
        &mut self,
        status_id: StatusId,
        duration_ticks: u32,
        source: Option<Entity>,
        registry: &StatusEffectRegistry,
    ) {
        let def = match registry.get(status_id) {
            Some(d) => d,
            None => return,
        };

        if let Some(existing) = self.effects.iter_mut().find(|e| e.status_id == status_id) {
            // Refresh duration
            existing.remaining_ticks = duration_ticks;
            // Stack if allowed
            if existing.stacks < def.max_stacks {
                existing.stacks += 1;
            }
        } else {
            self.effects.push(ActiveEffect {
                status_id,
                remaining_ticks: duration_ticks,
                stacks: 1,
                tick_timer: def.tick_interval_ticks,
                source,
            });
        }
    }

    /// Remove all expired effects. Returns list of removed status IDs.
    pub fn remove_expired(&mut self) -> Vec<StatusId> {
        let mut removed = Vec::new();
        self.effects.retain(|e| {
            if e.remaining_ticks == 0 {
                removed.push(e.status_id);
                false
            } else {
                true
            }
        });
        removed
    }

    /// Get the combined CC flags from all active effects.
    pub fn combined_cc_flags(&self, registry: &StatusEffectRegistry) -> CcFlags {
        let mut flags = CcFlags::default();
        for effect in &self.effects {
            if let Some(def) = registry.get(effect.status_id) {
                if def.cc_flags.stunned { flags.stunned = true; }
                if def.cc_flags.rooted { flags.rooted = true; }
                if def.cc_flags.silenced { flags.silenced = true; }
                if def.cc_flags.feared { flags.feared = true; }
                if def.cc_flags.sleeping { flags.sleeping = true; }
            }
        }
        flags
    }

    /// Get all stat modifiers from active effects (accounting for stacks).
    pub fn combined_stat_modifiers(&self, registry: &StatusEffectRegistry) -> Vec<(StatModifier, u32)> {
        let mut mods = Vec::new();
        for effect in &self.effects {
            if let Some(def) = registry.get(effect.status_id) {
                for modifier in &def.stat_modifiers {
                    mods.push((modifier.clone(), effect.stacks));
                }
            }
        }
        mods
    }
}
