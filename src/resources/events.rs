use bevy::prelude::*;

use super::damage::DamageType;

/// Fired when damage is dealt to an entity.
#[derive(Message, Clone, Debug)]
pub struct DamageDealtEvent {
    pub target: Entity,
    pub amount: f32,
    pub damage_type: DamageType,
    pub body_part_name: String,
    pub part_destroyed: bool,
    pub target_killed: bool,
}

/// Fired when an attack misses.
#[derive(Message, Clone, Debug)]
pub struct AttackMissedEvent {
    pub attacker: Entity,
    pub target: Entity,
}

/// Fired when a cast is interrupted by damage.
#[derive(Message, Clone, Debug)]
pub struct CastInterruptedEvent {
    pub caster: Entity,
    pub ability_id: super::abilities::AbilityId,
}
