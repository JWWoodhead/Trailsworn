use trailsworn::resources::damage::*;

#[test]
fn physical_vs_magical() {
    assert!(DamageType::Slashing.is_physical());
    assert!(DamageType::Fire.is_magical());
    assert!(!DamageType::Arcane.is_physical());
    assert!(DamageType::Storm.is_magical());
    assert!(DamageType::Nature.is_magical());
}

#[test]
fn zero_resistance_means_full_damage() {
    let res = Resistances::default();
    assert_eq!(res.apply(DamageType::Slashing, 100.0), 100.0);
}

#[test]
fn half_resistance_halves_damage() {
    let mut res = Resistances::default();
    res.set(DamageType::Slashing, 0.5);
    assert_eq!(res.apply(DamageType::Slashing, 100.0), 50.0);
}

#[test]
fn full_resistance_blocks_all() {
    let mut res = Resistances::default();
    res.set(DamageType::Fire, 1.0);
    assert_eq!(res.apply(DamageType::Fire, 100.0), 0.0);
}

#[test]
fn armor_covers_specific_parts() {
    let mut res = Resistances::default();
    res.set(DamageType::Slashing, 0.5);
    let armor = EquippedArmor {
        pieces: vec![ArmorPiece {
            name: "Helmet".into(),
            covered_parts: vec![0, 1, 2, 3, 4],
            resistances: res,
        }],
    };
    assert_eq!(armor.reduce_damage(0, DamageType::Slashing, 100.0), 50.0);
    assert_eq!(armor.reduce_damage(5, DamageType::Slashing, 100.0), 100.0);
}

#[test]
fn weapon_cooldown() {
    let weapon = WeaponDef {
        name: "Sword".into(),
        damage_type: DamageType::Slashing,
        base_damage: 10.0,
        attack_speed_ticks: 60,
        range: 1.5,
        projectile_speed: 0.0,
        is_melee: true,
        attack_sfx: None,
    };
    let mut equipped = EquippedWeapon::new(weapon);
    assert!(equipped.is_ready());

    equipped.start_cooldown();
    assert!(!equipped.is_ready());

    for _ in 0..60 {
        equipped.tick();
    }
    assert!(equipped.is_ready());
}
