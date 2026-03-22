use gold_and_glory::resources::stats::{
    spend_attribute_point, AttributeChoice, Attributes, CharacterLevel,
};

#[test]
fn default_attributes_are_5() {
    let attrs = Attributes::default();
    assert_eq!(attrs.strength, 5);
    assert_eq!(attrs.agility, 5);
    assert_eq!(attrs.intellect, 5);
    assert_eq!(attrs.toughness, 5);
    assert_eq!(attrs.willpower, 5);
}

#[test]
fn starts_at_level_1() {
    let level = CharacterLevel::default();
    assert_eq!(level.level, 1);
    assert_eq!(level.current_xp, 0);
    assert_eq!(level.unspent_points, 0);
}

#[test]
fn xp_causes_level_up() {
    let mut level = CharacterLevel::default();
    let gained = level.add_xp(100);
    assert_eq!(gained, 1);
    assert_eq!(level.level, 2);
    assert_eq!(level.unspent_points, 1);
}

#[test]
fn partial_xp_no_level_up() {
    let mut level = CharacterLevel::default();
    let gained = level.add_xp(50);
    assert_eq!(gained, 0);
    assert_eq!(level.level, 1);
    assert_eq!(level.current_xp, 50);
}

#[test]
fn xp_carries_over() {
    let mut level = CharacterLevel::default();
    level.add_xp(120); // 100 to level, 20 carries
    assert_eq!(level.level, 2);
    assert_eq!(level.current_xp, 20);
}

#[test]
fn multiple_level_ups_at_once() {
    let mut level = CharacterLevel::default();
    let gained = level.add_xp(10000);
    assert!(gained > 1);
    assert!(level.level > 2);
    assert_eq!(level.unspent_points, gained);
}

#[test]
fn spend_point_increases_attribute() {
    let mut level = CharacterLevel::default();
    level.add_xp(100); // level up, get a point
    let mut attrs = Attributes::default();

    assert!(spend_attribute_point(
        &mut level,
        &mut attrs,
        AttributeChoice::Strength
    ));
    assert_eq!(attrs.strength, 6);
    assert_eq!(level.unspent_points, 0);
}

#[test]
fn cant_spend_without_points() {
    let mut level = CharacterLevel::default();
    let mut attrs = Attributes::default();

    assert!(!spend_attribute_point(
        &mut level,
        &mut attrs,
        AttributeChoice::Strength
    ));
    assert_eq!(attrs.strength, 5);
}

#[test]
fn cant_exceed_20() {
    let mut level = CharacterLevel::default();
    level.unspent_points = 100;
    let mut attrs = Attributes::default();
    attrs.strength = 20;

    assert!(!spend_attribute_point(
        &mut level,
        &mut attrs,
        AttributeChoice::Strength
    ));
}

#[test]
fn xp_requirement_increases_per_level() {
    let mut level = CharacterLevel::default();
    let first_req = level.xp_to_next;
    level.add_xp(first_req);
    assert!(level.xp_to_next > first_req);
}
