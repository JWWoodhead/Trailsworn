use bevy::prelude::*;

use crate::resources::abilities::{Mana, Stamina};
use crate::resources::damage::{ArmorPiece, EquippedArmor, EquippedWeapon};
use crate::resources::equipment_bonuses::{compute_bonuses, EquipmentBonuses};
use crate::resources::items::{Equipment, ItemInstanceRegistry, ItemProperties, ItemRegistry};

/// Syncs `EquippedWeapon`, `EquippedArmor`, and `EquipmentBonuses` from the
/// item instances referenced by the `Equipment` component.
///
/// Runs whenever `Equipment` changes (equip/unequip). Bakes weapon affixes
/// (FlatDamage, PercentDamage, AttackSpeed) directly into the `WeaponDef`
/// stored in `EquippedWeapon`. Builds `ArmorPiece` entries from equipped
/// armor items. Computes `EquipmentBonuses` for non-weapon bonuses.
pub fn sync_equipment(
    item_registry: Res<ItemRegistry>,
    instance_registry: Res<ItemInstanceRegistry>,
    mut query: Query<
        (
            &Equipment,
            &mut EquippedWeapon,
            &mut EquippedArmor,
            &mut EquipmentBonuses,
            Option<&mut Mana>,
            Option<&mut Stamina>,
        ),
        Changed<Equipment>,
    >,
) {
    for (equipment, mut equipped_weapon, mut equipped_armor, mut bonuses, mana, stamina) in &mut query {
        // Collect all equipped instances for bonus computation
        let equipped_instances: Vec<_> = equipment
            .slots
            .values()
            .filter_map(|&id| instance_registry.get(id))
            .collect();

        // Compute aggregate bonuses from all affixes
        *bonuses = compute_bonuses(&equipped_instances);

        // --- Sync EquippedWeapon from MainHand slot ---
        if let Some(&instance_id) = equipment.slots.get(&crate::resources::items::EquipSlot::MainHand) {
            if let Some(instance) = instance_registry.get(instance_id) {
                if let Some(item_def) = item_registry.get(instance.base_item_id) {
                    if let ItemProperties::Weapon(base_weapon) = &item_def.properties {
                        // Start from base weapon stats
                        let mut weapon = base_weapon.clone();

                        // Bake in weapon affixes
                        weapon.base_damage += bonuses.flat_damage;
                        weapon.base_damage *= 1.0 + bonuses.percent_damage;
                        weapon.attack_speed_ticks = weapon
                            .attack_speed_ticks
                            .saturating_sub(bonuses.attack_speed_reduction);

                        // Preserve cooldown state
                        let cooldown = equipped_weapon.cooldown_remaining;
                        equipped_weapon.weapon = weapon;
                        equipped_weapon.cooldown_remaining = cooldown;
                    }
                }
            }
        }

        // --- Sync EquippedArmor from armor slots ---
        let mut pieces = Vec::new();
        for (&slot, &instance_id) in &equipment.slots {
            // Skip MainHand/OffHand — those are weapons
            if slot == crate::resources::items::EquipSlot::MainHand
                || slot == crate::resources::items::EquipSlot::OffHand
            {
                continue;
            }

            if let Some(instance) = instance_registry.get(instance_id) {
                if let Some(item_def) = item_registry.get(instance.base_item_id) {
                    if let ItemProperties::Armor {
                        covered_parts,
                        resistances: base_resistances,
                        ..
                    } = &item_def.properties
                    {
                        // Start from base armor resistances
                        let mut resistances = base_resistances.clone();

                        // Add resistance affixes from this specific item
                        for effect in instance.all_effects() {
                            if let crate::resources::affixes::AffixEffect::Resistance {
                                damage_type,
                                amount,
                            } = effect
                            {
                                let current = resistances.get(*damage_type);
                                resistances.set(*damage_type, current + amount);
                            }
                        }

                        pieces.push(ArmorPiece {
                            name: item_def.name.clone(),
                            covered_parts: covered_parts.clone(),
                            resistances,
                        });
                    }
                }
            }
        }
        equipped_armor.pieces = pieces;

        // --- Sync Mana/Stamina max from bonuses ---
        if let Some(mut mana) = mana {
            mana.max = mana.base_max + bonuses.max_mana_bonus;
            mana.current = mana.current.min(mana.max);
        }
        if let Some(mut stamina) = stamina {
            stamina.max = stamina.base_max + bonuses.max_stamina_bonus;
            stamina.current = stamina.current.min(stamina.max);
        }
    }
}
