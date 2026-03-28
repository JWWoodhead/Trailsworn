use rand::{Rng, RngExt};

use crate::resources::magic::MagicSchool;
use super::gods::GodId;

/// A god-forged artifact from the divine era.
#[derive(Clone, Debug)]
pub struct DivineArtifact {
    pub id: u32,
    pub name: String,
    pub kind: DivineArtifactKind,
    pub creator_god: GodId,
    pub created_year: i32,
    pub magic_school: MagicSchool,
    /// Significance: 1 (minor) to 5 (world-shaping).
    pub power_level: u32,
    pub location: ArtifactLocation,
    pub description: String,
    pub lore: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DivineArtifactKind {
    Weapon,
    Armor,
    Implement,
    Key,
    Vessel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactLocation {
    /// Stored at a DivineSite.
    AtSite(u32),
    /// Location unknown — discoverable in mortal era.
    Lost,
    /// Held by a god's mortal champion.
    HeldByChampion(GodId),
    /// Destroyed during divine conflict.
    Destroyed,
}

const DIVINE_PREFIXES: &[&str] = &[
    "Primordial", "Celestial", "Eternal", "Godforged", "First",
    "Undying", "Absolute", "Sovereign", "Mythic", "Elder",
];

fn domain_suffixes(school: MagicSchool) -> &'static [&'static str] {
    match school {
        MagicSchool::Fire => &["of Embers", "of the Forge", "of Molten Fury", "of Ash"],
        MagicSchool::Frost => &["of Winter", "of Stillness", "of the Glacier", "of Frozen Tears"],
        MagicSchool::Storm => &["of Thunder", "of the Tempest", "of Lightning", "of the Gale"],
        MagicSchool::Holy => &["of Radiance", "of the Dawn", "of Judgement", "of Grace"],
        MagicSchool::Shadow => &["of Twilight", "of the Void", "of Whispers", "of Dusk"],
        MagicSchool::Nature => &["of the Wild", "of Roots", "of Living Stone", "of Thorns"],
        MagicSchool::Necromancy => &["of Bone", "of the Grave", "of Final Rest", "of Decay"],
        MagicSchool::Arcane => &["of Stars", "of the Weave", "of Aether", "of Resonance"],
        _ => &["of Power", "of Mystery", "of the Unknown"],
    }
}

fn kind_nouns(kind: DivineArtifactKind) -> &'static [&'static str] {
    match kind {
        DivineArtifactKind::Weapon => &["Blade", "Hammer", "Spear", "Staff", "Bow"],
        DivineArtifactKind::Armor => &["Shield", "Crown", "Helm", "Mantle", "Aegis"],
        DivineArtifactKind::Implement => &["Orb", "Tome", "Crystal", "Scepter", "Chalice"],
        DivineArtifactKind::Key => &["Key", "Seal", "Sigil", "Rune", "Lock"],
        DivineArtifactKind::Vessel => &["Vessel", "Urn", "Phylactery", "Reliquary", "Ark"],
    }
}

/// Generate a name for a divine artifact.
pub fn divine_artifact_name(
    kind: DivineArtifactKind,
    school: MagicSchool,
    rng: &mut impl Rng,
) -> String {
    let prefix = DIVINE_PREFIXES[rng.random_range(0..DIVINE_PREFIXES.len())];
    let noun = kind_nouns(kind)[rng.random_range(0..kind_nouns(kind).len())];
    let suffixes = domain_suffixes(school);
    let suffix = suffixes[rng.random_range(0..suffixes.len())];
    format!("{} {} {}", prefix, noun, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn artifact_name_generation() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let name = divine_artifact_name(DivineArtifactKind::Weapon, MagicSchool::Fire, &mut rng);
        assert!(!name.is_empty());
        // Should have 3 parts
        assert!(name.split_whitespace().count() >= 3);
    }

    #[test]
    fn artifact_name_deterministic() {
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(99);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(99);
        let a = divine_artifact_name(DivineArtifactKind::Implement, MagicSchool::Arcane, &mut rng1);
        let b = divine_artifact_name(DivineArtifactKind::Implement, MagicSchool::Arcane, &mut rng2);
        assert_eq!(a, b);
    }
}
