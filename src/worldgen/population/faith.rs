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
            // But don't re-adopt a god the person has already abandoned
            if let Some(patron_god) = patron {
                if person.devotion_to(patron_god) == 0 {
                    let already_abandoned = person.life_events.iter()
                        .any(|e| matches!(&e.kind, LifeEventKind::AbandonedFaith { god_id } if *god_id == patron_god));
                    if !already_abandoned {
                        person.set_devotion(patron_god, 15); // community exposure
                    }
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

                // No unconditional non-patron drift — people hold their faith
                // unless there's active pressure (prophet, conditions, fading)

                new_devotion = new_devotion.min(100);

                // Determine the contextual cause for any faith decrease
                let decrease_cause = if faded {
                    Some(EventCause::Divine { god_id, action: DivineAction::Faded })
                } else if plague {
                    Some(EventCause::Divine { god_id, action: DivineAction::FlawTriggered })
                } else if !is_patron && prosperity < 30 {
                    Some(EventCause::Conditions { settlement_id, detail: "suffering while god is powerful" })
                } else {
                    None
                };

                // Generate events on threshold crossings (each fires ONCE per god)
                if old_devotion <= 80 && new_devotion > 80 {
                    let already_strengthened = person.life_events.iter()
                        .any(|e| matches!(&e.kind, LifeEventKind::FaithStrengthened { god_id: g } if *g == god_id));
                    if !already_strengthened {
                        person.life_events.push(LifeEvent {
                            year,
                            kind: LifeEventKind::FaithStrengthened { god_id },
                            cause: Some(EventCause::Conditions {
                                settlement_id,
                                detail: "settlement prospering under patron",
                            }),
                        });
                    }
                }
                if old_devotion > 20 && new_devotion <= 20 && new_devotion > 0 {
                    let already_shaken = person.life_events.iter()
                        .any(|e| matches!(&e.kind, LifeEventKind::FaithShaken { god_id: g } if *g == god_id));
                    if !already_shaken {
                        person.life_events.push(LifeEvent {
                            year,
                            kind: LifeEventKind::FaithShaken { god_id },
                            cause: decrease_cause.clone(),
                        });
                    }
                }
                if old_devotion > 0 && new_devotion == 0 {
                    let already_abandoned = person.life_events.iter()
                        .any(|e| matches!(&e.kind, LifeEventKind::AbandonedFaith { god_id: g } if *g == god_id));
                    if !already_abandoned {
                        person.life_events.push(LifeEvent {
                            year,
                            kind: LifeEventKind::AbandonedFaith { god_id },
                            cause: decrease_cause,
                        });
                    }
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

        // --- Derive dominant race from population ---
        let mut race_counts: std::collections::HashMap<crate::worldgen::names::Race, u32> = std::collections::HashMap::new();
        for &idx in index.residents(settlement_id) {
            let person = &people[idx];
            if !person.is_alive(year) { continue; }
            *race_counts.entry(person.race).or_insert(0) += 1;
        }

        let settlement = &mut settlements[si];

        // Patron hysteresis — incumbent god keeps patronage unless challenger
        // exceeds their total devotion by 10%. Prevents oscillation.
        let current_patron_total = settlement.patron_god
            .and_then(|pg| god_totals.get(&pg))
            .map(|(total, _)| *total)
            .unwrap_or(0);

        if let Some((&best_god, &(total_dev, count))) = god_totals.iter()
            .max_by_key(|(_, (total, _))| *total)
        {
            let is_incumbent = settlement.patron_god == Some(best_god);
            let exceeds_threshold = total_dev > (current_patron_total as f32 * 1.1) as u32;

            if is_incumbent || exceeds_threshold || settlement.patron_god.is_none() {
                settlement.patron_god = Some(best_god);
            }
            settlement.devotion = (total_dev / count).min(100);
        } else {
            settlement.patron_god = None;
            settlement.devotion = 0;
        }

        settlement.dominant_race = race_counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(race, _)| race);
    }
}
