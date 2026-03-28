//! Trade between settlements — surplus flows from exporters to importers,
//! limited by merchant count and distance.

use crate::worldgen::history::state::{FactionState, ResourceStockpile, SettlementState, WorldState};
use crate::worldgen::world_map::WorldPos;

use super::index::SettlementIndex;
use super::types::{Occupation, Person};

/// Maximum Manhattan distance for trade between two settlements.
const MAX_TRADE_DISTANCE: f32 = 100.0;

/// Trade throughput per merchant per year (across all resources).
const UNITS_PER_MERCHANT: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    Food,
    Timber,
    Ore,
    Leather,
    Stone,
}

#[derive(Clone, Debug)]
pub struct TradeRoute {
    pub from_settlement: u32,
    pub to_settlement: u32,
    pub resource: Resource,
    pub annual_volume: i32,
    pub year_established: i32,
}

/// Count living merchants in a settlement.
fn merchant_count(people: &[Person], index: &SettlementIndex, settlement_id: u32, year: i32) -> i32 {
    index.residents(settlement_id).iter()
        .filter(|&&idx| {
            let p = &people[idx];
            p.is_alive(year) && p.occupation == Occupation::Merchant && p.age(year) >= 16
        })
        .count() as i32
}

/// Distance-based trade efficiency (1.0 = adjacent, 0.0 = too far).
fn trade_efficiency(a: Option<WorldPos>, b: Option<WorldPos>) -> f32 {
    match (a, b) {
        (Some(pa), Some(pb)) => {
            let dist = pa.manhattan_distance(pb) as f32;
            (1.0 - dist / MAX_TRADE_DISTANCE).max(0.0)
        }
        _ => 0.5, // settlements without positions (faction-generated) trade at 50%
    }
}

/// Get a specific resource value from a stockpile.
fn get_resource(s: &ResourceStockpile, r: Resource) -> i32 {
    match r {
        Resource::Food => s.food,
        Resource::Timber => s.timber,
        Resource::Ore => s.ore,
        Resource::Leather => s.leather,
        Resource::Stone => s.stone,
    }
}

/// Mutate a specific resource value in a stockpile.
fn set_resource(s: &mut ResourceStockpile, r: Resource, val: i32) {
    match r {
        Resource::Food => s.food = val,
        Resource::Timber => s.timber = val,
        Resource::Ore => s.ore = val,
        Resource::Leather => s.leather = val,
        Resource::Stone => s.stone = val,
    }
}

/// Run trade between all settlements. Returns active trade routes for this year.
pub fn settle_trade(
    settlements: &mut [SettlementState],
    factions: &[FactionState],
    people: &[Person],
    index: &SettlementIndex,
    world_state: &WorldState,
    year: i32,
) -> Vec<TradeRoute> {
    let mut routes = Vec::new();

    // Intra-faction trade (full capacity)
    for faction in factions.iter() {
        if faction.dissolved_year.is_some() { continue; }
        let faction_settlements: Vec<u32> = faction.settlements.clone();
        trade_between_settlements(
            settlements, people, index, &faction_settlements, 1.0, year, &mut routes,
        );
    }

    // Inter-faction trade (allies at 50%, treaties at 30%)
    for alliance in &world_state.active_alliances {
        let a_settlements = faction_settlement_ids(factions, alliance.faction_a);
        let b_settlements = faction_settlement_ids(factions, alliance.faction_b);
        let mut combined = a_settlements;
        combined.extend(b_settlements);
        trade_between_settlements(
            settlements, people, index, &combined, 0.5, year, &mut routes,
        );
    }

    for treaty in &world_state.active_treaties {
        // Don't double-count if also allied
        if world_state.allied(treaty.faction_a, treaty.faction_b) { continue; }
        let a_settlements = faction_settlement_ids(factions, treaty.faction_a);
        let b_settlements = faction_settlement_ids(factions, treaty.faction_b);
        let mut combined = a_settlements;
        combined.extend(b_settlements);
        trade_between_settlements(
            settlements, people, index, &combined, 0.3, year, &mut routes,
        );
    }

    routes
}

fn faction_settlement_ids(factions: &[FactionState], faction_id: u32) -> Vec<u32> {
    factions.iter()
        .find(|f| f.id == faction_id)
        .map(|f| f.settlements.clone())
        .unwrap_or_default()
}

/// Trade resources between a group of settlements at a given capacity multiplier.
fn trade_between_settlements(
    settlements: &mut [SettlementState],
    people: &[Person],
    index: &SettlementIndex,
    settlement_ids: &[u32],
    capacity_multiplier: f32,
    year: i32,
    routes: &mut Vec<TradeRoute>,
) {
    if settlement_ids.len() < 2 { return; }

    // Calculate trade capacity per settlement (based on merchant count)
    let capacities: Vec<(u32, i32)> = settlement_ids.iter()
        .map(|&sid| {
            let merchants = merchant_count(people, index, sid, year);
            let capacity = (merchants as f32 * UNITS_PER_MERCHANT as f32 * capacity_multiplier) as i32;
            (sid, capacity)
        })
        .collect();

    // Track remaining capacity per settlement
    let mut remaining: std::collections::HashMap<u32, i32> = capacities.iter()
        .map(|&(sid, cap)| (sid, cap))
        .collect();

    for resource in [Resource::Food, Resource::Timber, Resource::Ore, Resource::Leather, Resource::Stone] {
        // Find exporters and importers
        let mut exporters: Vec<(usize, i32)> = Vec::new(); // (settlement index, surplus)
        let mut importers: Vec<(usize, i32)> = Vec::new(); // (settlement index, need)

        for (si, s) in settlements.iter().enumerate() {
            if !settlement_ids.contains(&s.id) { continue; }
            if s.destroyed_year.is_some() { continue; }

            let stock = get_resource(&s.stockpile, resource);
            let living = index.residents(s.id).iter()
                .filter(|&&idx| people[idx].is_alive(year))
                .count() as i32;

            // Import threshold: food below 1 year reserves, others below 0
            let threshold = if resource == Resource::Food { living } else { 0 };

            if stock > threshold {
                exporters.push((si, stock - threshold));
            } else if stock < threshold {
                importers.push((si, threshold - stock));
            }
        }

        if exporters.is_empty() || importers.is_empty() { continue; }

        // Sort: biggest surplus first, biggest need first
        exporters.sort_by(|a, b| b.1.cmp(&a.1));
        importers.sort_by(|a, b| b.1.cmp(&a.1));

        // Match exporters to importers
        for &(imp_idx, need) in &importers {
            let imp_id = settlements[imp_idx].id;
            let imp_cap = *remaining.get(&imp_id).unwrap_or(&0);
            if imp_cap <= 0 { continue; }

            let mut need_left = need;

            for &(exp_idx, _surplus) in &exporters {
                if need_left <= 0 { break; }

                let exp_id = settlements[exp_idx].id;
                let exp_cap = *remaining.get(&exp_id).unwrap_or(&0);
                if exp_cap <= 0 { continue; }

                let exp_stock = get_resource(&settlements[exp_idx].stockpile, resource);
                let living = index.residents(exp_id).iter()
                    .filter(|&&idx| people[idx].is_alive(year))
                    .count() as i32;
                let exp_threshold = if resource == Resource::Food { living } else { 0 };
                let available = (exp_stock - exp_threshold).max(0);
                if available <= 0 { continue; }

                let efficiency = trade_efficiency(
                    settlements[exp_idx].world_pos,
                    settlements[imp_idx].world_pos,
                );
                if efficiency <= 0.0 { continue; }

                let max_transfer = available
                    .min(need_left)
                    .min(imp_cap)
                    .min(exp_cap);
                let transfer = (max_transfer as f32 * efficiency) as i32;
                if transfer <= 0 { continue; }

                // Apply transfer
                let exp_val = get_resource(&settlements[exp_idx].stockpile, resource);
                set_resource(&mut settlements[exp_idx].stockpile, resource, exp_val - transfer);
                let imp_val = get_resource(&settlements[imp_idx].stockpile, resource);
                set_resource(&mut settlements[imp_idx].stockpile, resource, imp_val + transfer);

                // Deduct capacity
                *remaining.get_mut(&exp_id).unwrap() -= transfer;
                *remaining.get_mut(&imp_id).unwrap() -= transfer;
                need_left -= transfer;

                routes.push(TradeRoute {
                    from_settlement: exp_id,
                    to_settlement: imp_id,
                    resource,
                    annual_volume: transfer,
                    year_established: year,
                });
            }
        }
    }
}
