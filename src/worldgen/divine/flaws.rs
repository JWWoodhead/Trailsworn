//! Flaw pressure accumulation and flaw trigger events.

use rand::{Rng, RngExt};

use super::gods::{DrawnPantheon, GodId};
use crate::worldgen::world_map::WorldPos;
use super::personality::DivineFlaw;
use super::state::GodState;
use crate::worldgen::history::state::{SettlementState, WorldState};
use crate::worldgen::history::{CrossDomainEvent, EventKind, HistoricEvent};

fn make_event(year: i32, kind: EventKind, description: String, god_ids: Vec<GodId>) -> HistoricEvent {
    HistoricEvent { year, kind, description, participants: vec![], god_participants: god_ids, cause: None }
}

pub fn accumulate_flaw_pressure(
    gods: &mut [GodState],
    new_events: &[HistoricEvent],
    world_state: &WorldState,
) {
    use DivineFlaw::*;

    let active_god_ids: Vec<GodId> = gods.iter().filter(|g| g.is_active()).map(|g| g.god_id).collect();

    let pressure_gains: Vec<(usize, u32)> = gods.iter().enumerate()
        .filter(|(_, g)| g.is_active())
        .map(|(gi, god)| {
            let flaw = god.flaw();
            let god_id = god.god_id;

            let gain: u32 = match flaw {
                Hubris => {
                    new_events.iter().filter(|e| {
                        e.god_participants.contains(&god_id)
                            && matches!(e.kind, EventKind::DivineWarEnded | EventKind::DivineArtifactForged)
                    }).count() as u32 * 8
                }
                Jealousy => {
                    new_events.iter().filter(|e| {
                        !e.god_participants.contains(&god_id)
                            && matches!(e.kind, EventKind::WorshipEstablished | EventKind::DivineArtifactForged | EventKind::RaceCreated)
                    }).count() as u32 * 5
                }
                Obsession => 3,
                Cruelty => {
                    new_events.iter().filter(|e| {
                        e.god_participants.contains(&god_id)
                            && matches!(e.kind, EventKind::TerritoryContested | EventKind::PactBroken)
                    }).count() as u32 * 8
                }
                Blindness => 2,
                Isolation => {
                    let others: Vec<GodId> = active_god_ids.iter().copied().filter(|&id| id != god_id).collect();
                    let avg_sentiment: i32 = if others.is_empty() { 0 } else {
                        others.iter().map(|&id| world_state.divine_relations.get(god_id, id)).sum::<i32>() / others.len() as i32
                    };
                    if avg_sentiment < 0 { (-avg_sentiment / 10) as u32 } else { 1 }
                }
                Betrayal => {
                    let has_pact = world_state.divine_pacts.iter().any(|p| p.god_a == god_id || p.god_b == god_id);
                    if has_pact { 5 } else { 1 }
                }
                Sacrifice => if god.power < 40 { 5 } else { 1 },
                Rigidity => {
                    let disruptions = new_events.iter().filter(|e| {
                        matches!(e.kind, EventKind::DivineWarDeclared | EventKind::PactBroken)
                    }).count() as u32;
                    2 + disruptions * 3
                }
                Hollowness => {
                    new_events.iter().filter(|e| {
                        e.god_participants.contains(&god_id)
                            && matches!(e.kind, EventKind::DivineArtifactForged | EventKind::SacredSiteCreated)
                    }).count() as u32 * 6
                }
            };
            (gi, gain)
        })
        .collect();

    for (gi, gain) in pressure_gains {
        gods[gi].flaw_pressure = (gods[gi].flaw_pressure + gain).min(100);
    }
}

pub fn evaluate_flaw_triggers(
    year: i32,
    gods: &mut [GodState],
    events: &mut Vec<HistoricEvent>,
    settlements: &[SettlementState],
    cross_events: &mut Vec<CrossDomainEvent>,
    world_state: &mut WorldState,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    use DivineFlaw::*;

    let god_count = gods.len();
    for gi in 0..god_count {
        if !gods[gi].is_active() { continue; }
        if gods[gi].flaw_pressure < 80 { continue; }
        let trigger_prob = (gods[gi].flaw_pressure as f32 - 70.0) / 100.0;
        if rng.random::<f32>() >= trigger_prob { continue; }

        let flaw = gods[gi].flaw();
        let god_id = gods[gi].god_id;
        let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();

        match flaw {
            Hubris => {
                gods[gi].power = gods[gi].power.saturating_sub(15);
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{}, drunk on their own power, overreached and was diminished", god_name),
                    vec![god_id]));
            }
            Jealousy => {
                let target = gods.iter()
                    .filter(|g| g.is_active() && g.god_id != god_id)
                    .max_by_key(|g| g.worshipper_settlements.len());
                if let Some(t) = target {
                    let target_id = t.god_id;
                    let target_name = pantheon.name(target_id).unwrap_or("another god").to_string();
                    world_state.divine_relations.modify(god_id, target_id, -20);
                    events.push(make_event(year, EventKind::NarrativeAdvanced,
                        format!("{}, consumed by jealousy, turned against {} for having what they could not", god_name, target_name),
                        vec![god_id, target_id]));
                }
            }
            Obsession => {
                for (si, s) in settlements.iter().enumerate() {
                    if s.patron_god == Some(god_id) {
                        cross_events.push(CrossDomainEvent::DevotionChanged {
                            settlement_index: si, delta: -10,
                        });
                    }
                }
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{}, lost in obsession, neglected those who worshipped them", god_name),
                    vec![god_id]));
            }
            Cruelty => {
                for (si, s) in settlements.iter().enumerate() {
                    if s.patron_god == Some(god_id) {
                        cross_events.push(CrossDomainEvent::DevotionChanged {
                            settlement_index: si, delta: -15,
                        });
                    }
                }
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} lashed out in fury, and their own followers suffered for it", god_name),
                    vec![god_id]));
            }
            Blindness => {
                let other = gods.iter()
                    .filter(|g| g.is_active() && g.god_id != god_id)
                    .nth(rng.random_range(0..gods.iter().filter(|g| g.is_active() && g.god_id != god_id).count().max(1)));
                if let Some(t) = other {
                    let tid = t.god_id;
                    let tname = pantheon.name(tid).unwrap_or("another god").to_string();
                    world_state.divine_relations.modify(god_id, tid, -15);
                    events.push(make_event(year, EventKind::NarrativeAdvanced,
                        format!("{}, blind to the consequences, unknowingly trespassed against {}", god_name, tname),
                        vec![god_id, tid]));
                }
            }
            Isolation => {
                let lost: Vec<WorldPos> = gods[gi].worshipper_settlements.clone();
                for pos in &lost {
                    if let Some(si) = settlements.iter().position(|s| s.world_pos == Some(*pos) && s.patron_god == Some(god_id)) {
                        cross_events.push(CrossDomainEvent::DevotionChanged {
                            settlement_index: si, delta: -20,
                        });
                    }
                }
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} withdrew from the world, becoming distant and unreachable", god_name),
                    vec![god_id]));
            }
            Betrayal => {
                let pact_idx = world_state.divine_pacts.iter().position(|p| p.god_a == god_id || p.god_b == god_id);
                if let Some(idx) = pact_idx {
                    let pact = world_state.divine_pacts.remove(idx);
                    let other_id = if pact.god_a == god_id { pact.god_b } else { pact.god_a };
                    let other_name = pantheon.name(other_id).unwrap_or("another god").to_string();
                    world_state.divine_relations.modify(god_id, other_id, -30);
                    events.push(make_event(year, EventKind::PactBroken,
                        format!("{} betrayed {}, shattering the trust between them", god_name, other_name),
                        vec![god_id, other_id]));
                }
            }
            Sacrifice => {
                gods[gi].power = gods[gi].power.saturating_sub(20);
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} sacrificed a piece of themselves in pursuit of their deepest desire", god_name),
                    vec![god_id]));
            }
            Rigidity => {
                for other in gods.iter().filter(|g| g.is_active() && g.god_id != god_id) {
                    world_state.divine_relations.modify(god_id, other.god_id, -5);
                }
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} refused to bend, and the other gods grew weary of their inflexibility", god_name),
                    vec![god_id]));
            }
            Hollowness => {
                gods[gi].power = gods[gi].power.saturating_sub(10);
                events.push(make_event(year, EventKind::NarrativeAdvanced,
                    format!("{} achieved what they sought, and found it meant nothing", god_name),
                    vec![god_id]));
            }
        }

        gods[gi].flaw_pressure = 20;
    }
}
