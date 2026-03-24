use bevy::prelude::*;
use std::collections::HashMap;

use super::damage::{Resistances, WeaponDef};

// ---------------------------------------------------------------------------
// Item identity
// ---------------------------------------------------------------------------

/// Unique identifier for an item definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

/// Item rarity — determines UI border color and drop weighting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Rarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Legendary,
}

/// Broad item category — determines inventory tab and behavior.
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
// Item instance (in the world)
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
                false // remove this stack entirely
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
            rarity: Rarity::Common,
            weight: 0.5,
            max_stack: 10,
            icon: "potion_red".into(),
            properties: ItemProperties::Consumable {
                effect: ConsumableEffect::Heal { amount: 20.0 },
            },
        });
        reg.register(ItemDef {
            id: ItemId(2),
            name: "Iron Sword".into(),
            description: "A sturdy blade.".into(),
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
            id: ItemId(3),
            name: "Bone Fragment".into(),
            description: "Crafting material.".into(),
            kind: ItemKind::Material,
            rarity: Rarity::Common,
            weight: 0.1,
            max_stack: 50,
            icon: "bone".into(),
            properties: ItemProperties::Inert,
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
        assert_eq!(inv.items.len(), 1); // single stack
    }

    #[test]
    fn add_stacks_into_existing() {
        let reg = test_registry();
        let mut inv = Inventory::new(10);
        inv.add(ItemId(1), 7, &reg);
        inv.add(ItemId(1), 5, &reg); // 7+5=12, max_stack=10 → 10+2
        assert_eq!(inv.count(ItemId(1)), 12);
        assert_eq!(inv.items.len(), 2); // two stacks
    }

    #[test]
    fn add_non_stackable() {
        let reg = test_registry();
        let mut inv = Inventory::new(5);
        let leftover = inv.add(ItemId(2), 3, &reg); // max_stack=1
        assert_eq!(leftover, 0);
        assert_eq!(inv.items.len(), 3); // 3 separate stacks of 1
    }

    #[test]
    fn add_respects_capacity() {
        let reg = test_registry();
        let mut inv = Inventory::new(2);
        let leftover = inv.add(ItemId(2), 5, &reg); // max_stack=1, cap=2
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
        inv.add(ItemId(1), 4, &reg); // 4 × 0.5 = 2.0
        inv.add(ItemId(2), 1, &reg); // 1 × 3.0 = 3.0
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
        assert_eq!(prev, Some(ItemId(2))); // swapped out

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
}
