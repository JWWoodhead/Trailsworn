use std::collections::HashMap;

use bevy::prelude::*;

/// Categories of sound effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxKind {
    // Weapon attack impacts (per weapon class)
    SwordHit,
    MaceHit,
    DaggerHit,
    BowHit,
    StaffHit,
    // Generic fallbacks
    MeleeHit,
    MeleeMiss,
    RangedHit,
    // Ability cast sounds
    SpellCast,
    HealCast,
    WarCry,
    BandageRip,
    BowDraw,
    // Ability impact sounds
    SpellImpact,
    FireImpact,
    FrostImpact,
    HealLand,
    ShieldBashImpact,
    CleaveImpact,
    ArrowImpact,
    // Other
    Death,
    CastInterrupt,
}

/// Loaded audio handles, keyed by SfxKind.
/// Populated by the `setup_audio` startup system.
#[derive(Resource, Default)]
pub struct AudioAssets {
    pub handles: HashMap<SfxKind, Handle<AudioSource>>,
}

impl AudioAssets {
    /// Get a handle for a sound effect. Returns None if no asset is loaded for this kind.
    pub fn get(&self, kind: SfxKind) -> Option<&Handle<AudioSource>> {
        self.handles.get(&kind)
    }
}
