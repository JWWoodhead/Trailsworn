//! Compute faction statistics (military, wealth, stability, patron god) from
//! real population data instead of abstract gauges.

use std::collections::{BTreeMap, HashMap};

use crate::worldgen::divine::gods::GodId;
use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::history::state::{FactionState, SettlementState};
use crate::worldgen::names::{FactionType, Race};

use super::index::SettlementIndex;
use super::types::{LifeEventKind, Occupation, Person};
use super::war::faction_military_power;

/// Pre-computed faction statistics derived from population each year.
pub struct FactionStats {
    entries: HashMap<u32, FactionStatsEntry>,
}

struct FactionStatsEntry {
    military: u32,
    wealth: u32,
    stability: u32,
    patron_god: Option<GodId>,
}

impl FactionStats {
    /// Get computed military strength (0-100) for a faction.
    /// Returns `None` if the faction has no population data (e.g. just spawned).
    pub fn military(&self, faction_id: u32) -> Option<u32> {
        self.entries.get(&faction_id).map(|e| e.military)
    }

    /// Get computed wealth (0-100) for a faction.
    pub fn wealth(&self, faction_id: u32) -> Option<u32> {
        self.entries.get(&faction_id).map(|e| e.wealth)
    }

    /// Get computed stability (0-100) for a faction.
    pub fn stability(&self, faction_id: u32) -> Option<u32> {
        self.entries.get(&faction_id).map(|e| e.stability)
    }

    /// Get the most-worshipped god across all faction settlements.
    pub fn patron_god(&self, faction_id: u32) -> Option<GodId> {
        self.entries.get(&faction_id).and_then(|e| e.patron_god)
    }

    /// Whether the faction has any living allegiant people.
    pub fn has_population(&self, faction_id: u32) -> bool {
        self.entries.contains_key(&faction_id)
    }
}

/// Compute faction stats from population data for all alive factions.
///
/// Factions with no settlements or no living population are omitted — callers
/// should fall back to bootstrap/existing values for those factions.
pub fn compute_faction_stats(
    people: &[Person],
    index: &SettlementIndex,
    settlements: &[SettlementState],
    factions: &[FactionState],
    year: i32,
) -> FactionStats {
    let mut entries = HashMap::new();

    for faction in factions.iter().filter(|f| f.is_alive(year)) {
        // Base military: from all allegiant soldiers regardless of where they live
        let raw_military = faction_military_power(people, index, settlements, faction.id, year);
        let base_military = (raw_military / 2.0).round().clamp(0.0, 100.0) as u32;

        // Stats from all allegiant people (not settlement-based)
        let mut merchant_count = 0u32;
        let mut total_happiness = 0u64;
        let mut total_population = 0u64;
        let mut god_worship: BTreeMap<GodId, u64> = BTreeMap::new();
        // Trait counters for type-aware gauges
        let mut treacherous_cunning = 0u32;
        let mut devout_fanatical = 0u32;
        let mut scholarly_wise = 0u32;
        let mut total_patron_devotion = 0u64;
        let mut patron_devotion_count = 0u64;

        for person in people.iter() {
            if !person.is_alive(year) { continue; }
            if person.faction_allegiance != faction.id { continue; }

            if person.occupation == Occupation::Merchant && person.age(year) >= 16 {
                merchant_count += 1;
            }
            total_happiness += person.happiness as u64;
            total_population += 1;

            // Trait counting
            if person.traits.contains(&CharacterTrait::Treacherous)
                || person.traits.contains(&CharacterTrait::Cunning) {
                treacherous_cunning += 1;
            }
            if person.traits.contains(&CharacterTrait::Devout)
                || person.traits.contains(&CharacterTrait::Fanatical) {
                devout_fanatical += 1;
            }
            if person.traits.contains(&CharacterTrait::Scholarly)
                || person.traits.contains(&CharacterTrait::Wise) {
                scholarly_wise += 1;
            }

            // Devotion to faction's patron god (from previous year)
            if let Some(patron) = faction.patron_god {
                let d = person.devotion_to(patron) as u64;
                if d > 0 {
                    total_patron_devotion += d;
                    patron_devotion_count += 1;
                }
            }

            // Individual faith for patron god computation
            if let Some(primary) = person.primary_god() {
                let devotion = person.devotion_to(primary) as u64;
                *god_worship.entry(primary).or_insert(0) += devotion;
            }
        }

        if total_population == 0 { continue; } // no allegiant people — skip

        let avg_happiness = (total_happiness / total_population).min(100) as u32;
        let avg_devotion = if patron_devotion_count > 0 {
            (total_patron_devotion / patron_devotion_count).min(100) as u32
        } else {
            0
        };

        // Type-aware gauge computation
        let (military, wealth, stability) = match faction.faction_type {
            FactionType::ThievesGuild => (
                (treacherous_cunning * 5).min(100),          // shadow operatives
                (total_population as u32 * 3).min(100),      // stolen wealth
                avg_happiness,
            ),
            FactionType::MerchantGuild => (
                base_military,                                // hired guards
                (merchant_count * 4 + total_population as u32).min(100), // trade network
                avg_happiness,
            ),
            FactionType::ReligiousOrder => (
                (devout_fanatical * 3).min(100),             // zealot fighters
                (merchant_count * 2).min(100),               // donations
                if avg_devotion > 0 { avg_devotion } else { avg_happiness }, // faith-based cohesion
            ),
            FactionType::Theocracy => (
                base_military.max((devout_fanatical * 2).min(100)), // soldiers or holy warriors
                (merchant_count * 2).min(100),
                // 60% devotion + 40% happiness
                if avg_devotion > 0 {
                    ((avg_devotion as u64 * 60 + avg_happiness as u64 * 40) / 100).min(100) as u32
                } else {
                    avg_happiness
                },
            ),
            FactionType::MageCircle => (
                (scholarly_wise * 4).min(100),                // arcane warfare
                (scholarly_wise * 3).min(100),                // arcane knowledge value
                avg_happiness,
            ),
            _ => (
                // Kingdom, MercenaryCompany, BanditClan, TribalWarband — unchanged
                base_military,
                (merchant_count * 2).min(100),
                avg_happiness,
            ),
        };

        let patron_god = god_worship.into_iter()
            .max_by_key(|(_, v)| *v)
            .map(|(god, _)| god);

        entries.insert(faction.id, FactionStatsEntry {
            military,
            wealth,
            stability,
            patron_god,
        });
    }

    FactionStats { entries }
}

/// Active prophets visible to the faction-level simulation.
pub struct ProphetTensions {
    /// (faction_id, settlement_id, prophet_god_id) for each active prophet.
    pub active_prophets: Vec<(u32, u32, GodId)>,
}

/// Detect active prophets and which factions they belong to.
pub fn compute_prophet_tensions(
    people: &[Person],
    index: &SettlementIndex,
    settlements: &[SettlementState],
    year: i32,
) -> ProphetTensions {
    let mut active_prophets = Vec::new();

    for settlement in settlements.iter().filter(|s| s.destroyed_year.is_none()) {
        for &idx in index.residents(settlement.id) {
            let person = &people[idx];
            if !person.is_alive(year) { continue; }
            if let Some(ref prophet) = person.prophet_of {
                active_prophets.push((person.faction_allegiance, settlement.id, prophet.god_id));
            }
        }
    }

    ProphetTensions { active_prophets }
}

/// Recompute `controlling_faction` on each settlement from resident allegiances,
/// and rebuild each faction's `settlements` list. Call after population changes each year.
pub fn recompute_controlling_factions(
    people: &[Person],
    index: &SettlementIndex,
    settlements: &mut [SettlementState],
    factions: &mut [FactionState],
    year: i32,
) {
    for settlement in settlements.iter_mut() {
        if settlement.destroyed_year.is_some() { continue; }

        // Count allegiances among living adult residents
        let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
        for &idx in index.residents(settlement.id) {
            let person = &people[idx];
            if !person.is_alive(year) { continue; }
            if person.faction_allegiance == 0 { continue; } // skip unaligned
            *counts.entry(person.faction_allegiance).or_insert(0) += 1;
        }

        // Majority faction controls the settlement (BTreeMap = deterministic tie-break by lowest ID)
        settlement.controlling_faction = counts.into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(faction_id, _)| faction_id)
            .unwrap_or(0);
    }

    // Rebuild each faction's settlement list from the derived controlling_faction
    for faction in factions.iter_mut() {
        if faction.dissolved_year.is_some() { continue; }
        faction.settlements = settlements.iter()
            .filter(|s| s.controlling_faction == faction.id && s.destroyed_year.is_none())
            .map(|s| s.id)
            .collect();
    }
}

// ── Leader-Driven Faction Formation ──

/// A specific person who will found a new faction, with the reason why.
pub struct FactionFounder {
    pub person_id: u32,
    pub settlement_id: u32,
    pub faction_type: FactionType,
    pub race: Race,
    pub reason: String,
    /// Average happiness of the settlement population (0–100).
    pub avg_happiness: u8,
    /// Whether the dominant race differs from the ruling faction's race.
    pub race_mismatch: bool,
    /// Whether the dominant faith differs from the ruling faction's faith.
    pub faith_mismatch: bool,
    /// The god this faction is founded around (prophet's god, founder's faith, etc.).
    /// None means use the settlement's patron god as fallback.
    pub patron_god: Option<GodId>,
}

/// Results of scanning the population for potential faction founders and kingdom upgrades.
pub struct FormationCandidates {
    /// People who will found new factions based on their traits and circumstances.
    pub founders: Vec<FactionFounder>,
    /// Settlements ripe for rebellion (race/faith mismatch + unhappy leader figure).
    pub rebellions: Vec<FactionFounder>,
}

/// Scan population for individuals whose traits and circumstances drive them to found factions.
/// Each founder is a specific person with a specific reason — no arbitrary thresholds.
pub fn compute_formation_candidates(
    people: &[Person],
    index: &SettlementIndex,
    settlements: &[SettlementState],
    factions: &[FactionState],
    year: i32,
) -> FormationCandidates {
    use crate::worldgen::history::characters::CharacterTrait;

    let mut candidates = FormationCandidates {
        founders: Vec::new(),
        rebellions: Vec::new(),
    };

    // Track which settlements already have a founder this year (one per settlement)
    let mut claimed_settlements: Vec<u32> = Vec::new();

    for settlement in settlements.iter().filter(|s| s.destroyed_year.is_none() && s.controlling_faction != 0) {
        let controlling_faction = match factions.iter().find(|f| f.id == settlement.controlling_faction) {
            Some(f) => f,
            None => continue,
        };

        let living: Vec<usize> = index.residents(settlement.id).iter()
            .copied()
            .filter(|&idx| people[idx].is_alive(year) && people[idx].age(year) >= 16)
            .collect();

        // Compute settlement-level context for founders
        let avg_happiness = if living.is_empty() { 50 } else {
            (living.iter().map(|&idx| people[idx].happiness as u32).sum::<u32>() / living.len() as u32) as u8
        };
        let race_mismatch = settlement.dominant_race
            .map(|r| r != controlling_faction.race)
            .unwrap_or(false);
        let faith_mismatch = settlement.patron_god.is_some()
            && controlling_faction.patron_god.is_some()
            && settlement.patron_god != controlling_faction.patron_god;

        // Scan each adult for founding potential. A person founds a faction when:
        // 1. Their traits/circumstances create the motivation
        // 2. No existing faction already serves that motivation (they'd join it instead)
        // The allegiance shift system handles the actual recruitment into existing factions.
        for &idx in &living {
            if claimed_settlements.contains(&settlement.id) { break; }
            let person = &people[idx];

            // Helper: would an existing faction serve this person's motivation?
            // If so, they'll join it via allegiance shifts rather than founding a new one.
            let would_join_existing = |ft: FactionType, extra: Option<&dyn Fn(&FactionState) -> bool>| -> bool {
                factions.iter().any(|f| {
                    f.is_alive(year) && f.faction_type == ft
                        && extra.map_or(true, |check| check(f))
                })
            };

            // Prophet → Theocracy: only if no faction already champions their god
            if let Some(ref prophet) = person.prophet_of {
                if prophet.kind == super::types::ProphetKind::Zealot {
                    let god = prophet.god_id;
                    // Don't found a theocracy if any faction already has this god as patron —
                    // the god's worship is already represented in the world
                    let god_already_championed = factions.iter().any(|f| {
                        f.is_alive(year) && f.patron_god == Some(god)
                    });
                    if god_already_championed {
                        continue;
                    }
                    claimed_settlements.push(settlement.id);
                    candidates.founders.push(FactionFounder {
                        person_id: person.id,
                        settlement_id: settlement.id,
                        faction_type: FactionType::Theocracy,
                        race: person.race,
                        reason: format!("prophet of god {} declared a holy state", prophet.god_id),
                        avg_happiness,
                        race_mismatch,
                        faith_mismatch,
                        patron_god: Some(god),
                    });
                    break;
                }
            }

            // Must have a leadership trait to found a faction
            let is_ambitious = person.traits.contains(&CharacterTrait::Ambitious)
                || person.traits.contains(&CharacterTrait::PowerHungry);
            let is_charismatic = person.traits.contains(&CharacterTrait::Charismatic);
            if !is_ambitious && !is_charismatic { continue; }

            // Must be unhappy (motivation to break away)
            if person.happiness >= 40 { continue; }

            // Different race from controlling faction → tribal breakaway
            if person.race != controlling_faction.race && is_ambitious {
                let race = person.race;
                if would_join_existing(FactionType::TribalWarband, Some(&|f: &FactionState| f.race == race)) {
                    continue;
                }
                claimed_settlements.push(settlement.id);
                candidates.founders.push(FactionFounder {
                    person_id: person.id,
                    settlement_id: settlement.id,
                    faction_type: FactionType::TribalWarband,
                    race: person.race,
                    reason: format!("{:?} declared independence for their people", person.race),
                    avg_happiness,
                    race_mismatch,
                    faith_mismatch,
                    patron_god: None,
                });
                break;
            }

            // Treacherous/Cunning + unhappy → criminal syndicate
            if person.traits.contains(&CharacterTrait::Treacherous)
                || person.traits.contains(&CharacterTrait::Cunning) {
                if would_join_existing(FactionType::ThievesGuild, None) { continue; }
                claimed_settlements.push(settlement.id);
                candidates.founders.push(FactionFounder {
                    person_id: person.id,
                    settlement_id: settlement.id,
                    faction_type: FactionType::ThievesGuild,
                    race: person.race,
                    reason: "built a criminal network from the shadows".into(),
                    avg_happiness,
                    race_mismatch,
                    faith_mismatch,
                    patron_god: None,
                });
                break;
            }

            // Soldier who survived wars + ambitious → military order
            let wars_survived = person.life_events.iter()
                .filter(|e| matches!(e.kind, LifeEventKind::SurvivedWar { .. }))
                .count();
            if wars_survived >= 2 && person.occupation == Occupation::Soldier {
                if would_join_existing(FactionType::MercenaryCompany, None) { continue; }
                claimed_settlements.push(settlement.id);
                candidates.founders.push(FactionFounder {
                    person_id: person.id,
                    settlement_id: settlement.id,
                    faction_type: FactionType::MercenaryCompany,
                    race: person.race,
                    reason: "rallied fellow veterans into a military order".into(),
                    avg_happiness,
                    race_mismatch,
                    faith_mismatch,
                    patron_god: None,
                });
                break;
            }

            // Merchant + charismatic → merchant guild
            if person.occupation == Occupation::Merchant && is_charismatic {
                if would_join_existing(FactionType::MerchantGuild, None) { continue; }
                claimed_settlements.push(settlement.id);
                candidates.founders.push(FactionFounder {
                    person_id: person.id,
                    settlement_id: settlement.id,
                    faction_type: FactionType::MerchantGuild,
                    race: person.race,
                    reason: "united the merchants into a trading guild".into(),
                    avg_happiness,
                    race_mismatch,
                    faith_mismatch,
                    patron_god: None,
                });
                break;
            }

            // Devout + different faith from controlling faction → religious order
            // Only if no faction already champions this god (same logic as Theocracy)
            if person.traits.contains(&CharacterTrait::Devout) {
                let person_god = person.primary_god();
                if person_god.is_some() && person_god != controlling_faction.patron_god {
                    let god = person_god.unwrap();
                    let god_already_championed = factions.iter().any(|f| {
                        f.is_alive(year) && f.patron_god == Some(god)
                    });
                    if god_already_championed { continue; }
                    claimed_settlements.push(settlement.id);
                    candidates.founders.push(FactionFounder {
                        person_id: person.id,
                        settlement_id: settlement.id,
                        faction_type: FactionType::ReligiousOrder,
                        race: person.race,
                        reason: "founded a religious order devoted to their faith".into(),
                        avg_happiness,
                        race_mismatch,
                        faith_mismatch,
                        patron_god: Some(god),
                    });
                    break;
                }
            }

            // Scholarly/Wise → mage circle
            if person.traits.contains(&CharacterTrait::Scholarly)
                || person.traits.contains(&CharacterTrait::Wise)
            {
                if would_join_existing(FactionType::MageCircle, None) { continue; }
                claimed_settlements.push(settlement.id);
                candidates.founders.push(FactionFounder {
                    person_id: person.id,
                    settlement_id: settlement.id,
                    faction_type: FactionType::MageCircle,
                    race: person.race,
                    reason: "gathered fellow scholars into an arcane circle".into(),
                    avg_happiness,
                    race_mismatch,
                    faith_mismatch,
                    patron_god: None,
                });
                break;
            }
        }

        // Rebellion check: an ambitious/charismatic person in an unhappy conquered settlement
        if !claimed_settlements.contains(&settlement.id) {
            let race_mismatch = settlement.dominant_race
                .map(|r| r != controlling_faction.race)
                .unwrap_or(false);
            let faith_mismatch = settlement.patron_god.is_some()
                && controlling_faction.patron_god.is_some()
                && settlement.patron_god != controlling_faction.patron_god;

            if race_mismatch || faith_mismatch {
                // Find a rebel leader
                let rebel = living.iter()
                    .map(|&idx| &people[idx])
                    .filter(|p| p.happiness < 30)
                    .find(|p| {
                        p.traits.contains(&CharacterTrait::Ambitious)
                            || p.traits.contains(&CharacterTrait::Brave)
                            || p.traits.contains(&CharacterTrait::Charismatic)
                    });

                if let Some(rebel_person) = rebel {
                    let ft = if race_mismatch { FactionType::TribalWarband } else { FactionType::ReligiousOrder };

                    // Faith rebellions: don't rebel if a faction already champions the rebel's god
                    if ft == FactionType::ReligiousOrder {
                        if let Some(god) = rebel_person.primary_god() {
                            let already = factions.iter().any(|f| f.is_alive(year) && f.patron_god == Some(god));
                            if already { continue; }
                        }
                    }

                    candidates.rebellions.push(FactionFounder {
                        person_id: rebel_person.id,
                        settlement_id: settlement.id,
                        faction_type: ft,
                        race: rebel_person.race,
                        reason: "led a rebellion against foreign rule".into(),
                        avg_happiness,
                        race_mismatch,
                        faith_mismatch,
                        patron_god: rebel_person.primary_god(),
                    });
                }
            }
        }
    }

    candidates
}
