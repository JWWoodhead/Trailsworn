use trailsworn::resources::faction::{Disposition, FactionRelations, FACTION_PLAYER};

#[test]
fn same_faction_is_friendly() {
    let relations = FactionRelations::default();
    assert_eq!(relations.get(FACTION_PLAYER, FACTION_PLAYER), Disposition::Friendly);
}

#[test]
fn unknown_factions_are_neutral() {
    let relations = FactionRelations::default();
    assert_eq!(relations.get(10, 20), Disposition::Neutral);
}

#[test]
fn set_and_get_hostile() {
    let mut relations = FactionRelations::default();
    relations.set(1, 2, Disposition::Hostile);
    assert!(relations.is_hostile(1, 2));
    assert!(relations.is_hostile(2, 1)); // order-independent
}

#[test]
fn overwrite_relation() {
    let mut relations = FactionRelations::default();
    relations.set(1, 2, Disposition::Hostile);
    relations.set(1, 2, Disposition::Friendly);
    assert_eq!(relations.get(1, 2), Disposition::Friendly);
}
