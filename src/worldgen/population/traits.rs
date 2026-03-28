//! Person trait seeding, earning, and event-driven trait changes.
//! Traits are seeded at birth (2, influenced by parents) and earned from life events.
//! Opposing traits replace each other.

use rand::{Rng, RngExt};

use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::history::characters::CharacterTrait::*;

use super::types::*;

/// Maximum traits per person.
const MAX_TRAITS: usize = 5;

/// Opposing trait pairs — earning one removes the other.
const OPPOSING_PAIRS: &[(CharacterTrait, CharacterTrait)] = &[
    (Brave, Cowardly),
    (Warlike, Peaceful),
    (Ambitious, Content),
    (Loyal, Treacherous),
    (Honorable, Cruel),
    (Just, Corrupt),
    (Humble, PowerHungry),
    (Wise, Foolish),
    (Charismatic, Reclusive),
    (Devout, Skeptical),
];

/// All traits available for random seeding.
const ALL_TRAITS: &[CharacterTrait] = &[
    Warlike, Peaceful, Diplomatic, Ruthless,
    Ambitious, Content, PowerHungry, Humble,
    Loyal, Treacherous, Pragmatic, Fanatical,
    Cunning, Wise, Foolish, Scholarly,
    Honorable, Cruel, Just, Corrupt,
    Paranoid, Charismatic, Reclusive, Devout,
    Greedy, Brave, Cowardly, Skeptical,
];

/// Seed 2 traits for a newborn, influenced by parents.
pub fn seed_traits(
    parent_a: Option<&[CharacterTrait]>,
    parent_b: Option<&[CharacterTrait]>,
    rng: &mut impl Rng,
) -> Vec<CharacterTrait> {
    let mut traits = Vec::with_capacity(2);

    // Try to inherit from each parent (50% chance per trait)
    for parent in [parent_a, parent_b] {
        if let Some(parent_traits) = parent {
            for &t in parent_traits {
                if traits.len() >= 2 { break; }
                if traits.contains(&t) { continue; }
                if has_opposing(&traits, t) { continue; }
                if rng.random::<f32>() < 0.5 {
                    traits.push(t);
                }
            }
        }
    }

    // Fill remaining slots with random traits
    while traits.len() < 2 {
        let t = ALL_TRAITS[rng.random_range(0..ALL_TRAITS.len())];
        if traits.contains(&t) { continue; }
        if has_opposing(&traits, t) { continue; }
        traits.push(t);
    }

    traits
}

/// Check if any existing trait opposes the candidate.
fn has_opposing(traits: &[CharacterTrait], candidate: CharacterTrait) -> bool {
    OPPOSING_PAIRS.iter().any(|&(a, b)| {
        (candidate == a && traits.contains(&b)) || (candidate == b && traits.contains(&a))
    })
}

/// Find the opposite of a trait, if one exists.
fn opposite_of(t: CharacterTrait) -> Option<CharacterTrait> {
    for &(a, b) in OPPOSING_PAIRS {
        if t == a { return Some(b); }
        if t == b { return Some(a); }
    }
    None
}

/// Add a trait to a person, removing its opposite if present. Caps at MAX_TRAITS.
pub fn earn_trait(person: &mut Person, new_trait: CharacterTrait) {
    if person.traits.contains(&new_trait) { return; }

    // Remove opposing trait
    if let Some(opp) = opposite_of(new_trait) {
        person.traits.retain(|t| *t != opp);
    }

    if person.traits.len() < MAX_TRAITS {
        person.traits.push(new_trait);
    }
}

/// Remove a trait from a person (e.g., losing Devout without gaining Skeptical).
pub fn lose_trait(person: &mut Person, trait_to_lose: CharacterTrait) {
    person.traits.retain(|t| *t != trait_to_lose);
}

/// Evaluate trait changes from a life event. Called after each event is applied.
pub fn evaluate_trait_change(person: &mut Person, event: &LifeEvent, rng: &mut impl Rng) {
    match &event.kind {
        LifeEventKind::LostChild { cause, .. } => {
            match cause {
                DeathCause::Plague | DeathCause::Famine => {
                    if person.traits.contains(&Devout) && rng.random::<f32>() < 0.3 {
                        lose_trait(person, Devout);
                    }
                }
                DeathCause::War => {
                    if person.traits.contains(&Peaceful) {
                        // Peaceful parent who lost child to war — could go either way
                        if rng.random::<f32>() < 0.4 {
                            earn_trait(person, Warlike); // vengeful
                        }
                    } else if !person.traits.contains(&Warlike) && rng.random::<f32>() < 0.3 {
                        earn_trait(person, Peaceful); // never again
                    }
                }
                _ => {}
            }
            // Multiple child deaths → paranoia
            let child_deaths = person.life_events.iter()
                .filter(|e| matches!(e.kind, LifeEventKind::LostChild { .. }))
                .count();
            if child_deaths >= 2 && rng.random::<f32>() < 0.4 {
                earn_trait(person, Paranoid);
            }
        }

        LifeEventKind::SurvivedWar { .. } => {
            if !person.traits.contains(&Brave) {
                earn_trait(person, Brave);
            } else if rng.random::<f32>() < 0.3 {
                earn_trait(person, Warlike); // hardened
            }
            // Lost family to war + survived → may become ruthless
            let war_losses = person.life_events.iter()
                .filter(|e| matches!(e.kind,
                    LifeEventKind::LostParent { cause: DeathCause::War, .. }
                    | LifeEventKind::LostSpouse { cause: DeathCause::War, .. }
                    | LifeEventKind::LostChild { cause: DeathCause::War, .. }
                ))
                .count();
            if war_losses >= 2 && rng.random::<f32>() < 0.3 {
                earn_trait(person, Ruthless);
            }
        }

        LifeEventKind::SurvivedPlague => {
            if person.traits.contains(&Devout) {
                // Devout survivor — faith tested
                let children_died_plague = person.life_events.iter()
                    .filter(|e| matches!(e.kind, LifeEventKind::LostChild { cause: DeathCause::Plague, .. }))
                    .count();
                if children_died_plague > 0 && rng.random::<f32>() < 0.5 {
                    lose_trait(person, Devout);
                    if children_died_plague >= 2 {
                        earn_trait(person, Skeptical);
                    }
                }
            } else if rng.random::<f32>() < 0.2 {
                earn_trait(person, Devout); // thanking the gods for survival
            }
        }

        LifeEventKind::LostSpouse { .. } => {
            if rng.random::<f32>() < 0.2 {
                earn_trait(person, Reclusive);
            }
            if person.age(event.year) < 30 && rng.random::<f32>() < 0.2 {
                earn_trait(person, Ambitious); // redirect grief into purpose
            }
        }

        LifeEventKind::DraftedToWar { .. } => {
            if person.traits.contains(&Peaceful) && rng.random::<f32>() < 0.3 {
                earn_trait(person, Cowardly); // forced into something they hate
            }
        }

        LifeEventKind::SettlementConquered { .. } => {
            let roll: f32 = rng.random();
            if roll < 0.2 {
                earn_trait(person, Ambitious); // desire to reclaim
            } else if roll < 0.35 {
                earn_trait(person, Treacherous); // resentful of new rulers
            }
        }

        LifeEventKind::FaithStrengthened { .. } => {
            if !person.traits.contains(&Devout) && rng.random::<f32>() < 0.3 {
                earn_trait(person, Devout);
            }
            if person.traits.contains(&Devout) && rng.random::<f32>() < 0.1 {
                earn_trait(person, Fanatical);
            }
        }

        LifeEventKind::FaithShaken { .. } => {
            if person.traits.contains(&Devout) {
                lose_trait(person, Devout);
            }
        }

        LifeEventKind::AbandonedFaith { .. } => {
            lose_trait(person, Devout);
            lose_trait(person, Fanatical);
            if rng.random::<f32>() < 0.3 {
                earn_trait(person, Skeptical);
            }
        }

        _ => {} // ChildBorn, MarriedTo, LostParent, LostSibling, ConvertedFaith — no trait change
    }
}
