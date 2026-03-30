//! World event evaluation — faction wars, plagues, alliances, heroes, artifacts,
//! settlement founding, and all other mortal-world simulation functions.

use rand::{Rng, RngExt};

use crate::worldgen::names::{FactionType, Race, faction_name, settlement_name};
use crate::worldgen::world_map::WorldPos;
use crate::worldgen::population::faction_stats::{FactionStats, FormationCandidates, ProphetTensions};
use crate::worldgen::population_table::PopTable;

use super::artifacts::*;
use super::characters::{*, Ambition};
use super::state::*;
use super::{EventKind, HistoricEvent};

/// Returns `(person_id, faction_id)` pairs for people who founded factions
/// this year, so the caller can shift their allegiance.
pub(super) fn simulate_year(
    year: i32,
    factions: &mut Vec<FactionState>,
    settlements: &mut Vec<SettlementState>,
    characters: &mut Vec<Character>,
    artifacts_list: &mut Vec<Artifact>,
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState,
    regions: &[String],
    next_id: &mut u32,
    _faction_type_table: &PopTable<FactionType>,
    _race_table: &PopTable<Race>,
    faction_stats: Option<&FactionStats>,
    prophet_tensions: Option<&ProphetTensions>,
    formation_candidates: Option<&FormationCandidates>,
    rng: &mut impl Rng,
) -> Vec<(u32, u32)> {
    let living: Vec<u32> = factions.iter().filter(|f| f.is_alive(year)).map(|f| f.id).collect();
    if living.is_empty() { return Vec::new(); }

    // Phase 0: Aging and death
    let mut dead_leaders: Vec<(u32, u32)> = Vec::new(); // (faction_id, character_id)
    for character in characters.iter_mut() {
        if !character.is_alive(year) { continue; }
        if character.natural_death_check(year, rng) {
            character.death_year = Some(year);
            // Check if this was a faction leader
            if character.role == CharacterRole::Leader {
                if let Some(fid) = character.faction_id {
                    dead_leaders.push((fid, character.id));
                }
            }
        }
    }

    // Handle leader succession for dead leaders
    for (faction_id, dead_leader_id) in &dead_leaders {
        let faction = match factions.iter().find(|f| f.id == *faction_id) {
            Some(f) => f,
            None => continue,
        };
        if !faction.is_alive(year) { continue; }
        let race = faction.race;
        let fname = faction.name.clone();
        let dead_name = characters.iter()
            .find(|c| c.id == *dead_leader_id)
            .map(|c| c.full_display_name())
            .unwrap_or_else(|| "Unknown".into());

        // Try to promote an existing notable member
        let successor = characters.iter_mut()
            .filter(|c| c.is_alive(year) && c.faction_id == Some(*faction_id) && c.role != CharacterRole::Leader)
            .max_by_key(|c| c.renown);

        let new_leader_name = if let Some(s) = successor {
            s.role = CharacterRole::Leader;
            let name = s.full_display_name();
            let sid = s.id;
            if let Some(f) = factions.iter_mut().find(|f| f.id == *faction_id) {
                f.leader_id = Some(sid);
                f.leader_name = name.clone();
            }
            name
        } else {
            // Generate a new character
            let new_id = *next_id;
            *next_id += 1;
            let birth = year - rng.random_range(25..40);
            let mut new_leader = generate_character(new_id, race, CharacterRole::Leader, Some(*faction_id), birth, rng);
            new_leader.epithet = Some(generate_epithet(&new_leader, rng));
            let name = new_leader.full_display_name();
            characters.push(new_leader);
            if let Some(f) = factions.iter_mut().find(|f| f.id == *faction_id) {
                f.leader_id = Some(new_id);
                f.leader_name = name.clone();
            }
            name
        };

        // Succession crisis: multiple ambitious claimants cause instability
        let claimants = characters.iter()
            .filter(|c| c.is_alive(year) && c.faction_id == Some(*faction_id) && c.role != CharacterRole::Leader)
            .filter(|c| c.has_trait(CharacterTrait::Ambitious) || c.has_trait(CharacterTrait::PowerHungry))
            .count();

        if claimants >= 2 {
            // Succession crisis accelerates potential dissolution
            if let Some(f) = factions.iter_mut().find(|f| f.id == *faction_id) {
                f.unhappy_years = f.unhappy_years.saturating_add(2);
            }
            events.push(HistoricEvent {
                year, kind: EventKind::LeaderChanged,
                description: format!("A succession crisis erupted in {} after the death of {}, with {} claimed power",
                    fname, dead_name, new_leader_name),
                participants: vec![*faction_id],
                god_participants: vec![],
                cause: None,
            });
        } else {
            events.push(HistoricEvent {
                year, kind: EventKind::LeaderChanged,
                description: format!("After the death of {}, {} became leader of {}", dead_name, new_leader_name, fname),
                participants: vec![*faction_id],
                god_participants: vec![],
                cause: None,
            });
        }
    }

    // Snapshot for cross-references during upkeep
    let factions_snapshot: Vec<FactionState> = factions.clone();

    // Phase 1: Faction upkeep — gauges derived from population, sentiment from proximity/faith
    for faction in factions.iter_mut() {
        if !faction.is_alive(year) { continue; }

        // Update gauges from real population data (military, wealth, stability, patron god)
        if let Some(stats) = faction_stats {
            faction.update_from_stats(stats);
        }

        // Track sustained unhappiness for dissolution
        if faction.stability < 20 {
            faction.unhappy_years = faction.unhappy_years.saturating_add(1);
        } else {
            faction.unhappy_years = 0;
        }

        // Territorial & religious friction between factions
        for other in &living {
            if *other == faction.id { continue; }
            if let Some(other_f) = factions_snapshot.iter().find(|f| f.id == *other) {
                // Same region — proximity breeds friction
                if other_f.home_region == faction.home_region {
                    world_state.relations.modify(faction.id, *other, -2);
                }
                // Both have settlements — competing for resources and influence
                if !faction.settlements.is_empty() && !other_f.settlements.is_empty() {
                    world_state.relations.modify(faction.id, *other, -1);
                }
                // God-based sentiment
                if let (Some(our_god), Some(their_god)) = (faction.patron_god, other_f.patron_god) {
                    if our_god == their_god {
                        world_state.relations.modify(faction.id, *other, 2);
                    } else {
                        let divine_sentiment = world_state.divine_relations.get(our_god, their_god);
                        if divine_sentiment < -30 {
                            world_state.relations.modify(faction.id, *other, -3);
                        } else if divine_sentiment > 30 {
                            world_state.relations.modify(faction.id, *other, 1);
                        }
                    }
                }
            }
        }
    }

    // Prophet-driven religious tension: prophets inflame hostility with rival-god factions
    if let Some(tensions) = prophet_tensions {
        for &(prophet_faction, _sid, prophet_god) in &tensions.active_prophets {
            for &other_id in &living {
                if other_id == prophet_faction { continue; }
                if let Some(other_f) = factions_snapshot.iter().find(|f| f.id == other_id) {
                    if let Some(their_god) = other_f.patron_god {
                        if their_god != prophet_god {
                            let divine_sentiment = world_state.divine_relations.get(prophet_god, their_god);
                            if divine_sentiment < -30 {
                                world_state.relations.modify(prophet_faction, other_id, -5);
                            }
                        }
                    }
                }
            }
        }
    }

    // Faction type-specific sentiment modifiers
    for faction in factions.iter() {
        if !faction.is_alive(year) { continue; }
        for &other_id in &living {
            if other_id == faction.id { continue; }
            match faction.faction_type {
                // Theocracies hostile to factions with different patron gods
                FactionType::Theocracy => {
                    if let Some(other_f) = factions_snapshot.iter().find(|f| f.id == other_id) {
                        if faction.patron_god.is_some() && other_f.patron_god != faction.patron_god {
                            world_state.relations.modify(faction.id, other_id, -2);
                        }
                    }
                }
                // Merchant guilds smooth relations with everyone
                FactionType::MerchantGuild => {
                    world_state.relations.modify(faction.id, other_id, 1);
                }
                // Bandits/thieves hostile to territorial factions in same region
                FactionType::BanditClan | FactionType::ThievesGuild => {
                    if let Some(other_f) = factions_snapshot.iter().find(|f| f.id == other_id) {
                        if other_f.faction_type.is_territorial() && other_f.home_region == faction.home_region {
                            world_state.relations.modify(faction.id, other_id, -1);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Phase 2: Settlement upkeep — prosperity requires ongoing investment
    for settlement in settlements.iter_mut() {
        if settlement.destroyed_year.is_some() { continue; }

        // Base maintenance cost — things decay without effort
        settlement.prosperity = settlement.prosperity.saturating_sub(1);
        // Larger settlements cost more to maintain
        match settlement.population_class {
            PopulationClass::City => settlement.prosperity = settlement.prosperity.saturating_sub(1),
            _ => {}
        }

        // Prosperity from real conditions (not just "peace")
        if settlement.stockpile.food > 0 {
            settlement.prosperity = (settlement.prosperity + 2).min(100);
        }
        if settlement.stockpile.timber > 0 && settlement.stockpile.stone > 0 {
            settlement.prosperity = (settlement.prosperity + 1).min(100);
        }

        // Growth check
        if settlement.prosperity > 70 && rng.random::<f32>() < 0.05 {
            settlement.population_class = settlement.population_class.grow();
        }
        // Shrink check
        if settlement.prosperity < 30 && rng.random::<f32>() < 0.05 {
            settlement.population_class = settlement.population_class.shrink();
        }
    }

    // Phase 3: New character generation (stable factions produce notable members)
    for &fid in &living {
        let f = match factions.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => continue,
        };
        if f.stability < 40 { continue; } // unstable factions don't attract talent
        // ~1 new notable per faction per 15 years
        if rng.random::<f32>() >= (1.0 / 15.0) { continue; }

        let role_table = PopTable::pick_one(vec![
            (CharacterRole::General, 30.0),
            (CharacterRole::Advisor, 25.0),
            (CharacterRole::Scholar, 20.0),
            (CharacterRole::Hero, 15.0),
            (CharacterRole::Villain, 10.0),
        ]);
        let role = role_table.roll_one(rng).unwrap();
        let char_id = *next_id;
        *next_id += 1;
        let birth = year - rng.random_range(18..30);
        let new_char = generate_character(char_id, f.race, role, Some(fid), birth, rng);
        characters.push(new_char);
    }

    // Phase 3b: Random friction — border disputes, rivalries, incidents
    if living.len() >= 2 && rng.random::<f32>() < 0.40 {
        let a = living[rng.random_range(0..living.len())];
        let b_candidates: Vec<u32> = living.iter().copied().filter(|&x| x != a).collect();
        if !b_candidates.is_empty() {
            let b = b_candidates[rng.random_range(0..b_candidates.len())];
            let severity = rng.random_range(2..8);
            world_state.relations.modify(a, b, -(severity as i32));
        }
    }

    // Phase 4: Event evaluation (prerequisite-based, character-driven)
    evaluate_war_declared(year, factions, characters, events, world_state, &living, rng);
    evaluate_war_ended(year, factions, settlements, events, world_state, rng);
    evaluate_betrayal(year, factions, characters, events, world_state, rng);
    evaluate_alliance(year, factions, characters, events, world_state, &living, rng);
    evaluate_alliance_broken(year, factions, characters, events, world_state, rng);
    evaluate_trade_agreement(year, factions, events, world_state, &living, rng);
    evaluate_leader_changed(year, factions, characters, events, &living, next_id, rng);
    evaluate_plague(year, factions, settlements, events, regions, rng, world_state);
    // monster_attack and hero removed — monsters are gameplay, heroes emerge from population
    evaluate_artifact_discovered(year, factions, characters, artifacts_list, events, &living, next_id, rng);
    evaluate_settlement_founded(year, factions, settlements, characters, events, next_id, &living, regions, rng);
    let mut founder_shifts = evaluate_faction_formation(year, factions, settlements, characters, events, world_state, next_id, formation_candidates, rng);
    founder_shifts.extend(evaluate_rebellion(year, factions, settlements, characters, events, world_state, next_id, formation_candidates, rng));
    evaluate_absorption(year, factions, settlements, events, world_state, &living, rng);
    evaluate_faction_dissolved(year, factions, settlements, events, world_state, &living, rng);

    // Phase 4: Sentiment drift
    world_state.relations.drift_toward_neutral();

    founder_shifts
}

// ── Event evaluation functions ──
// Each checks prerequisites against world state, rolls probability, and applies consequences.

fn evaluate_war_declared(
    year: i32, factions: &[FactionState], characters: &[Character],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], _rng: &mut impl Rng,
) {
    if living.len() < 2 { return; }

    // Check all hostile pairs — war starts when a leader decides to act, not random chance
    let mut hostile_pairs: Vec<(u32, u32, i32)> = Vec::new();
    for (i, &a) in living.iter().enumerate() {
        for &b in &living[i+1..] {
            let sentiment = world_state.relations.get(a, b);
            if sentiment < -15 {
                hostile_pairs.push((a, b, sentiment));
            }
        }
    }
    hostile_pairs.sort_by_key(|(_, _, s)| *s); // most hostile first

    for (a, b, sentiment) in hostile_pairs {
        if world_state.at_war(a, b) { continue; }
        if world_state.war_count(a) > 1 || world_state.war_count(b) > 1 { continue; }

        // Non-territorial factions can't wage or be targeted by conventional war
        let a_territorial = factions.iter().find(|f| f.id == a).map_or(true, |f| f.faction_type.is_territorial());
        let b_territorial = factions.iter().find(|f| f.id == b).map_or(true, |f| f.faction_type.is_territorial());
        if !a_territorial || !b_territorial { continue; }

        let aggressor_mil = factions.iter().find(|f| f.id == a).map(|f| f.military_strength).unwrap_or(0);
        if aggressor_mil < 10 { continue; }

        // Leader's ambition and traits determine if war is declared — no random roll
        let leader_ambition = leader_get_ambition(a, factions, characters);
        let has_war_ambition = matches!(leader_ambition,
            Some(Ambition::ExpandTerritory) | Some(Ambition::DestroyEnemy { .. })
        );
        let has_specific_enemy = matches!(leader_ambition,
            Some(Ambition::DestroyEnemy { target_faction }) if target_faction == b
        );
        let is_warlike = leader_has_trait(a, CharacterTrait::Warlike, factions, characters);
        let is_peaceful = leader_has_trait(a, CharacterTrait::Peaceful, factions, characters);
        let is_diplomatic = leader_has_trait(a, CharacterTrait::Diplomatic, factions, characters);

        let declare_war = if has_specific_enemy {
            true // personal vendetta — always act
        } else if is_peaceful || is_diplomatic {
            sentiment < -50 // only extreme provocation overcomes peaceful nature
        } else if has_war_ambition || is_warlike {
            sentiment < -15 // already hostile enough for an aggressive leader
        } else {
            sentiment < -40 // cautious leaders need stronger provocation
        };

        if !declare_war { continue; }

        world_state.active_wars.push(War { aggressor: a, defender: b, start_year: year });
        world_state.relations.modify(a, b, -20);

        let fa = faction_name_by_id(factions, a);
        let fb = faction_name_by_id(factions, b);
        events.push(HistoricEvent {
            year, kind: EventKind::WarDeclared,
            description: format!("{} declared war on {}", fa, fb),
            participants: vec![a, b],
            god_participants: vec![],
            cause: None,
        });
    }
}

fn evaluate_war_ended(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    events: &mut Vec<HistoricEvent>, world_state: &mut WorldState, _rng: &mut impl Rng,
) {
    let mut ended_wars = Vec::new();
    for (i, war) in world_state.active_wars.iter().enumerate() {
        let duration = year - war.start_year;
        if duration < 1 { continue; } // minimum 1 year for casualties to matter

        let a_mil = factions.iter().find(|f| f.id == war.aggressor).map(|f| f.military_strength).unwrap_or(0);
        let b_mil = factions.iter().find(|f| f.id == war.defender).map(|f| f.military_strength).unwrap_or(0);
        let (stronger, weaker) = if a_mil >= b_mil { (a_mil, b_mil) } else { (b_mil, a_mil) };

        // War ends from military reality, not random duration
        let decisive_victory = stronger > 0 && (weaker as f32) < (stronger as f32) * 0.3;
        let mutual_exhaustion = a_mil < 10 && b_mil < 10;

        if decisive_victory || mutual_exhaustion {
            ended_wars.push(i);
        }
    }

    // Process ended wars in reverse order to preserve indices
    for &i in ended_wars.iter().rev() {
        let war = world_state.active_wars.remove(i);
        let a_mil = factions.iter().find(|f| f.id == war.aggressor).map(|f| f.military_strength).unwrap_or(0);
        let b_mil = factions.iter().find(|f| f.id == war.defender).map(|f| f.military_strength).unwrap_or(0);

        let (winner, loser) = if a_mil >= b_mil {
            (war.aggressor, war.defender)
        } else {
            (war.defender, war.aggressor)
        };

        let fw = faction_name_by_id(factions, winner);
        let fl = faction_name_by_id(factions, loser);

        // War consequences flow through population: soldier deaths reduce military,
        // conquest reduces prosperity/food, and happiness drops reduce stability.
        // No direct gauge manipulation needed — values are derived from population.

        // Settlement conquest: winner takes settlements proportional to military dominance
        let loser_settlements: Vec<u32> = settlements.iter()
            .filter(|s| s.controlling_faction == loser && s.destroyed_year.is_none())
            .map(|s| s.id)
            .collect();
        // Decisive victories (>3x military) take more settlements
        let w_mil = factions.iter().find(|f| f.id == winner).map(|f| f.military_strength).unwrap_or(0);
        let l_mil = factions.iter().find(|f| f.id == loser).map(|f| f.military_strength).unwrap_or(0);
        let num_conquests = if loser_settlements.is_empty() { 0 }
            else if l_mil == 0 || w_mil > l_mil * 3 { 2.min(loser_settlements.len()) }
            else { 1.min(loser_settlements.len()) };

        for i in 0..num_conquests.min(loser_settlements.len()) {
            let target_sid = loser_settlements[i];
            if let Some(s) = settlements.iter_mut().find(|s| s.id == target_sid) {
                let old_name = s.name.clone();
                // Don't set controlling_faction directly — let allegiance shifts handle it
                s.conquered_this_year = true;
                s.conquered_by = Some(winner);
                s.prosperity = s.prosperity.saturating_sub(30);
                s.population_class = s.population_class.shrink();
                s.stockpile.food = s.stockpile.food / 2;
                // Settlement lists are derived — no need to update faction.settlements

                events.push(HistoricEvent {
                    year, kind: EventKind::Conquest,
                    description: format!("{} conquered {} from {}", fw, old_name, fl),
                    participants: vec![winner, loser],
            god_participants: vec![],
            cause: None,
                });
                world_state.relations.modify(winner, loser, -15);
            }
        }

        events.push(HistoricEvent {
            year, kind: EventKind::WarEnded,
            description: format!("The war between {} and {} ended; {} emerged victorious", fw, fl, fw),
            participants: vec![winner, loser],
            god_participants: vec![],
            cause: None,
        });
    }
}

/// A treacherous character betrays an alliance when they have reason to.
fn evaluate_betrayal(
    year: i32, factions: &mut Vec<FactionState>, characters: &mut Vec<Character>,
    events: &mut Vec<HistoricEvent>, world_state: &mut WorldState, _rng: &mut impl Rng,
) {
    if world_state.active_alliances.is_empty() { return; }

    for i in 0..world_state.active_alliances.len() {
        let alliance = &world_state.active_alliances[i];
        let a = alliance.faction_a;
        let b = alliance.faction_b;

        // Find a treacherous/corrupt character with reason to betray
        let betrayer = characters.iter()
            .filter(|c| c.is_alive(year))
            .filter(|c| c.faction_id == Some(a) || c.faction_id == Some(b))
            .filter(|c| c.has_trait(CharacterTrait::Treacherous) || c.has_trait(CharacterTrait::Corrupt))
            .filter(|c| {
                // Must have motivation: SeizePower ambition or faction is unstable
                let faction_id = c.faction_id.unwrap();
                let unstable = factions.iter().find(|f| f.id == faction_id)
                    .map(|f| f.stability < 30)
                    .unwrap_or(false);
                c.ambition == Ambition::SeizePower || unstable
            })
            .max_by_key(|c| c.renown);

        let betrayer_id = match betrayer {
            Some(c) => c.id,
            None => continue,
        };

        // Betrayer must see an advantage: victim faction is weaker
        let betrayer_char = characters.iter().find(|c| c.id == betrayer_id).unwrap();
        let betrayer_faction = betrayer_char.faction_id.unwrap();
        let victim_faction = if betrayer_faction == a { b } else { a };
        let b_mil = factions.iter().find(|f| f.id == betrayer_faction).map(|f| f.military_strength).unwrap_or(0);
        let v_mil = factions.iter().find(|f| f.id == victim_faction).map(|f| f.military_strength).unwrap_or(0);
        if v_mil > b_mil { continue; } // won't betray a stronger ally

        let betrayer_name = betrayer_char.full_display_name();
        let fb = faction_name_by_id(factions, betrayer_faction);
        let fv = faction_name_by_id(factions, victim_faction);

        world_state.active_alliances.remove(i);
        world_state.relations.modify(betrayer_faction, victim_faction, -40);

        if let Some(c) = characters.iter_mut().find(|c| c.id == betrayer_id) {
            if c.epithet.is_none() {
                c.epithet = Some("the Betrayer".into());
            }
            c.renown += 5;
        }

        events.push(HistoricEvent {
            year, kind: EventKind::Betrayal,
            description: format!(
                "{} of {} betrayed the alliance with {}, shattering the pact",
                betrayer_name, fb, fv
            ),
            participants: vec![betrayer_faction, victim_faction],
            god_participants: vec![],
            cause: None,
        });
        return; // one betrayal per year
    }
}

/// Alliances form when diplomatic leaders reach out to friendly factions.
fn evaluate_alliance(
    year: i32, factions: &[FactionState], characters: &[Character],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], _rng: &mut impl Rng,
) {
    if living.len() < 2 { return; }

    // Only factions with Diplomatic leaders or shared threats initiate alliances
    for &a in living {
        let a_diplomatic = leader_has_trait(a, CharacterTrait::Diplomatic, factions, characters);
        for &b in living {
            if a >= b { continue; }
            if world_state.allied(a, b) { continue; }
            if world_state.at_war(a, b) { continue; }

            let sentiment = world_state.relations.get(a, b);
            let b_diplomatic = leader_has_trait(b, CharacterTrait::Diplomatic, factions, characters);

            // Shared patron god lowers the sentiment threshold
            let shared_god = factions.iter().find(|f| f.id == a).and_then(|f| f.patron_god)
                == factions.iter().find(|f| f.id == b).and_then(|f| f.patron_god)
                && factions.iter().find(|f| f.id == a).and_then(|f| f.patron_god).is_some();

            // Shared enemy (both at war with the same faction)
            let shared_enemy = living.iter().any(|&e| {
                e != a && e != b && world_state.at_war(a, e) && world_state.at_war(b, e)
            });

            let should_ally = if shared_enemy && sentiment > 0 {
                true // practical alliance against common foe
            } else if (a_diplomatic || b_diplomatic) && sentiment > 15 {
                true // diplomat reaches out to friendly faction
            } else if shared_god && sentiment > 15 {
                true // religious bond
            } else {
                sentiment > 30 // naturally friendly
            };

            if !should_ally { continue; }

            world_state.active_alliances.push(Alliance { faction_a: a, faction_b: b, formed_year: year });
            world_state.relations.modify(a, b, 10);

            let fa = faction_name_by_id(factions, a);
            let fb = faction_name_by_id(factions, b);
            events.push(HistoricEvent {
                year, kind: EventKind::AllianceFormed,
                description: format!("{} and {} formed an alliance", fa, fb),
                participants: vec![a, b],
                god_participants: vec![],
                cause: None,
            });
            return; // one alliance per year — diplomatic effort takes time
        }
    }
}

/// Alliances break when sentiment turns hostile or a treacherous leader acts.
fn evaluate_alliance_broken(
    year: i32, factions: &[FactionState], characters: &[Character],
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, _rng: &mut impl Rng,
) {
    let mut broken = Vec::new();
    for (i, alliance) in world_state.active_alliances.iter().enumerate() {
        let sentiment = world_state.relations.get(alliance.faction_a, alliance.faction_b);
        let treacherous_leader = leader_has_trait(alliance.faction_a, CharacterTrait::Treacherous, factions, characters)
            || leader_has_trait(alliance.faction_b, CharacterTrait::Treacherous, factions, characters);

        // Break deterministically when conditions warrant it
        let should_break = sentiment < 0 // relationship turned hostile
            || (treacherous_leader && sentiment < 10); // treacherous leader abandons lukewarm ally

        if should_break {
            broken.push(i);
        }
    }
    for &i in broken.iter().rev() {
        let alliance = world_state.active_alliances.remove(i);
        world_state.relations.modify(alliance.faction_a, alliance.faction_b, -25);
        let fa = faction_name_by_id(factions, alliance.faction_a);
        let fb = faction_name_by_id(factions, alliance.faction_b);
        events.push(HistoricEvent {
            year, kind: EventKind::AllianceBroken,
            description: format!("The alliance between {} and {} collapsed", fa, fb),
            participants: vec![alliance.faction_a, alliance.faction_b],
            god_participants: vec![],
            cause: None,
        });
    }
}

/// Trade agreements form automatically when factions aren't hostile.
fn evaluate_trade_agreement(
    year: i32, factions: &[FactionState], events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], _rng: &mut impl Rng,
) {
    if living.len() < 2 { return; }

    let mut new_treaties = 0;
    for &a in living {
        for &b in living {
            if a >= b { continue; }
            if new_treaties >= 2 { return; } // limit diplomatic bandwidth per year
            let sentiment = world_state.relations.get(a, b);
            if sentiment < 0 { continue; }
            if world_state.at_war(a, b) { continue; }
            let has_treaty = world_state.active_treaties.iter().any(|t| {
                (t.faction_a == a && t.faction_b == b) || (t.faction_a == b && t.faction_b == a)
            });
            if has_treaty { continue; }

            world_state.active_treaties.push(Treaty { faction_a: a, faction_b: b, formed_year: year });
            world_state.relations.modify(a, b, 5);

            let fa = faction_name_by_id(factions, a);
            let fb = faction_name_by_id(factions, b);
            events.push(HistoricEvent {
                year, kind: EventKind::TradeAgreement,
                description: format!("{} and {} signed a trade agreement", fa, fb),
                participants: vec![a, b],
            god_participants: vec![],
            cause: None,
            });
            new_treaties += 1;
        }
    }
}

fn evaluate_leader_changed(
    year: i32, factions: &mut Vec<FactionState>, characters: &mut Vec<Character>,
    events: &mut Vec<HistoricEvent>, living: &[u32],
    next_id: &mut u32, rng: &mut impl Rng,
) {
    for &fid in living {
        let f = match factions.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => continue,
        };

        // Check for ambitious coup: a PowerHungry/Ambitious character with more renown than leader
        let leader_renown = f.leader_id
            .and_then(|lid| characters.iter().find(|c| c.id == lid))
            .map(|c| c.renown)
            .unwrap_or(0);

        let usurper = characters.iter()
            .filter(|c| c.is_alive(year) && c.faction_id == Some(fid) && c.role != CharacterRole::Leader)
            .filter(|c| c.has_trait(CharacterTrait::PowerHungry) || c.has_trait(CharacterTrait::Ambitious))
            .filter(|c| c.renown > leader_renown)
            .max_by_key(|c| c.renown)
            .map(|c| c.id);

        // Coup happens when conditions are right — no random roll
        let should_coup = usurper.is_some() && f.stability < 30;
        if !should_coup { continue; }

        let old_name = f.leader_name.clone();
        let race = f.race;
        let fname = f.name.clone();
        let is_coup = usurper.is_some();

        // Determine new leader
        let new_leader_name = if let Some(uid) = usurper {
            // Usurper takes over
            if let Some(old_leader) = f.leader_id.and_then(|lid| characters.iter_mut().find(|c| c.id == lid)) {
                old_leader.role = CharacterRole::Advisor; // demoted
                if is_coup {
                    old_leader.renown = (old_leader.renown - 10).max(-50);
                }
            }
            if let Some(u) = characters.iter_mut().find(|c| c.id == uid) {
                u.role = CharacterRole::Leader;
                u.renown += 10;
                if u.epithet.is_none() {
                    u.epithet = Some(generate_epithet(u, rng));
                }
                let name = u.full_display_name();
                if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
                    f.leader_id = Some(uid);
                    f.leader_name = name.clone();
                }
                name
            } else {
                continue;
            }
        } else {
            // Generate new character
            let new_id = *next_id;
            *next_id += 1;
            let birth = year - rng.random_range(25..40);
            let mut new_leader = generate_character(new_id, race, CharacterRole::Leader, Some(fid), birth, rng);
            new_leader.epithet = Some(generate_epithet(&new_leader, rng));
            let name = new_leader.full_display_name();
            characters.push(new_leader);
            if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
                f.leader_id = Some(new_id);
                f.leader_name = name.clone();
            }
            name
        };

        let desc = if is_coup {
            format!("{} seized power from {} in {}", new_leader_name, old_name, fname)
        } else {
            format!("{} succeeded {} as leader of {}", new_leader_name, old_name, fname)
        };

        events.push(HistoricEvent {
            year, kind: EventKind::LeaderChanged,
            description: desc,
            participants: vec![fid],
            god_participants: vec![],
            cause: None,
        });
    }
}

fn evaluate_plague(
    year: i32, _factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    events: &mut Vec<HistoricEvent>, _regions: &[String], rng: &mut impl Rng,
    world_state: &WorldState,
) {
    // Plague is condition-driven, not purely random. Each settlement checks independently.
    let mut affected_factions: Vec<u32> = Vec::new();
    let mut any_plague = false;

    for s in settlements.iter_mut() {
        if s.destroyed_year.is_none() {
            // Plague chance driven purely by conditions — no artificial cooldowns.
            // Natural feedback: plague kills people → population drops → overcrowding
            // bonus drops → plague chance drops.
            let mut plague_chance: f32 = 0.001;

            // Overcrowding — only large settlements are vulnerable
            match s.population_class {
                PopulationClass::City => plague_chance += 0.005,
                PopulationClass::Town => plague_chance += 0.002,
                _ => {} // hamlets/villages rarely get plague
            }

            // Famine — malnourished populations get sick
            if s.stockpile.food <= 0 {
                plague_chance += 0.015;
            }

            // War — corpses, displacement, breakdown of sanitation
            if world_state.war_count(s.controlling_faction) > 0 {
                plague_chance += 0.008;
            }

            // Low prosperity = poor conditions
            if s.prosperity < 20 {
                plague_chance += 0.005;
            }

            if rng.random::<f32>() < plague_chance {
                s.prosperity = s.prosperity.saturating_sub(25);
                s.plague_this_year = true;
                any_plague = true;
                if !affected_factions.contains(&s.controlling_faction) {
                    affected_factions.push(s.controlling_faction);
                }
            }
        }
    }

    if any_plague {
        events.push(HistoricEvent {
            year, kind: EventKind::PlagueStruck,
            description: "Plague struck settlements across the realm".into(),
            participants: affected_factions,
            god_participants: vec![],
            cause: None,
        });
    }
}

fn evaluate_artifact_discovered(
    year: i32, factions: &[FactionState], characters: &mut Vec<Character>,
    artifacts_list: &mut Vec<Artifact>, events: &mut Vec<HistoricEvent>,
    living: &[u32], next_id: &mut u32, rng: &mut impl Rng,
) {
    // Artifacts discovered by factions with Scholar characters in stable conditions
    let fid = match living.iter().copied().find(|&fid| {
        let has_scholar = faction_has_member_with_trait(fid, CharacterTrait::Scholarly, characters, year);
        let stable = factions.iter().find(|f| f.id == fid).map(|f| f.stability > 40).unwrap_or(false);
        has_scholar && stable
    }) {
        Some(fid) => fid,
        None => return,
    };
    let fname = faction_name_by_id(factions, fid);

    let kind_table = PopTable::pick_one(vec![
        (ArtifactKind::Weapon, 25.0),
        (ArtifactKind::Armor, 15.0),
        (ArtifactKind::Tome, 20.0),
        (ArtifactKind::Crown, 10.0),
        (ArtifactKind::Relic, 20.0),
        (ArtifactKind::Gem, 10.0),
    ]);
    let kind = kind_table.roll_one(rng).unwrap();

    let artifact_id = *next_id;
    *next_id += 1;
    let mut artifact = generate_artifact(artifact_id, kind, year, fid, rng);

    // If a hero or scholar exists in the faction, they hold it
    let holder = characters.iter()
        .filter(|c| c.is_alive(year) && c.faction_id == Some(fid))
        .filter(|c| matches!(c.role, CharacterRole::Hero | CharacterRole::Scholar))
        .max_by_key(|c| c.renown);

    let discoverer_desc = if let Some(h) = holder {
        artifact.holder_character = Some(h.id);
        format!("{} of {}", h.full_display_name(), fname)
    } else {
        format!("scholars of {}", fname)
    };

    let artifact_name = artifact.name.clone();
    artifacts_list.push(artifact);

    events.push(HistoricEvent {
        year, kind: EventKind::ArtifactDiscovered,
        description: format!("{} discovered {}", discoverer_desc, artifact_name),
        participants: vec![fid],
            god_participants: vec![],
            cause: None,
    });
}

fn evaluate_settlement_founded(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    characters: &[Character],
    events: &mut Vec<HistoricEvent>, next_id: &mut u32, living: &[u32],
    regions: &[String], rng: &mut impl Rng,
) {
    // Settlement founded when leader has ExpandTerritory ambition and faction can afford it
    // Find first qualifying faction — no random selection
    let fid = match living.iter().copied().find(|&fid| {
        let f = match factions.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => return false,
        };
        if f.wealth < 30 { return false; }
        // Leader must want to expand
        let ambition = leader_get_ambition(fid, factions, characters);
        matches!(ambition, Some(Ambition::ExpandTerritory))
    }) {
        Some(fid) => fid,
        None => return,
    };
    let f = factions.iter().find(|f| f.id == fid).unwrap();

    let fname = f.name.clone();
    let region = f.territory.first().cloned().unwrap_or_else(|| regions[0].clone());
    let sname = settlement_name(rng);
    let sid = *next_id;
    *next_id += 1;

    settlements.push(SettlementState {
        id: sid, name: sname.clone(), founded_year: year,
        controlling_faction: fid, destroyed_year: None, region,
        population_class: PopulationClass::Hamlet, prosperity: 40, defenses: 20,
        patron_god: None, devotion: 0, world_pos: None,
        zone_type: None, stockpile: ResourceStockpile::default(), plague_this_year: false, conquered_this_year: false, conquered_by: None,
        dominant_race: None,
    });

    if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
        f.settlements.push(sid);
    }

    events.push(HistoricEvent {
        year, kind: EventKind::SettlementFounded,
        description: format!("{} established {}", fname, sname),
        participants: vec![fid],
            god_participants: vec![],
            cause: None,
    });
}

/// Condition-based faction formation — replaces the old random 3% spawn.
/// Every settlement that meets formation conditions spawns a new faction.
/// Leader-driven faction formation — specific people found factions based on their traits and life.
fn evaluate_faction_formation(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    characters: &mut Vec<Character>,
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, next_id: &mut u32,
    candidates: Option<&FormationCandidates>,
    rng: &mut impl Rng,
) -> Vec<(u32, u32)> {
    let candidates = match candidates {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut founder_shifts = Vec::new();

    // Each founder creates a new faction
    for founder in &candidates.founders {
        let faction_id = spawn_faction_from_founder(
            founder, year, factions, settlements, characters, events,
            world_state, next_id, rng,
        );
        if let Some(fid) = faction_id {
            founder_shifts.push((founder.person_id, fid));
        }
    }

    founder_shifts
}

/// Create a new faction from a specific person who founded it.
/// Returns the new faction's ID so the caller can shift the founder's allegiance.
fn spawn_faction_from_founder(
    founder: &crate::worldgen::population::faction_stats::FactionFounder,
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    characters: &mut Vec<Character>, events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, next_id: &mut u32, rng: &mut impl Rng,
) -> Option<u32> {
    let settlement = match settlements.iter().find(|s| s.id == founder.settlement_id) {
        Some(s) => s,
        None => return None,
    };
    let region = settlement.region.clone();
    let ft = founder.faction_type;
    let race = founder.race;

    let faction_id = *next_id; *next_id += 1;
    let name = faction_name(ft, race, rng);
    let leader_id = *next_id; *next_id += 1;
    let leader_birth = year - rng.random_range(25..45);
    let leader = generate_character(leader_id, race, CharacterRole::Leader, Some(faction_id), leader_birth, rng);
    let leader_display = leader.full_display_name();
    characters.push(leader);

    let (mil, wealth, stab) = FactionState::initialize_gauges(ft);
    let new_faction = FactionState {
        id: faction_id, name: name.clone(), faction_type: ft, race,
        founded_year: year, home_region: region.clone(),
        dissolved_year: None, leader_name: leader_display, leader_id: Some(leader_id),
        military_strength: mil, wealth, stability: stab,
        territory: vec![region], settlements: vec![], // derived from allegiance
        patron_god: founder.patron_god.or(settlement.patron_god),
        devotion: settlement.devotion,
        unhappy_years: 0,
    };

    for existing in factions.iter() {
        if existing.is_alive(year) {
            world_state.relations.initialize_pair(&new_faction, existing);
        }
    }

    factions.push(new_faction);
    events.push(HistoricEvent {
        year, kind: EventKind::FactionFounded,
        description: format!("{} — {}", name, founder.reason),
        participants: vec![faction_id],
        god_participants: vec![],
        cause: None,
    });

    Some(faction_id)
}

fn evaluate_faction_dissolved(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], _rng: &mut impl Rng,
) {
    for &fid in living {
        let f = match factions.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => continue,
        };

        // Type-aware dissolution: each faction type dies for different reasons.
        let age = year - f.founded_year;
        let all_gauges_zero = f.military_strength == 0 && f.wealth == 0 && f.stability == 0;
        let should_dissolve = match f.faction_type {
            // Territorial factions: depopulation or sustained misery
            FactionType::TribalWarband | FactionType::BanditClan => {
                (all_gauges_zero && age >= 2) || f.unhappy_years >= 5
            }
            // No fighters = no company
            FactionType::MercenaryCompany => {
                f.military_strength == 0 && age >= 3
            }
            // Shadow orgs and arcane circles only die when truly empty
            FactionType::ThievesGuild | FactionType::MageCircle => {
                all_gauges_zero && age >= 2
            }
            // No trade = no guild
            FactionType::MerchantGuild => {
                f.wealth == 0 && age >= 3
            }
            // No devotion = no order
            FactionType::ReligiousOrder => {
                f.stability == 0 && age >= 3
            }
            // Needs faith or fighters
            FactionType::Theocracy => {
                f.stability == 0 && f.military_strength == 0 && age >= 3
            }
        };

        if !should_dissolve { continue; }

        let fname = f.name.clone();

        if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
            f.dissolved_year = Some(year);
            f.settlements.clear();
        }
        // Allegiant people become unaligned — the allegiance shift system
        // and recompute_controlling_factions will handle redistribution

        // Clean up diplomatic ties
        world_state.active_wars.retain(|w| w.aggressor != fid && w.defender != fid);
        world_state.active_alliances.retain(|a| a.faction_a != fid && a.faction_b != fid);
        world_state.active_treaties.retain(|t| t.faction_a != fid && t.faction_b != fid);

        events.push(HistoricEvent {
            year, kind: EventKind::FactionDissolved,
            description: format!("{} collapsed under the weight of popular discontent", fname),
            participants: vec![fid],
            god_participants: vec![],
            cause: None,
        });
    }
}

/// Rebellions from leader-identified rebel figures in mismatched settlements.
fn evaluate_rebellion(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    characters: &mut Vec<Character>,
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, next_id: &mut u32,
    candidates: Option<&FormationCandidates>,
    rng: &mut impl Rng,
) -> Vec<(u32, u32)> {
    let candidates = match candidates {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut founder_shifts = Vec::new();

    // Each rebellion candidate is a specific person who leads the revolt
    for rebel in &candidates.rebellions {
        let old_owner = settlements.iter()
            .find(|s| s.id == rebel.settlement_id)
            .map(|s| s.controlling_faction)
            .unwrap_or(0);

        let faction_id = spawn_faction_from_founder(
            rebel, year, factions, settlements, characters, events,
            world_state, next_id, rng,
        );

        if let Some(fid) = faction_id {
            founder_shifts.push((rebel.person_id, fid));
            // Hostile to former owner
            world_state.relations.modify(fid, old_owner, -30);
        }
    }

    founder_shifts
}

/// Weak factions near strong same-race/faith factions may be absorbed.
fn evaluate_absorption(
    year: i32, factions: &mut Vec<FactionState>, settlements: &mut Vec<SettlementState>,
    events: &mut Vec<HistoricEvent>,
    world_state: &mut WorldState, living: &[u32], _rng: &mut impl Rng,
) {
    for &fid in living {
        let f = match factions.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => continue,
        };
        // Non-territorial factions can't be absorbed by force
        if !f.faction_type.is_territorial() { continue; }
        if f.settlements.len() > 1 || f.military_strength >= 20 { continue; }

        // Find a strong neighboring faction with shared identity
        let absorber = living.iter()
            .filter(|&&oid| oid != fid)
            .filter_map(|&oid| {
                let other = factions.iter().find(|f| f.id == oid)?;
                if other.military_strength < f.military_strength * 3 { return None; }
                let shared_race = other.race == f.race;
                let shared_god = other.patron_god.is_some() && other.patron_god == f.patron_god;
                if !shared_race && !shared_god { return None; }
                Some(oid)
            })
            .next();

        let absorber_id = match absorber {
            Some(id) => id,
            None => continue,
        };

        // Absorption: dissolve weak faction, allegiance recompute will handle settlement control
        let fname = f.name.clone();
        let absorber_name = faction_name_by_id(factions, absorber_id);

        if let Some(f) = factions.iter_mut().find(|f| f.id == fid) {
            f.dissolved_year = Some(year);
            f.settlements.clear();
        }
        // Note: allegiant people will become unaligned when dissolution processes,
        // then the allegiance shift system + recompute will handle the rest

        world_state.active_wars.retain(|w| w.aggressor != fid && w.defender != fid);
        world_state.active_alliances.retain(|a| a.faction_a != fid && a.faction_b != fid);
        world_state.active_treaties.retain(|t| t.faction_a != fid && t.faction_b != fid);

        events.push(HistoricEvent {
            year, kind: EventKind::FactionDissolved,
            description: format!("{} was absorbed into {}", fname, absorber_name),
            participants: vec![fid, absorber_id],
            god_participants: vec![],
            cause: None,
        });
        return; // one absorption per year
    }
}

/// Find the nearest living faction to a world position (excluding a specific faction).
fn find_nearest_faction(
    pos: Option<WorldPos>, excluded: u32,
    factions: &[FactionState], settlements: &[SettlementState], year: i32,
) -> Option<u32> {
    let pos = pos?;
    factions.iter()
        .filter(|f| f.is_alive(year) && f.id != excluded && !f.settlements.is_empty())
        .filter_map(|f| {
            // Find nearest settlement owned by this faction
            let min_dist = f.settlements.iter()
                .filter_map(|&sid| settlements.iter().find(|s| s.id == sid))
                .filter_map(|s| s.world_pos.map(|p| pos.manhattan_distance(p)))
                .min()?;
            Some((f.id, min_dist))
        })
        .min_by_key(|&(_, dist)| dist)
        .map(|(fid, _)| fid)
}

/// Check if a faction's leader has a specific trait.
/// Get the leader's ambition for a faction.
fn leader_get_ambition(faction_id: u32, factions: &[FactionState], characters: &[Character]) -> Option<Ambition> {
    let leader_id = factions.iter()
        .find(|f| f.id == faction_id)
        .and_then(|f| f.leader_id)?;
    characters.iter().find(|c| c.id == leader_id).map(|c| c.ambition.clone())
}

fn leader_has_trait(faction_id: u32, trait_: CharacterTrait, factions: &[FactionState], characters: &[Character]) -> bool {
    let leader_id = factions.iter()
        .find(|f| f.id == faction_id)
        .and_then(|f| f.leader_id);
    leader_id.map_or(false, |lid| {
        characters.iter().find(|c| c.id == lid).map_or(false, |c| c.has_trait(trait_))
    })
}

/// Check if any notable member of a faction has a trait.
fn faction_has_member_with_trait(faction_id: u32, trait_: CharacterTrait, characters: &[Character], year: i32) -> bool {
    characters.iter().any(|c| {
        c.is_alive(year) && c.faction_id == Some(faction_id) && c.has_trait(trait_)
    })
}

fn faction_name_by_id(factions: &[FactionState], id: u32) -> String {
    factions.iter().find(|f| f.id == id).map(|f| f.name.clone()).unwrap_or("Unknown".into())
}
