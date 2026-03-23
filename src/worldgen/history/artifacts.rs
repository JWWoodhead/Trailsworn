use rand::{Rng, RngExt};

/// A persistent artifact in the world history.
#[derive(Clone, Debug)]
pub struct Artifact {
    pub id: u32,
    pub name: String,
    pub kind: ArtifactKind,
    pub discovered_year: i32,
    pub discovered_by_faction: u32,
    pub holder_character: Option<u32>,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    Weapon,
    Armor,
    Tome,
    Crown,
    Relic,
    Gem,
}

const ARTIFACT_PREFIXES: &[&str] = &[
    "Ancient", "Cursed", "Blessed", "Forgotten", "Eternal", "Shattered",
    "Burning", "Frozen", "Shadow", "Radiant", "Bloodstained", "Whispering",
    "Iron", "Golden", "Obsidian", "Crystal", "Bone", "Storm",
];

const WEAPON_NAMES: &[&str] = &[
    "Blade", "Sword", "Axe", "Hammer", "Spear", "Mace", "Halberd", "Dagger",
    "Greatsword", "Warblade", "Cleaver",
];

const ARMOR_NAMES: &[&str] = &[
    "Shield", "Helm", "Breastplate", "Gauntlets", "Greaves", "Crown",
];

const TOME_NAMES: &[&str] = &[
    "Tome", "Codex", "Grimoire", "Scroll", "Chronicle", "Manuscript",
];

const RELIC_NAMES: &[&str] = &[
    "Chalice", "Amulet", "Orb", "Scepter", "Ring", "Horn", "Lantern", "Mirror",
];

const GEM_NAMES: &[&str] = &[
    "Ruby", "Sapphire", "Emerald", "Diamond", "Opal", "Amethyst", "Topaz",
];

const ARTIFACT_SUFFIXES: &[&str] = &[
    "of Ages", "of Ruin", "of the Fallen", "of Dominion", "of Truth",
    "of Whispers", "of the Deep", "of the First King", "of Wrath",
    "of the Void", "of Starlight", "of Binding", "of the Forgotten God",
    "of Sorrow", "of Valor",
];

/// Generate a named artifact.
pub fn generate_artifact(
    id: u32,
    kind: ArtifactKind,
    discovered_year: i32,
    discovered_by_faction: u32,
    rng: &mut impl Rng,
) -> Artifact {
    let prefix = ARTIFACT_PREFIXES[rng.random_range(0..ARTIFACT_PREFIXES.len())];
    let base = match kind {
        ArtifactKind::Weapon => WEAPON_NAMES[rng.random_range(0..WEAPON_NAMES.len())],
        ArtifactKind::Armor => ARMOR_NAMES[rng.random_range(0..ARMOR_NAMES.len())],
        ArtifactKind::Tome => TOME_NAMES[rng.random_range(0..TOME_NAMES.len())],
        ArtifactKind::Relic => RELIC_NAMES[rng.random_range(0..RELIC_NAMES.len())],
        ArtifactKind::Gem => GEM_NAMES[rng.random_range(0..GEM_NAMES.len())],
        ArtifactKind::Crown => &"Crown",
    };
    let suffix = ARTIFACT_SUFFIXES[rng.random_range(0..ARTIFACT_SUFFIXES.len())];

    let name = format!("The {} {} {}", prefix, base, suffix);

    let description = match kind {
        ArtifactKind::Weapon => format!("A legendary weapon said to grant its wielder unmatched prowess in battle"),
        ArtifactKind::Armor => format!("Forged in an age long past, this armor is said to turn aside any blow"),
        ArtifactKind::Tome => format!("A volume of forbidden knowledge, its pages hum with latent power"),
        ArtifactKind::Crown => format!("A symbol of absolute authority, coveted by every would-be ruler"),
        ArtifactKind::Relic => format!("A sacred object of immense spiritual significance"),
        ArtifactKind::Gem => format!("A gemstone of supernatural beauty that clouds the minds of the greedy"),
    };

    Artifact {
        id,
        name,
        kind,
        discovered_year,
        discovered_by_faction,
        holder_character: None,
        description,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn artifact_name_not_empty() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let a = generate_artifact(1, ArtifactKind::Weapon, 50, 1, &mut rng);
        assert!(!a.name.is_empty());
        assert!(a.name.starts_with("The "));
    }

    #[test]
    fn all_kinds_generate() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let kinds = [ArtifactKind::Weapon, ArtifactKind::Armor, ArtifactKind::Tome,
            ArtifactKind::Crown, ArtifactKind::Relic, ArtifactKind::Gem];
        for kind in kinds {
            let a = generate_artifact(1, kind, 50, 1, &mut rng);
            assert!(!a.name.is_empty());
            assert!(!a.description.is_empty());
        }
    }
}
