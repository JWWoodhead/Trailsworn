//! Resource production, consumption, spoilage, and famine.
//! Workers produce resources based on their occupation and settlement terrain.

use crate::worldgen::history::state::{ResourceStockpile, SettlementState};
use crate::worldgen::zone::ZoneType;

use super::index::SettlementIndex;
use super::types::{Occupation, Person};

/// Base production per worker per year (before terrain modifiers).
fn base_production(occupation: Occupation) -> (i32, i32, i32, i32, i32) {
    // (food, timber, ore, leather, stone)
    match occupation {
        Occupation::Farmer => (8, 0, 0, 0, 0),
        Occupation::Woodcutter => (0, 5, 0, 0, 0),
        Occupation::Miner => (0, 0, 4, 0, 0),
        Occupation::Hunter => (2, 0, 0, 4, 0), // hunters also produce food from game
        Occupation::Quarrier => (0, 0, 0, 0, 4),
        _ => (0, 0, 0, 0, 0),
    }
}

/// Terrain modifier as (food%, timber%, ore%, leather%, stone%).
/// 100 = no change, 150 = +50%, 50 = -50%.
fn terrain_modifier(zone_type: Option<ZoneType>) -> (u32, u32, u32, u32, u32) {
    match zone_type {
        Some(ZoneType::Grassland) => (150, 100, 100, 100, 100),
        Some(ZoneType::Forest) => (100, 150, 100, 125, 100),
        Some(ZoneType::Mountain) => (75, 100, 150, 100, 150),
        Some(ZoneType::Coast) => (125, 100, 100, 100, 100),
        Some(ZoneType::Swamp) => (75, 100, 100, 125, 100),
        Some(ZoneType::Desert) => (50, 50, 125, 100, 125),
        Some(ZoneType::Tundra) => (50, 75, 100, 150, 100),
        _ => (100, 100, 100, 100, 100), // Settlement, Ocean, or None
    }
}

/// Apply a percentage modifier to a base value.
fn apply_modifier(base: i32, pct: u32) -> i32 {
    (base as i64 * pct as i64 / 100) as i32
}

/// Compute total resource production for one settlement.
pub fn compute_production(
    people: &[Person],
    index: &SettlementIndex,
    settlement: &SettlementState,
    year: i32,
) -> ResourceStockpile {
    let (mf, mt, mo, ml, ms) = terrain_modifier(settlement.zone_type);
    let mut food = 0i32;
    let mut timber = 0i32;
    let mut ore = 0i32;
    let mut leather = 0i32;
    let mut stone = 0i32;

    for &idx in index.residents(settlement.id) {
        let person = &people[idx];
        if !person.is_alive(year) { continue; }
        // Children under 16 don't work
        if person.age(year) < 16 { continue; }

        let (bf, bt, bo, bl, bs) = base_production(person.occupation);
        food += apply_modifier(bf, mf);
        timber += apply_modifier(bt, mt);
        ore += apply_modifier(bo, mo);
        leather += apply_modifier(bl, ml);
        stone += apply_modifier(bs, ms);
    }

    ResourceStockpile { food, timber, ore, leather, stone }
}

/// Compute total resource consumption for one settlement.
pub fn compute_consumption(
    people: &[Person],
    index: &SettlementIndex,
    settlement: &SettlementState,
    year: i32,
) -> ResourceStockpile {
    let residents = index.residents(settlement.id);
    let living_count = residents.iter()
        .filter(|&&idx| people[idx].is_alive(year))
        .count() as i32;

    let mut ore_consumption = 0i32;
    let mut leather_consumption = 0i32;
    for &idx in residents {
        let person = &people[idx];
        if !person.is_alive(year) { continue; }
        match person.occupation {
            Occupation::Smith => ore_consumption += 1,
            Occupation::Soldier => leather_consumption += 1,
            _ => {}
        }
    }

    // Timber consumption depends on terrain — grassland uses mud/thatch, not wood
    let timber_per_person = match settlement.zone_type {
        Some(ZoneType::Forest) | Some(ZoneType::Swamp) => 0.5,
        Some(ZoneType::Grassland) | Some(ZoneType::Coast) => 0.2,
        _ => 0.1, // Mountain/Desert/Tundra use stone/sand/ice
    };
    let stone_per_person = 0.1;

    ResourceStockpile {
        food: living_count, // 1 food per person
        timber: (living_count as f64 * timber_per_person) as i32,
        ore: ore_consumption,
        leather: leather_consumption,
        stone: (living_count as f64 * stone_per_person) as i32,
    }
}

/// Apply yearly spoilage to stockpile. Food decays 30% per year.
pub fn apply_spoilage(stockpile: &mut ResourceStockpile) {
    if stockpile.food > 0 {
        let spoiled = (stockpile.food as f32 * 0.30) as i32;
        stockpile.food -= spoiled.max(1);
    }
}

/// Cap stockpiles at a reasonable maximum (prevents runaway accumulation).
pub fn cap_stockpile(stockpile: &mut ResourceStockpile, living_count: i32) {
    let food_cap = living_count * 3; // ~3 years of reserves
    let material_cap = living_count * 2;
    stockpile.food = stockpile.food.min(food_cap);
    stockpile.timber = stockpile.timber.min(material_cap);
    stockpile.ore = stockpile.ore.min(material_cap);
    stockpile.leather = stockpile.leather.min(material_cap);
    stockpile.stone = stockpile.stone.min(material_cap);
}

/// Update a settlement's stockpile for one year.
/// Returns the food deficit (positive = people to kill from famine, 0 = no famine).
pub fn update_stockpile(
    stockpile: &mut ResourceStockpile,
    production: &ResourceStockpile,
    consumption: &ResourceStockpile,
    living_count: i32,
) -> i32 {
    apply_spoilage(stockpile);

    stockpile.food += production.food - consumption.food;
    stockpile.timber += production.timber - consumption.timber;
    stockpile.ore += production.ore - consumption.ore;
    stockpile.leather += production.leather - consumption.leather;
    stockpile.stone += production.stone - consumption.stone;

    // Clamp non-food resources to 0 (food can go negative to trigger famine)
    stockpile.timber = stockpile.timber.max(0);
    stockpile.ore = stockpile.ore.max(0);
    stockpile.leather = stockpile.leather.max(0);
    stockpile.stone = stockpile.stone.max(0);

    cap_stockpile(stockpile, living_count);

    // Return famine deficit
    if stockpile.food < 0 {
        let deficit = -stockpile.food;
        stockpile.food = 0; // reset — the deaths will reduce population
        deficit
    } else {
        0
    }
}

/// When food is critically low, switch non-essential workers to Farmer.
/// Prevents famine death spirals by adapting the workforce.
pub fn rebalance_occupations(
    people: &mut [Person],
    index: &SettlementIndex,
    settlement: &SettlementState,
    year: i32,
) {
    if settlement.stockpile.food >= 0 { return; }

    let deficit = -settlement.stockpile.food;
    let switches = (deficit / 4).max(1) as usize; // gradual, not instant

    // Priority: least essential occupations switch first. Soldiers never switch.
    let switch_priority = [
        Occupation::Quarrier, Occupation::Woodcutter, Occupation::Miner,
        Occupation::Merchant, Occupation::Scholar, Occupation::Priest,
    ];

    let mut switched = 0usize;
    for &occ in &switch_priority {
        if switched >= switches { break; }
        for &idx in index.residents(settlement.id) {
            if switched >= switches { break; }
            let p = &mut people[idx];
            if !p.is_alive(year) { continue; }
            if p.occupation != occ { continue; }
            p.occupation = Occupation::Farmer;
            switched += 1;
        }
    }
}
