use bevy::prelude::*;

use super::body::{Body, BodyTemplate};
use super::damage::{DamageType, EquippedArmor};
use super::stats::Attributes;

/// Result of a hit resolution attempt.
#[derive(Clone, Debug)]
pub enum HitResult {
    Miss,
    Hit {
        body_part_index: usize,
        raw_damage: f32,
        damage_after_armor: f32,
        damage_type: DamageType,
    },
}

/// Result of applying damage to a body.
#[derive(Clone, Debug)]
pub struct DamageResult {
    pub damage_dealt: f32,
    pub part_destroyed: bool,
    pub target_killed: bool,
}

/// Resolve whether an attack hits and which body part.
///
/// `accuracy` is the attacker's total accuracy (0.0 to 1.0+).
/// `dodge` is the defender's dodge chance (0.0 to 1.0).
/// `roll` is a random value 0.0 to 1.0.
/// Resolve a confirmed hit: select body part and apply armor.
/// Call `accuracy_check` first to determine if the attack lands.
pub fn resolve_hit(
    raw_damage: f32,
    damage_type: DamageType,
    body_template: &BodyTemplate,
    armor: &EquippedArmor,
    coverage_roll: f32,
) -> HitResult {
    let part_index = select_body_part(body_template, coverage_roll);

    // Apply armor
    let damage_after_armor = armor.reduce_damage(part_index, damage_type, raw_damage);

    HitResult::Hit {
        body_part_index: part_index,
        raw_damage,
        damage_after_armor,
        damage_type,
    }
}

/// Check if an attack hits based on accuracy vs dodge.
/// Returns true if the attack lands.
pub fn accuracy_check(accuracy: f32, dodge: f32, roll: f32) -> bool {
    let hit_chance = (accuracy - dodge).clamp(0.05, 0.95);
    roll < hit_chance
}

/// Select a body part to hit based on coverage weights.
/// `roll` is a random value 0.0 to 1.0.
pub fn select_body_part(template: &BodyTemplate, roll: f32) -> usize {
    let total_coverage: f32 = template
        .parts
        .iter()
        .map(|p| p.coverage)
        .sum();

    let mut target = roll * total_coverage;
    for (i, part) in template.parts.iter().enumerate() {
        target -= part.coverage;
        if target <= 0.0 {
            return i;
        }
    }

    // Fallback: last part
    template.parts.len() - 1
}

/// Apply a resolved hit to a body. Returns the damage result.
pub fn apply_damage(
    body: &mut Body,
    template: &BodyTemplate,
    part_index: usize,
    damage: f32,
) -> DamageResult {
    let dealt = body.damage_part(part_index, damage, template);
    let part_destroyed = body.parts[part_index].destroyed;
    let target_killed = body.is_dead(template);

    DamageResult {
        damage_dealt: dealt,
        part_destroyed,
        target_killed,
    }
}

/// Calculate total accuracy for an attacker.
pub fn calculate_accuracy(attacker_attrs: &Attributes, base_weapon_accuracy: f32, range_penalty: f32) -> f32 {
    let skill_bonus = attacker_attrs.agility as f32 * 0.02;
    (base_weapon_accuracy + skill_bonus - range_penalty).clamp(0.0, 1.0)
}

/// Calculate dodge chance for a defender.
pub fn calculate_dodge(defender_attrs: &Attributes) -> f32 {
    (defender_attrs.agility as f32 * 0.02).clamp(0.0, 0.5)
}

/// Calculate raw damage for an attack.
pub fn calculate_damage(attacker_attrs: &Attributes, base_weapon_damage: f32, is_melee: bool) -> f32 {
    let stat_bonus = if is_melee {
        attacker_attrs.strength as f32 * 0.1
    } else {
        attacker_attrs.agility as f32 * 0.05
    };
    base_weapon_damage * (1.0 + stat_bonus)
}

/// A projectile in flight — will resolve on arrival.
#[derive(Component, Clone, Debug)]
pub struct Projectile {
    pub source: Entity,
    pub target: Entity,
    pub damage_type: DamageType,
    pub raw_damage: f32,
    pub accuracy: f32,
    /// Ticks remaining until the projectile arrives.
    pub remaining_ticks: u32,
}

/// Marker for entities currently in combat.
#[derive(Component, Clone, Copy, Debug)]
pub struct InCombat;
