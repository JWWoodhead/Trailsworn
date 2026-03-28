use trailsworn::resources::abilities::*;

#[test]
fn mana_spend_and_regen() {
    let mut mana = Mana::new(100.0);
    assert!(mana.spend(30.0));
    assert_eq!(mana.current, 70.0);

    mana.regenerate(10.0);
    assert_eq!(mana.current, 80.0);
}

#[test]
fn mana_cant_overspend() {
    let mut mana = Mana::new(100.0);
    assert!(!mana.spend(150.0));
    assert_eq!(mana.current, 100.0);
}

#[test]
fn mana_regen_caps_at_max() {
    let mut mana = Mana::new(100.0);
    mana.regenerate(200.0);
    assert_eq!(mana.current, 100.0);
}

#[test]
fn stamina_spend_and_regen() {
    let mut stamina = Stamina::new(50.0);
    assert!(stamina.spend(20.0));
    assert_eq!(stamina.current, 30.0);

    stamina.regenerate(5.0);
    assert_eq!(stamina.current, 35.0);
}

#[test]
fn ability_slots_cooldowns() {
    let mut slots = AbilitySlots::new(vec![1, 2, 3]);
    assert!(slots.is_ready(0));

    slots.start_cooldown(0, 60);
    assert!(!slots.is_ready(0));
    assert!(slots.is_ready(1));

    for _ in 0..60 {
        slots.tick_cooldowns();
    }
    assert!(slots.is_ready(0));
}

#[test]
fn ability_registry() {
    let mut registry = AbilityRegistry::default();
    registry.register(AbilityDef {
        id: 1,
        name: "Fireball".into(),
        cast_time_ticks: 90,
        cooldown_ticks: 300,
        mana_cost: 30,
        stamina_cost: 0,
        range: 10.0,
        target_type: TargetType::CircleAoE,
        aoe_radius: 3.0,
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
    });

    assert!(registry.get(1).is_some());
    assert_eq!(registry.get(1).unwrap().name, "Fireball");
    assert!(registry.get(999).is_none());
}
