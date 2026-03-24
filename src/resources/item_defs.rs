use super::damage::{DamageType, Resistances, WeaponDef};
use super::items::*;

// --- Item IDs ---

// Weapons
pub const ITEM_RUSTY_SWORD: ItemId = ItemId(1);
pub const ITEM_IRON_SWORD: ItemId = ItemId(2);
pub const ITEM_IRON_MACE: ItemId = ItemId(3);
pub const ITEM_HUNTING_BOW: ItemId = ItemId(4);
pub const ITEM_IRON_DAGGER: ItemId = ItemId(5);
pub const ITEM_WOODEN_STAFF: ItemId = ItemId(6);

// Armor
pub const ITEM_LEATHER_CAP: ItemId = ItemId(100);
pub const ITEM_LEATHER_VEST: ItemId = ItemId(101);
pub const ITEM_LEATHER_GLOVES: ItemId = ItemId(102);
pub const ITEM_LEATHER_PANTS: ItemId = ItemId(103);
pub const ITEM_LEATHER_BOOTS: ItemId = ItemId(104);
pub const ITEM_IRON_HELM: ItemId = ItemId(110);
pub const ITEM_IRON_CUIRASS: ItemId = ItemId(111);
pub const ITEM_WOODEN_SHIELD: ItemId = ItemId(120);

// Consumables
pub const ITEM_HEALTH_POTION: ItemId = ItemId(200);
pub const ITEM_MANA_POTION: ItemId = ItemId(201);
pub const ITEM_STAMINA_POTION: ItemId = ItemId(202);
pub const ITEM_BANDAGE: ItemId = ItemId(203);

// Materials
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
// Weapons
// ---------------------------------------------------------------------------

fn register_weapons(reg: &mut ItemRegistry) {
    reg.register(ItemDef {
        id: ITEM_RUSTY_SWORD,
        name: "Rusty Sword".into(),
        description: "A chipped blade caked in old blood.".into(),
        kind: ItemKind::Weapon,
        rarity: Rarity::Common,
        weight: 2.5,
        max_stack: 1,
        icon: "sword_rusty".into(),
        properties: ItemProperties::Weapon(WeaponDef {
            name: "Rusty Sword".into(),
            damage_type: DamageType::Slashing,
            base_damage: 5.0,
            attack_speed_ticks: 120,
            range: 1.5,
            projectile_speed: 0.0,
            is_melee: true,
        }),
    });

    reg.register(ItemDef {
        id: ITEM_IRON_SWORD,
        name: "Iron Sword".into(),
        description: "A sturdy blade, well-balanced.".into(),
        kind: ItemKind::Weapon,
        rarity: Rarity::Common,
        weight: 3.0,
        max_stack: 1,
        icon: "sword_iron".into(),
        properties: ItemProperties::Weapon(WeaponDef {
            name: "Iron Sword".into(),
            damage_type: DamageType::Slashing,
            base_damage: 8.0,
            attack_speed_ticks: 90,
            range: 1.5,
            projectile_speed: 0.0,
            is_melee: true,
        }),
    });

    reg.register(ItemDef {
        id: ITEM_IRON_MACE,
        name: "Iron Mace".into(),
        description: "Slow but punishing. Crushes armor.".into(),
        kind: ItemKind::Weapon,
        rarity: Rarity::Common,
        weight: 4.0,
        max_stack: 1,
        icon: "mace_iron".into(),
        properties: ItemProperties::Weapon(WeaponDef {
            name: "Iron Mace".into(),
            damage_type: DamageType::Blunt,
            base_damage: 10.0,
            attack_speed_ticks: 150,
            range: 1.5,
            projectile_speed: 0.0,
            is_melee: true,
        }),
    });

    reg.register(ItemDef {
        id: ITEM_HUNTING_BOW,
        name: "Hunting Bow".into(),
        description: "Simple but reliable at range.".into(),
        kind: ItemKind::Weapon,
        rarity: Rarity::Common,
        weight: 1.5,
        max_stack: 1,
        icon: "bow_hunting".into(),
        properties: ItemProperties::Weapon(WeaponDef {
            name: "Hunting Bow".into(),
            damage_type: DamageType::Piercing,
            base_damage: 6.0,
            attack_speed_ticks: 100,
            range: 8.0,
            projectile_speed: 12.0,
            is_melee: false,
        }),
    });

    reg.register(ItemDef {
        id: ITEM_IRON_DAGGER,
        name: "Iron Dagger".into(),
        description: "Quick and light. Favored by scouts.".into(),
        kind: ItemKind::Weapon,
        rarity: Rarity::Common,
        weight: 1.0,
        max_stack: 1,
        icon: "dagger_iron".into(),
        properties: ItemProperties::Weapon(WeaponDef {
            name: "Iron Dagger".into(),
            damage_type: DamageType::Piercing,
            base_damage: 4.0,
            attack_speed_ticks: 60,
            range: 1.5,
            projectile_speed: 0.0,
            is_melee: true,
        }),
    });

    reg.register(ItemDef {
        id: ITEM_WOODEN_STAFF,
        name: "Wooden Staff".into(),
        description: "A channeling focus for the arcane arts.".into(),
        kind: ItemKind::Weapon,
        rarity: Rarity::Common,
        weight: 2.0,
        max_stack: 1,
        icon: "staff_wooden".into(),
        properties: ItemProperties::Weapon(WeaponDef {
            name: "Wooden Staff".into(),
            damage_type: DamageType::Blunt,
            base_damage: 3.0,
            attack_speed_ticks: 110,
            range: 1.5,
            projectile_speed: 0.0,
            is_melee: true,
        }),
    });
}

// ---------------------------------------------------------------------------
// Armor
// ---------------------------------------------------------------------------

fn register_armor(reg: &mut ItemRegistry) {
    // Leather set — light, small physical resistances
    reg.register(armor_def(
        ITEM_LEATHER_CAP,
        "Leather Cap",
        "Hardened hide shaped into a skullcap.",
        EquipSlot::Head,
        Rarity::Common,
        1.0,
        "armor_leather_cap",
        vec![0], // Head
        &[(DamageType::Slashing, 0.1), (DamageType::Piercing, 0.05)],
    ));

    reg.register(armor_def(
        ITEM_LEATHER_VEST,
        "Leather Vest",
        "Boiled leather over a padded lining.",
        EquipSlot::Chest,
        Rarity::Common,
        3.0,
        "armor_leather_vest",
        vec![5, 10, 12], // Torso, L.Arm, R.Arm
        &[(DamageType::Slashing, 0.15), (DamageType::Piercing, 0.1)],
    ));

    reg.register(armor_def(
        ITEM_LEATHER_GLOVES,
        "Leather Gloves",
        "Supple hide with reinforced knuckles.",
        EquipSlot::Hands,
        Rarity::Common,
        0.5,
        "armor_leather_gloves",
        vec![11, 13], // L.Hand, R.Hand
        &[(DamageType::Slashing, 0.1)],
    ));

    reg.register(armor_def(
        ITEM_LEATHER_PANTS,
        "Leather Pants",
        "Thick hide legwear. Stiff but protective.",
        EquipSlot::Legs,
        Rarity::Common,
        2.0,
        "armor_leather_pants",
        vec![14, 16], // L.Leg, R.Leg
        &[(DamageType::Slashing, 0.1), (DamageType::Piercing, 0.05)],
    ));

    reg.register(armor_def(
        ITEM_LEATHER_BOOTS,
        "Leather Boots",
        "Worn but reliable footwear.",
        EquipSlot::Feet,
        Rarity::Common,
        1.0,
        "armor_leather_boots",
        vec![15, 17], // L.Foot, R.Foot
        &[(DamageType::Slashing, 0.1)],
    ));

    // Iron pieces — heavier, better physical resistance
    reg.register(armor_def(
        ITEM_IRON_HELM,
        "Iron Helm",
        "A dented helm. Still stops a blade.",
        EquipSlot::Head,
        Rarity::Uncommon,
        2.5,
        "armor_iron_helm",
        vec![0], // Head
        &[
            (DamageType::Slashing, 0.25),
            (DamageType::Piercing, 0.2),
            (DamageType::Blunt, 0.15),
        ],
    ));

    reg.register(armor_def(
        ITEM_IRON_CUIRASS,
        "Iron Cuirass",
        "Plate over chain. Heavy but formidable.",
        EquipSlot::Chest,
        Rarity::Uncommon,
        6.0,
        "armor_iron_cuirass",
        vec![5, 10, 12], // Torso, L.Arm, R.Arm
        &[
            (DamageType::Slashing, 0.3),
            (DamageType::Piercing, 0.25),
            (DamageType::Blunt, 0.2),
        ],
    ));

    reg.register(armor_def(
        ITEM_WOODEN_SHIELD,
        "Wooden Shield",
        "Splintered around the edges but still holds.",
        EquipSlot::OffHand,
        Rarity::Common,
        3.5,
        "shield_wooden",
        vec![10, 11], // L.Arm, L.Hand (shield arm)
        &[
            (DamageType::Slashing, 0.2),
            (DamageType::Piercing, 0.15),
            (DamageType::Blunt, 0.1),
        ],
    ));
}

/// Helper to build armor ItemDefs without repeating boilerplate.
fn armor_def(
    id: ItemId,
    name: &str,
    description: &str,
    slot: EquipSlot,
    rarity: Rarity,
    weight: f32,
    icon: &str,
    covered_parts: Vec<usize>,
    resists: &[(DamageType, f32)],
) -> ItemDef {
    let mut resistances = Resistances::default();
    for &(dt, val) in resists {
        resistances.set(dt, val);
    }
    ItemDef {
        id,
        name: name.into(),
        description: description.into(),
        kind: ItemKind::Armor,
        rarity,
        weight,
        max_stack: 1,
        icon: icon.into(),
        properties: ItemProperties::Armor {
            slot,
            covered_parts,
            resistances,
        },
    }
}

// ---------------------------------------------------------------------------
// Consumables
// ---------------------------------------------------------------------------

fn register_consumables(reg: &mut ItemRegistry) {
    reg.register(ItemDef {
        id: ITEM_HEALTH_POTION,
        name: "Health Potion".into(),
        description: "Bitter red liquid. Mends flesh.".into(),
        kind: ItemKind::Consumable,
        rarity: Rarity::Common,
        weight: 0.5,
        max_stack: 10,
        icon: "potion_red".into(),
        properties: ItemProperties::Consumable {
            effect: ConsumableEffect::Heal { amount: 25.0 },
        },
    });

    reg.register(ItemDef {
        id: ITEM_MANA_POTION,
        name: "Mana Potion".into(),
        description: "Shimmering blue draught. Restores arcane reserves.".into(),
        kind: ItemKind::Consumable,
        rarity: Rarity::Common,
        weight: 0.5,
        max_stack: 10,
        icon: "potion_blue".into(),
        properties: ItemProperties::Consumable {
            effect: ConsumableEffect::RestoreMana { amount: 30.0 },
        },
    });

    reg.register(ItemDef {
        id: ITEM_STAMINA_POTION,
        name: "Stamina Potion".into(),
        description: "Green tonic that steadies the limbs.".into(),
        kind: ItemKind::Consumable,
        rarity: Rarity::Common,
        weight: 0.5,
        max_stack: 10,
        icon: "potion_green".into(),
        properties: ItemProperties::Consumable {
            effect: ConsumableEffect::RestoreStamina { amount: 30.0 },
        },
    });

    reg.register(ItemDef {
        id: ITEM_BANDAGE,
        name: "Linen Bandage".into(),
        description: "Stops bleeding. Won't fix what's broken.".into(),
        kind: ItemKind::Consumable,
        rarity: Rarity::Common,
        weight: 0.2,
        max_stack: 20,
        icon: "bandage".into(),
        properties: ItemProperties::Consumable {
            effect: ConsumableEffect::Heal { amount: 10.0 },
        },
    });
}

// ---------------------------------------------------------------------------
// Materials
// ---------------------------------------------------------------------------

fn register_materials(reg: &mut ItemRegistry) {
    reg.register(ItemDef {
        id: ITEM_BONE_FRAGMENT,
        name: "Bone Fragment".into(),
        description: "Splintered bone. Useful for crude tools.".into(),
        kind: ItemKind::Material,
        rarity: Rarity::Common,
        weight: 0.1,
        max_stack: 50,
        icon: "mat_bone".into(),
        properties: ItemProperties::Inert,
    });

    reg.register(ItemDef {
        id: ITEM_IRON_INGOT,
        name: "Iron Ingot".into(),
        description: "Smelted bar of pig iron. Foundation of arms and armor.".into(),
        kind: ItemKind::Material,
        rarity: Rarity::Common,
        weight: 1.0,
        max_stack: 20,
        icon: "mat_iron".into(),
        properties: ItemProperties::Inert,
    });

    reg.register(ItemDef {
        id: ITEM_LEATHER_SCRAP,
        name: "Leather Scrap".into(),
        description: "Rough-cut hide. Can be worked into gear.".into(),
        kind: ItemKind::Material,
        rarity: Rarity::Common,
        weight: 0.3,
        max_stack: 30,
        icon: "mat_leather".into(),
        properties: ItemProperties::Inert,
    });

    reg.register(ItemDef {
        id: ITEM_HERB,
        name: "Meadow Herb".into(),
        description: "Common wildflower with mild restorative properties.".into(),
        kind: ItemKind::Material,
        rarity: Rarity::Common,
        weight: 0.1,
        max_stack: 30,
        icon: "mat_herb".into(),
        properties: ItemProperties::Inert,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_items_registered() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);

        // Weapons (6)
        assert!(reg.get(ITEM_RUSTY_SWORD).is_some());
        assert!(reg.get(ITEM_IRON_SWORD).is_some());
        assert!(reg.get(ITEM_IRON_MACE).is_some());
        assert!(reg.get(ITEM_HUNTING_BOW).is_some());
        assert!(reg.get(ITEM_IRON_DAGGER).is_some());
        assert!(reg.get(ITEM_WOODEN_STAFF).is_some());

        // Armor (8)
        assert!(reg.get(ITEM_LEATHER_CAP).is_some());
        assert!(reg.get(ITEM_LEATHER_VEST).is_some());
        assert!(reg.get(ITEM_LEATHER_GLOVES).is_some());
        assert!(reg.get(ITEM_LEATHER_PANTS).is_some());
        assert!(reg.get(ITEM_LEATHER_BOOTS).is_some());
        assert!(reg.get(ITEM_IRON_HELM).is_some());
        assert!(reg.get(ITEM_IRON_CUIRASS).is_some());
        assert!(reg.get(ITEM_WOODEN_SHIELD).is_some());

        // Consumables (4)
        assert!(reg.get(ITEM_HEALTH_POTION).is_some());
        assert!(reg.get(ITEM_MANA_POTION).is_some());
        assert!(reg.get(ITEM_STAMINA_POTION).is_some());
        assert!(reg.get(ITEM_BANDAGE).is_some());

        // Materials (4)
        assert!(reg.get(ITEM_BONE_FRAGMENT).is_some());
        assert!(reg.get(ITEM_IRON_INGOT).is_some());
        assert!(reg.get(ITEM_LEATHER_SCRAP).is_some());
        assert!(reg.get(ITEM_HERB).is_some());
    }

    #[test]
    fn rusty_sword_matches_existing_weapon() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);
        let item = reg.get(ITEM_RUSTY_SWORD).unwrap();
        if let ItemProperties::Weapon(w) = &item.properties {
            assert_eq!(w.base_damage, 5.0);
            assert_eq!(w.attack_speed_ticks, 120);
            assert!(w.is_melee);
        } else {
            panic!("Rusty Sword should be a weapon");
        }
    }

    #[test]
    fn armor_body_part_coverage() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);
        let vest = reg.get(ITEM_LEATHER_VEST).unwrap();
        if let ItemProperties::Armor { covered_parts, .. } = &vest.properties {
            assert!(covered_parts.contains(&5));  // Torso
            assert!(covered_parts.contains(&10)); // L.Arm
            assert!(covered_parts.contains(&12)); // R.Arm
        } else {
            panic!("Leather Vest should be armor");
        }
    }

    #[test]
    fn consumables_are_stackable() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);
        let potion = reg.get(ITEM_HEALTH_POTION).unwrap();
        assert!(potion.max_stack > 1);
        let bandage = reg.get(ITEM_BANDAGE).unwrap();
        assert!(bandage.max_stack > 1);
    }

    #[test]
    fn weapons_are_not_stackable() {
        let mut reg = ItemRegistry::default();
        register_starter_items(&mut reg);
        let sword = reg.get(ITEM_IRON_SWORD).unwrap();
        assert_eq!(sword.max_stack, 1);
    }
}
