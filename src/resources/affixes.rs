use std::collections::HashMap;

use super::damage::DamageType;
use super::items::ItemKind;
use super::stats::AttributeChoice;

// ---------------------------------------------------------------------------
// Affix identity
// ---------------------------------------------------------------------------

/// Unique identifier for an affix definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AffixId(pub u32);

/// Whether an affix occupies a prefix or suffix slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AffixSlotType {
    Prefix,
    Suffix,
}

// ---------------------------------------------------------------------------
// Affix effects — what an affix actually does
// ---------------------------------------------------------------------------

/// The concrete effect of an affix. Extensible — add new variants for new
/// affix types (crit chance, life steal, etc.) without changing existing code.
#[derive(Clone, Debug)]
pub enum AffixEffect {
    /// +flat damage on weapons.
    FlatDamage(f32),
    /// +% damage on weapons (0.15 = +15%).
    PercentDamage(f32),
    /// +resistance to a specific damage type (0.1 = +10%).
    Resistance { damage_type: DamageType, amount: f32 },
    /// +attribute points.
    Attribute { attribute: AttributeChoice, amount: u32 },
    /// Reduce attack cooldown by this many ticks.
    AttackSpeed(u32),
    /// +max mana.
    MaxMana(f32),
    /// +max stamina.
    MaxStamina(f32),
}

// ---------------------------------------------------------------------------
// Affix definition and tiers
// ---------------------------------------------------------------------------

/// One tier of values for an affix. Item level gates which tiers can roll.
#[derive(Clone, Debug)]
pub struct AffixTier {
    /// Minimum item level required to roll this tier.
    pub min_item_level: u32,
    /// The effect with its value for this tier.
    pub effect: AffixEffect,
    /// Display label (e.g., "Keen" prefix or "of the Brute" suffix).
    pub label: String,
}

/// Static definition of an affix type. Registered in the `AffixRegistry`.
#[derive(Clone, Debug)]
pub struct AffixDef {
    pub id: AffixId,
    /// Display group name (e.g., "Flat Damage", "Fire Resistance").
    pub name: String,
    pub slot_type: AffixSlotType,
    /// Which item kinds this affix can appear on.
    pub allowed_kinds: Vec<ItemKind>,
    /// Tiers sorted by min_item_level ascending.
    pub tiers: Vec<AffixTier>,
}

impl AffixDef {
    /// Get the highest tier available for the given item level.
    pub fn best_tier(&self, item_level: u32) -> Option<&AffixTier> {
        self.tiers
            .iter()
            .rev()
            .find(|t| t.min_item_level <= item_level)
    }

    /// Get all tiers available at or below the given item level.
    pub fn available_tiers(&self, item_level: u32) -> Vec<&AffixTier> {
        self.tiers
            .iter()
            .filter(|t| t.min_item_level <= item_level)
            .collect()
    }
}

/// A rolled affix instance on a specific item.
#[derive(Clone, Debug)]
pub struct RolledAffix {
    pub affix_id: AffixId,
    /// Which tier was rolled (index into AffixDef.tiers).
    pub tier_index: usize,
    /// The concrete effect value (copied from the tier).
    pub effect: AffixEffect,
    /// Display label (e.g., "Keen" or "of the Brute").
    pub label: String,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Global registry of all affix definitions.
#[derive(bevy::prelude::Resource, Default)]
pub struct AffixRegistry {
    affixes: HashMap<AffixId, AffixDef>,
}

impl AffixRegistry {
    pub fn register(&mut self, def: AffixDef) {
        self.affixes.insert(def.id, def);
    }

    pub fn get(&self, id: AffixId) -> Option<&AffixDef> {
        self.affixes.get(&id)
    }

    /// Get all affixes valid for a given item kind, slot type, and item level.
    pub fn candidates(
        &self,
        kind: ItemKind,
        slot_type: AffixSlotType,
        item_level: u32,
    ) -> Vec<&AffixDef> {
        self.affixes
            .values()
            .filter(|def| {
                def.slot_type == slot_type
                    && def.allowed_kinds.contains(&kind)
                    && def.best_tier(item_level).is_some()
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_affix() -> AffixDef {
        AffixDef {
            id: AffixId(1),
            name: "Flat Damage".into(),
            slot_type: AffixSlotType::Prefix,
            allowed_kinds: vec![ItemKind::Weapon],
            tiers: vec![
                AffixTier { min_item_level: 1,  effect: AffixEffect::FlatDamage(2.0), label: "Keen".into() },
                AffixTier { min_item_level: 10, effect: AffixEffect::FlatDamage(5.0), label: "Honed".into() },
                AffixTier { min_item_level: 20, effect: AffixEffect::FlatDamage(9.0), label: "Wrathful".into() },
            ],
        }
    }

    #[test]
    fn best_tier_at_ilvl_1() {
        let affix = test_affix();
        let tier = affix.best_tier(1).unwrap();
        assert_eq!(tier.label, "Keen");
    }

    #[test]
    fn best_tier_at_ilvl_15() {
        let affix = test_affix();
        let tier = affix.best_tier(15).unwrap();
        assert_eq!(tier.label, "Honed");
    }

    #[test]
    fn best_tier_at_ilvl_50() {
        let affix = test_affix();
        let tier = affix.best_tier(50).unwrap();
        assert_eq!(tier.label, "Wrathful");
    }

    #[test]
    fn best_tier_below_minimum() {
        let affix = AffixDef {
            id: AffixId(99),
            name: "High Level".into(),
            slot_type: AffixSlotType::Prefix,
            allowed_kinds: vec![ItemKind::Weapon],
            tiers: vec![
                AffixTier { min_item_level: 10, effect: AffixEffect::FlatDamage(5.0), label: "Test".into() },
            ],
        };
        assert!(affix.best_tier(5).is_none());
    }

    #[test]
    fn available_tiers_filters_by_ilvl() {
        let affix = test_affix();
        assert_eq!(affix.available_tiers(1).len(), 1);
        assert_eq!(affix.available_tiers(10).len(), 2);
        assert_eq!(affix.available_tiers(25).len(), 3);
    }

    #[test]
    fn candidates_filters_kind_and_slot() {
        let mut reg = AffixRegistry::default();
        reg.register(test_affix());
        reg.register(AffixDef {
            id: AffixId(2),
            name: "Max Mana".into(),
            slot_type: AffixSlotType::Prefix,
            allowed_kinds: vec![ItemKind::Armor],
            tiers: vec![
                AffixTier { min_item_level: 1, effect: AffixEffect::MaxMana(10.0), label: "Shimmering".into() },
            ],
        });

        let weapon_prefixes = reg.candidates(ItemKind::Weapon, AffixSlotType::Prefix, 10);
        assert_eq!(weapon_prefixes.len(), 1);
        assert_eq!(weapon_prefixes[0].name, "Flat Damage");

        let armor_prefixes = reg.candidates(ItemKind::Armor, AffixSlotType::Prefix, 10);
        assert_eq!(armor_prefixes.len(), 1);
        assert_eq!(armor_prefixes[0].name, "Max Mana");

        let weapon_suffixes = reg.candidates(ItemKind::Weapon, AffixSlotType::Suffix, 10);
        assert_eq!(weapon_suffixes.len(), 0);
    }
}
