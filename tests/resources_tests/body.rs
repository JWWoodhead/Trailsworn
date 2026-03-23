use trailsworn::resources::body::{humanoid_template, Body, BodyTemplates, Capability};

fn setup() -> (Body, trailsworn::resources::body::BodyTemplate) {
    let template = humanoid_template();
    let body = Body::from_template(&template);
    (body, template)
}

#[test]
fn all_parts_start_at_full_hp() {
    let (body, template) = setup();
    for (i, def) in template.parts.iter().enumerate() {
        assert_eq!(body.parts[i].current_hp, def.max_hp);
        assert!(!body.parts[i].destroyed);
    }
}

#[test]
fn damage_reduces_hp() {
    let (mut body, template) = setup();
    // Damage torso (index 5)
    let dealt = body.damage_part(5, 10.0, &template);
    assert_eq!(dealt, 10.0);
    assert_eq!(body.parts[5].current_hp, 30.0); // 40 - 10
    assert!(!body.parts[5].destroyed);
}

#[test]
fn damage_capped_at_remaining_hp() {
    let (mut body, template) = setup();
    // Torso has 40 HP, deal 100
    let dealt = body.damage_part(5, 100.0, &template);
    assert_eq!(dealt, 40.0);
    assert!(body.parts[5].destroyed);
}

#[test]
fn destroying_part_destroys_children() {
    let (mut body, template) = setup();
    // Destroy left arm (index 10) — should also destroy left hand (index 11)
    body.damage_part(10, 100.0, &template);
    assert!(body.parts[10].destroyed);
    assert!(body.parts[11].destroyed);
}

#[test]
fn vital_part_destruction_means_death() {
    let (mut body, template) = setup();
    // Destroy heart (index 6, vital)
    body.damage_part(6, 100.0, &template);
    assert!(body.is_dead(&template));
}

#[test]
fn non_vital_destruction_not_death() {
    let (mut body, template) = setup();
    // Destroy left arm
    body.damage_part(10, 100.0, &template);
    assert!(!body.is_dead(&template));
}

#[test]
fn capability_lost_when_all_providers_destroyed() {
    let (mut body, template) = setup();
    // Destroy both eyes (indices 2, 3)
    body.damage_part(2, 100.0, &template);
    body.damage_part(3, 100.0, &template);
    assert!(!body.has_capability(&Capability::Sight, &template));
}

#[test]
fn capability_partial_with_one_provider() {
    let (mut body, template) = setup();
    // Destroy left eye only
    body.damage_part(2, 100.0, &template);
    assert!(body.has_capability(&Capability::Sight, &template));
    assert_eq!(body.capability_fraction(&Capability::Sight, &template), 0.5);
}

#[test]
fn pain_level_increases_with_damage() {
    let (mut body, template) = setup();
    let pain_before = body.pain_level(&template);
    body.damage_part(5, 20.0, &template);
    let pain_after = body.pain_level(&template);
    assert!(pain_after > pain_before);
}

#[test]
fn no_damage_to_destroyed_part() {
    let (mut body, template) = setup();
    body.damage_part(6, 100.0, &template); // destroy heart
    let dealt = body.damage_part(6, 10.0, &template); // try again
    assert_eq!(dealt, 0.0);
}

#[test]
fn coverage_sums_roughly_to_one() {
    let template = humanoid_template();
    let total: f32 = template.parts.iter().map(|p| p.coverage).sum();
    // Should be close to 1.0 (doesn't have to be exact)
    assert!(total > 0.9 && total < 1.1, "coverage total was {total}");
}

#[test]
fn template_registry() {
    let mut registry = BodyTemplates::default();
    registry.register(humanoid_template());
    assert!(registry.get("humanoid").is_some());
    assert!(registry.get("dragon").is_none());
}
