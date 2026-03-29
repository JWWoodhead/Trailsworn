//! Person trait seeding, earning, and event-driven trait changes.
//! Traits are seeded at birth (2, influenced by parents) and earned from life events.
//! Opposing traits replace each other. Trait changes are DETERMINISTIC based on
//! the person's existing traits and life history — no random rolls.

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
    (Purist, Tolerant),
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
    Purist, Tolerant,
];

/// Seed 2 traits for a newborn, influenced by parents.
/// This is the ONLY place randomness is used for traits.
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

/// Add a trait to a person, removing its opposite if present. Caps at MAX_TRAITS.
pub fn earn_trait(person: &mut Person, new_trait: CharacterTrait) {
    if person.traits.contains(&new_trait) { return; }

    // Remove opposing trait
    for &(a, b) in OPPOSING_PAIRS {
        if new_trait == a { person.traits.retain(|t| *t != b); }
        if new_trait == b { person.traits.retain(|t| *t != a); }
    }

    if person.traits.len() < MAX_TRAITS {
        person.traits.push(new_trait);
    }
}

/// Remove a trait from a person.
pub fn lose_trait(person: &mut Person, trait_to_lose: CharacterTrait) {
    person.traits.retain(|t| *t != trait_to_lose);
}

/// Helper: count events of a specific pattern in a person's history.
fn count_events(person: &Person, pred: impl Fn(&LifeEventKind) -> bool) -> usize {
    person.life_events.iter().filter(|e| pred(&e.kind)).count()
}

/// Evaluate trait changes from a life event. DETERMINISTIC — no random rolls.
/// Existing traits determine the reaction. No trait without a prerequisite trait.
pub fn evaluate_trait_change(person: &mut Person, event: &LifeEvent, _rng: &mut impl Rng) {
    match &event.kind {
        LifeEventKind::LostChild { cause, .. } => {
            // Reaction depends on WHO THE PERSON IS, not just what happened
            match cause {
                DeathCause::War => {
                    if person.traits.contains(&Peaceful) {
                        // Peaceful person's child killed by war → rage overrides peace
                        earn_trait(person, Warlike);
                    } else if person.traits.contains(&Honorable) {
                        // Honorable person → seeks justice
                        earn_trait(person, Just);
                    }
                }
                DeathCause::Plague | DeathCause::Famine => {
                    if person.traits.contains(&Devout) {
                        // Devout person loses child to preventable cause → faith shaken
                        lose_trait(person, Devout);
                    }
                }
                _ => {}
            }

            // Ambitious person losing a child → channels grief into drive for control
            if person.traits.contains(&Ambitious) {
                earn_trait(person, PowerHungry);
            }
        }

        LifeEventKind::SurvivedWar { .. } => {
            let wars_survived = count_events(person, |k| matches!(k, LifeEventKind::SurvivedWar { .. }));
            let war_losses = count_events(person, |k| matches!(k,
                LifeEventKind::LostParent { cause: DeathCause::War, .. }
                | LifeEventKind::LostSpouse { cause: DeathCause::War, .. }
                | LifeEventKind::LostChild { cause: DeathCause::War, .. }
            ));

            // First war survived — brave (this one is universal, surviving war IS brave)
            if wars_survived == 1 && !person.traits.contains(&Brave) {
                earn_trait(person, Brave);
            }
            // Brave person survives multiple wars → hardened into warlike
            if wars_survived >= 2 && person.traits.contains(&Brave) {
                earn_trait(person, Warlike);
            }
            // Lost family to war AND survived → cruel person becomes ruthless
            if war_losses >= 2 && person.traits.contains(&Cruel) {
                earn_trait(person, Ruthless);
            }
        }

        LifeEventKind::SurvivedPlague => {
            let children_died_plague = count_events(person, |k| matches!(k,
                LifeEventKind::LostChild { cause: DeathCause::Plague, .. }
            ));

            if children_died_plague > 0 && person.traits.contains(&Devout) {
                // Devout person survived plague but children didn't → loses faith
                lose_trait(person, Devout);
            }
            if children_died_plague >= 2 && !person.traits.contains(&Devout) {
                // Already lost faith AND lost multiple children → actively rejects gods
                earn_trait(person, Skeptical);
            }
        }

        LifeEventKind::LostSpouse { .. } => {
            // No universal trait reaction to losing a spouse — most people grieve and move on.
            // Specific reactions may come from future systems (e.g., losing spouse to murder → seeks justice).
        }

        LifeEventKind::DraftedToWar { .. } => {
            if person.traits.contains(&Peaceful) {
                // Peaceful person forced into war → becomes cowardly
                earn_trait(person, Cowardly);
            }
            if person.traits.contains(&Loyal) {
                // Loyal person drafted → accepts duty, becomes brave
                earn_trait(person, Brave);
            }
        }

        LifeEventKind::SettlementConquered { .. } => {
            if person.traits.contains(&Loyal) {
                // Loyal person conquered → desire to reclaim
                earn_trait(person, Ambitious);
            } else if person.traits.contains(&Cunning) || person.traits.contains(&Greedy) {
                // Opportunistic person conquered → works against new rulers
                earn_trait(person, Treacherous);
            }
            // Most people just adapt — no trait change
        }

        LifeEventKind::FaithShaken { .. } => {
            // Faith shaken — lose devout if you have it
            if person.traits.contains(&Devout) {
                lose_trait(person, Devout);
            }
        }

        LifeEventKind::AbandonedFaith { .. } => {
            // Fully abandoned faith — becomes skeptical
            lose_trait(person, Devout);
            lose_trait(person, Fanatical);
            earn_trait(person, Skeptical);
        }

        // No trait changes for: ChildBorn, MarriedTo, LostParent, LostSibling,
        // FaithStrengthened, ConvertedFaith
        _ => {}
    }
}
