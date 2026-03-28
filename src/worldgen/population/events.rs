//! Family life-event generation.
//! When a person dies, marries, or is born, their kin get LifeEvent records.

use std::collections::HashMap;

use super::lifecycle::{BirthRecord, DeathRecord, MarriageRecord, YearOutcome};
use super::types::{LifeEvent, LifeEventKind, Person};

/// Look up a living person by ID using direct index (IDs are sequential from 1).
fn person_mut(people: &mut [Person], id: u32) -> Option<&mut Person> {
    let idx = id.checked_sub(1)? as usize;
    let person = people.get_mut(idx)?;
    if person.death_year.is_some() { return None; }
    Some(person)
}

/// Build parent → children index for living people.
fn build_children_index(people: &[Person]) -> HashMap<u32, Vec<u32>> {
    let mut index: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in people {
        if p.death_year.is_some() { continue; }
        if let Some(mid) = p.mother {
            index.entry(mid).or_default().push(p.id);
        }
        if let Some(fid) = p.father {
            index.entry(fid).or_default().push(p.id);
        }
    }
    index
}

/// Apply family events from a year's lifecycle outcomes.
pub fn apply_family_events(people: &mut [Person], outcome: &YearOutcome, year: i32) {
    // Build children index once for all deaths (O(n) instead of O(deaths × n))
    let children_index = build_children_index(people);

    apply_death_events(people, &outcome.deaths, &children_index, year);
    apply_marriage_events(people, &outcome.marriages, year);
    apply_birth_events(people, &outcome.births, year);
}

fn apply_death_events(
    people: &mut [Person],
    deaths: &[DeathRecord],
    children_index: &HashMap<u32, Vec<u32>>,
    year: i32,
) {
    // Collect death info first to avoid borrow issues
    let death_info: Vec<(u32, super::types::DeathCause, Option<u32>, Option<u32>, Option<u32>)> = deaths
        .iter()
        .map(|d| {
            let p = &people[d.person_index];
            (p.id, d.cause, p.spouse, p.mother, p.father)
        })
        .collect();

    for &(dead_id, cause, spouse_id, mother_id, father_id) in &death_info {
        // Notify spouse
        if let Some(sid) = spouse_id {
            if let Some(spouse) = person_mut(people, sid) {
                spouse.life_events.push(LifeEvent {
                    year,
                    kind: LifeEventKind::LostSpouse { spouse_id: dead_id, cause },
                });
            }
        }

        // Notify living parents
        for parent_id in [mother_id, father_id].into_iter().flatten() {
            if let Some(parent) = person_mut(people, parent_id) {
                parent.life_events.push(LifeEvent {
                    year,
                    kind: LifeEventKind::LostChild { child_id: dead_id, cause },
                });
            }
        }

        // Notify living children (via index — O(children) not O(all people))
        if let Some(child_ids) = children_index.get(&dead_id) {
            for &child_id in child_ids {
                if let Some(child) = person_mut(people, child_id) {
                    child.life_events.push(LifeEvent {
                        year,
                        kind: LifeEventKind::LostParent { parent_id: dead_id, cause },
                    });
                }
            }
        }

        // Notify living siblings (share at least one parent with the deceased)
        let mut notified: Vec<u32> = Vec::new();
        for parent_id in [mother_id, father_id].into_iter().flatten() {
            if let Some(sibling_ids) = children_index.get(&parent_id) {
                for &sib_id in sibling_ids {
                    if sib_id == dead_id { continue; }
                    if notified.contains(&sib_id) { continue; }
                    if let Some(sibling) = person_mut(people, sib_id) {
                        sibling.life_events.push(LifeEvent {
                            year,
                            kind: LifeEventKind::LostSibling { sibling_id: dead_id, cause },
                        });
                        notified.push(sib_id);
                    }
                }
            }
        }
    }
}

fn apply_marriage_events(people: &mut [Person], marriages: &[MarriageRecord], year: i32) {
    let pairs: Vec<(u32, u32)> = marriages
        .iter()
        .map(|m| (people[m.male_index].id, people[m.female_index].id))
        .collect();

    for (m_id, f_id) in pairs {
        if let Some(m) = person_mut(people, m_id) {
            m.life_events.push(LifeEvent {
                year,
                kind: LifeEventKind::MarriedTo { spouse_id: f_id },
            });
        }
        if let Some(f) = person_mut(people, f_id) {
            f.life_events.push(LifeEvent {
                year,
                kind: LifeEventKind::MarriedTo { spouse_id: m_id },
            });
        }
    }
}

fn apply_birth_events(people: &mut [Person], births: &[BirthRecord], year: i32) {
    let parent_pairs: Vec<(u32, Option<u32>, Option<u32>)> = births
        .iter()
        .map(|b| (b.person.id, b.person.mother, b.person.father))
        .collect();

    for (child_id, mother_id, father_id) in parent_pairs {
        if let Some(mid) = mother_id {
            // Direct index lookup instead of linear scan
            if let Some(mother) = person_mut(people, mid) {
                mother.life_events.push(LifeEvent {
                    year,
                    kind: LifeEventKind::ChildBorn { child_id },
                });
            }
        }
        if let Some(fid) = father_id {
            if let Some(father) = person_mut(people, fid) {
                father.life_events.push(LifeEvent {
                    year,
                    kind: LifeEventKind::ChildBorn { child_id },
                });
            }
        }
    }
}
