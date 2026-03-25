use super::damage::{DamageType, Resistances, WeaponDef};
use super::items::*;

// ---------------------------------------------------------------------------
// Weapon IDs (by class × tier)
// ---------------------------------------------------------------------------

// Swords
pub const ITEM_CHIPPED_BLADE: ItemId = ItemId(1);
pub const ITEM_WATCHMANS_SWORD: ItemId = ItemId(2);
pub const ITEM_OATHBRAND: ItemId = ItemId(3);

// Maces
pub const ITEM_KNOBWOOD_CLUB: ItemId = ItemId(10);
pub const ITEM_IRONBOUND_MAUL: ItemId = ItemId(11);
pub const ITEM_CINDERHAMMER: ItemId = ItemId(12);

// Daggers
pub const ITEM_WHITTLED_SHIV: ItemId = ItemId(20);
pub const ITEM_REAVERS_FANG: ItemId = ItemId(21);
pub const ITEM_WHISPERPOINT: ItemId = ItemId(22);

// Bows
pub const ITEM_BENT_STAVE: ItemId = ItemId(30);
pub const ITEM_RANGERS_LONGBOW: ItemId = ItemId(31);
pub const ITEM_GALESTRING: ItemId = ItemId(32);

// Staves
pub const ITEM_GNARLED_BRANCH: ItemId = ItemId(40);
pub const ITEM_WANDERERS_STAFF: ItemId = ItemId(41);
pub const ITEM_SPELLWARDEN: ItemId = ItemId(42);

// ---------------------------------------------------------------------------
// Armor IDs (by slot × tier)
// ---------------------------------------------------------------------------

// Head
pub const ITEM_STITCHED_HOOD: ItemId = ItemId(100);
pub const ITEM_RIVETED_COIF: ItemId = ItemId(101);
pub const ITEM_WARDCREST_HELM: ItemId = ItemId(102);

// Chest
pub const ITEM_SCRAPED_HIDE_VEST: ItemId = ItemId(110);
pub const ITEM_BANDED_HAUBERK: ItemId = ItemId(111);
pub const ITEM_RUNEWOVEN_CUIRASS: ItemId = ItemId(112);

// Hands
pub const ITEM_FRAYED_WRAPS: ItemId = ItemId(120);
pub const ITEM_PADDED_GRIPS: ItemId = ItemId(121);
pub const ITEM_IRONWEAVE_GLOVES: ItemId = ItemId(122);

// Legs
pub const ITEM_PATCHED_LEGGINGS: ItemId = ItemId(130);
pub const ITEM_PLATED_GREAVES: ItemId = ItemId(131);
pub const ITEM_VANGUARD_TASSETS: ItemId = ItemId(132);

// Feet
pub const ITEM_COBBLED_BOOTS: ItemId = ItemId(140);
pub const ITEM_STRIDERS_TREADS: ItemId = ItemId(141);
pub const ITEM_ASHWALKERS: ItemId = ItemId(142);

// Shields
pub const ITEM_SPLINTERED_BUCKLER: ItemId = ItemId(150);
pub const ITEM_BOSSED_KITE_SHIELD: ItemId = ItemId(151);
pub const ITEM_BULWARK_OF_THE_MARK: ItemId = ItemId(152);

// ---------------------------------------------------------------------------
// Consumables
// ---------------------------------------------------------------------------

pub const ITEM_HEALTH_POTION: ItemId = ItemId(200);
pub const ITEM_MANA_POTION: ItemId = ItemId(201);
pub const ITEM_STAMINA_POTION: ItemId = ItemId(202);
pub const ITEM_BANDAGE: ItemId = ItemId(203);

// ---------------------------------------------------------------------------
// Materials
// ---------------------------------------------------------------------------

pub const ITEM_BONE_FRAGMENT: ItemId = ItemId(300);
pub const ITEM_IRON_INGOT: ItemId = ItemId(301);
pub const ITEM_LEATHER_SCRAP: ItemId = ItemId(302);
pub const ITEM_HERB: ItemId = ItemId(303);

/// Populate the item registry with all starter items.
pub fn register_starter_items(registry: &mut ItemRegistry) {
    register_weapons(registry);
    register_armor(registry);
    register_consumables(registry);
    register_materials(registry);
}

// ---------------------------------------------------------------------------
// Weapons (5 classes × 3 tiers = 15 weapons)
// ---------------------------------------------------------------------------

fn register_weapons(reg: &mut ItemRegistry) {
    // --- Swords ---
    reg.register(weapon(ITEM_CHIPPED_BLADE, "Chipped Blade", "A notched edge, barely sharp.", BaseTier::Crude, 1, WeaponClass::Sword,
        DamageType::Slashing, 5.0, 100, 1.5, true, 2.5));
    reg.register(weapon(ITEM_WATCHMANS_SWORD, "Watchman's Sword", "Standard-issue. Gets the job done.", BaseTier::Tempered, 15, WeaponClass::Sword,
        DamageType::Slashing, 12.0, 90, 1.5, true, 3.0));
    reg.register(weapon(ITEM_OATHBRAND, "Oathbrand", "Etched with binding glyphs along the fuller.", BaseTier::Runic, 30, WeaponClass::Sword,
        DamageType::Slashing, 20.0, 80, 1.5, true, 3.5));

    // --- Maces ---
    reg.register(weapon(ITEM_KNOBWOOD_CLUB, "Knobwood Club", "A heavy branch with a purpose.", BaseTier::Crude, 1, WeaponClass::Mace,
        DamageType::Blunt, 6.0, 130, 1.5, true, 3.5));
    reg.register(weapon(ITEM_IRONBOUND_MAUL, "Ironbound Maul", "Iron bands reinforce each crushing blow.", BaseTier::Tempered, 15, WeaponClass::Mace,
        DamageType::Blunt, 15.0, 120, 1.5, true, 4.5));
    reg.register(weapon(ITEM_CINDERHAMMER, "Cinderhammer", "The head glows faintly, as if freshly quenched.", BaseTier::Runic, 30, WeaponClass::Mace,
        DamageType::Blunt, 25.0, 110, 1.5, true, 5.5));

    // --- Daggers ---
    reg.register(weapon(ITEM_WHITTLED_SHIV, "Whittled Shiv", "Bone handle, crude but quick.", BaseTier::Crude, 1, WeaponClass::Dagger,
        DamageType::Piercing, 3.0, 50, 1.5, true, 0.8));
    reg.register(weapon(ITEM_REAVERS_FANG, "Reaver's Fang", "Hooked blade for close work.", BaseTier::Tempered, 15, WeaponClass::Dagger,
        DamageType::Piercing, 7.0, 45, 1.5, true, 1.0));
    reg.register(weapon(ITEM_WHISPERPOINT, "Whisperpoint", "So thin it parts the air silently.", BaseTier::Runic, 30, WeaponClass::Dagger,
        DamageType::Piercing, 12.0, 40, 1.5, true, 1.0));

    // --- Bows ---
    reg.register(weapon(ITEM_BENT_STAVE, "Bent Stave", "Barely holds tension. Better than nothing.", BaseTier::Crude, 1, WeaponClass::Bow,
        DamageType::Piercing, 4.0, 110, 8.0, false, 1.5));
    reg.register(weapon(ITEM_RANGERS_LONGBOW, "Ranger's Longbow", "Yew and sinew, built for distance.", BaseTier::Tempered, 15, WeaponClass::Bow,
        DamageType::Piercing, 9.0, 100, 10.0, false, 2.0));
    reg.register(weapon(ITEM_GALESTRING, "Galestring", "The string hums with stored force.", BaseTier::Runic, 30, WeaponClass::Bow,
        DamageType::Piercing, 16.0, 90, 12.0, false, 2.0));

    // --- Staves ---
    reg.register(weapon(ITEM_GNARLED_BRANCH, "Gnarled Branch", "Knotted wood, faintly warm to the touch.", BaseTier::Crude, 1, WeaponClass::Staff,
        DamageType::Blunt, 2.0, 110, 1.5, true, 1.5));
    reg.register(weapon(ITEM_WANDERERS_STAFF, "Wanderer's Staff", "Carved with road-signs of forgotten trails.", BaseTier::Tempered, 15, WeaponClass::Staff,
        DamageType::Blunt, 5.0, 100, 1.5, true, 2.0));
    reg.register(weapon(ITEM_SPELLWARDEN, "Spellwarden", "The grain spirals inward, drawing the eye.", BaseTier::Runic, 30, WeaponClass::Staff,
        DamageType::Blunt, 9.0, 90, 1.5, true, 2.5));
}

fn weapon(
    id: ItemId, name: &str, desc: &str, tier: BaseTier, ilvl: u32, class: WeaponClass,
    damage_type: DamageType, base_damage: f32, speed: u32, range: f32, melee: bool, weight: f32,
) -> ItemDef {
    let proj_speed = if melee { 0.0 } else { 12.0 };
    ItemDef {
        id, name: name.into(), description: desc.into(),
        kind: ItemKind::Weapon, rarity: Rarity::Normal,
        weight, max_stack: 1, icon: format!("weapon_{}", name.to_lowercase().replace(' ', "_").replace('\'', "")),
        properties: ItemProperties::Weapon(WeaponDef {
            name: name.into(), damage_type, base_damage,
            attack_speed_ticks: speed, range, projectile_speed: proj_speed, is_melee: melee,
        }),
        base_tier: Some(tier), item_level_req: ilvl,
        weapon_class: Some(class), armor_class: None,
    }
}

// ---------------------------------------------------------------------------
// Armor (6 slots × 3 tiers + 3 shields = 21 armor pieces)
// ---------------------------------------------------------------------------

fn register_armor(reg: &mut ItemRegistry) {
    // --- Head ---
    reg.register(armor_def(ITEM_STITCHED_HOOD, "Stitched Hood", "Scraps of hide sewn into a cap.",
        EquipSlot::Head, BaseTier::Crude, 1, ArmorClass::Light, 1.0, vec![0],
        &[(DamageType::Slashing, 0.05), (DamageType::Piercing, 0.03)]));
    reg.register(armor_def(ITEM_RIVETED_COIF, "Riveted Coif", "Iron rings linked over padded cloth.",
        EquipSlot::Head, BaseTier::Tempered, 15, ArmorClass::Medium, 2.0, vec![0],
        &[(DamageType::Slashing, 0.15), (DamageType::Piercing, 0.10), (DamageType::Blunt, 0.08)]));
    reg.register(armor_def(ITEM_WARDCREST_HELM, "Wardcrest Helm", "Rune-scored steel with a crest of hammered bronze.",
        EquipSlot::Head, BaseTier::Runic, 30, ArmorClass::Heavy, 3.0, vec![0],
        &[(DamageType::Slashing, 0.25), (DamageType::Piercing, 0.20), (DamageType::Blunt, 0.15)]));

    // --- Chest ---
    reg.register(armor_def(ITEM_SCRAPED_HIDE_VEST, "Scraped Hide Vest", "Stiff and poorly tanned. Smells worse.",
        EquipSlot::Chest, BaseTier::Crude, 1, ArmorClass::Light, 2.5, vec![5, 10, 12],
        &[(DamageType::Slashing, 0.08), (DamageType::Piercing, 0.05)]));
    reg.register(armor_def(ITEM_BANDED_HAUBERK, "Banded Hauberk", "Overlapping bands of iron over leather.",
        EquipSlot::Chest, BaseTier::Tempered, 15, ArmorClass::Medium, 4.5, vec![5, 10, 12],
        &[(DamageType::Slashing, 0.20), (DamageType::Piercing, 0.15), (DamageType::Blunt, 0.12)]));
    reg.register(armor_def(ITEM_RUNEWOVEN_CUIRASS, "Runewoven Cuirass", "Plate and chainmail interlaced with silver thread.",
        EquipSlot::Chest, BaseTier::Runic, 30, ArmorClass::Heavy, 6.5, vec![5, 10, 12],
        &[(DamageType::Slashing, 0.30), (DamageType::Piercing, 0.25), (DamageType::Blunt, 0.20)]));

    // --- Hands ---
    reg.register(armor_def(ITEM_FRAYED_WRAPS, "Frayed Wraps", "Strips of linen wound tight.",
        EquipSlot::Hands, BaseTier::Crude, 1, ArmorClass::Light, 0.3, vec![11, 13],
        &[(DamageType::Slashing, 0.04)]));
    reg.register(armor_def(ITEM_PADDED_GRIPS, "Padded Grips", "Leather over wool, reinforced at the knuckles.",
        EquipSlot::Hands, BaseTier::Tempered, 15, ArmorClass::Medium, 0.6, vec![11, 13],
        &[(DamageType::Slashing, 0.10), (DamageType::Blunt, 0.08)]));
    reg.register(armor_def(ITEM_IRONWEAVE_GLOVES, "Ironweave Gloves", "Flexible mail woven into supple hide.",
        EquipSlot::Hands, BaseTier::Runic, 30, ArmorClass::Heavy, 1.0, vec![11, 13],
        &[(DamageType::Slashing, 0.18), (DamageType::Piercing, 0.12), (DamageType::Blunt, 0.10)]));

    // --- Legs ---
    reg.register(armor_def(ITEM_PATCHED_LEGGINGS, "Patched Leggings", "More patch than original cloth.",
        EquipSlot::Legs, BaseTier::Crude, 1, ArmorClass::Light, 1.5, vec![14, 16],
        &[(DamageType::Slashing, 0.05), (DamageType::Piercing, 0.03)]));
    reg.register(armor_def(ITEM_PLATED_GREAVES, "Plated Greaves", "Shin guards of riveted iron.",
        EquipSlot::Legs, BaseTier::Tempered, 15, ArmorClass::Medium, 2.5, vec![14, 16],
        &[(DamageType::Slashing, 0.12), (DamageType::Piercing, 0.08), (DamageType::Blunt, 0.10)]));
    reg.register(armor_def(ITEM_VANGUARD_TASSETS, "Vanguard Tassets", "Articulated plate protecting thigh to knee.",
        EquipSlot::Legs, BaseTier::Runic, 30, ArmorClass::Heavy, 3.5, vec![14, 16],
        &[(DamageType::Slashing, 0.22), (DamageType::Piercing, 0.18), (DamageType::Blunt, 0.15)]));

    // --- Feet ---
    reg.register(armor_def(ITEM_COBBLED_BOOTS, "Cobbled Boots", "Mismatched soles. Better than barefoot.",
        EquipSlot::Feet, BaseTier::Crude, 1, ArmorClass::Light, 1.0, vec![15, 17],
        &[(DamageType::Slashing, 0.04)]));
    reg.register(armor_def(ITEM_STRIDERS_TREADS, "Strider's Treads", "Soft-soled for long marches.",
        EquipSlot::Feet, BaseTier::Tempered, 15, ArmorClass::Medium, 1.5, vec![15, 17],
        &[(DamageType::Slashing, 0.10), (DamageType::Piercing, 0.06)]));
    reg.register(armor_def(ITEM_ASHWALKERS, "Ashwalkers", "Charcoal-grey boots that leave no mark.",
        EquipSlot::Feet, BaseTier::Runic, 30, ArmorClass::Heavy, 2.0, vec![15, 17],
        &[(DamageType::Slashing, 0.16), (DamageType::Piercing, 0.12), (DamageType::Blunt, 0.08)]));

    // --- Shields ---
    reg.register(armor_def(ITEM_SPLINTERED_BUCKLER, "Splintered Buckler", "Held together by stubbornness.",
        EquipSlot::OffHand, BaseTier::Crude, 1, ArmorClass::Shield, 2.5, vec![10, 11],
        &[(DamageType::Slashing, 0.10), (DamageType::Piercing, 0.08), (DamageType::Blunt, 0.05)]));
    reg.register(armor_def(ITEM_BOSSED_KITE_SHIELD, "Bossed Kite Shield", "Iron boss deflects thrusts, kite shape covers the flank.",
        EquipSlot::OffHand, BaseTier::Tempered, 15, ArmorClass::Shield, 4.0, vec![10, 11],
        &[(DamageType::Slashing, 0.20), (DamageType::Piercing, 0.18), (DamageType::Blunt, 0.12)]));
    reg.register(armor_def(ITEM_BULWARK_OF_THE_MARK, "Bulwark of the Mark", "Tower shield branded with a warding sigil.",
        EquipSlot::OffHand, BaseTier::Runic, 30, ArmorClass::Shield, 6.0, vec![10, 11],
        &[(DamageType::Slashing, 0.30), (DamageType::Piercing, 0.25), (DamageType::Blunt, 0.20)]));
}

fn armor_def(
    id: ItemId, name: &str, desc: &str, slot: EquipSlot, tier: BaseTier, ilvl: u32,
    class: ArmorClass, weight: f32, covered_parts: Vec<usize>, resists: &[(DamageType, f32)],
) -> ItemDef {
    let mut resistances = Resistances::default();
    for &(dt, val) in resists {
        resistances.set(dt, val);
    }
    ItemDef {
        id, name: name.into(), description: desc.into(),
        kind: ItemKind::Armor, rarity: Rarity::Normal,
        weight, max_stack: 1, icon: format!("armor_{}", name.to_lowercase().replace(' ', "_").replace('\'', "")),
        properties: ItemProperties::Armor { slot, covered_parts, resistances },
        base_tier: Some(tier), item_level_req: ilvl,
        weapon_class: None, armor_class: Some(class),
    }
}

// ---------------------------------------------------------------------------
// Consumables
// ---------------------------------------------------------------------------

fn register_consumables(reg: &mut ItemRegistry) {
    let consumable = |id, name: &str, desc: &str, effect, weight| ItemDef {
        id, name: name.into(), description: desc.into(),
        kind: ItemKind::Consumable, rarity: Rarity::Normal,
        weight, max_stack: 10, icon: format!("consumable_{}", name.to_lowercase().replace(' ', "_")),
        properties: ItemProperties::Consumable { effect },
        base_tier: None, item_level_req: 0, weapon_class: None, armor_class: None,
    };

    reg.register(consumable(ITEM_HEALTH_POTION, "Health Potion", "Bitter red liquid. Mends flesh.",
        ConsumableEffect::Heal { amount: 25.0 }, 0.5));
    reg.register(consumable(ITEM_MANA_POTION, "Mana Potion", "Shimmering blue draught. Restores arcane reserves.",
        ConsumableEffect::RestoreMana { amount: 30.0 }, 0.5));
    reg.register(consumable(ITEM_STAMINA_POTION, "Stamina Potion", "Green tonic that steadies the limbs.",
        ConsumableEffect::RestoreStamina { amount: 30.0 }, 0.5));
    reg.register(consumable(ITEM_BANDAGE, "Linen Bandage", "Stops bleeding. Won't fix what's broken.",
        ConsumableEffect::Heal { amount: 10.0 }, 0.2));
}

// ---------------------------------------------------------------------------
// Materials
// ---------------------------------------------------------------------------

fn register_materials(reg: &mut ItemRegistry) {
    let material = |id, name: &str, desc: &str, max_stack, weight| ItemDef {
        id, name: name.into(), description: desc.into(),
        kind: ItemKind::Material, rarity: Rarity::Normal,
        weight, max_stack, icon: format!("mat_{}", name.to_lowercase().replace(' ', "_")),
        properties: ItemProperties::Inert,
        base_tier: None, item_level_req: 0, weapon_class: None, armor_class: None,
    };

    reg.register(material(ITEM_BONE_FRAGMENT, "Bone Fragment", "Splintered bone. Useful for crude tools.", 50, 0.1));
    reg.register(material(ITEM_IRON_INGOT, "Iron Ingot", "Smelted bar of pig iron.", 20, 1.0));
    reg.register(material(ITEM_LEATHER_SCRAP, "Leather Scrap", "Rough-cut hide. Can be worked into gear.", 30, 0.3));
    reg.register(material(ITEM_HERB, "Meadow Herb", "Common wildflower with mild restorative properties.", 30, 0.1));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_items_registered() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);

        // 15 weapons + 18 armor (6 slots × 3) + 4 consumables + 4 materials = 41
        let count = reg.all().count();
        assert_eq!(count, 41, "Expected 41 items, got {}", count);
    }

    #[test]
    fn three_tiers_per_weapon_class() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);

        for class in [WeaponClass::Sword, WeaponClass::Mace, WeaponClass::Dagger, WeaponClass::Bow, WeaponClass::Staff] {
            let count = reg.all()
                .filter(|d| d.weapon_class == Some(class))
                .count();
            assert_eq!(count, 3, "{:?} should have 3 tiers, got {}", class, count);
        }
    }

    #[test]
    fn three_tiers_per_armor_slot() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);

        for slot in [EquipSlot::Head, EquipSlot::Chest, EquipSlot::Hands, EquipSlot::Legs, EquipSlot::Feet, EquipSlot::OffHand] {
            let count = reg.all()
                .filter(|d| d.kind == ItemKind::Armor)
                .filter(|d| matches!(&d.properties, ItemProperties::Armor { slot: s, .. } if *s == slot))
                .count();
            assert_eq!(count, 3, "{:?} should have 3 armor tiers, got {}", slot, count);
        }
    }

    #[test]
    fn base_tiers_ordered_by_ilvl() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);

        for def in reg.all().filter(|d| d.base_tier.is_some()) {
            match def.base_tier.unwrap() {
                BaseTier::Crude => assert!(def.item_level_req <= 5),
                BaseTier::Tempered => assert!(def.item_level_req >= 10 && def.item_level_req <= 20),
                BaseTier::Runic => assert!(def.item_level_req >= 25),
            }
        }
    }

    #[test]
    fn consumables_are_stackable() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);
        let potion = reg.get(ITEM_HEALTH_POTION).unwrap();
        assert!(potion.max_stack > 1);
    }

    #[test]
    fn weapons_are_not_stackable() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);
        let sword = reg.get(ITEM_CHIPPED_BLADE).unwrap();
        assert_eq!(sword.max_stack, 1);
    }

    #[test]
    fn armor_body_part_coverage() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);
        let vest = reg.get(ITEM_SCRAPED_HIDE_VEST).unwrap();
        if let ItemProperties::Armor { covered_parts, .. } = &vest.properties {
            assert!(covered_parts.contains(&5));  // Torso
        } else {
            panic!("Should be armor");
        }
    }
}
