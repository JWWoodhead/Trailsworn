/// Schools of magic — domains of magical energy, not methods of practice.
/// A druid and a witch might both use Nature magic differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MagicSchool {
    // Elemental
    Fire,
    Frost,
    Storm,
    // Divine
    Holy,
    Shadow,
    // Arcane
    Arcane,
    Enchantment,
    // Primal
    Nature,
    Blood,
    // Death
    Necromancy,
}

impl MagicSchool {
    pub const ALL: &[MagicSchool] = &[
        Self::Fire, Self::Frost, Self::Storm,
        Self::Holy, Self::Shadow,
        Self::Arcane, Self::Enchantment,
        Self::Nature, Self::Blood,
        Self::Necromancy,
    ];

    pub fn category(self) -> MagicCategory {
        match self {
            Self::Fire | Self::Frost | Self::Storm => MagicCategory::Elemental,
            Self::Holy | Self::Shadow => MagicCategory::Divine,
            Self::Arcane | Self::Enchantment => MagicCategory::Arcane,
            Self::Nature | Self::Blood => MagicCategory::Primal,
            Self::Necromancy => MagicCategory::Death,
        }
    }

    /// Whether this school is generally feared/forbidden by civilized factions.
    pub fn is_forbidden(self) -> bool {
        matches!(self, Self::Blood | Self::Necromancy)
    }

    /// The damage type this school primarily deals.
    pub fn primary_damage_type(self) -> super::damage::DamageType {
        use super::damage::DamageType;
        match self {
            Self::Fire => DamageType::Fire,
            Self::Frost => DamageType::Frost,
            Self::Storm => DamageType::Storm,
            Self::Holy => DamageType::Holy,
            Self::Shadow => DamageType::Shadow,
            Self::Arcane => DamageType::Arcane,
            Self::Enchantment => DamageType::Arcane,
            Self::Nature => DamageType::Nature,
            Self::Blood => DamageType::Shadow,
            Self::Necromancy => DamageType::Shadow,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MagicCategory {
    Elemental,
    Divine,
    Arcane,
    Primal,
    Death,
}
