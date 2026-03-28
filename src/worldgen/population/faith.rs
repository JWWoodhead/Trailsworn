//! Faith evaluation — devotion changes based on observable conditions.
//! People can have relationships with multiple gods. Settlement patronage
//! is derived from the aggregate faith of its residents.

use crate::worldgen::divine::gods::GodId;
use crate::worldgen::divine::state::GodState;
use crate::worldgen::history::state::SettlementState;

use super::index::SettlementIndex;
use super::types::*;

/// Evaluate faith for all living people and update settlement patronage.
pub fn evaluate_faith(
    people: &mut [Person],
    index: &SettlementIndex,
    settlements: &mut [SettlementState],
    gods: &[GodState],
    year: i32,
) {
    // Pre-compute god info
    let god_power: std::collections::HashMap<GodId, u32> = gods.iter()
        .map(|g| (g.god_id, g.power))
        .collect();
    let god_faded: std::collections::HashMap<GodId, bool> = gods.iter()
        .map(|g| (g.god_id, g.faded))
        .collect();

    for si in 0..settlements.len() {
        let settlement = &settlements[si];
        if settlement.destroyed_year.is_some() { continue; }

        let patron = settlement.patron_god;
        let prosperity = settlement.prosperity;
        let plague = settlement.plague_this_year;
        let settlement_id = settlement.id;

        for &idx in index.residents(settlement_id) {
            let person = &mut people[idx];
            if !person.is_alive(year) { continue; }

            // If settlement has a patron and person has no relationship with them, start one
            if let Some(patron_god) = patron {
                if person.devotion_to(patron_god) == 0 {
                    person.set_devotion(patron_god, 15); // community exposure
                }
            }

            // Evaluate each god relationship
            let faith_snapshot: Vec<(GodId, u8)> = person.faith.clone();
            for (god_id, old_devotion) in faith_snapshot {
                let is_patron = patron == Some(god_id);
                let power = god_power.get(&god_id).copied().unwrap_or(0);
                let faded = god_faded.get(&god_id).copied().unwrap_or(false);

                let mut new_devotion = old_devotion;

                // --- Increases ---

                // Settlement prospering under this god's patronage
                if is_patron && prosperity > 70 {
                    new_devotion = new_devotion.saturating_add(3);
                }

                // God has a champion (active divine presence)
                let has_champion = gods.iter()
                    .find(|g| g.god_id == god_id)
                    .is_some_and(|g| g.champion_name.is_some());
                if has_champion && is_patron {
                    new_devotion = new_devotion.saturating_add(2);
                }

                // --- Decreases ---

                // Settlement suffering while god is powerful
                if is_patron && prosperity < 30 && power > 50 {
                    new_devotion = new_devotion.saturating_sub(5);
                }

                // Plague under this god's patronage
                if plague && is_patron {
                    new_devotion = new_devotion.saturating_sub(8);
                }

                // God has faded
                if faded {
                    new_devotion = new_devotion.saturating_sub(10);
                }

                // Not the patron — slow drift down (community pressure toward patron)
                if !is_patron && patron.is_some() {
                    new_devotion = new_devotion.saturating_sub(1);
                }

                new_devotion = new_devotion.min(100);

                // Generate events on threshold crossings
                if old_devotion <= 80 && new_devotion > 80 {
                    person.life_events.push(LifeEvent {
                        year,
                        kind: LifeEventKind::FaithStrengthened { god_id },
                        cause: Some(EventCause::Conditions {
                            settlement_id,
                            detail: "settlement prospering under patron",
                        }),
                    });
                }
                if old_devotion > 20 && new_devotion <= 20 && new_devotion > 0 {
                    person.life_events.push(LifeEvent {
                        year,
                        kind: LifeEventKind::FaithShaken { god_id },
                        cause: if faded {
                            Some(EventCause::Divine { god_id, action: DivineAction::Faded })
                        } else if plague {
                            Some(EventCause::Divine { god_id, action: DivineAction::FlawTriggered })
                        } else {
                            Some(EventCause::Conditions { settlement_id, detail: "suffering while god is powerful" })
                        },
                    });
                }
                if old_devotion > 0 && new_devotion == 0 {
                    person.life_events.push(LifeEvent {
                        year,
                        kind: LifeEventKind::AbandonedFaith { god_id },
                        cause: Some(EventCause::Divine { god_id, action: DivineAction::Faded }),
                    });
                }

                person.set_devotion(god_id, new_devotion);
            }

            // Clean up: remove gods with 0 devotion
            person.faith.retain(|(_, d)| *d > 0);
        }

        // --- Derive settlement patronage from population faith ---
        let mut god_totals: std::collections::HashMap<GodId, (u32, u32)> = std::collections::HashMap::new();
        for &idx in index.residents(settlement_id) {
            let person = &people[idx];
            if !person.is_alive(year) { continue; }
            for &(god_id, devotion) in &person.faith {
                let entry = god_totals.entry(god_id).or_insert((0, 0));
                entry.0 += devotion as u32; // total devotion
                entry.1 += 1;               // follower count
            }
        }

        let settlement = &mut settlements[si];
        if let Some((&best_god, &(total_dev, count))) = god_totals.iter()
            .max_by_key(|(_, (total, _))| *total)
        {
            settlement.patron_god = Some(best_god);
            settlement.devotion = (total_dev / count).min(100);
        } else {
            settlement.patron_god = None;
            settlement.devotion = 0;
        }
    }
}
