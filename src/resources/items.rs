use bevy::prelude::*;
use std::collections::HashMap;

use super::affixes::RolledAffix;
use super::damage::{Resistances, WeaponDef};

// ---------------------------------------------------------------------------
// Item identity
// ---------------------------------------------------------------------------

/// Unique identifier for an item definition (base type template).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

/// Item rarity — determines affix count and UI color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Rarity {
    #[default]
    Normal,     // 0 affixes
    Magic,      // 1-2 affixes (max 1 prefix + 1 suffix)
    Rare,       // 3-6 affixes (max 3 prefix + 3 suffix)
    Unique,     // Fixed affixes (future)
}

/// Broad item category — determines inventory tab, affix pool, and behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Weapon,
    Armor,
    Consumable,
    Material,
    Quest,
}

/// Which body slot an equippable item occupies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquipSlot {
    Head,
    Chest,
    Hands,
    Legs,
    Feet,
    MainHand,
    OffHand,
}

/// Craftsmanship quality tier for base item types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BaseTier {
    Crude,      // Starting gear
    Tempered,   // Mid-game
    Runic,      // End-game
}

/// Weapon archetype — groups base types across tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponClass {
    Sword,
    Mace,
    Dagger,
    Bow,
    Staff,
}

/// Armor archetype — groups base types across tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArmorClass {
    Light,
    Medium,
    Heavy,
    Shield,
}

// ---------------------------------------------------------------------------
// Item definition (template)
// ---------------------------------------------------------------------------

/// Static definition of an item type. Lives in the `ItemRegistry`.
#[derive(Clone, Debug)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: String,
    pub description: String,
    pub kind: ItemKind,
    pub rarity: Rarity,
    /// Weight per unit. Contributes to encumbrance.
    pub weight: f32,
    /// Maximum stack size. 1 = not stackable.
    pub max_stack: u32,
    /// Sprite index or asset path for rendering.
    pub icon: String,
    /// What this item does when equipped or used.
    pub properties: ItemProperties,
    /// Craftsmanship tier (Crude/Tempered/Runic). None for non-equipment.
    pub base_tier: Option<BaseTier>,
    /// Minimum item level for this base type to drop.
    pub item_level_req: u32,
    /// Weapon archetype (for generation picking).
    pub weapon_class: Option<WeaponClass>,
    /// Armor archetype (for generation picking).
    pub armor_class: Option<ArmorClass>,
}

/// Type-specific data attached to an item definition.
#[derive(Clone, Debug)]
pub enum ItemProperties {
    /// Weapon items generate a `WeaponDef` when equipped.
    Weapon(WeaponDef),
    /// Armor items provide resistances to specific body part indices.
    Armor {
        slot: EquipSlot,
        covered_parts: Vec<usize>,
        resistances: Resistances,
    },
    /// Consumables apply an effect when used.
    Consumable {
        effect: ConsumableEffect,
    },
    /// No special properties (materials, quest items).
    Inert,
}

/// What happens when a consumable is used.
#[derive(Clone, Debug)]
pub enum ConsumableEffect {
    /// Heal distributed across all body parts.
    Heal { amount: f32 },
    /// Restore mana.
    RestoreMana { amount: f32 },
    /// Restore stamina.
    RestoreStamina { amount: f32 },
    /// Apply a status effect by id.
    ApplyStatus { status_id: u32, duration_ticks: u32 },
}

// ---------------------------------------------------------------------------
// Item instance (rolled item with affixes)
// ---------------------------------------------------------------------------

/// Unique identifier for a specific rolled item instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemInstanceId(pub u64);

/// A specific rolled item with affixes. Each dropped equipment item is unique.
#[derive(Clone, Debug)]
pub struct ItemInstance {
    pub id: ItemInstanceId,
    /// Which base item template this was rolled from.
    pub base_item_id: ItemId,
    /// The rarity that was rolled (determines affix count).
    pub rarity: Rarity,
    /// The item level this was generated at.
    pub item_level: u32,
    /// Rolled prefix affixes.
    pub prefixes: Vec<RolledAffix>,
    /// Rolled suffix affixes.
    pub suffixes: Vec<RolledAffix>,
}

impl ItemInstance {
    /// Generated display name: "Prefix BaseType Suffix"
    pub fn display_name(&self, item_registry: &ItemRegistry) -> String {
        let base_name = item_registry
            .get(self.base_item_id)
            .map(|d| d.name.as_str())
            .unwrap_or("Unknown");

        let prefix_label = self.prefixes.first().map(|a| a.label.as_str());
        let suffix_label = self.suffixes.first().map(|a| a.label.as_str());

        match (prefix_label, suffix_label) {
            (Some(pre), Some(suf)) => format!("{} {} {}", pre, base_name, suf),
            (Some(pre), None) => format!("{} {}", pre, base_name),
            (None, Some(suf)) => format!("{} {}", base_name, suf),
            (None, None) => base_name.to_string(),
        }
    }

    /// Get all affix effects as a flat iterator.
    pub fn all_effects(&self) -> impl Iterator<Item = &super::affixes::AffixEffect> {
        self.prefixes.iter().chain(self.suffixes.iter()).map(|a| &a.effect)
    }
}

/// Global registry of all live item instances in the game.
#[derive(Resource, Default)]
pub struct ItemInstanceRegistry {
    instances: HashMap<ItemInstanceId, ItemInstance>,
    next_id: u64,
}

impl ItemInstanceRegistry {
    pub fn next_id(&mut self) -> ItemInstanceId {
        self.next_id += 1;
        ItemInstanceId(self.next_id)
    }

    pub fn insert(&mut self, instance: ItemInstance) -> ItemInstanceId {
        let id = instance.id;
        self.instances.insert(id, instance);
        id
    }

    pub fn get(&self, id: ItemInstanceId) -> Option<&ItemInstance> {
        self.instances.get(&id)
    }

    pub fn remove(&mut self, id: ItemInstanceId) -> Option<ItemInstance> {
        self.instances.remove(&id)
    }
}

// ---------------------------------------------------------------------------
// Stackable item reference
// ---------------------------------------------------------------------------

/// A specific item instance in an inventory or on the ground.
/// References an `ItemDef` by id for shared data.
#[derive(Clone, Debug)]
pub struct ItemStack {
    pub item_id: ItemId,
    pub count: u32,
}

impl ItemStack {
    pub fn new(item_id: ItemId, count: u32) -> Self {
        Self { item_id, count }
    }

    pub fn single(item_id: ItemId) -> Self {
        Self { item_id, count: 1 }
    }
}

// ---------------------------------------------------------------------------
// Inventory component
// ---------------------------------------------------------------------------

/// Bag of items carried by an entity.
#[derive(Component, Clone, Debug, Default)]
pub struct Inventory {
    pub items: Vec<ItemStack>,
    /// Maximum number of distinct stacks (slots).
    pub capacity: usize,
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: Vec::new(),
            capacity,
        }
    }

    /// Try to add items. Stacks with existing matching items first, then uses
    /// empty slots. Returns the leftover count that didn't fit (0 = all added).
    pub fn add(&mut self, item_id: ItemId, mut count: u32, registry: &ItemRegistry) -> u32 {
        let max_stack = registry
            .get(item_id)
            .map(|def| def.max_stack)
            .unwrap_or(1);

        // Stack into existing slots first
        for stack in &mut self.items {
            if stack.item_id == item_id && stack.count < max_stack {
                let space = max_stack - stack.count;
                let added = count.min(space);
                stack.count += added;
                count -= added;
                if count == 0 {
                    return 0;
                }
            }
        }

        // Fill new slots
        while count > 0 && self.items.len() < self.capacity {
            let added = count.min(max_stack);
            self.items.push(ItemStack::new(item_id, added));
            count -= added;
        }

        count
    }

    /// Remove up to `count` of an item. Returns actual amount removed.
    pub fn remove(&mut self, item_id: ItemId, mut count: u32) -> u32 {
        let mut removed = 0;
        self.items.retain_mut(|stack| {
            if stack.item_id != item_id || count == 0 {
                return true;
            }
            if stack.count <= count {
                count -= stack.count;
                removed += stack.count;
                false
            } else {
                stack.count -= count;
                removed += count;
                count = 0;
                true
            }
        });
        removed
    }

    /// Count total of an item across all stacks.
    pub fn count(&self, item_id: ItemId) -> u32 {
        self.items
            .iter()
            .filter(|s| s.item_id == item_id)
            .map(|s| s.count)
            .sum()
    }

    /// Total weight of all items.
    pub fn total_weight(&self, registry: &ItemRegistry) -> f32 {
        self.items
            .iter()
            .map(|stack| {
                registry
                    .get(stack.item_id)
                    .map(|def| def.weight * stack.count as f32)
                    .unwrap_or(0.0)
            })
            .sum()
    }
}

// ---------------------------------------------------------------------------
// Equipment component
// ---------------------------------------------------------------------------

/// Currently equipped items, keyed by slot.
#[derive(Component, Clone, Debug, Default)]
pub struct Equipment {
    pub slots: HashMap<EquipSlot, ItemId>,
}

impl Equipment {
    /// Equip an item in its slot. Returns the previously equipped item id, if any.
    pub fn equip(&mut self, slot: EquipSlot, item_id: ItemId) -> Option<ItemId> {
        self.slots.insert(slot, item_id)
    }

    /// Unequip an item from a slot. Returns the item id if something was there.
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<ItemId> {
        self.slots.remove(&slot)
    }

    pub fn in_slot(&self, slot: EquipSlot) -> Option<ItemId> {
        self.slots.get(&slot).copied()
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Global registry of all item definitions.
#[derive(Resource, Default)]
pub struct ItemRegistry {
    items: HashMap<ItemId, ItemDef>,
}

impl ItemRegistry {
    pub fn register(&mut self, def: ItemDef) {
        self.items.insert(def.id, def);
    }

    pub fn get(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(&id)
    }

    /// Iterate over all registered item definitions.
    pub fn all(&self) -> impl Iterator<Item = &ItemDef> {
        self.items.values()
    }
}

// ---------------------------------------------------------------------------
// Rarity helpers
// ---------------------------------------------------------------------------

/// Max prefixes and suffixes for a given rarity.
pub fn max_affixes_per_slot(rarity: Rarity) -> (u32, u32) {
    match rarity {
        Rarity::Normal => (0, 0),
        Rarity::Magic => (1, 1),
        Rarity::Rare => (3, 3),
        Rarity::Unique => (0, 0), // fixed, not rolled
    }
}

/// Total affix count range for a given rarity.
pub fn affix_count_range(rarity: Rarity) -> (u32, u32) {
    match rarity {
        Rarity::Normal => (0, 0),
        Rarity::Magic => (1, 2),
        Rarity::Rare => (3, 6),
        Rarity::Unique => (0, 0),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::damage::DamageType;

    fn test_registry() -> ItemRegistry {
        let mut reg = ItemRegistry::default();
        reg.register(ItemDef {
            id: ItemId(1),
            name: "Health Potion".into(),
            description: "Restores health.".into(),
            kind: ItemKind::Consumable,
            rarity: Rarity::Normal,
            weight: 0.5,
            max_stack: 10,
            icon: "potion_red".into(),
            properties: ItemProperties::Consumable {
                effect: ConsumableEffect::Heal { amount: 20.0 },
            },
            base_tier: None,
            item_level_req: 0,
            weapon_class: None,
            armor_class: None,
        });
        reg.register(ItemDef {
            id: ItemId(2),
            name: "Iron Sword".into(),
            description: "A sturdy blade.".into(),
            kind: ItemKind::Weapon,
            rarity: Rarity::Normal,
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
            base_tier: Some(BaseTier::Crude),
            item_level_req: 1,
            weapon_class: Some(WeaponClass::Sword),
            armor_class: None,
        });
        reg.register(ItemDef {
            id: ItemId(3),
            name: "Bone Fragment".into(),
            description: "Crafting material.".into(),
            kind: ItemKind::Material,
            rarity: Rarity::Normal,
            weight: 0.1,
            max_stack: 50,
            icon: "bone".into(),
            properties: ItemProperties::Inert,
            base_tier: None,
            item_level_req: 0,
            weapon_class: None,
            armor_class: None,
        });
        reg
    }

    #[test]
    fn add_stackable_items() {
        let reg = test_registry();
        let mut inv = Inventory::new(10);
        let leftover = inv.add(ItemId(1), 5, &reg);
        assert_eq!(leftover, 0);
        assert_eq!(inv.count(ItemId(1)), 5);
        assert_eq!(inv.items.len(), 1);
    }

    #[test]
    fn add_stacks_into_existing() {
        let reg = test_registry();
        let mut inv = Inventory::new(10);
        inv.add(ItemId(1), 7, &reg);
        inv.add(ItemId(1), 5, &reg);
        assert_eq!(inv.count(ItemId(1)), 12);
        assert_eq!(inv.items.len(), 2);
    }

    #[test]
    fn add_non_stackable() {
        let reg = test_registry();
        let mut inv = Inventory::new(5);
        let leftover = inv.add(ItemId(2), 3, &reg);
        assert_eq!(leftover, 0);
        assert_eq!(inv.items.len(), 3);
    }

    #[test]
    fn add_respects_capacity() {
        let reg = test_registry();
        let mut inv = Inventory::new(2);
        let leftover = inv.add(ItemId(2), 5, &reg);
        assert_eq!(leftover, 3);
        assert_eq!(inv.items.len(), 2);
    }

    #[test]
    fn remove_partial_stack() {
        let reg = test_registry();
        let mut inv = Inventory::new(10);
        inv.add(ItemId(1), 8, &reg);
        let removed = inv.remove(ItemId(1), 3);
        assert_eq!(removed, 3);
        assert_eq!(inv.count(ItemId(1)), 5);
    }

    #[test]
    fn remove_entire_stack() {
        let reg = test_registry();
        let mut inv = Inventory::new(10);
        inv.add(ItemId(1), 5, &reg);
        let removed = inv.remove(ItemId(1), 5);
        assert_eq!(removed, 5);
        assert_eq!(inv.items.len(), 0);
    }

    #[test]
    fn remove_more_than_available() {
        let reg = test_registry();
        let mut inv = Inventory::new(10);
        inv.add(ItemId(1), 3, &reg);
        let removed = inv.remove(ItemId(1), 10);
        assert_eq!(removed, 3);
        assert_eq!(inv.items.len(), 0);
    }

    #[test]
    fn total_weight() {
        let reg = test_registry();
        let mut inv = Inventory::new(10);
        inv.add(ItemId(1), 4, &reg);
        inv.add(ItemId(2), 1, &reg);
        let w = inv.total_weight(&reg);
        assert!((w - 5.0).abs() < 0.001);
    }

    #[test]
    fn equip_and_unequip() {
        let mut eq = Equipment::default();
        assert!(eq.in_slot(EquipSlot::MainHand).is_none());

        let prev = eq.equip(EquipSlot::MainHand, ItemId(2));
        assert!(prev.is_none());
        assert_eq!(eq.in_slot(EquipSlot::MainHand), Some(ItemId(2)));

        let prev = eq.equip(EquipSlot::MainHand, ItemId(5));
        assert_eq!(prev, Some(ItemId(2)));

        let removed = eq.unequip(EquipSlot::MainHand);
        assert_eq!(removed, Some(ItemId(5)));
        assert!(eq.in_slot(EquipSlot::MainHand).is_none());
    }

    #[test]
    fn registry_lookup() {
        let reg = test_registry();
        let sword = reg.get(ItemId(2)).unwrap();
        assert_eq!(sword.name, "Iron Sword");
        assert_eq!(sword.kind, ItemKind::Weapon);
        assert!(reg.get(ItemId(999)).is_none());
    }

    #[test]
    fn registry_all_iterates() {
        let reg = test_registry();
        assert_eq!(reg.all().count(), 3);
    }

    #[test]
    fn affix_counts_by_rarity() {
        assert_eq!(affix_count_range(Rarity::Normal), (0, 0));
        assert_eq!(affix_count_range(Rarity::Magic), (1, 2));
        assert_eq!(affix_count_range(Rarity::Rare), (3, 6));
        assert_eq!(max_affixes_per_slot(Rarity::Magic), (1, 1));
        assert_eq!(max_affixes_per_slot(Rarity::Rare), (3, 3));
    }
}
