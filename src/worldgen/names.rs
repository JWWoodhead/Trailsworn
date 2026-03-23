use rand::Rng;
use rand::RngExt;

/// The core races of the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Race {
    Human,
    Dwarf,
    Elf,
    Orc,
    Goblin,
}

impl Race {
    pub const ALL: &[Race] = &[Race::Human, Race::Dwarf, Race::Elf, Race::Orc, Race::Goblin];
}

// ── Character Names ──

const HUMAN_FIRST: &[&str] = &[
    "Aldric", "Bram", "Cael", "Dorin", "Edric", "Falk", "Gareth", "Hadrian",
    "Isen", "Jareth", "Kael", "Leoric", "Maren", "Nolan", "Osric", "Percival",
    "Quinn", "Roland", "Soren", "Theron", "Ulric", "Voss", "Wren", "Yorin",
    "Alara", "Brenna", "Cira", "Dara", "Elara", "Freya", "Gwen", "Hild",
    "Iris", "Jora", "Kira", "Lyra", "Mira", "Nessa", "Orla", "Petra",
    "Rhea", "Sigrid", "Talia", "Ulla", "Vera", "Wynn", "Yara", "Zara",
];

const HUMAN_LAST: &[&str] = &[
    "Ashford", "Blackwood", "Crestfall", "Dunmere", "Everhart", "Foxglove",
    "Grimshaw", "Halloway", "Ironside", "Kettleburn", "Longmire", "Moorland",
    "Northcott", "Oakenshield", "Pemberton", "Ravenscar", "Stonehall",
    "Thornwick", "Underhill", "Whitmore", "Yarrow", "Ashborne", "Coldwell",
    "Duskwalker", "Flintmoor", "Greyvale", "Hearthstone", "Kingsley",
];

const DWARF_FIRST: &[&str] = &[
    "Bronn", "Durgan", "Grundi", "Haldor", "Korrin", "Magni", "Nori",
    "Thorin", "Ulfar", "Balin", "Dvalin", "Gimrik", "Khelgar", "Rurik",
    "Storn", "Thrain", "Vondal", "Bera", "Dagny", "Groa", "Hild",
    "Ketil", "Marga", "Sigrun", "Thyra", "Yrsa", "Brynhild", "Astrid",
];

const DWARF_LAST: &[&str] = &[
    "Ironforge", "Stonehammer", "Deepdelve", "Goldvein", "Coppermantle",
    "Steelbeard", "Anvilborn", "Coalfoot", "Darkmine", "Emberheart",
    "Flintspark", "Granitehold", "Hammerfall", "Leadbottom", "Mithrilaxe",
    "Obsidiancrest", "Quartzblood", "Rubyeye", "Silverhand", "Tinsmelter",
];

const ELF_FIRST: &[&str] = &[
    "Aelindra", "Caelith", "Elowen", "Faelorn", "Galathil", "Ithilwen",
    "Luthien", "Mirael", "Naelith", "Olorin", "Raelith", "Sylvarin",
    "Thalion", "Vaelith", "Arannis", "Celendil", "Elrohir", "Finrod",
    "Galadir", "Haldir", "Idril", "Laurelin", "Melian", "Nimloth",
    "Oropher", "Thranduil", "Arwen", "Celeborn",
];

const ELF_LAST: &[&str] = &[
    "Starweaver", "Moonwhisper", "Dawnstrider", "Leafshadow", "Silverbough",
    "Windwalker", "Thornbloom", "Dewfall", "Gladekeeper", "Nightbloom",
    "Ashenveil", "Brightwater", "Crystalsong", "Duskmere", "Everglen",
    "Frostpetal", "Greenthorn", "Hollowoak", "Ivymantle", "Sunfire",
];

const ORC_FIRST: &[&str] = &[
    "Grakk", "Mogul", "Thokk", "Urzag", "Borkul", "Durgash", "Ghash",
    "Krulak", "Lurbag", "Nazgash", "Shagrat", "Ufthak", "Bolg", "Gorbag",
    "Muzgash", "Razbag", "Snaga", "Yagul", "Baghra", "Durza", "Grisha",
    "Karga", "Lurza", "Mogra", "Shulga", "Uzra",
];

const ORC_LAST: &[&str] = &[
    "Skullcrusher", "Bonesnapper", "Ironjaw", "Bloodfist", "Goreclaw",
    "Deathbringer", "Wargrender", "Doomhowl", "Ashburner", "Blacktusk",
    "Fleshripper", "Gutspiller", "Headtaker", "Maimeye", "Ragefang",
    "Spinebreaker", "Thundermaw", "Vileblood",
];

const GOBLIN_FIRST: &[&str] = &[
    "Snik", "Grib", "Nix", "Pok", "Rik", "Tik", "Zik", "Blix",
    "Crik", "Drib", "Fiz", "Gnik", "Hix", "Jink", "Krik", "Lix",
    "Mik", "Nib", "Plik", "Skiz", "Trix", "Wik", "Yik", "Zib",
];

const GOBLIN_LAST: &[&str] = &[
    "Rattooth", "Mudfoot", "Sharpnose", "Quickfingers", "Darkear",
    "Greenpox", "Wormtongue", "Snagtooth", "Bilebreath", "Crookneck",
    "Dirthand", "Fleabag", "Grimtoe", "Hogsnout", "Inkstain",
    "Jinxeye", "Knobknee", "Louseback",
];

/// Generate a character name for a given race.
pub fn character_name(race: Race, rng: &mut impl Rng) -> (String, String) {
    let (firsts, lasts) = match race {
        Race::Human => (HUMAN_FIRST, HUMAN_LAST),
        Race::Dwarf => (DWARF_FIRST, DWARF_LAST),
        Race::Elf => (ELF_FIRST, ELF_LAST),
        Race::Orc => (ORC_FIRST, ORC_LAST),
        Race::Goblin => (GOBLIN_FIRST, GOBLIN_LAST),
    };

    let first = firsts[rng.random_range(0..firsts.len())].to_string();
    let last = lasts[rng.random_range(0..lasts.len())].to_string();
    (first, last)
}

/// Generate a full display name.
pub fn full_name(race: Race, rng: &mut impl Rng) -> String {
    let (first, last) = character_name(race, rng);
    format!("{first} {last}")
}

// ── Settlement Names ──

const SETTLEMENT_PREFIX: &[&str] = &[
    "Old", "New", "East", "West", "North", "South", "Upper", "Lower",
    "Fort", "Port", "High", "Deep", "Black", "White", "Red", "Grey",
    "Iron", "Stone", "Dark", "Bright",
];

const SETTLEMENT_ROOT: &[&str] = &[
    "haven", "hold", "reach", "fall", "gate", "watch", "crossing",
    "hollow", "mire", "dale", "ford", "stead", "thorpe", "wick",
    "brook", "ridge", "vale", "crest", "moor", "march",
    "keep", "bridge", "heath", "well", "barrow", "cairn",
];

/// Generate a settlement name.
pub fn settlement_name(rng: &mut impl Rng) -> String {
    // 60% prefix+root, 40% just root with capital
    if rng.random::<f32>() < 0.6 {
        let prefix = SETTLEMENT_PREFIX[rng.random_range(0..SETTLEMENT_PREFIX.len())];
        let root = SETTLEMENT_ROOT[rng.random_range(0..SETTLEMENT_ROOT.len())];
        format!("{prefix}{root}")
    } else {
        let root = SETTLEMENT_ROOT[rng.random_range(0..SETTLEMENT_ROOT.len())];
        let mut name = root.to_string();
        name[..1].make_ascii_uppercase();
        name
    }
}

// ── Faction Names ──

/// Types of factions that can exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FactionType {
    MercenaryCompany,
    Kingdom,
    ReligiousOrder,
    ThievesGuild,
    MerchantGuild,
    MageCircle,
    BanditClan,
    TribalWarband,
}

const MERCENARY_PATTERNS: &[&str] = &[
    "The {adj} Company", "The {adj} {noun}s", "{name}'s {noun}s",
    "The {noun}s of {place}", "The {adj} Guard",
];

const KINGDOM_PATTERNS: &[&str] = &[
    "The Kingdom of {place}", "The {adj} Realm", "The {place} Dominion",
    "The {adj} Crown", "{place}",
];

const RELIGIOUS_PATTERNS: &[&str] = &[
    "The Order of the {adj} {noun}", "The {noun} of {place}",
    "The {adj} Brotherhood", "The Temple of the {adj} {noun}",
    "Disciples of the {adj} {noun}",
];

const THIEVES_PATTERNS: &[&str] = &[
    "The {adj} Hand", "The {noun}s", "The {adj} {noun}s",
    "The Shadow {noun}s", "The {place} Syndicate",
];

const MERCHANT_PATTERNS: &[&str] = &[
    "The {place} Trading Company", "The {adj} Merchants",
    "{name} & Sons", "The {adj} Exchange", "The {place} Consortium",
];

const MAGE_PATTERNS: &[&str] = &[
    "The {adj} Circle", "The Conclave of {place}",
    "The {adj} Enclave", "The Order of the {adj} {noun}",
    "The {noun} Cabal",
];

const BANDIT_PATTERNS: &[&str] = &[
    "The {adj} {noun}s", "{name}'s Raiders", "The {place} Marauders",
    "The {adj} Wolves", "The {noun} Gang",
];

const WARBAND_PATTERNS: &[&str] = &[
    "The {adj} Horde", "Clan {name}", "The {noun} Tribe",
    "The {adj} Warband", "The {noun}s of the {adj} {noun}",
];

const FACTION_ADJ: &[&str] = &[
    "Iron", "Black", "Red", "Silver", "Golden", "Crimson", "Shadow",
    "Burning", "Frozen", "Silent", "Broken", "Fallen", "Ancient",
    "Dark", "Ashen", "Hollow", "Pale", "Scarlet", "Storm", "Dire",
    "Blood", "Grey", "White", "Sacred", "Cursed", "Veiled", "Grim",
];

const FACTION_NOUN: &[&str] = &[
    "Blade", "Shield", "Fang", "Flame", "Star", "Crown", "Hammer",
    "Raven", "Wolf", "Serpent", "Hawk", "Lion", "Bear", "Dragon",
    "Rose", "Thorn", "Hand", "Eye", "Sun", "Moon", "Skull", "Bone",
    "Claw", "Talon", "Arrow", "Spear", "Mask", "Veil",
];

const FACTION_PLACE: &[&str] = &[
    "Ashenmoor", "Blackhaven", "Crystalreach", "Dunmere", "Eldergrove",
    "Frostpeak", "Grimhold", "Highcairn", "Irondeep", "Kingsfall",
    "Lostmere", "Mournwatch", "Northreach", "Oldgate", "Ravenspire",
    "Shadowfen", "Thornwall", "Westmarch", "Hollowdale", "Stormbreak",
];

/// Generate a faction name for a given type.
pub fn faction_name(faction_type: FactionType, race: Race, rng: &mut impl Rng) -> String {
    let patterns = match faction_type {
        FactionType::MercenaryCompany => MERCENARY_PATTERNS,
        FactionType::Kingdom => KINGDOM_PATTERNS,
        FactionType::ReligiousOrder => RELIGIOUS_PATTERNS,
        FactionType::ThievesGuild => THIEVES_PATTERNS,
        FactionType::MerchantGuild => MERCHANT_PATTERNS,
        FactionType::MageCircle => MAGE_PATTERNS,
        FactionType::BanditClan => BANDIT_PATTERNS,
        FactionType::TribalWarband => WARBAND_PATTERNS,
    };

    let pattern = patterns[rng.random_range(0..patterns.len())];
    let (_, last) = character_name(race, rng);

    pattern
        .replace("{adj}", FACTION_ADJ[rng.random_range(0..FACTION_ADJ.len())])
        .replace("{noun}", FACTION_NOUN[rng.random_range(0..FACTION_NOUN.len())])
        .replace("{place}", FACTION_PLACE[rng.random_range(0..FACTION_PLACE.len())])
        .replace("{name}", &last)
}

// ── Region Names ──

const REGION_PREFIX: &[&str] = &[
    "The", "The Great", "The Northern", "The Southern", "The Eastern",
    "The Western", "The Central", "The Ancient",
];

const REGION_ROOT: &[&str] = &[
    "Wastes", "Highlands", "Lowlands", "Marshes", "Steppes",
    "Reaches", "Expanse", "Wilds", "Barrens", "Heartlands",
    "Frontier", "Borderlands", "Hinterlands", "Outlands",
    "Plains", "Forests", "Mountains", "Valleys", "Coast",
];

/// Generate a region name.
pub fn region_name(rng: &mut impl Rng) -> String {
    if rng.random::<f32>() < 0.5 {
        let prefix = REGION_PREFIX[rng.random_range(0..REGION_PREFIX.len())];
        let place = FACTION_PLACE[rng.random_range(0..FACTION_PLACE.len())];
        let root = REGION_ROOT[rng.random_range(0..REGION_ROOT.len())];
        format!("{prefix} {place} {root}")
    } else {
        let place = FACTION_PLACE[rng.random_range(0..FACTION_PLACE.len())];
        place.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn rng() -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(42)
    }

    #[test]
    fn character_names_for_all_races() {
        let mut rng = rng();
        for race in Race::ALL {
            let (first, last) = character_name(*race, &mut rng);
            assert!(!first.is_empty());
            assert!(!last.is_empty());
        }
    }

    #[test]
    fn full_name_has_space() {
        let mut rng = rng();
        let name = full_name(Race::Human, &mut rng);
        assert!(name.contains(' '));
    }

    #[test]
    fn settlement_names_not_empty() {
        let mut rng = rng();
        for _ in 0..20 {
            let name = settlement_name(&mut rng);
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn faction_names_for_all_types() {
        let mut rng = rng();
        let types = [
            FactionType::MercenaryCompany,
            FactionType::Kingdom,
            FactionType::ReligiousOrder,
            FactionType::ThievesGuild,
            FactionType::MerchantGuild,
            FactionType::MageCircle,
            FactionType::BanditClan,
            FactionType::TribalWarband,
        ];
        for ft in types {
            let name = faction_name(ft, Race::Human, &mut rng);
            assert!(!name.is_empty());
            // Should have resolved all placeholders
            assert!(!name.contains('{'), "unresolved placeholder in: {name}");
        }
    }

    #[test]
    fn dwarf_names_feel_dwarven() {
        let mut rng = rng();
        let (first, last) = character_name(Race::Dwarf, &mut rng);
        // Just verify we get dwarf-specific names, not human ones
        assert!(DWARF_FIRST.contains(&first.as_str()) || DWARF_LAST.contains(&last.as_str()));
    }

    #[test]
    fn deterministic_with_same_seed() {
        let name1 = full_name(Race::Elf, &mut rng());
        let name2 = full_name(Race::Elf, &mut rng());
        assert_eq!(name1, name2);
    }

    #[test]
    fn region_names_not_empty() {
        let mut rng = rng();
        for _ in 0..10 {
            let name = region_name(&mut rng);
            assert!(!name.is_empty());
        }
    }
}
