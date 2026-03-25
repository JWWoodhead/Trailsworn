use bevy::prelude::*;

use super::affixes::AffixEffect;
use super::damage::Resistances;
use super::items::ItemInstance;
use super::stats::AttributeChoice;

// ---------------------------------------------------------------------------
// Attribute bonuses from equipment
// ---------------------------------------------------------------------------

/// Flat attribute bonuses accumulated from equipped item affixes.
#[derive(Clone, Debug, Default)]
pub struct AttributeBonuses {
    pub strength: u32,
    pub agility: u32,
    pub intellect: u32,
    pub toughness: u32,
    pub willpower: u32,
}

// ---------------------------------------------------------------------------
// EquipmentBonuses component
// ---------------------------------------------------------------------------

/// Aggregated bonuses from all equipped items' affixes. Computed by the
/// `sync_equipment` system whenever equipment changes.
///
/// Weapon-specific bonuses (FlatDamage, PercentDamage, AttackSpeed) are baked
/// directly into `EquippedWeapon` by `sync_equipment`. This component stores
/// bonuses that affect non-weapon systems: attributes, resources, and
/// armor resistances from affixes.
#[derive(Component, Clone, Debug, Default)]
pub struct EquipmentBonuses {
    /// +flat damage from affixes (applied to weapon by sync_equipment).
    pub flat_damage: f32,
    /// +% damage from affixes (applied to weapon by sync_equipment).
    pub percent_damage: f32,
    /// Ticks to subtract from weapon attack speed (applied to weapon by sync_equipment).
    pub attack_speed_reduction: u32,
    /// Resistances accumulated from armor affixes (merged into EquippedArmor by sync_equipment).
    pub resistances: Resistances,
    /// Flat attribute bonuses from affixes (accumulated but not yet applied to combat).
    pub attribute_bonuses: AttributeBonuses,
    /// Bonus max mana from affixes.
    pub max_mana_bonus: f32,
    /// Bonus max stamina from affixes.
    pub max_stamina_bonus: f32,
}

/// Compute aggregated bonuses from a set of equipped item instances.
/// Pure function — no Bevy dependencies, fully testable.
pub fn compute_bonuses(instances: &[&ItemInstance]) -> EquipmentBonuses {
    let mut bonuses = EquipmentBonuses::default();

    for instance in instances {
        for effect in instance.all_effects() {
            match effect {
                AffixEffect::FlatDamage(amount) => bonuses.flat_damage += amount,
                AffixEffect::PercentDamage(amount) => bonuses.percent_damage += amount,
                AffixEffect::AttackSpeed(ticks) => bonuses.attack_speed_reduction += ticks,
                AffixEffect::Resistance { damage_type, amount } => {
                    let current = bonuses.resistances.get(*damage_type);
                    bonuses.resistances.set(*damage_type, current + amount);
                }
                AffixEffect::Attribute { attribute, amount } => {
                    match attribute {
                        AttributeChoice::Strength => bonuses.attribute_bonuses.strength += amount,
                        AttributeChoice::Agility => bonuses.attribute_bonuses.agility += amount,
                        AttributeChoice::Intellect => bonuses.attribute_bonuses.intellect += amount,
                        AttributeChoice::Toughness => bonuses.attribute_bonuses.toughness += amount,
                        AttributeChoice::Willpower => bonuses.attribute_bonuses.willpower += amount,
                    }
                }
                AffixEffect::MaxMana(amount) => bonuses.max_mana_bonus += amount,
                AffixEffect::MaxStamina(amount) => bonuses.max_stamina_bonus += amount,
            }
        }
    }

    bonuses
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::affixes::RolledAffix;
    use crate::resources::damage::DamageType;
    use crate::resources::items::{ItemId, ItemInstanceId, Rarity};

    fn make_instance(prefixes: Vec<RolledAffix>, suffixes: Vec<RolledAffix>) -> ItemInstance {
        ItemInstance {
            id: ItemInstanceId(1),
            base_item_id: ItemId(1),
            rarity: Rarity::Magic,
            item_level: 10,
            prefixes,
            suffixes,
        }
    }

    fn make_affix(effect: AffixEffect) -> RolledAffix {
        RolledAffix {
            affix_id: crate::resources::affixes::AffixId(1),
            tier_index: 0,
            effect,
            label: "Test".into(),
        }
    }

    #[test]
    fn empty_instances_produce_default_bonuses() {
        let bonuses = compute_bonuses(&[]);
        assert_eq!(bonuses.flat_damage, 0.0);
        assert_eq!(bonuses.percent_damage, 0.0);
        assert_eq!(bonuses.attack_speed_reduction, 0);
        assert_eq!(bonuses.max_mana_bonus, 0.0);
        assert_eq!(bonuses.max_stamina_bonus, 0.0);
    }

    #[test]
    fn flat_damage_accumulated() {
        let inst = make_instance(
            vec![make_affix(AffixEffect::FlatDamage(5.0))],
            vec![],
        );
        let bonuses = compute_bonuses(&[&inst]);
        assert!((bonuses.flat_damage - 5.0).abs() < 0.001);
    }

    #[test]
    fn percent_damage_accumulated() {
        let inst = make_instance(
            vec![make_affix(AffixEffect::PercentDamage(0.15))],
            vec![],
        );
        let bonuses = compute_bonuses(&[&inst]);
        assert!((bonuses.percent_damage - 0.15).abs() < 0.001);
    }

    #[test]
    fn attack_speed_accumulated() {
        let inst = make_instance(
            vec![],
            vec![make_affix(AffixEffect::AttackSpeed(10))],
        );
        let bonuses = compute_bonuses(&[&inst]);
        assert_eq!(bonuses.attack_speed_reduction, 10);
    }

    #[test]
    fn resistance_accumulated() {
        let inst = make_instance(
            vec![],
            vec![make_affix(AffixEffect::Resistance {
                damage_type: DamageType::Fire,
                amount: 0.1,
            })],
        );
        let bonuses = compute_bonuses(&[&inst]);
        assert!((bonuses.resistances.get(DamageType::Fire) - 0.1).abs() < 0.001);
        assert_eq!(bonuses.resistances.get(DamageType::Frost), 0.0);
    }

    #[test]
    fn attribute_bonuses_accumulated() {
        let inst = make_instance(
            vec![],
            vec![make_affix(AffixEffect::Attribute {
                attribute: AttributeChoice::Strength,
                amount: 3,
            })],
        );
        let bonuses = compute_bonuses(&[&inst]);
        assert_eq!(bonuses.attribute_bonuses.strength, 3);
        assert_eq!(bonuses.attribute_bonuses.agility, 0);
    }

    #[test]
    fn max_mana_stamina_accumulated() {
        let inst = make_instance(
            vec![make_affix(AffixEffect::MaxMana(25.0))],
            vec![make_affix(AffixEffect::MaxStamina(15.0))],
        );
        let bonuses = compute_bonuses(&[&inst]);
        assert!((bonuses.max_mana_bonus - 25.0).abs() < 0.001);
        assert!((bonuses.max_stamina_bonus - 15.0).abs() < 0.001);
    }

    #[test]
    fn multiple_items_aggregate() {
        let weapon = make_instance(
            vec![make_affix(AffixEffect::FlatDamage(3.0))],
            vec![make_affix(AffixEffect::AttackSpeed(5))],
        );
        let armor = make_instance(
            vec![make_affix(AffixEffect::MaxMana(20.0))],
            vec![make_affix(AffixEffect::Resistance {
                damage_type: DamageType::Fire,
                amount: 0.1,
            })],
        );
        let bonuses = compute_bonuses(&[&weapon, &armor]);
        assert!((bonuses.flat_damage - 3.0).abs() < 0.001);
        assert_eq!(bonuses.attack_speed_reduction, 5);
        assert!((bonuses.max_mana_bonus - 20.0).abs() < 0.001);
        assert!((bonuses.resistances.get(DamageType::Fire) - 0.1).abs() < 0.001);
    }
}
