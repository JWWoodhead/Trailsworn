//! Drive-based divine actions: temples, artifacts, champions, sacred sites, races.

use rand::{Rng, RngExt};

use super::artifacts::{divine_artifact_name, ArtifactLocation, DivineArtifact, DivineArtifactKind};
use super::gods::{DrawnPantheon, GodId, GodPool};
use super::personality::DivineDrive;
use super::races::CreatedRace;
use super::sites::{divine_site_name, DivineSite, DivineSiteKind};
use super::state::GodState;
use super::terrain_scars::DivineTerrainType;
use crate::worldgen::history::state::WorldState;
use crate::worldgen::history::{EventKind, HistoricEvent};
use crate::worldgen::names::{full_name, Race};
use crate::worldgen::population_table::PopTable;
use crate::worldgen::world_map::WorldMap;

fn make_event(year: i32, kind: EventKind, description: String, god_ids: Vec<GodId>) -> HistoricEvent {
    HistoricEvent { year, kind, description, participants: vec![], god_participants: god_ids }
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_drive_actions(
    year: i32,
    race_window: (i32, i32),
    gods: &mut Vec<GodState>,
    events: &mut Vec<HistoricEvent>,
    sites: &mut Vec<DivineSite>,
    artifacts: &mut Vec<DivineArtifact>,
    created_races: &mut Vec<CreatedRace>,
    _world_state: &WorldState,
    _world_map: &WorldMap,
    god_pool: &GodPool,
    pantheon: &DrawnPantheon,
    next_id: &mut u32,
    rng: &mut impl Rng,
) {
    let god_count = gods.len();
    for gi in 0..god_count {
        if !gods[gi].is_active() { continue; }
        if gods[gi].power < 10 { continue; }
        let drive = gods[gi].drive();

        match drive {
            DivineDrive::Knowledge => {
                eval_sacred_site(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.12);
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.10);
            }
            DivineDrive::Dominion => {
                eval_champion(year, gi, gods, events, pantheon, rng, 0.08);
                eval_temple(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.12);
            }
            DivineDrive::Worship => {
                eval_temple(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.15);
                eval_champion(year, gi, gods, events, pantheon, rng, 0.08);
            }
            DivineDrive::Perfection => {
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.15);
                eval_sacred_site(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.10);
            }
            DivineDrive::Justice => {
                eval_temple(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.10);
                eval_champion(year, gi, gods, events, pantheon, rng, 0.06);
            }
            DivineDrive::Love => {
                eval_champion(year, gi, gods, events, pantheon, rng, 0.06);
                if year >= race_window.0 && year <= race_window.1 {
                    eval_race(year, gi, gods, events, created_races, god_pool, pantheon, next_id, rng, 0.10);
                }
            }
            DivineDrive::Freedom => {
                eval_sacred_site(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.08);
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.06);
            }
            DivineDrive::Legacy => {
                if year >= race_window.0 && year <= race_window.1 {
                    eval_race(year, gi, gods, events, created_races, god_pool, pantheon, next_id, rng, 0.12);
                }
                eval_sacred_site(year, gi, gods, events, sites, god_pool, pantheon, next_id, rng, 0.10);
            }
            DivineDrive::Vindication => {
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.12);
                eval_champion(year, gi, gods, events, pantheon, rng, 0.08);
            }
            DivineDrive::Supremacy => {
                eval_artifact(year, gi, gods, events, artifacts, god_pool, pantheon, next_id, rng, 0.10);
                eval_champion(year, gi, gods, events, pantheon, rng, 0.10);
            }
        }
    }
}

fn eval_temple(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    sites: &mut Vec<DivineSite>, _god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].power < 40 || gods[gi].territory.is_empty() { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let pos = gods[gi].territory[rng.random_range(0..gods[gi].territory.len())];
    let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();
    let site_id = *next_id; *next_id += 1;
    let name = divine_site_name(DivineSiteKind::Temple, &god_name, rng);
    sites.push(DivineSite {
        id: site_id, name: name.clone(), kind: DivineSiteKind::Temple,
        world_pos: pos, creator_god: god_id, created_year: year,
        persists: true, description: format!("{} established {}", god_name, name),
        terrain_effect: None,
    });
    gods[gi].sites_created += 1;
    events.push(make_event(year, EventKind::TempleEstablished,
        format!("{} founded {}", god_name, name), vec![god_id]));
}

fn eval_champion(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    pantheon: &DrawnPantheon, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].power < 50 || gods[gi].champion_name.is_some() { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let race_table = PopTable::pick_one(vec![
        (Race::Human, 40.0), (Race::Dwarf, 20.0), (Race::Elf, 15.0),
        (Race::Orc, 15.0), (Race::Goblin, 10.0),
    ]);
    let race = race_table.roll_one(rng).unwrap();
    let name = full_name(race, rng);
    let god_name = pantheon.name(god_id).unwrap_or("A god");

    gods[gi].champion_name = Some(name.clone());
    gods[gi].champion_race = Some(race);
    events.push(make_event(year, EventKind::ChampionChosen,
        format!("{} chose {} as their mortal champion", god_name, name), vec![god_id]));
}

#[allow(clippy::too_many_arguments)]
fn eval_race(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    created_races: &mut Vec<CreatedRace>, god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].created_race_id.is_some() || gods[gi].power < 60 { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let god_def = match god_pool.get(god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();
    let race_id = *next_id; *next_id += 1;
    let core_territory = gods[gi].core_territory.clone();
    let race = super::races::race_template(race_id, god_def, &god_name, year, &core_territory, rng);

    gods[gi].created_race_id = Some(race_id);
    events.push(make_event(year, EventKind::RaceCreated,
        format!("{} created the {}", god_name, race.name), vec![god_id]));
    created_races.push(race);
}

#[allow(clippy::too_many_arguments)]
fn eval_artifact(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    artifacts: &mut Vec<DivineArtifact>, god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].power < 30 { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let power = gods[gi].power;
    let has_champion = gods[gi].champion_name.is_some();
    let god_def = match god_pool.get(god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();

    let kind_table = PopTable::pick_one(vec![
        (DivineArtifactKind::Weapon, 30.0), (DivineArtifactKind::Armor, 25.0),
        (DivineArtifactKind::Implement, 20.0), (DivineArtifactKind::Key, 10.0),
        (DivineArtifactKind::Vessel, 15.0),
    ]);
    let kind = kind_table.roll_one(rng).unwrap();
    let power_level = (power / 20).clamp(1, 5);
    let name = divine_artifact_name(kind, god_def.domain, rng);
    let artifact_id = *next_id; *next_id += 1;

    let location = if has_champion && rng.random::<f32>() < 0.3 {
        ArtifactLocation::HeldByChampion(god_id)
    } else { ArtifactLocation::Lost };

    artifacts.push(DivineArtifact {
        id: artifact_id, name: name.clone(), kind, creator_god: god_id,
        created_year: year, magic_school: god_def.domain, power_level, location,
        description: format!("{} forged {}", god_name, name),
        lore: format!("Created by {} in the age of gods", god_name),
    });
    gods[gi].artifacts_created += 1;
    events.push(make_event(year, EventKind::DivineArtifactForged,
        format!("{} forged the {}", god_name, name), vec![god_id]));
}

#[allow(clippy::too_many_arguments)]
fn eval_sacred_site(
    year: i32, gi: usize, gods: &mut [GodState], events: &mut Vec<HistoricEvent>,
    sites: &mut Vec<DivineSite>, god_pool: &GodPool, pantheon: &DrawnPantheon,
    next_id: &mut u32, rng: &mut impl Rng, prob: f32,
) {
    if gods[gi].territory.is_empty() { return; }
    if rng.random::<f32>() >= prob { return; }

    let god_id = gods[gi].god_id;
    let god_def = match god_pool.get(god_id) { Some(d) => d, None => return };
    let god_name = pantheon.name(god_id).unwrap_or("A god").to_string();
    let kind = DivineSiteKind::for_domain(god_def.domain);
    let pos = gods[gi].territory[rng.random_range(0..gods[gi].territory.len())];
    let site_id = *next_id; *next_id += 1;
    let divine_terrain = god_def.terrain_influence.future_terrain.as_deref()
        .and_then(DivineTerrainType::from_future_terrain);
    let name = divine_site_name(kind, &god_name, rng);

    sites.push(DivineSite {
        id: site_id, name: name.clone(), kind, world_pos: pos,
        creator_god: god_id, created_year: year, persists: true,
        description: format!("{} created {}", god_name, name),
        terrain_effect: divine_terrain,
    });
    gods[gi].sites_created += 1;
    events.push(make_event(year, EventKind::SacredSiteCreated,
        format!("{} created {}", god_name, name), vec![god_id]));
}
