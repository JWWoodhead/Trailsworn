use bevy::prelude::*;

/// Types of damage that can be dealt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageType {
    // Physical
    Slashing,
    Piercing,
    Blunt,
    // Magical — matches magic schools
    Fire,
    Frost,
    Storm,
    Arcane,
    Holy,
    Shadow,
    Nature,
}

impl DamageType {
    /// All damage types. Update this when adding a new variant.
    pub const ALL: [DamageType; 10] = [
        Self::Slashing, Self::Piercing, Self::Blunt,
        Self::Fire, Self::Frost, Self::Storm,
        Self::Arcane, Self::Holy, Self::Shadow, Self::Nature,
    ];

    pub fn is_physical(self) -> bool {
        matches!(self, Self::Slashing | Self::Piercing | Self::Blunt)
    }

    pub fn is_magical(self) -> bool {
        !self.is_physical()
    }
}

/// Resistance values per damage type (0.0 = no resistance, 1.0 = immune).
/// Uses a HashMap internally to avoid having to update a struct for every new type.
#[derive(Clone, Debug, Default)]
pub struct Resistances {
    values: std::collections::HashMap<DamageType, f32>,
}

impl Resistances {
    pub fn get(&self, damage_type: DamageType) -> f32 {
        self.values.get(&damage_type).copied().unwrap_or(0.0)
    }

    pub fn set(&mut self, damage_type: DamageType, value: f32) {
        self.values.insert(damage_type, value);
    }

    pub fn apply(&self, damage_type: DamageType, raw_damage: f32) -> f32 {
        let resistance = self.get(damage_type).clamp(0.0, 1.0);
        raw_damage * (1.0 - resistance)
    }
}

/// An armor piece that covers specific body parts.
#[derive(Clone, Debug)]
pub struct ArmorPiece {
    pub name: String,
    /// Which body part indices this armor covers.
    pub covered_parts: Vec<usize>,
    pub resistances: Resistances,
}

/// All armor currently worn by a character.
#[derive(Component, Clone, Debug, Default)]
pub struct EquippedArmor {
    pub pieces: Vec<ArmorPiece>,
}

impl EquippedArmor {
    /// Get the best resistance for a given body part and damage type.
    pub fn resistance_for_part(&self, part_index: usize, damage_type: DamageType) -> f32 {
        self.pieces
            .iter()
            .filter(|piece| piece.covered_parts.contains(&part_index))
            .map(|piece| piece.resistances.get(damage_type))
            .fold(0.0, f32::max)
    }

    /// Apply armor reduction for a specific body part hit.
    pub fn reduce_damage(
        &self,
        part_index: usize,
        damage_type: DamageType,
        raw_damage: f32,
    ) -> f32 {
        let resistance = self.resistance_for_part(part_index, damage_type).clamp(0.0, 1.0);
        raw_damage * (1.0 - resistance)
    }
}

/// A weapon definition.
#[derive(Clone, Debug)]
pub struct WeaponDef {
    pub name: String,
    pub damage_type: DamageType,
    pub base_damage: f32,
    /// Attack cooldown in simulation ticks.
    pub attack_speed_ticks: u32,
    /// Range in tiles. 1.5 = melee (adjacent + diagonal), >1.5 = ranged.
    pub range: f32,
    /// For ranged weapons: projectile travel speed in tiles per second.
    pub projectile_speed: f32,
    /// Whether this is a melee weapon.
    pub is_melee: bool,
}

/// The weapon a character currently has equipped.
#[derive(Component, Clone, Debug)]
pub struct EquippedWeapon {
    pub weapon: WeaponDef,
    /// Ticks remaining until next attack is ready.
    pub cooldown_remaining: u32,
}

impl EquippedWeapon {
    pub fn new(weapon: WeaponDef) -> Self {
        Self {
            weapon,
            cooldown_remaining: 0,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.cooldown_remaining == 0
    }

    pub fn tick(&mut self) {
        if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
        }
    }

    pub fn start_cooldown(&mut self) {
        self.cooldown_remaining = self.weapon.attack_speed_ticks;
    }
}
