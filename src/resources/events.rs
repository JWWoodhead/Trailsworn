use bevy::prelude::*;

use super::abilities::AbilityId;
use super::damage::DamageType;

/// Fired when damage is dealt to an entity.
#[derive(Message, Clone, Debug)]
pub struct DamageDealtEvent {
    pub attacker: Entity,
    pub target: Entity,
    pub amount: f32,
    pub damage_type: DamageType,
    pub body_part_name: String,
    pub part_destroyed: bool,
    pub target_killed: bool,
    /// Set when damage comes from an ability rather than an auto-attack.
    pub ability_name: Option<String>,
    /// Ability ID for SFX lookup. None for auto-attacks.
    pub ability_id: Option<AbilityId>,
}

/// Fired when an attack misses.
#[derive(Message, Clone, Debug)]
pub struct AttackMissedEvent {
    pub attacker: Entity,
    pub target: Entity,
}

/// Fired when a cast begins (channeled or instant).
#[derive(Message, Clone, Debug)]
pub struct AbilityCastEvent {
    pub caster: Entity,
    pub ability_name: String,
    pub target_description: String,
    /// Ability ID for SFX lookup.
    pub ability_id: Option<AbilityId>,
}

/// Fired once when an ability resolves at a location (regardless of targets hit).
/// Used for AoE impact VFX at the landing point.
#[derive(Message, Clone, Debug)]
pub struct AbilityLandedEvent {
    pub caster: Entity,
    pub ability_id: AbilityId,
    /// Tile-space position where the ability landed.
    pub position: (f32, f32),
    /// Scale multiplier for the impact VFX.
    pub impact_vfx_scale: f32,
}

/// Fired when a cast is interrupted by damage.
#[derive(Message, Clone, Debug)]
pub struct CastInterruptedEvent {
    pub caster: Entity,
    pub ability_id: super::abilities::AbilityId,
}
