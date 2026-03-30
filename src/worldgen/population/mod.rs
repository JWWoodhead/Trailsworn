//! Population simulation — every person in the world tracked with life events.
//! Notable characters emerge from accumulated experiences.

pub mod allegiance;
pub mod events;
pub mod faith;
pub mod happiness;
pub mod index;
pub mod lifecycle;
pub mod migration;
pub mod notable;
pub mod plague;
pub mod prophet;
pub mod resources;
pub mod seed;
pub mod trade;
pub mod traits;
pub mod types;
pub mod faction_stats;
pub mod war;

pub use types::{DeathCause, LifeEvent, LifeEventKind, Occupation, Person, Sex};

use rand::Rng;

use crate::worldgen::divine::state::GodState;
use crate::worldgen::history::state::SettlementState;

use index::SettlementIndex;
use types::LifeEvent as LifeEvt;
use types::LifeEventKind as LifeEvtKind;

#[cfg(test)]
mod tests;

/// All mutable population state during the history simulation.
pub struct PopulationSim {
    pub people: Vec<Person>,
    next_person_id: u32,
    newly_notable: Vec<u32>,
    /// Track which settlements were at war last year (to detect war ending).
    prev_at_war: std::collections::HashSet<u32>,
    /// Notable count per settlement per generation (resets every 25 years).
    /// Key: settlement_id, Value: count this generation.
    notable_counts: std::collections::HashMap<u32, usize>,
    /// Year when the current generation count started.
    notable_gen_start: i32,
}

impl PopulationSim {
    /// Initialize from settlement list at the start of history.
    pub fn new(settlements: &[SettlementState], factions: &[crate::worldgen::history::state::FactionState], start_year: i32, rng: &mut impl Rng) -> Self {
        let (people, next_person_id) = seed::seed_population(settlements, factions, start_year, rng);
        Self {
            people,
            next_person_id,
            newly_notable: Vec::new(),
            prev_at_war: std::collections::HashSet::new(),
            notable_counts: std::collections::HashMap::new(),
            notable_gen_start: start_year,
        }
    }

    /// Advance one year: lifecycle, war, plague, faith, happiness, migration, resources, famine.
    /// Mutates settlement stockpiles in place. Returns person IDs that became notable this year.
    pub fn advance_year(
        &mut self,
        settlements: &mut [SettlementState],
        gods: &[GodState],
        factions: &[crate::worldgen::history::state::FactionState],
        world_state: &crate::worldgen::history::state::WorldState,
        year: i32,
        rng: &mut impl Rng,
    ) -> &[u32] {
        self.newly_notable.clear();

        let index = SettlementIndex::build(&self.people, year);

        // Natural lifecycle: death, marriage, birth
        let outcome = lifecycle::run_lifecycle_year(
            &mut self.people,
            &index,
            settlements,
            year,
            &mut self.next_person_id,
            rng,
        );
        events::apply_family_events(&mut self.people, &outcome, year);

        // Rebuild index after lifecycle changes
        let index = SettlementIndex::build(&self.people, year);

        // War effects: drafting + casualties
        let mut war_dead: Vec<u32> = Vec::new();
        let mut current_at_war = std::collections::HashSet::new();
        for settlement in settlements.iter() {
            if settlement.destroyed_year.is_some() { continue; }
            let is_at_war = world_state.war_count(settlement.controlling_faction) > 0;
            if is_at_war {
                current_at_war.insert(settlement.id);
                let dead = war::apply_war_effects(
                    &mut self.people, &index, settlement,
                    settlement.controlling_faction, // enemy_faction_id approximation
                    year, rng,
                );
                war_dead.extend(dead);
            } else if self.prev_at_war.contains(&settlement.id) {
                // War just ended for this settlement
                war::apply_war_ended(&mut self.people, &index, settlement, settlement.controlling_faction, year);
            }
        }
        self.prev_at_war = current_at_war;

        // Conquest events: notify all residents of conquered settlements
        for settlement in settlements.iter_mut() {
            if settlement.conquered_this_year {
                for &idx in index.residents(settlement.id) {
                    if self.people[idx].is_alive(year) {
                        self.people[idx].life_events.push(LifeEvt {
                            year,
                            kind: LifeEvtKind::SettlementConquered { new_faction_id: settlement.controlling_faction },
                            cause: Some(types::EventCause::Faction {
                                faction_id: settlement.controlling_faction,
                                detail: "settlement conquered",
                            }),
                        });
                    }
                }
                settlement.conquered_this_year = false;
            }
        }

        // Plague effects: one-time kill pulse
        let mut plague_dead: Vec<u32> = Vec::new();
        for settlement in settlements.iter_mut() {
            if settlement.plague_this_year {
                let dead = plague::apply_plague(&mut self.people, &index, settlement, year, rng);
                plague_dead.extend(dead);
            }
        }

        // Generate family events for war and plague deaths
        self.apply_death_kin_events(&war_dead, DeathCause::War, year);
        self.apply_death_kin_events(&plague_dead, DeathCause::Plague, year);

        // Rebuild index after war/plague deaths
        let index = SettlementIndex::build(&self.people, year);

        // Faith evaluation
        faith::evaluate_faith(&mut self.people, &index, settlements, gods, year);

        // Prophet emergence + influence
        prophet::evaluate_prophets(&mut self.people, &index, settlements, gods, year);

        // Happiness evaluation
        happiness::evaluate_happiness(&mut self.people, &index, settlements, world_state, year);

        // Allegiance shifts — people change faction loyalty based on circumstances
        allegiance::evaluate_allegiance_shifts(
            &mut self.people, &index, settlements, factions, world_state, year,
        );

        // Migration — unhappy families leave for better settlements
        migration::evaluate_migration(&mut self.people, &index, settlements, factions, world_state, year);

        // Rebuild index after migration (people moved settlements)
        let index = SettlementIndex::build(&self.people, year);

        // Occupation rebalancing (before resource computation)
        for settlement in settlements.iter() {
            if settlement.destroyed_year.is_some() { continue; }
            resources::rebalance_occupations(&mut self.people, &index, settlement, year);
        }

        // Resource production, consumption, famine
        self.process_resources(settlements, &index, year);

        // Trait evaluation — process this year's events to evolve personalities
        for person in self.people.iter_mut() {
            if person.death_year.is_some() { continue; }
            // Only evaluate events from this year
            let year_events: Vec<_> = person.life_events.iter()
                .filter(|e| e.year == year)
                .cloned()
                .collect();
            for event in &year_events {
                traits::evaluate_trait_change(person, event, rng);
            }
        }

        // Reset generation counts every 25 years
        if year - self.notable_gen_start >= 25 {
            self.notable_counts.clear();
            self.notable_gen_start = year;
        }

        // Check all living people for notable promotion (with per-settlement cap)
        for person in self.people.iter_mut() {
            if person.death_year.is_some() { continue; }
            if notable::check_notable(person) {
                let count = self.notable_counts.entry(person.settlement_id).or_insert(0);
                if *count < notable::MAX_NOTABLES_PER_SETTLEMENT_PER_GEN {
                    *count += 1;
                    self.newly_notable.push(person.id);
                }
            }
        }

        &self.newly_notable
    }

    /// Compute resources and apply famine for all settlements.
    fn process_resources(
        &mut self,
        settlements: &mut [SettlementState],
        index: &SettlementIndex,
        year: i32,
    ) {
        for settlement in settlements.iter_mut() {
            if settlement.destroyed_year.is_some() { continue; }

            let production = resources::compute_production(
                &self.people, index, settlement, year,
            );
            let consumption = resources::compute_consumption(
                &self.people, index, settlement, year,
            );

            let living_count = index.residents(settlement.id).iter()
                .filter(|&&idx| self.people[idx].is_alive(year))
                .count() as i32;

            let famine_deficit = resources::update_stockpile(
                &mut settlement.stockpile,
                &production,
                &consumption,
                living_count,
            );

            // Famine kills
            if famine_deficit > 0 {
                self.apply_famine(settlement.id, index, famine_deficit, year);
            }

            // Famine hits prosperity hard
            if famine_deficit > 0 {
                settlement.prosperity = settlement.prosperity.saturating_sub(5);
            }

            // Timber + stone surplus → defenses
            if settlement.stockpile.timber > 0 && settlement.stockpile.stone > 0 {
                settlement.defenses = (settlement.defenses + 1).min(100);
            }
        }
    }

    /// Kill people from famine, prioritizing infants and elderly.
    fn apply_famine(
        &mut self,
        settlement_id: u32,
        index: &SettlementIndex,
        deficit: i32,
        year: i32,
    ) {
        let mut victims: Vec<(usize, i32)> = index.residents(settlement_id)
            .iter()
            .filter(|&&idx| self.people[idx].is_alive(year))
            .map(|&idx| (idx, self.people[idx].age(year)))
            .collect();

        // Sort: infants first (<5), then elderly (>60 descending), then others
        victims.sort_by(|a, b| {
            let priority_a = if a.1 < 5 { 0 } else if a.1 > 60 { 1 } else { 2 };
            let priority_b = if b.1 < 5 { 0 } else if b.1 > 60 { 1 } else { 2 };
            priority_a.cmp(&priority_b)
                .then_with(|| {
                    // Within elderly, oldest first
                    if a.1 > 60 && b.1 > 60 { b.1.cmp(&a.1) }
                    else { std::cmp::Ordering::Equal }
                })
        });

        let kills = (deficit as usize).min(victims.len());
        let dead_info: Vec<(usize, u32)> = victims[..kills]
            .iter()
            .map(|&(idx, _)| {
                self.people[idx].death_year = Some(year);
                self.people[idx].death_cause = Some(DeathCause::Famine);
                (idx, self.people[idx].id)
            })
            .collect();

        let dead_ids: Vec<u32> = dead_info.iter().map(|&(_, id)| id).collect();
        self.apply_death_kin_events(&dead_ids, DeathCause::Famine, year);
    }

    /// Generate LostFamily events for a batch of deaths with a given cause.
    /// Uses direct index lookups for performance.
    fn apply_death_kin_events(&mut self, dead_ids: &[u32], cause: DeathCause, year: i32) {
        for &dead_id in dead_ids {
            let didx = (dead_id - 1) as usize;
            if didx >= self.people.len() { continue; }
            let spouse = self.people[didx].spouse;
            let mother = self.people[didx].mother;
            let father = self.people[didx].father;

            if let Some(sid) = spouse {
                let sidx = (sid - 1) as usize;
                if sidx < self.people.len() && self.people[sidx].death_year.is_none() {
                    self.people[sidx].life_events.push(LifeEvt {
                        year,
                        kind: LifeEvtKind::LostSpouse { spouse_id: dead_id, cause },
                        cause: None,
                    });
                }
            }
            for parent_id in [mother, father].into_iter().flatten() {
                let pidx = (parent_id - 1) as usize;
                if pidx < self.people.len() && self.people[pidx].death_year.is_none() {
                    self.people[pidx].life_events.push(LifeEvt {
                        year,
                        kind: LifeEvtKind::LostChild { child_id: dead_id, cause },
                        cause: None,
                    });
                }
            }
            // Find children by scanning all people (limited to those with matching parent IDs)
            for p in self.people.iter_mut() {
                if p.death_year.is_some() || p.id == dead_id { continue; }
                if p.mother == Some(dead_id) || p.father == Some(dead_id) {
                    p.life_events.push(LifeEvt {
                        year,
                        kind: LifeEvtKind::LostParent { parent_id: dead_id, cause },
                        cause: None,
                    });
                }
            }
        }
    }

    /// Look up a person by ID.
    pub fn person(&self, id: u32) -> Option<&Person> {
        let idx = id.checked_sub(1)? as usize;
        self.people.get(idx)
    }

    /// Count of people currently alive.
    pub fn living_count(&self, year: i32) -> usize {
        self.people.iter().filter(|p| p.is_alive(year)).count()
    }
}
