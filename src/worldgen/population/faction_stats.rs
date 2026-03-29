//! Compute faction statistics (military, wealth, stability, patron god) from
//! real population data instead of abstract gauges.

use std::collections::{BTreeMap, HashMap};

use crate::worldgen::divine::gods::GodId;
use crate::worldgen::history::state::{FactionState, SettlementState};

use super::index::SettlementIndex;
use super::types::{Occupation, Person};
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
        let faction_settlements: Vec<&SettlementState> = settlements.iter()
            .filter(|s| s.owner_faction == faction.id && s.destroyed_year.is_none())
            .collect();

        if faction_settlements.is_empty() {
            continue;
        }

        // Military: reuse existing faction_military_power, normalize to 0-100
        let raw_military = faction_military_power(people, index, settlements, faction.id, year);
        let military = (raw_military / 2.0).round().clamp(0.0, 100.0) as u32;

        // Wealth: count merchants across faction settlements
        let mut merchant_count = 0u32;
        let mut total_happiness = 0u64;
        let mut total_population = 0u64;

        // Patron god: aggregate (god_id -> weighted worship) across settlements
        // BTreeMap for deterministic iteration order (ties broken by god ID)
        let mut god_worship: BTreeMap<GodId, u64> = BTreeMap::new();

        for settlement in &faction_settlements {
            for &idx in index.residents(settlement.id) {
                let person = &people[idx];
                if !person.is_alive(year) { continue; }

                if person.occupation == Occupation::Merchant && person.age(year) >= 16 {
                    merchant_count += 1;
                }

                total_happiness += person.happiness as u64;
                total_population += 1;
            }

            // Use settlement-level patron god weighted by (devotion * population)
            if let Some(patron) = settlement.patron_god {
                let pop = index.residents(settlement.id).iter()
                    .filter(|&&idx| people[idx].is_alive(year))
                    .count() as u64;
                *god_worship.entry(patron).or_insert(0) += settlement.devotion as u64 * pop;
            }
        }

        let wealth = (merchant_count * 2).min(100);

        let stability = if total_population > 0 {
            (total_happiness / total_population).min(100) as u32
        } else {
            0
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
