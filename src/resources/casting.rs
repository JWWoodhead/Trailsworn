use super::abilities::{AbilityDef, AbilityEffect, AbilitySlots, CastError, Mana, Stamina, StatScaling, TargetType};
use super::stats::{AttributeChoice, Attributes};
use super::status_effects::CcFlags;

/// Validate whether a cast attempt is legal. Does NOT spend resources.
pub fn validate_cast(
    ability: &AbilityDef,
    slots: &AbilitySlots,
    slot_index: usize,
    mana: &Mana,
    stamina: &Stamina,
    cc_flags: &CcFlags,
    caster_pos: (u32, u32),
    target_pos: Option<(u32, u32)>,
) -> Result<(), CastError> {
    // Slot must be valid and contain this ability
    if slot_index >= slots.abilities.len() || slots.abilities[slot_index] != ability.id {
        return Err(CastError::InvalidSlot);
    }

    // Must not be silenced / stunned / sleeping
    if !cc_flags.can_cast() {
        return Err(CastError::Silenced);
    }

    // Cooldown must be ready
    if !slots.is_ready(slot_index) {
        return Err(CastError::OnCooldown);
    }

    // Resource checks (costs are u32, pools are f32)
    if ability.mana_cost > 0 && mana.current < ability.mana_cost as f32 {
        return Err(CastError::NotEnoughMana);
    }
    if ability.stamina_cost > 0 && stamina.current < ability.stamina_cost as f32 {
        return Err(CastError::NotEnoughStamina);
    }

    // Range check (skip for self-targeted abilities)
    if ability.target_type != TargetType::SelfOnly {
        if let Some(target) = target_pos {
            let dx = caster_pos.0 as f32 - target.0 as f32;
            let dy = caster_pos.1 as f32 - target.1 as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > ability.range {
                return Err(CastError::OutOfRange);
            }
        }
    }

    Ok(())
}

/// Resolve an attribute value from an Attributes component.
fn get_attribute(attributes: &Attributes, choice: AttributeChoice) -> u32 {
    match choice {
        AttributeChoice::Strength => attributes.strength,
        AttributeChoice::Agility => attributes.agility,
        AttributeChoice::Intellect => attributes.intellect,
        AttributeChoice::Toughness => attributes.toughness,
        AttributeChoice::Willpower => attributes.willpower,
    }
}

/// Apply stat scaling to a base amount.
fn apply_scaling(base: f32, scaling: &Option<StatScaling>, attributes: &Attributes) -> f32 {
    match scaling {
        Some(s) => base + get_attribute(attributes, s.attribute) as f32 * s.factor,
        None => base,
    }
}

/// Calculate the final damage for a Damage ability effect, after stat scaling.
pub fn calculate_ability_damage(effect: &AbilityEffect, attributes: &Attributes) -> f32 {
    match effect {
        AbilityEffect::Damage { base_amount, scaling, .. } => {
            apply_scaling(*base_amount, scaling, attributes)
        }
        _ => 0.0,
    }
}

/// Calculate the final heal amount for a Heal ability effect, after stat scaling.
pub fn calculate_ability_heal(effect: &AbilityEffect, attributes: &Attributes) -> f32 {
    match effect {
        AbilityEffect::Heal { base_amount, scaling } => {
            apply_scaling(*base_amount, scaling, attributes)
        }
        _ => 0.0,
    }
}

/// AoE parameters bundled for resolve_aoe_targets.
pub struct AoeParams {
    pub aoe_radius: f32,
    pub cone_half_angle: f32,
    pub aoe_length: f32,
    pub aoe_width: f32,
}

/// Find all entities within an AoE shape. Pure geometry — works with any (f32, f32) positions.
/// Returns indices into the candidates slice for entities that are hit.
pub fn resolve_aoe_targets(
    caster_pos: (f32, f32),
    target_type: &TargetType,
    target_pos: (f32, f32),
    params: &AoeParams,
    candidates: &[(f32, f32)],
) -> Vec<usize> {
    match target_type {
        TargetType::CircleAoE => {
            let r_sq = params.aoe_radius * params.aoe_radius;
            candidates
                .iter()
                .enumerate()
                .filter(|(_, pos)| {
                    let dx = pos.0 - target_pos.0;
                    let dy = pos.1 - target_pos.1;
                    dx * dx + dy * dy <= r_sq
                })
                .map(|(i, _)| i)
                .collect()
        }
        TargetType::ConeAoE => {
            // Cone from caster in the direction of target_pos
            let dir_x = target_pos.0 - caster_pos.0;
            let dir_y = target_pos.1 - caster_pos.1;
            let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
            if dir_len < 0.001 {
                return Vec::new();
            }
            let dir_x = dir_x / dir_len;
            let dir_y = dir_y / dir_len;
            let half_angle_rad = params.cone_half_angle.to_radians();
            let cos_threshold = half_angle_rad.cos();
            let max_dist_sq = params.aoe_length * params.aoe_length;

            candidates
                .iter()
                .enumerate()
                .filter(|(_, pos)| {
                    let dx = pos.0 - caster_pos.0;
                    let dy = pos.1 - caster_pos.1;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq > max_dist_sq || dist_sq < 0.001 {
                        return false;
                    }
                    let dist = dist_sq.sqrt();
                    let cos_angle = (dx * dir_x + dy * dir_y) / dist;
                    cos_angle >= cos_threshold
                })
                .map(|(i, _)| i)
                .collect()
        }
        TargetType::LineAoE => {
            // Line from caster in the direction of target_pos, with width and length
            let dir_x = target_pos.0 - caster_pos.0;
            let dir_y = target_pos.1 - caster_pos.1;
            let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
            if dir_len < 0.001 {
                return Vec::new();
            }
            let dir_x = dir_x / dir_len;
            let dir_y = dir_y / dir_len;
            // Perpendicular direction
            let perp_x = -dir_y;
            let perp_y = dir_x;
            let half_width = params.aoe_width / 2.0;

            candidates
                .iter()
                .enumerate()
                .filter(|(_, pos)| {
                    let dx = pos.0 - caster_pos.0;
                    let dy = pos.1 - caster_pos.1;
                    // Project onto line direction
                    let along = dx * dir_x + dy * dir_y;
                    if along < 0.0 || along > params.aoe_length {
                        return false;
                    }
                    // Project onto perpendicular
                    let across = (dx * perp_x + dy * perp_y).abs();
                    across <= half_width
                })
                .map(|(i, _)| i)
                .collect()
        }
        // Non-AoE types return empty
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::abilities::*;
    use crate::resources::damage::DamageType;

    fn test_ability(id: AbilityId, mana_cost: u32, stamina_cost: u32, range: f32, target_type: TargetType) -> AbilityDef {
        AbilityDef {
            id,
            name: "Test".into(),
            cast_time_ticks: 0,
            cooldown_ticks: 60,
            mana_cost,
            stamina_cost,
            range,
            target_type,
            aoe_radius: 0.0,
            cone_half_angle: 0.0,
            aoe_length: 0.0,
            aoe_width: 0.0,
            effects: vec![],
            interruptible: true,
            cast_sfx: None,
            impact_sfx: None,
            impact_vfx: None,
            impact_vfx_scale: 1.0,
            cast_vfx: None,
        }
    }

    fn default_cc() -> CcFlags {
        CcFlags::default()
    }

    #[test]
    fn validate_cast_passes_on_valid() {
        let ability = test_ability(1, 10, 0, 5.0, TargetType::SingleEnemy);
        let slots = AbilitySlots::new(vec![1]);
        let mana = Mana::new(100.0);
        let stamina = Stamina::new(100.0);
        let result = validate_cast(&ability, &slots, 0, &mana, &stamina, &default_cc(), (0, 0), Some((3, 4)));
        assert!(result.is_ok());
    }

    #[test]
    fn validate_cast_rejects_invalid_slot() {
        let ability = test_ability(1, 0, 0, 5.0, TargetType::SelfOnly);
        let slots = AbilitySlots::new(vec![1]);
        let result = validate_cast(&ability, &slots, 5, &Mana::new(100.0), &Stamina::new(100.0), &default_cc(), (0, 0), None);
        assert_eq!(result, Err(CastError::InvalidSlot));
    }

    #[test]
    fn validate_cast_rejects_on_cooldown() {
        let ability = test_ability(1, 0, 0, 5.0, TargetType::SelfOnly);
        let mut slots = AbilitySlots::new(vec![1]);
        slots.start_cooldown(0, 30);
        let result = validate_cast(&ability, &slots, 0, &Mana::new(100.0), &Stamina::new(100.0), &default_cc(), (0, 0), None);
        assert_eq!(result, Err(CastError::OnCooldown));
    }

    #[test]
    fn validate_cast_rejects_insufficient_mana() {
        let ability = test_ability(1, 50, 0, 5.0, TargetType::SelfOnly);
        let result = validate_cast(&ability, &AbilitySlots::new(vec![1]), 0, &Mana::new(10.0), &Stamina::new(100.0), &default_cc(), (0, 0), None);
        assert_eq!(result, Err(CastError::NotEnoughMana));
    }

    #[test]
    fn validate_cast_rejects_insufficient_stamina() {
        let ability = test_ability(1, 0, 50, 5.0, TargetType::SelfOnly);
        let result = validate_cast(&ability, &AbilitySlots::new(vec![1]), 0, &Mana::new(100.0), &Stamina::new(10.0), &default_cc(), (0, 0), None);
        assert_eq!(result, Err(CastError::NotEnoughStamina));
    }

    #[test]
    fn validate_cast_rejects_silenced() {
        let ability = test_ability(1, 0, 0, 5.0, TargetType::SelfOnly);
        let cc = CcFlags { silenced: true, ..Default::default() };
        let result = validate_cast(&ability, &AbilitySlots::new(vec![1]), 0, &Mana::new(100.0), &Stamina::new(100.0), &cc, (0, 0), None);
        assert_eq!(result, Err(CastError::Silenced));
    }

    #[test]
    fn validate_cast_rejects_out_of_range() {
        let ability = test_ability(1, 0, 0, 3.0, TargetType::SingleEnemy);
        // Distance: sqrt(10^2 + 10^2) = ~14.1, way beyond range 3.0
        let result = validate_cast(&ability, &AbilitySlots::new(vec![1]), 0, &Mana::new(100.0), &Stamina::new(100.0), &default_cc(), (0, 0), Some((10, 10)));
        assert_eq!(result, Err(CastError::OutOfRange));
    }

    #[test]
    fn validate_cast_self_only_skips_range() {
        let ability = test_ability(1, 0, 0, 0.0, TargetType::SelfOnly);
        // Range is 0 but SelfOnly skips range check
        let result = validate_cast(&ability, &AbilitySlots::new(vec![1]), 0, &Mana::new(100.0), &Stamina::new(100.0), &default_cc(), (0, 0), None);
        assert!(result.is_ok());
    }

    #[test]
    fn calculate_damage_with_scaling() {
        let effect = AbilityEffect::Damage {
            damage_type: DamageType::Fire,
            base_amount: 20.0,
            scaling: Some(StatScaling {
                attribute: AttributeChoice::Intellect,
                factor: 0.5,
            }),
        };
        let attrs = Attributes { intellect: 10, ..Default::default() };
        // 20.0 + (10 * 0.5) = 25.0
        assert!((calculate_ability_damage(&effect, &attrs) - 25.0).abs() < 0.001);
    }

    #[test]
    fn calculate_damage_without_scaling() {
        let effect = AbilityEffect::Damage {
            damage_type: DamageType::Slashing,
            base_amount: 12.0,
            scaling: None,
        };
        let attrs = Attributes::default();
        assert!((calculate_ability_damage(&effect, &attrs) - 12.0).abs() < 0.001);
    }

    #[test]
    fn calculate_heal_with_scaling() {
        let effect = AbilityEffect::Heal {
            base_amount: 15.0,
            scaling: Some(StatScaling {
                attribute: AttributeChoice::Willpower,
                factor: 0.3,
            }),
        };
        let attrs = Attributes { willpower: 8, ..Default::default() };
        // 15.0 + (8 * 0.3) = 17.4
        assert!((calculate_ability_heal(&effect, &attrs) - 17.4).abs() < 0.001);
    }

    #[test]
    fn aoe_circle_includes_within_radius() {
        let params = AoeParams { aoe_radius: 3.0, cone_half_angle: 0.0, aoe_length: 0.0, aoe_width: 0.0 };
        let candidates = vec![(5.0, 5.0), (5.0, 7.0), (5.0, 9.0), (10.0, 10.0)];
        let hits = resolve_aoe_targets((0.0, 0.0), &TargetType::CircleAoE, (5.0, 5.0), &params, &candidates);
        // (5,5) dist=0 ✓, (5,7) dist=2 ✓, (5,9) dist=4 ✗, (10,10) dist=~7.1 ✗
        assert_eq!(hits, vec![0, 1]);
    }

    #[test]
    fn aoe_circle_excludes_outside_radius() {
        let params = AoeParams { aoe_radius: 1.0, cone_half_angle: 0.0, aoe_length: 0.0, aoe_width: 0.0 };
        let candidates = vec![(10.0, 10.0)];
        let hits = resolve_aoe_targets((0.0, 0.0), &TargetType::CircleAoE, (5.0, 5.0), &params, &candidates);
        assert!(hits.is_empty());
    }

    #[test]
    fn aoe_cone_includes_within_arc() {
        // Caster at origin, aiming right (+x direction), 45-degree half-angle, 10 length
        let params = AoeParams { aoe_radius: 0.0, cone_half_angle: 45.0, aoe_length: 10.0, aoe_width: 0.0 };
        let candidates = vec![
            (5.0, 0.0),   // directly ahead — should hit
            (5.0, 4.0),   // ~38 degrees off — should hit (within 45)
            (5.0, 6.0),   // ~50 degrees off — should miss
            (-3.0, 0.0),  // behind — should miss
        ];
        let hits = resolve_aoe_targets((0.0, 0.0), &TargetType::ConeAoE, (10.0, 0.0), &params, &candidates);
        assert_eq!(hits, vec![0, 1]);
    }

    #[test]
    fn aoe_line_includes_within_rect() {
        // Caster at origin, aiming right, length 10, width 2 (±1 perpendicular)
        let params = AoeParams { aoe_radius: 0.0, cone_half_angle: 0.0, aoe_length: 10.0, aoe_width: 2.0 };
        let candidates = vec![
            (5.0, 0.0),   // dead center — hit
            (5.0, 0.8),   // within width — hit
            (5.0, 1.5),   // outside width — miss
            (12.0, 0.0),  // beyond length — miss
            (-1.0, 0.0),  // behind caster — miss
        ];
        let hits = resolve_aoe_targets((0.0, 0.0), &TargetType::LineAoE, (10.0, 0.0), &params, &candidates);
        assert_eq!(hits, vec![0, 1]);
    }

    #[test]
    fn non_aoe_returns_empty() {
        let params = AoeParams { aoe_radius: 5.0, cone_half_angle: 0.0, aoe_length: 0.0, aoe_width: 0.0 };
        let candidates = vec![(1.0, 1.0)];
        let hits = resolve_aoe_targets((0.0, 0.0), &TargetType::SingleEnemy, (1.0, 1.0), &params, &candidates);
        assert!(hits.is_empty());
    }
}
