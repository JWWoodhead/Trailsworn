use bevy::prelude::*;
use trailsworn::resources::threat::ThreatTable;

fn fake_entity(id: u32) -> Entity {
    Entity::from_bits(id as u64)
}

#[test]
fn empty_threat_table() {
    let table = ThreatTable::default();
    assert!(table.is_empty());
    assert!(table.highest_threat().is_none());
}

#[test]
fn add_and_query_threat() {
    let mut table = ThreatTable::default();
    let e1 = fake_entity(1);
    table.add_threat(e1, 50.0);
    assert_eq!(table.get_threat(e1), 50.0);
}

#[test]
fn highest_threat_target() {
    let mut table = ThreatTable::default();
    let e1 = fake_entity(1);
    let e2 = fake_entity(2);
    table.add_threat(e1, 50.0);
    table.add_threat(e2, 100.0);
    assert_eq!(table.highest_threat(), Some(e2));
}

#[test]
fn threat_accumulates() {
    let mut table = ThreatTable::default();
    let e1 = fake_entity(1);
    table.add_threat(e1, 30.0);
    table.add_threat(e1, 20.0);
    assert_eq!(table.get_threat(e1), 50.0);
}

#[test]
fn remove_entity_from_threat() {
    let mut table = ThreatTable::default();
    let e1 = fake_entity(1);
    table.add_threat(e1, 50.0);
    table.remove(e1);
    assert_eq!(table.get_threat(e1), 0.0);
    assert!(table.is_empty());
}
