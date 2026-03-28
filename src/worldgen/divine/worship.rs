//! Worship competition and god power updates.

use rand::{Rng, RngExt};

use super::gods::{DrawnPantheon, GodId};
use super::personality::DivineDrive;
use super::state::GodState;
use crate::worldgen::history::state::{SettlementState, WorldState};
use crate::worldgen::history::{CrossDomainEvent, EventKind, HistoricEvent};
use crate::worldgen::world_map::WorldMap;

fn make_event(year: i32, kind: EventKind, description: String, god_ids: Vec<GodId>) -> HistoricEvent {
    HistoricEvent { year, kind, description, participants: vec![], god_participants: god_ids }
}

pub fn evaluate_worship(
    year: i32,
    gods: &mut [GodState],
    events: &mut Vec<HistoricEvent>,
    settlements: &[SettlementState],
    cross_events: &mut Vec<CrossDomainEvent>,
    world_state: &mut WorldState,
    world_map: &WorldMap,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    for si in 0..settlements.len() {
        let settlement_pos = match settlements[si].world_pos {
            Some(pos) => pos,
            None => continue,
        };
        let current_patron = settlements[si].patron_god;
        let current_devotion = settlements[si].devotion;

        let territory_owner = world_map.idx(settlement_pos)
            .and_then(|idx| world_state.territory_map[idx]);

        let owner_god_id = match territory_owner {
            Some(id) => id,
            None => {
                if current_devotion > 0 {
                    cross_events.push(CrossDomainEvent::DevotionChanged {
                        settlement_index: si, delta: -2,
                    });
                }
                continue;
            }
        };

        let owner_active = gods.iter().any(|g| g.god_id == owner_god_id && g.is_active());
        if !owner_active { continue; }

        if current_patron == Some(owner_god_id) {
            let drive = gods.iter().find(|g| g.god_id == owner_god_id).map(|g| g.drive());
            let growth = match drive {
                Some(DivineDrive::Worship) => 5,
                Some(DivineDrive::Love) => 4,
                Some(DivineDrive::Dominion) => 3,
                _ => 2,
            };
            cross_events.push(CrossDomainEvent::DevotionChanged {
                settlement_index: si, delta: growth,
            });
        } else if current_patron.is_none() {
            let drive = gods.iter().find(|g| g.god_id == owner_god_id).map(|g| g.drive());
            let claim_prob = match drive {
                Some(DivineDrive::Worship) => 0.40,
                Some(DivineDrive::Love) => 0.30,
                Some(DivineDrive::Dominion) => 0.35,
                Some(DivineDrive::Legacy) => 0.25,
                _ => 0.15,
            };
            if rng.random::<f32>() < claim_prob {
                let sname = settlements[si].name.clone();
                cross_events.push(CrossDomainEvent::WorshipEstablished {
                    settlement_index: si, god_id: owner_god_id, devotion: 20,
                });

                if let Some(g) = gods.iter_mut().find(|g| g.god_id == owner_god_id) {
                    g.worshipper_settlements.push(settlement_pos);
                }

                let god_name = pantheon.name(owner_god_id).unwrap_or("A god");
                events.push(make_event(year, EventKind::WorshipEstablished,
                    format!("The people of {} began worshipping {}", sname, god_name),
                    vec![owner_god_id],
                ));
            }
        } else {
            if current_devotion > 30 { continue; }

            let drive = gods.iter().find(|g| g.god_id == owner_god_id).map(|g| g.drive());
            let convert_prob = match drive {
                Some(DivineDrive::Dominion) => 0.20,
                Some(DivineDrive::Worship) => 0.15,
                Some(DivineDrive::Supremacy) => 0.15,
                Some(DivineDrive::Vindication) => 0.10,
                _ => 0.05,
            };

            if rng.random::<f32>() < convert_prob {
                let old_patron = current_patron.unwrap();
                let sname = settlements[si].name.clone();
                cross_events.push(CrossDomainEvent::WorshipConverted {
                    settlement_index: si, new_god_id: owner_god_id,
                    old_god_id: old_patron, devotion: 15,
                });

                if let Some(g) = gods.iter_mut().find(|g| g.god_id == old_patron) {
                    g.worshipper_settlements.retain(|p| *p != settlement_pos);
                }
                if let Some(g) = gods.iter_mut().find(|g| g.god_id == owner_god_id) {
                    g.worshipper_settlements.push(settlement_pos);
                }

                world_state.divine_relations.modify(owner_god_id, old_patron, -10);

                let god_name = pantheon.name(owner_god_id).unwrap_or("A god");
                let old_name = pantheon.name(old_patron).unwrap_or("another god");
                events.push(make_event(year, EventKind::WorshipConverted,
                    format!("The people of {} abandoned {} and turned to {}", sname, old_name, god_name),
                    vec![owner_god_id, old_patron],
                ));
            } else {
                cross_events.push(CrossDomainEvent::DevotionChanged {
                    settlement_index: si, delta: -1,
                });
            }
        }
    }

    // Gods without worshippers slowly weaken
    for god in gods.iter_mut() {
        if !god.is_active() { continue; }
        if god.worshipper_settlements.is_empty() {
            god.years_without_worship += 1;
            if god.years_without_worship >= 20 {
                god.faded = true;
            }
        } else {
            god.years_without_worship = 0;
            if god.faded {
                god.faded = false;
            }
        }
    }
}

pub fn update_god_power(gods: &mut [GodState]) {
    for god in gods.iter_mut() {
        if !god.is_active() { continue; }
        let worshipper_count = god.worshipper_settlements.len() as u32;
        god.power = (worshipper_count * 15).min(100);
    }
}
