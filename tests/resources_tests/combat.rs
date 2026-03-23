use trailsworn::resources::body::humanoid_template;
use trailsworn::resources::body::Body;
use trailsworn::resources::combat::*;
use trailsworn::resources::damage::*;

#[test]
fn accuracy_check_always_hits_at_max() {
    assert!(accuracy_check(1.0, 0.0, 0.5));
}

#[test]
fn accuracy_check_can_miss() {
    // Very low accuracy, high dodge
    assert!(!accuracy_check(0.1, 0.8, 0.5));
}

#[test]
fn minimum_hit_chance_5_percent() {
    // Even with 0 accuracy and 1.0 dodge, 5% floor
    assert!(accuracy_check(0.0, 1.0, 0.04));
}

#[test]
fn maximum_hit_chance_95_percent() {
    // Even with perfect accuracy, 95% cap
    assert!(!accuracy_check(1.0, 0.0, 0.96));
}

#[test]
fn resolve_hit_selects_part_and_applies_armor() {
    let template = humanoid_template();
    let mut res = Resistances::default();
    res.set(DamageType::Slashing, 0.5);
    let armor = EquippedArmor {
        pieces: vec![ArmorPiece {
            name: "Plate".into(),
            covered_parts: vec![5], // torso
            resistances: res,
        }],
    };

    // Roll 0.5 should land on some body part
    let result = resolve_hit(100.0, DamageType::Slashing, &template, &armor, 0.5);
    match result {
        HitResult::Hit { damage_after_armor, .. } => {
            assert!(damage_after_armor <= 100.0);
        }
        HitResult::Miss => panic!("resolve_hit should not miss"),
    }
}

#[test]
fn apply_damage_kills_on_vital_part() {
    let template = humanoid_template();
    let mut body = Body::from_template(&template);

    // Heart is index 6
    let result = apply_damage(&mut body, &template, 6, 100.0);
    assert!(result.part_destroyed);
    assert!(result.target_killed);
}

#[test]
fn apply_damage_non_lethal() {
    let template = humanoid_template();
    let mut body = Body::from_template(&template);

    // Small damage to torso
    let result = apply_damage(&mut body, &template, 5, 5.0);
    assert_eq!(result.damage_dealt, 5.0);
    assert!(!result.part_destroyed);
    assert!(!result.target_killed);
}

#[test]
fn calculate_damage_scales_with_strength_for_melee() {
    use trailsworn::resources::stats::Attributes;

    let weak = Attributes { strength: 1, ..Default::default() };
    let strong = Attributes { strength: 20, ..Default::default() };

    let weak_dmg = calculate_damage(&weak, 10.0, true);
    let strong_dmg = calculate_damage(&strong, 10.0, true);
    assert!(strong_dmg > weak_dmg);
}

#[test]
fn body_part_selection_coverage() {
    let template = humanoid_template();
    // Roll 0.0 should hit the first part with coverage
    let part = select_body_part(&template, 0.0);
    assert_eq!(part, 0); // Head has first coverage

    // Roll ~1.0 should hit one of the last parts
    let part = select_body_part(&template, 0.99);
    assert!(part > 0);
}
