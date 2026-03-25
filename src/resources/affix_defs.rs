use super::affixes::*;
use super::damage::DamageType;
use super::items::ItemKind;
use super::stats::AttributeChoice;

/// Populate the affix registry with all starter affixes.
pub fn register_starter_affixes(registry: &mut AffixRegistry) {
    register_weapon_prefixes(registry);
    register_armor_prefixes(registry);
    register_weapon_suffixes(registry);
    register_resistance_suffixes(registry);
    register_attribute_suffixes(registry);
}

// ---------------------------------------------------------------------------
// Weapon prefixes
// ---------------------------------------------------------------------------

fn register_weapon_prefixes(reg: &mut AffixRegistry) {
    // +Flat Damage
    reg.register(AffixDef {
        id: AffixId(1),
        name: "Flat Damage".into(),
        slot_type: AffixSlotType::Prefix,
        allowed_kinds: vec![ItemKind::Weapon],
        tiers: vec![
            AffixTier { min_item_level: 1,  effect: AffixEffect::FlatDamage(2.0),  label: "Keen".into() },
            AffixTier { min_item_level: 10, effect: AffixEffect::FlatDamage(5.0),  label: "Honed".into() },
            AffixTier { min_item_level: 20, effect: AffixEffect::FlatDamage(9.0),  label: "Wrathful".into() },
            AffixTier { min_item_level: 35, effect: AffixEffect::FlatDamage(14.0), label: "Ruinous".into() },
        ],
    });

    // +% Damage
    reg.register(AffixDef {
        id: AffixId(2),
        name: "Enhanced Damage".into(),
        slot_type: AffixSlotType::Prefix,
        allowed_kinds: vec![ItemKind::Weapon],
        tiers: vec![
            AffixTier { min_item_level: 5,  effect: AffixEffect::PercentDamage(0.10), label: "Tempered".into() },
            AffixTier { min_item_level: 15, effect: AffixEffect::PercentDamage(0.20), label: "Brutal".into() },
            AffixTier { min_item_level: 28, effect: AffixEffect::PercentDamage(0.35), label: "Devastating".into() },
        ],
    });
}

// ---------------------------------------------------------------------------
// Armor prefixes
// ---------------------------------------------------------------------------

fn register_armor_prefixes(reg: &mut AffixRegistry) {
    // +Max Mana
    reg.register(AffixDef {
        id: AffixId(3),
        name: "Max Mana".into(),
        slot_type: AffixSlotType::Prefix,
        allowed_kinds: vec![ItemKind::Armor],
        tiers: vec![
            AffixTier { min_item_level: 3,  effect: AffixEffect::MaxMana(10.0), label: "Shimmering".into() },
            AffixTier { min_item_level: 15, effect: AffixEffect::MaxMana(25.0), label: "Luminous".into() },
            AffixTier { min_item_level: 28, effect: AffixEffect::MaxMana(45.0), label: "Wellspring".into() },
        ],
    });

    // +Max Stamina
    reg.register(AffixDef {
        id: AffixId(4),
        name: "Max Stamina".into(),
        slot_type: AffixSlotType::Prefix,
        allowed_kinds: vec![ItemKind::Armor],
        tiers: vec![
            AffixTier { min_item_level: 3,  effect: AffixEffect::MaxStamina(10.0), label: "Stalwart".into() },
            AffixTier { min_item_level: 15, effect: AffixEffect::MaxStamina(25.0), label: "Enduring".into() },
            AffixTier { min_item_level: 28, effect: AffixEffect::MaxStamina(45.0), label: "Tireless".into() },
        ],
    });
}

// ---------------------------------------------------------------------------
// Weapon suffixes
// ---------------------------------------------------------------------------

fn register_weapon_suffixes(reg: &mut AffixRegistry) {
    // +Attack Speed (reduce cooldown ticks)
    reg.register(AffixDef {
        id: AffixId(5),
        name: "Attack Speed".into(),
        slot_type: AffixSlotType::Suffix,
        allowed_kinds: vec![ItemKind::Weapon],
        tiers: vec![
            AffixTier { min_item_level: 5,  effect: AffixEffect::AttackSpeed(5),  label: "of Alacrity".into() },
            AffixTier { min_item_level: 18, effect: AffixEffect::AttackSpeed(10), label: "of the Gale".into() },
            AffixTier { min_item_level: 32, effect: AffixEffect::AttackSpeed(18), label: "of Frenzy".into() },
        ],
    });
}

// ---------------------------------------------------------------------------
// Resistance suffixes (all 10 damage types, armor only)
// ---------------------------------------------------------------------------

fn register_resistance_suffixes(reg: &mut AffixRegistry) {
    let resist_affixes: Vec<(u32, DamageType, [&str; 3])> = vec![
        (10, DamageType::Slashing, ["of the Hide", "of Iron Skin", "of the Carapace"]),
        (11, DamageType::Piercing, ["of Thorns", "of the Quill", "of Deflection"]),
        (12, DamageType::Blunt,    ["of Padding", "of the Anvil", "of Fortification"]),
        (13, DamageType::Fire,     ["of Embers", "of the Hearth", "of Firewalking"]),
        (14, DamageType::Frost,    ["of Thaw", "of the Glacier", "of Permafrost"]),
        (15, DamageType::Storm,    ["of Grounding", "of the Calm", "of Stormbreak"]),
        (16, DamageType::Arcane,   ["of Warding", "of Nullification", "of Spellguard"]),
        (17, DamageType::Holy,     ["of Doubt", "of the Skeptic", "of Heresy"]),
        (18, DamageType::Shadow,   ["of Twilight", "of the Veil", "of Shadowbane"]),
        (19, DamageType::Nature,   ["of the Root", "of Wild Growth", "of the Deepwood"]),
    ];

    for (id, damage_type, labels) in resist_affixes {
        reg.register(AffixDef {
            id: AffixId(id),
            name: format!("{:?} Resistance", damage_type),
            slot_type: AffixSlotType::Suffix,
            allowed_kinds: vec![ItemKind::Armor],
            tiers: vec![
                AffixTier { min_item_level: 1,  effect: AffixEffect::Resistance { damage_type, amount: 0.05 }, label: labels[0].into() },
                AffixTier { min_item_level: 12, effect: AffixEffect::Resistance { damage_type, amount: 0.10 }, label: labels[1].into() },
                AffixTier { min_item_level: 25, effect: AffixEffect::Resistance { damage_type, amount: 0.20 }, label: labels[2].into() },
            ],
        });
    }
}

// ---------------------------------------------------------------------------
// Attribute suffixes (weapon + armor)
// ---------------------------------------------------------------------------

fn register_attribute_suffixes(reg: &mut AffixRegistry) {
    let attr_affixes: Vec<(u32, AttributeChoice, [&str; 3])> = vec![
        (20, AttributeChoice::Strength,  ["of the Brute", "of the Ox", "of the Colossus"]),
        (21, AttributeChoice::Agility,   ["of the Fox", "of the Wind", "of the Phantom"]),
        (22, AttributeChoice::Intellect, ["of the Scholar", "of the Sage", "of the Oracle"]),
        (23, AttributeChoice::Toughness, ["of the Stone", "of the Mountain", "of the Unyielding"]),
        (24, AttributeChoice::Willpower, ["of Resolve", "of the Iron Mind", "of Dominion"]),
    ];

    for (id, attribute, labels) in attr_affixes {
        reg.register(AffixDef {
            id: AffixId(id),
            name: format!("{:?}", attribute),
            slot_type: AffixSlotType::Suffix,
            allowed_kinds: vec![ItemKind::Weapon, ItemKind::Armor],
            tiers: vec![
                AffixTier { min_item_level: 1,  effect: AffixEffect::Attribute { attribute, amount: 1 }, label: labels[0].into() },
                AffixTier { min_item_level: 15, effect: AffixEffect::Attribute { attribute, amount: 2 }, label: labels[1].into() },
                AffixTier { min_item_level: 30, effect: AffixEffect::Attribute { attribute, amount: 3 }, label: labels[2].into() },
            ],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_affixes_registered() {
        let mut reg = AffixRegistry::default();
        register_starter_affixes(&mut reg);

        // 2 weapon prefix + 2 armor prefix + 1 weapon suffix + 10 resist + 5 attribute = 20
        assert!(reg.get(AffixId(1)).is_some(), "Flat Damage");
        assert!(reg.get(AffixId(2)).is_some(), "Enhanced Damage");
        assert!(reg.get(AffixId(3)).is_some(), "Max Mana");
        assert!(reg.get(AffixId(4)).is_some(), "Max Stamina");
        assert!(reg.get(AffixId(5)).is_some(), "Attack Speed");
        for id in 10..=19 {
            assert!(reg.get(AffixId(id)).is_some(), "Resistance {}", id);
        }
        for id in 20..=24 {
            assert!(reg.get(AffixId(id)).is_some(), "Attribute {}", id);
        }
    }

    #[test]
    fn weapon_has_prefix_and_suffix_candidates() {
        let mut reg = AffixRegistry::default();
        register_starter_affixes(&mut reg);

        let prefixes = reg.candidates(ItemKind::Weapon, AffixSlotType::Prefix, 10);
        assert!(prefixes.len() >= 2, "Weapon should have at least flat+% damage prefixes");

        let suffixes = reg.candidates(ItemKind::Weapon, AffixSlotType::Suffix, 10);
        assert!(suffixes.len() >= 6, "Weapon should have attack speed + 5 attribute suffixes");
    }

    #[test]
    fn armor_has_resist_and_attribute_suffixes() {
        let mut reg = AffixRegistry::default();
        register_starter_affixes(&mut reg);

        let suffixes = reg.candidates(ItemKind::Armor, AffixSlotType::Suffix, 10);
        assert!(suffixes.len() >= 15, "Armor should have 10 resist + 5 attribute suffixes");
    }

    #[test]
    fn ilvl_gates_affixes() {
        let mut reg = AffixRegistry::default();
        register_starter_affixes(&mut reg);

        // % damage requires ilvl 5
        let prefixes_ilvl1 = reg.candidates(ItemKind::Weapon, AffixSlotType::Prefix, 1);
        let prefixes_ilvl5 = reg.candidates(ItemKind::Weapon, AffixSlotType::Prefix, 5);
        assert!(prefixes_ilvl5.len() > prefixes_ilvl1.len());
    }
}
