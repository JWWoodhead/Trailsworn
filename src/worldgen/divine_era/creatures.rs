use crate::resources::magic::MagicSchool;

/// Role a divine creature serves for its patron god.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CreatureRole {
    /// Guards sacred sites and territory.
    Guardian,
    /// Fights in divine wars as the god's army.
    Warrior,
    /// Appears to mortals as a sign or messenger.
    Emissary,
    /// Accompanies the god as a personal companion.
    Companion,
}

/// A type of mythical creature associated with a god's domain.
#[derive(Clone, Debug)]
pub struct DivineCreatureType {
    pub name: &'static str,
    pub role: CreatureRole,
    pub description: &'static str,
}

/// Get the creature types associated with a magic school.
pub fn creatures_for_domain(domain: MagicSchool) -> &'static [DivineCreatureType] {
    match domain {
        MagicSchool::Fire => &FIRE_CREATURES,
        MagicSchool::Frost => &FROST_CREATURES,
        MagicSchool::Storm => &STORM_CREATURES,
        MagicSchool::Holy => &HOLY_CREATURES,
        MagicSchool::Shadow => &SHADOW_CREATURES,
        MagicSchool::Nature => &NATURE_CREATURES,
        MagicSchool::Necromancy => &DEATH_CREATURES,
        MagicSchool::Arcane => &ARCANE_CREATURES,
        _ => &[],
    }
}

static FIRE_CREATURES: [DivineCreatureType; 4] = [
    DivineCreatureType {
        name: "Phoenix",
        role: CreatureRole::Emissary,
        description: "A bird of living flame that dies and is reborn from its ashes",
    },
    DivineCreatureType {
        name: "Salamander",
        role: CreatureRole::Guardian,
        description: "A serpentine creature that dwells in the hottest fires",
    },
    DivineCreatureType {
        name: "Forge Golem",
        role: CreatureRole::Warrior,
        description: "A construct of molten metal and hammered stone, forged for war",
    },
    DivineCreatureType {
        name: "Fire Drake",
        role: CreatureRole::Companion,
        description: "A lesser dragon wreathed in flame, fiercely loyal to its creator",
    },
];

static FROST_CREATURES: [DivineCreatureType; 4] = [
    DivineCreatureType {
        name: "Ice Wyrm",
        role: CreatureRole::Guardian,
        description: "An ancient serpent encased in living ice, coiled around frozen vaults",
    },
    DivineCreatureType {
        name: "Frost Giant",
        role: CreatureRole::Warrior,
        description: "A towering humanoid of ice and stone, slow but unstoppable",
    },
    DivineCreatureType {
        name: "Winter Wolf",
        role: CreatureRole::Emissary,
        description: "A spectral white wolf whose howl brings blizzards",
    },
    DivineCreatureType {
        name: "Crystal Spirit",
        role: CreatureRole::Companion,
        description: "A shimmering presence within a perfect ice crystal",
    },
];

static STORM_CREATURES: [DivineCreatureType; 4] = [
    DivineCreatureType {
        name: "Thunderbird",
        role: CreatureRole::Emissary,
        description: "A vast raptor whose wingbeats crack thunder across the sky",
    },
    DivineCreatureType {
        name: "Lightning Drake",
        role: CreatureRole::Warrior,
        description: "A serpentine dragon that arcs between clouds, crackling with energy",
    },
    DivineCreatureType {
        name: "Storm Elemental",
        role: CreatureRole::Guardian,
        description: "A roiling vortex of wind, rain, and lightning given will",
    },
    DivineCreatureType {
        name: "Kraken",
        role: CreatureRole::Companion,
        description: "A deep-sea titan that commands the currents and drowns the unwary",
    },
];

static HOLY_CREATURES: [DivineCreatureType; 4] = [
    DivineCreatureType {
        name: "Celestial",
        role: CreatureRole::Emissary,
        description: "A radiant winged being that speaks with the voice of divine law",
    },
    DivineCreatureType {
        name: "Griffin",
        role: CreatureRole::Warrior,
        description: "A noble beast with the body of a lion and wings of an eagle",
    },
    DivineCreatureType {
        name: "Unicorn",
        role: CreatureRole::Companion,
        description: "A pure white horse whose horn can heal any wound or purge any corruption",
    },
    DivineCreatureType {
        name: "Solar Lion",
        role: CreatureRole::Guardian,
        description: "A great cat wreathed in golden light, guardian of holy places",
    },
];

static SHADOW_CREATURES: [DivineCreatureType; 4] = [
    DivineCreatureType {
        name: "Wraith",
        role: CreatureRole::Warrior,
        description: "A shade given form and hunger, slipping between darkness and flesh",
    },
    DivineCreatureType {
        name: "Shade Spider",
        role: CreatureRole::Guardian,
        description: "An arachnid woven from shadow, spinning webs that trap light itself",
    },
    DivineCreatureType {
        name: "Night Stalker",
        role: CreatureRole::Emissary,
        description: "A silent figure seen only at the edge of vision, delivering whispered omens",
    },
    DivineCreatureType {
        name: "Dark Raven",
        role: CreatureRole::Companion,
        description: "A corvid with eyes like starless voids, seeing through all deception",
    },
];

static NATURE_CREATURES: [DivineCreatureType; 4] = [
    DivineCreatureType {
        name: "Treant",
        role: CreatureRole::Guardian,
        description: "An ancient tree given slow, deliberate life, rooted in centuries of memory",
    },
    DivineCreatureType {
        name: "Dire Beast",
        role: CreatureRole::Warrior,
        description: "A massive predator — wolf, bear, or boar — grown far beyond natural size",
    },
    DivineCreatureType {
        name: "Dryad",
        role: CreatureRole::Emissary,
        description: "A spirit bound to a living grove, speaking for the wild to those who listen",
    },
    DivineCreatureType {
        name: "Swarm",
        role: CreatureRole::Companion,
        description: "A living cloud of insects, birds, or vermin moving with a single will",
    },
];

static DEATH_CREATURES: [DivineCreatureType; 4] = [
    DivineCreatureType {
        name: "Revenant",
        role: CreatureRole::Warrior,
        description: "A corpse animated by unfinished purpose, tireless and unyielding",
    },
    DivineCreatureType {
        name: "Bone Construct",
        role: CreatureRole::Guardian,
        description: "An edifice of fused bones shaped into a terrible sentinel",
    },
    DivineCreatureType {
        name: "Spectral Hound",
        role: CreatureRole::Emissary,
        description: "A ghostly canine that appears before death, guiding souls to the threshold",
    },
    DivineCreatureType {
        name: "Death Moth",
        role: CreatureRole::Companion,
        description: "A pale moth the size of a hawk, drawn to the dying, feeding on last breaths",
    },
];

static ARCANE_CREATURES: [DivineCreatureType; 4] = [
    DivineCreatureType {
        name: "Arcane Construct",
        role: CreatureRole::Guardian,
        description: "A geometric form of pure magic given purpose, precise and relentless",
    },
    DivineCreatureType {
        name: "Crystal Guardian",
        role: CreatureRole::Warrior,
        description: "A humanoid of living crystal that refracts and redirects magical energy",
    },
    DivineCreatureType {
        name: "Wisp",
        role: CreatureRole::Emissary,
        description: "A mote of raw magic that drifts through the world, drawn to potential",
    },
    DivineCreatureType {
        name: "Familiar",
        role: CreatureRole::Companion,
        description: "A small creature — owl, cat, or raven — infused with arcane awareness",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_domains_have_creatures() {
        for domain in [
            MagicSchool::Fire, MagicSchool::Frost, MagicSchool::Storm,
            MagicSchool::Holy, MagicSchool::Shadow, MagicSchool::Nature,
            MagicSchool::Necromancy, MagicSchool::Arcane,
        ] {
            let creatures = creatures_for_domain(domain);
            assert_eq!(creatures.len(), 4, "Domain {:?} should have 4 creatures", domain);
        }
    }

    #[test]
    fn each_domain_has_all_roles() {
        for domain in [
            MagicSchool::Fire, MagicSchool::Frost, MagicSchool::Storm,
            MagicSchool::Holy, MagicSchool::Shadow, MagicSchool::Nature,
            MagicSchool::Necromancy, MagicSchool::Arcane,
        ] {
            let creatures = creatures_for_domain(domain);
            let roles: Vec<CreatureRole> = creatures.iter().map(|c| c.role).collect();
            assert!(roles.contains(&CreatureRole::Guardian), "{:?} missing Guardian", domain);
            assert!(roles.contains(&CreatureRole::Warrior), "{:?} missing Warrior", domain);
            assert!(roles.contains(&CreatureRole::Emissary), "{:?} missing Emissary", domain);
            assert!(roles.contains(&CreatureRole::Companion), "{:?} missing Companion", domain);
        }
    }
}
