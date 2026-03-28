//! Divine wars, pacts, and power drain.

use rand::{Rng, RngExt};

use super::gods::{DrawnPantheon, GodId, GodPool};
use super::state::{DivineWar, GodState, PactKind, DivinePact};
use super::terrain_scars::{DivineTerrainType, TerrainScar, TerrainScarCause};
use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::history::state::WorldState;
use crate::worldgen::history::{EventKind, HistoricEvent};
use crate::worldgen::world_map::{WorldMap, WorldPos};

fn make_event(year: i32, kind: EventKind, description: String, god_ids: Vec<GodId>) -> HistoricEvent {
    HistoricEvent { year, kind, description, participants: vec![], god_participants: god_ids }
}

pub fn evaluate_divine_war_declared(
    year: i32,
    gods: &[GodState],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    active_ids: &[GodId],
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    if active_ids.len() < 2 { return; }
    let Some((a, b, sentiment)) = world_state.divine_relations.most_hostile_pair(active_ids) else { return };
    if sentiment >= -30 { return; }
    if world_state.gods_at_war(a, b) { return; }
    if world_state.god_war_count(a) > 0 || world_state.god_war_count(b) > 0 { return; }

    let hostility_bonus = ((-sentiment - 30) as f32 * 0.8).min(40.0);
    let mut prob = 25.0 + hostility_bonus;

    let a_traits = pantheon.traits(a);
    if a_traits.contains(&CharacterTrait::Warlike) { prob += 20.0; }
    if a_traits.contains(&CharacterTrait::Ambitious) { prob += 10.0; }
    if a_traits.contains(&CharacterTrait::Peaceful) { prob -= 25.0; }
    if a_traits.contains(&CharacterTrait::Diplomatic) { prob -= 15.0; }

    let a_power = gods.iter().find(|g| g.god_id == a).map(|g| g.power).unwrap_or(0);
    let b_power = gods.iter().find(|g| g.god_id == b).map(|g| g.power).unwrap_or(0);
    if a_power > b_power + 20 { prob += 15.0; }

    let prob = (prob / 100.0).clamp(0.05, 0.60);
    if rng.random::<f32>() >= prob { return; }

    let a_territory: std::collections::HashSet<WorldPos> = gods.iter()
        .find(|g| g.god_id == a)
        .map(|g| g.territory.iter().copied().collect())
        .unwrap_or_default();

    let contested: Vec<WorldPos> = a_territory.iter()
        .flat_map(|pos| pos.neighbors())
        .filter(|pos| {
            gods.iter().find(|g| g.god_id == b)
                .is_some_and(|g| g.territory.contains(pos))
        })
        .collect();

    world_state.divine_wars.push(DivineWar {
        aggressor: a, defender: b, start_year: year, contested_cells: contested,
    });
    world_state.divine_relations.modify(a, b, -20);

    let na = pantheon.name(a).unwrap_or("Unknown");
    let nb = pantheon.name(b).unwrap_or("Unknown");
    events.push(make_event(year, EventKind::DivineWarDeclared,
        format!("{} declared war upon {}", na, nb), vec![a, b]));
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_divine_war_resolution(
    year: i32,
    gods: &mut Vec<GodState>,
    events: &mut Vec<HistoricEvent>,
    terrain_scars: &mut Vec<TerrainScar>,
    world_state: &mut WorldState,
    world_map: &mut WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    next_id: &mut u32,
    rng: &mut impl Rng,
) {
    let mut ended_wars: Vec<usize> = Vec::new();

    for (i, war) in world_state.divine_wars.iter().enumerate() {
        let duration = year - war.start_year;
        if duration < 2 { continue; }
        let a_power = gods.iter().find(|g| g.god_id == war.aggressor).map(|g| g.power).unwrap_or(0);
        let b_power = gods.iter().find(|g| g.god_id == war.defender).map(|g| g.power).unwrap_or(0);
        let weakness_bonus = if a_power < 20 || b_power < 20 { 0.30 } else { 0.0 };
        let prob = (0.15 + duration as f32 * 0.05 + weakness_bonus).min(0.90);
        if rng.random::<f32>() < prob {
            ended_wars.push(i);
        }
    }

    for &i in ended_wars.iter().rev() {
        let war = world_state.divine_wars.remove(i);
        let a_power = gods.iter().find(|g| g.god_id == war.aggressor).map(|g| g.power).unwrap_or(0);
        let b_power = gods.iter().find(|g| g.god_id == war.defender).map(|g| g.power).unwrap_or(0);

        let (winner, loser) = if a_power >= b_power {
            (war.aggressor, war.defender)
        } else {
            (war.defender, war.aggressor)
        };

        let nw = pantheon.name(winner).unwrap_or("Unknown").to_string();
        let nl = pantheon.name(loser).unwrap_or("Unknown").to_string();

        if let Some(g) = gods.iter_mut().find(|g| g.god_id == winner) {
            g.wars_fought += 1; g.wars_won += 1;
        }
        if let Some(g) = gods.iter_mut().find(|g| g.god_id == loser) {
            g.wars_fought += 1;
        }

        let aggressor_def = god_pool.get(war.aggressor);
        let scar_type = aggressor_def
            .and_then(|d| d.terrain_influence.future_terrain.as_deref())
            .and_then(DivineTerrainType::from_future_terrain);

        if let Some(dt) = scar_type {
            for &pos in &war.contested_cells {
                if rng.random::<f32>() < 0.45 {
                    if let Some(cell) = world_map.get_mut(pos) {
                        cell.divine_terrain = Some(dt);
                        let scar_id = *next_id; *next_id += 1;
                        terrain_scars.push(TerrainScar {
                            id: scar_id, world_pos: pos, terrain_type: dt,
                            cause: TerrainScarCause::DivineWarBattle, caused_year: year,
                            caused_by: vec![war.aggressor, war.defender],
                            description: format!("Scarred by the war between {} and {}", nw, nl),
                        });
                    }
                }
            }
        }

        world_state.divine_relations.modify(winner, loser, -40);
        events.push(make_event(year, EventKind::DivineWarEnded,
            format!("The divine war ended; {} prevailed over {}", nw, nl),
            vec![winner, loser]));
    }
}

pub fn evaluate_divine_pact(
    year: i32,
    _gods: &[GodState],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    active_ids: &[GodId],
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    if active_ids.len() < 2 { return; }
    if rng.random::<f32>() >= 0.10 { return; }

    let pact_kinds = [PactKind::NonAggression, PactKind::SharedDomain, PactKind::MutualDefense];

    for &a in active_ids {
        for &b in active_ids {
            if a >= b { continue; }
            if !world_state.divine_relations.is_friendly(a, b) { continue; }
            if world_state.gods_have_pact(a, b) { continue; }
            if world_state.gods_at_war(a, b) { continue; }

            let kind = pact_kinds[rng.random_range(0..pact_kinds.len())];
            world_state.divine_pacts.push(DivinePact {
                god_a: a, god_b: b, formed_year: year, kind,
            });
            world_state.divine_relations.modify(a, b, 10);

            let na = pantheon.name(a).unwrap_or("Unknown");
            let nb = pantheon.name(b).unwrap_or("Unknown");
            let kind_str = match kind {
                PactKind::NonAggression => "a pact of non-aggression",
                PactKind::SharedDomain => "a pact to share their domains",
                PactKind::MutualDefense => "a pact of mutual defense",
            };
            events.push(make_event(year, EventKind::PactFormed,
                format!("{} and {} formed {}", na, nb, kind_str), vec![a, b]));
            return;
        }
    }
}

pub fn evaluate_pact_broken(
    year: i32,
    _gods: &[GodState],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    pantheon: &DrawnPantheon,
    rng: &mut impl Rng,
) {
    let mut broken: Vec<usize> = Vec::new();
    for (i, pact) in world_state.divine_pacts.iter().enumerate() {
        let sentiment = world_state.divine_relations.get(pact.god_a, pact.god_b);
        let a_treacherous = pantheon.traits(pact.god_a).contains(&CharacterTrait::Treacherous);
        let b_treacherous = pantheon.traits(pact.god_b).contains(&CharacterTrait::Treacherous);
        let break_prob = if a_treacherous || b_treacherous { 0.35 } else { 0.20 };

        if sentiment < 10 && rng.random::<f32>() < break_prob {
            broken.push(i);
        }
    }

    for &i in broken.iter().rev() {
        let pact = world_state.divine_pacts.remove(i);
        world_state.divine_relations.modify(pact.god_a, pact.god_b, -25);
        let na = pantheon.name(pact.god_a).unwrap_or("Unknown");
        let nb = pantheon.name(pact.god_b).unwrap_or("Unknown");
        events.push(make_event(year, EventKind::PactBroken,
            format!("The pact between {} and {} shattered", na, nb),
            vec![pact.god_a, pact.god_b]));
    }
}

/// Drain power from gods engaged in active divine wars.
pub fn drain_divine_war_power(gods: &mut [GodState], world_state: &WorldState) {
    for war in &world_state.divine_wars {
        if let Some(g) = gods.iter_mut().find(|g| g.god_id == war.aggressor) {
            g.power = g.power.saturating_sub(5);
        }
        if let Some(g) = gods.iter_mut().find(|g| g.god_id == war.defender) {
            g.power = g.power.saturating_sub(5);
        }
    }
}
