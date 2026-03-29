//! Prophet emergence, influence, and martyrdom.
//! Prophets emerge deterministically from faith gap + personality + persistence.
//! They actively influence their settlement's faith over time.

use crate::worldgen::divine::gods::GodId;
use crate::worldgen::divine::personality::DivineDrive;
use crate::worldgen::divine::state::GodState;
use crate::worldgen::history::characters::CharacterTrait::*;
use crate::worldgen::history::state::SettlementState;

use super::index::SettlementIndex;
use super::traits;
use super::types::*;

const FAITH_GAP_THRESHOLD: u8 = 35;
const OUTLIER_YEARS_REQUIRED: u8 = 5;

fn doctrine_for_drive(drive: DivineDrive) -> Doctrine {
    match drive {
        DivineDrive::Worship => Doctrine::SpreadTheWord,
        DivineDrive::Dominion => Doctrine::ConquerForTheGod,
        DivineDrive::Knowledge => Doctrine::SeekTruth,
        DivineDrive::Vindication => Doctrine::PunishUnbelievers,
        DivineDrive::Love => Doctrine::ProtectTheFaithful,
        DivineDrive::Justice => Doctrine::PurifyTheLand,
        DivineDrive::Freedom => Doctrine::BreakTheChains,
        DivineDrive::Legacy => Doctrine::BuildForEternity,
        DivineDrive::Supremacy => Doctrine::ProveSupremacy,
        DivineDrive::Perfection => Doctrine::AchievePerfection,
    }
}

/// Has the right personality to become a prophet.
/// Only people who would actually speak up and persuade others.
fn has_prophet_traits(person: &Person) -> bool {
    person.traits.contains(&Charismatic)
        || person.traits.contains(&Fanatical)
}

/// Check for prophet emergence and apply prophet influence.
pub fn evaluate_prophets(
    people: &mut [Person],
    index: &SettlementIndex,
    settlements: &[SettlementState],
    gods: &[GodState],
    year: i32,
) {
    // Precompute average devotion per (settlement, god) — O(n) instead of O(n²)
    let mut avg_cache: std::collections::HashMap<(u32, GodId), u8> = std::collections::HashMap::new();
    for settlement in settlements {
        if settlement.destroyed_year.is_some() { continue; }
        let mut god_totals: std::collections::HashMap<GodId, (u32, u32)> = std::collections::HashMap::new();
        for &idx in index.residents(settlement.id) {
            let p = &people[idx];
            if !p.is_alive(year) { continue; }
            for &(god_id, devotion) in &p.faith {
                let entry = god_totals.entry(god_id).or_insert((0, 0));
                entry.0 += devotion as u32;
                entry.1 += 1;
            }
        }
        for (god_id, (total, count)) in god_totals {
            avg_cache.insert((settlement.id, god_id), (total / count).min(100) as u8);
        }
    }

    // Phase 1: Check for new prophet emergence
    let mut new_prophets: Vec<(usize, GodId, ProphetKind)> = Vec::new();

    for settlement in settlements {
        if settlement.destroyed_year.is_some() { continue; }

        for &idx in index.residents(settlement.id) {
            let person = &people[idx];
            if !person.is_alive(year) { continue; }
            if person.prophet_of.is_some() { continue; }
            if person.age(year) < 16 { continue; }
            if !has_prophet_traits(person) { continue; }

            // Check zealot potential
            if let Some(primary) = person.primary_god() {
                let person_dev = person.devotion_to(primary);
                let avg_dev = avg_cache.get(&(settlement.id, primary)).copied().unwrap_or(0);

                if person_dev > avg_dev + FAITH_GAP_THRESHOLD {
                    if people[idx].years_as_outlier >= OUTLIER_YEARS_REQUIRED {
                        new_prophets.push((idx, primary, ProphetKind::Zealot));
                        continue;
                    }
                }
            }

            // Check heretic potential
            if person.traits.contains(&Skeptical) {
                if let Some(patron) = settlement.patron_god {
                    let avg_dev = avg_cache.get(&(settlement.id, patron)).copied().unwrap_or(0);
                    let person_dev = person.devotion_to(patron);

                    if avg_dev > person_dev + FAITH_GAP_THRESHOLD {
                        if people[idx].years_as_outlier >= OUTLIER_YEARS_REQUIRED {
                            new_prophets.push((idx, patron, ProphetKind::Heretic));
                            continue;
                        }
                    }
                }
            }
        }
    }

    // Apply new prophet status
    for (idx, god_id, kind) in &new_prophets {
        let doctrine = match kind {
            ProphetKind::Zealot => {
                gods.iter().find(|g| g.god_id == *god_id)
                    .map(|g| doctrine_for_drive(g.drive()))
                    .unwrap_or(Doctrine::SpreadTheWord)
            }
            ProphetKind::Heretic => {
                if people[*idx].faith.is_empty() {
                    Doctrine::GodsAreFalse
                } else {
                    Doctrine::GodHasAbandoned
                }
            }
        };

        people[*idx].prophet_of = Some(ProphetRole {
            god_id: *god_id,
            kind: *kind,
            doctrine,
            became_prophet_year: year,
        });
        people[*idx].years_as_outlier = 0;
        people[*idx].life_events.push(LifeEvent {
            year,
            kind: LifeEventKind::BecameProphet { god_id: *god_id, kind: *kind },
            cause: Some(EventCause::Conditions {
                settlement_id: people[*idx].settlement_id,
                detail: match kind {
                    ProphetKind::Zealot => "faith outlier — more devout than community",
                    ProphetKind::Heretic => "faith outlier — lost faith while community believes",
                },
            }),
        });
    }

    // Phase 2: Update outlier years for non-prophets
    for settlement in settlements {
        if settlement.destroyed_year.is_some() { continue; }

        for &idx in index.residents(settlement.id) {
            let person = &people[idx];
            if !person.is_alive(year) || person.prophet_of.is_some() { continue; }

            let is_outlier = if let Some(primary) = person.primary_god() {
                let person_dev = person.devotion_to(primary);
                let avg_dev = avg_cache.get(&(settlement.id, primary)).copied().unwrap_or(0);
                person_dev > avg_dev + FAITH_GAP_THRESHOLD
            } else if person.traits.contains(&Skeptical) {
                if let Some(patron) = settlement.patron_god {
                    let avg_dev = avg_cache.get(&(settlement.id, patron)).copied().unwrap_or(0);
                    let person_dev = person.devotion_to(patron);
                    avg_dev > person_dev + FAITH_GAP_THRESHOLD
                } else { false }
            } else { false };

            if is_outlier {
                people[idx].years_as_outlier = people[idx].years_as_outlier.saturating_add(1);
            } else {
                people[idx].years_as_outlier = 0;
            }
        }
    }

    // Phase 3: Prophet influence on settlement
    apply_prophet_influence(people, index, settlements, year);

    // Phase 4: Check for martyrdom
    check_martyrdom(people, index, settlements, year);
}

/// Active prophets influence their settlement's faith.
fn apply_prophet_influence(
    people: &mut [Person],
    index: &SettlementIndex,
    settlements: &[SettlementState],
    year: i32,
) {
    // Collect prophet info first
    let prophets: Vec<(u32, GodId, ProphetKind, bool, u32)> = people.iter()
        .filter(|p| p.is_alive(year) && p.prophet_of.is_some())
        .map(|p| {
            let role = p.prophet_of.as_ref().unwrap();
            let is_charismatic = p.traits.contains(&Charismatic);
            (p.settlement_id, role.god_id, role.kind, is_charismatic, p.id)
        })
        .collect();

    for (settlement_id, god_id, kind, is_charismatic, _prophet_id) in &prophets {
        let settlement = match settlements.iter().find(|s| s.id == *settlement_id) {
            Some(s) => s,
            None => continue,
        };
        if settlement.destroyed_year.is_some() { continue; }

        let bonus = if *is_charismatic { 1u8 } else { 0 };

        for &idx in index.residents(*settlement_id) {
            let person = &mut people[idx];
            if !person.is_alive(year) { continue; }
            if person.prophet_of.is_some() { continue; } // prophets don't influence themselves

            match kind {
                ProphetKind::Zealot => {
                    // Skeptical people are resistant
                    if person.traits.contains(&Skeptical) { continue; }
                    let gain = if person.traits.contains(&Devout) { 4 + bonus } else { 2 + bonus };
                    let current = person.devotion_to(*god_id);
                    person.set_devotion(*god_id, current.saturating_add(gain));
                }
                ProphetKind::Heretic => {
                    // Fanatical people are resistant
                    if person.traits.contains(&Fanatical) { continue; }
                    let loss = if person.traits.contains(&Skeptical) { 4 + bonus } else { 2 + bonus };
                    let current = person.devotion_to(*god_id);
                    person.set_devotion(*god_id, current.saturating_sub(loss));
                }
            }
        }
    }
}

/// Check if any prophets died this year and apply martyrdom effects.
fn check_martyrdom(
    people: &mut [Person],
    index: &SettlementIndex,
    _settlements: &[SettlementState],
    year: i32,
) {
    // Find prophets who died this year — use death_cause field directly
    let martyrs: Vec<(u32, u32, GodId, ProphetKind, DeathCause)> = people.iter()
        .filter(|p| p.death_year == Some(year) && p.prophet_of.is_some())
        .map(|p| {
            let role = p.prophet_of.as_ref().unwrap();
            let cause = p.death_cause.unwrap_or(DeathCause::OldAge);
            (p.id, p.settlement_id, role.god_id, role.kind, cause)
        })
        .collect();

    for (prophet_id, settlement_id, god_id, kind, death_cause) in martyrs {
        let is_violent = matches!(death_cause,
            DeathCause::War | DeathCause::Violence | DeathCause::Monster
        );

        if !is_violent { continue; } // natural death — no martyrdom

        // Martyrdom: boost devotion for all followers in the settlement
        for &idx in index.residents(settlement_id) {
            let person = &mut people[idx];
            if !person.is_alive(year) { continue; }

            match kind {
                ProphetKind::Zealot => {
                    let current = person.devotion_to(god_id);
                    person.set_devotion(god_id, current.saturating_add(20));

                    // Devout followers may become Fanatical
                    if person.traits.contains(&Devout) && !person.traits.contains(&Fanatical) {
                        traits::earn_trait(person, Fanatical);
                    }
                }
                ProphetKind::Heretic => {
                    let current = person.devotion_to(god_id);
                    person.set_devotion(god_id, current.saturating_sub(20));

                    // Skeptical followers become more entrenched
                    if !person.traits.contains(&Skeptical) {
                        // Witnessing a heretic's martyrdom can create doubt
                        if person.devotion_to(god_id) < 30 {
                            traits::earn_trait(person, Skeptical);
                        }
                    }
                }
            }

            person.life_events.push(LifeEvent {
                year,
                kind: LifeEventKind::WitnessedMartyrdom { prophet_id, god_id },
                cause: Some(EventCause::PersonAction {
                    person_id: prophet_id,
                    role: "martyred prophet",
                }),
            });
        }
    }
}
