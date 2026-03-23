use trailsworn::resources::status_effects::*;

fn stun_def() -> StatusEffectDef {
    StatusEffectDef {
        id: 1,
        name: "Stun".into(),
        max_stacks: 1,
        tick_interval_ticks: 0,
        tick_effect: None,
        stat_modifiers: vec![],
        cc_flags: CcFlags { stunned: true, ..Default::default() },
        is_buff: false,
    }
}

fn poison_def() -> StatusEffectDef {
    StatusEffectDef {
        id: 2,
        name: "Poison".into(),
        max_stacks: 3,
        tick_interval_ticks: 30,
        tick_effect: Some(TickEffect {
            damage_type: Some(trailsworn::resources::damage::DamageType::Shadow),
            amount: 5.0,
            is_heal: false,
        }),
        stat_modifiers: vec![],
        cc_flags: CcFlags::default(),
        is_buff: false,
    }
}

fn setup_registry() -> StatusEffectRegistry {
    let mut reg = StatusEffectRegistry::default();
    reg.register(stun_def());
    reg.register(poison_def());
    reg
}

#[test]
fn apply_new_effect() {
    let registry = setup_registry();
    let mut active = ActiveStatusEffects::default();
    active.apply(1, 120, None, &registry);
    assert_eq!(active.effects.len(), 1);
    assert_eq!(active.effects[0].stacks, 1);
}

#[test]
fn refresh_duration_on_reapply() {
    let registry = setup_registry();
    let mut active = ActiveStatusEffects::default();
    active.apply(1, 120, None, &registry);
    active.apply(1, 180, None, &registry); // refresh
    assert_eq!(active.effects.len(), 1);
    assert_eq!(active.effects[0].remaining_ticks, 180);
}

#[test]
fn stacking_up_to_max() {
    let registry = setup_registry();
    let mut active = ActiveStatusEffects::default();
    active.apply(2, 120, None, &registry); // poison, max 3 stacks
    active.apply(2, 120, None, &registry);
    active.apply(2, 120, None, &registry);
    active.apply(2, 120, None, &registry); // 4th should not increase stacks
    assert_eq!(active.effects[0].stacks, 3);
}

#[test]
fn expired_effects_removed() {
    let registry = setup_registry();
    let mut active = ActiveStatusEffects::default();
    active.apply(1, 0, None, &registry); // expires immediately
    let removed = active.remove_expired();
    assert_eq!(removed.len(), 1);
    assert!(active.effects.is_empty());
}

#[test]
fn combined_cc_flags() {
    let registry = setup_registry();
    let mut active = ActiveStatusEffects::default();
    active.apply(1, 60, None, &registry); // stun

    let flags = active.combined_cc_flags(&registry);
    assert!(flags.stunned);
    assert!(!flags.can_move());
    assert!(!flags.can_cast());
    assert!(!flags.can_attack());
}

#[test]
fn no_cc_when_clean() {
    let registry = setup_registry();
    let active = ActiveStatusEffects::default();
    let flags = active.combined_cc_flags(&registry);
    assert!(flags.can_move());
    assert!(flags.can_cast());
    assert!(flags.can_attack());
}

#[test]
fn root_blocks_move_but_not_cast() {
    let mut registry = StatusEffectRegistry::default();
    registry.register(StatusEffectDef {
        id: 10,
        name: "Root".into(),
        max_stacks: 1,
        tick_interval_ticks: 0,
        tick_effect: None,
        stat_modifiers: vec![],
        cc_flags: CcFlags { rooted: true, ..Default::default() },
        is_buff: false,
    });

    let mut active = ActiveStatusEffects::default();
    active.apply(10, 60, None, &registry);

    let flags = active.combined_cc_flags(&registry);
    assert!(!flags.can_move());
    assert!(flags.can_cast());
    assert!(flags.can_attack());
}
