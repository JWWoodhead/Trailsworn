use std::collections::HashMap;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

/// Categories of particle visual effects, analogous to SfxKind for audio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VfxKind {
    // Generic per-damage-type impacts (fallback for auto-attacks)
    ImpactSlash,
    ImpactPierce,
    ImpactBlunt,
    ImpactFire,
    ImpactFrost,
    ImpactStorm,
    ImpactArcane,
    ImpactHoly,
    ImpactShadow,
    ImpactNature,
    ImpactHeal,

    // Ability-specific overrides (bigger/unique versions)
    CleaveImpact,
    ShieldBashImpact,
    FireballImpact,
    FrostBoltImpact,
    HealLand,
    AimedShotImpact,
}

/// Pre-built particle effect handles, keyed by VfxKind.
/// Populated by the `setup_particle_effects` startup system.
#[derive(Resource, Default)]
pub struct ParticleAssets {
    pub handles: HashMap<VfxKind, Handle<EffectAsset>>,
}

impl ParticleAssets {
    pub fn get(&self, kind: VfxKind) -> Option<&Handle<EffectAsset>> {
        self.handles.get(&kind)
    }
}
