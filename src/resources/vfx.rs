use bevy::prelude::*;

use super::damage::DamageType;
use super::particles::VfxKind;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// How far the attacker lunges toward the target (in pixels).
pub const LUNGE_MAGNITUDE: f32 = 8.0;
/// How long the lunge animation takes (seconds).
pub const LUNGE_DURATION: f32 = 0.15;

/// How long the white hit flash lasts (seconds). ~5 frames at 60fps.
pub const HIT_FLASH_DURATION: f32 = 0.08;

/// Max camera offset from screen shake (pixels).
pub const SHAKE_MAX_OFFSET: f32 = 6.0;
/// How fast trauma decays per second (exponential).
pub const SHAKE_DECAY_RATE: f32 = 5.0;
/// Trauma added per normal hit.
pub const SHAKE_TRAUMA_HIT: f32 = 0.1;
/// Trauma added when a body part is destroyed.
pub const SHAKE_TRAUMA_DESTROY: f32 = 0.25;
/// Trauma added on kill.
pub const SHAKE_TRAUMA_KILL: f32 = 0.5;

// ---------------------------------------------------------------------------
// Components — attached to gameplay entities
// ---------------------------------------------------------------------------

/// Small lunge toward the attack target. Applied as a pixel offset in `sync_transforms`.
/// Progress 0..0.5 = lunge out, 0.5..1.0 = snap back. Sine-eased.
#[derive(Component, Clone, Debug)]
pub struct AttackLunge {
    /// Normalized direction toward the target (world-space).
    pub direction: Vec2,
    /// 0.0 to 1.0 animation progress.
    pub progress: f32,
    /// Total duration in seconds.
    pub duration: f32,
}

impl AttackLunge {
    pub fn new(direction: Vec2) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            progress: 0.0,
            duration: LUNGE_DURATION,
        }
    }

    /// Current pixel offset to apply. Peaks at progress=0.5, returns to zero at 1.0.
    pub fn current_offset(&self) -> Vec2 {
        // Sine wave: 0 at 0, peak at 0.5, 0 at 1.0
        let t = (self.progress * std::f32::consts::PI).sin();
        self.direction * t * LUNGE_MAGNITUDE
    }

    /// Whether the animation is finished.
    pub fn is_done(&self) -> bool {
        self.progress >= 1.0
    }
}

/// Brief white flash on an entity that was hit.
#[derive(Component, Clone, Debug)]
pub struct HitFlash {
    /// Time remaining (seconds).
    pub timer: f32,
    /// Total duration (seconds).
    pub duration: f32,
    /// Original sprite color to restore when the flash ends.
    pub original_color: Color,
}

impl HitFlash {
    pub fn new(original_color: Color) -> Self {
        Self {
            timer: HIT_FLASH_DURATION,
            duration: HIT_FLASH_DURATION,
            original_color,
        }
    }

    pub fn is_done(&self) -> bool {
        self.timer <= 0.0
    }
}

// ---------------------------------------------------------------------------
// Resource — screen shake
// ---------------------------------------------------------------------------

/// Camera screen shake state. Single instance as a resource.
/// Trauma is added by combat events and decays exponentially.
#[derive(Resource, Clone, Debug)]
pub struct ScreenTrauma {
    /// Current trauma level (0.0 = calm, 1.0 = max shake).
    pub trauma: f32,
    /// Noise seed for smooth random offsets.
    pub seed: f32,
}

impl Default for ScreenTrauma {
    fn default() -> Self {
        Self {
            trauma: 0.0,
            seed: 0.0,
        }
    }
}

impl ScreenTrauma {
    /// Add trauma, clamped to 1.0.
    pub fn add(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    /// Decay trauma by dt. Returns the shake offset to apply this frame.
    pub fn tick(&mut self, dt: f32) -> Vec2 {
        if self.trauma <= 0.001 {
            self.trauma = 0.0;
            return Vec2::ZERO;
        }

        // Exponential decay
        self.trauma *= (-SHAKE_DECAY_RATE * dt).exp();

        // Intensity is trauma squared for a non-linear feel
        let intensity = self.trauma * self.trauma;

        // Simple deterministic pseudo-random using seed
        self.seed += dt * 50.0;
        let offset_x = self.seed.sin() * SHAKE_MAX_OFFSET * intensity;
        let offset_y = (self.seed * 1.3).cos() * SHAKE_MAX_OFFSET * intensity;

        Vec2::new(offset_x, offset_y)
    }
}

// ---------------------------------------------------------------------------
// Components — spawned as short-lived effect entities
// ---------------------------------------------------------------------------

/// Generic lifetime timer. Entity is despawned when remaining reaches zero.
#[derive(Component, Clone, Debug)]
pub struct DespawnTimer {
    pub remaining: f32,
}

impl DespawnTimer {
    pub fn new(seconds: f32) -> Self {
        Self { remaining: seconds }
    }

    pub fn is_done(&self) -> bool {
        self.remaining <= 0.0
    }
}

/// What kind of impact visual to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactKind {
    Slash,
    Pierce,
    Blunt,
    Fire,
    Frost,
    Storm,
    Arcane,
    Holy,
    Shadow,
    Nature,
    Heal,
}

impl ImpactKind {
    /// Map a damage type to an impact kind.
    pub fn from_damage_type(dt: DamageType) -> Self {
        match dt {
            DamageType::Slashing => Self::Slash,
            DamageType::Piercing => Self::Pierce,
            DamageType::Blunt => Self::Blunt,
            DamageType::Fire => Self::Fire,
            DamageType::Frost => Self::Frost,
            DamageType::Storm => Self::Storm,
            DamageType::Arcane => Self::Arcane,
            DamageType::Holy => Self::Holy,
            DamageType::Shadow => Self::Shadow,
            DamageType::Nature => Self::Nature,
        }
    }

    /// Placeholder color for this impact type.
    pub fn color(&self) -> Color {
        match self {
            Self::Slash => Color::srgba(0.9, 0.9, 0.9, 0.8),
            Self::Pierce => Color::srgba(0.7, 0.7, 0.9, 0.8),
            Self::Blunt => Color::srgba(0.6, 0.5, 0.4, 0.8),
            Self::Fire => Color::srgba(1.0, 0.4, 0.1, 0.9),
            Self::Frost => Color::srgba(0.5, 0.8, 1.0, 0.9),
            Self::Storm => Color::srgba(0.7, 0.7, 1.0, 0.9),
            Self::Arcane => Color::srgba(0.8, 0.3, 1.0, 0.9),
            Self::Holy => Color::srgba(1.0, 1.0, 0.7, 0.9),
            Self::Shadow => Color::srgba(0.3, 0.1, 0.4, 0.9),
            Self::Nature => Color::srgba(0.3, 0.8, 0.2, 0.9),
            Self::Heal => Color::srgba(0.3, 1.0, 0.4, 0.9),
        }
    }

    /// Default particle VFX for this impact category.
    pub fn default_vfx(&self) -> VfxKind {
        match self {
            Self::Slash => VfxKind::ImpactSlash,
            Self::Pierce => VfxKind::ImpactPierce,
            Self::Blunt => VfxKind::ImpactBlunt,
            Self::Fire => VfxKind::ImpactFire,
            Self::Frost => VfxKind::ImpactFrost,
            Self::Storm => VfxKind::ImpactStorm,
            Self::Arcane => VfxKind::ImpactArcane,
            Self::Holy => VfxKind::ImpactHoly,
            Self::Shadow => VfxKind::ImpactShadow,
            Self::Nature => VfxKind::ImpactNature,
            Self::Heal => VfxKind::ImpactHeal,
        }
    }
}
