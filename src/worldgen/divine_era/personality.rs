use rand::Rng;

use crate::resources::magic::MagicSchool;
use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::population_table::PopTable;

/// What a god wants most — the engine of their story.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DivineDrive {
    /// Understand the secret of creation. Odin, Thoth.
    Knowledge,
    /// Everything under their rule, their way. Zeus, Ra.
    Dominion,
    /// Be the most revered, feared, loved by mortals. Apollo, Amun.
    Worship,
    /// Create the ultimate work, master their craft. Hephaestus.
    Perfection,
    /// Make the world fair and ordered. Athena, Ma'at.
    Justice,
    /// Protect what they've become attached to. Isis, Freya.
    Love,
    /// Be unbound, break every rule. Loki, Hermes.
    Freedom,
    /// Their creations must outlast everything. Prometheus.
    Legacy,
    /// Prove they were right, punish those who doubted. Hera, Set.
    Vindication,
    /// Be the strongest, defeat all rivals. Ares, Thor.
    Supremacy,
}

/// How pursuing the drive destroys them — the tragic flaw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DivineFlaw {
    /// Believes they're above consequences. Zeus's affairs.
    Hubris,
    /// Can't stand others having what they want. Set murdering Osiris.
    Jealousy,
    /// Pursues the drive past all reason. Odin sacrificing everything.
    Obsession,
    /// Punishes the wrong people for their frustrations. Hera targeting Zeus's children.
    Cruelty,
    /// Can't see the cost of their actions on others. Hades taking Persephone.
    Blindness,
    /// Pushes everyone away. Hades ruling alone.
    Isolation,
    /// Breaks trust to get what they want. Loki killing Baldr.
    Betrayal,
    /// Gives up too much, loses themselves. Odin, Tyr.
    Sacrifice,
    /// So committed they can't adapt. Athena and Arachne.
    Rigidity,
    /// Gets what they want and it's meaningless. Set as king, unloved.
    Hollowness,
}

/// A god's complete personality — derived from domain + traits.
#[derive(Clone, Debug)]
pub struct DivinePersonality {
    pub drive: DivineDrive,
    pub flaw: DivineFlaw,
}

// ---------------------------------------------------------------------------
// Drive assignment: domain base weights + trait modifiers
// ---------------------------------------------------------------------------

/// Base drive weights for a magic school. Each domain has 2-3 preferred drives.
fn domain_drive_weights(domain: MagicSchool) -> Vec<(DivineDrive, f32)> {
    use DivineDrive::*;
    let base = 5.0;
    let preferred = 15.0;
    let moderate = 10.0;

    let mut weights = vec![
        (Knowledge, base), (Dominion, base), (Worship, base),
        (Perfection, base), (Justice, base), (Love, base),
        (Freedom, base), (Legacy, base), (Vindication, base),
        (Supremacy, base),
    ];

    let boosts: &[(DivineDrive, f32)] = match domain {
        MagicSchool::Fire => &[(Supremacy, preferred), (Perfection, moderate), (Vindication, moderate)],
        MagicSchool::Frost => &[(Knowledge, preferred), (Perfection, moderate), (Justice, moderate)],
        MagicSchool::Storm => &[(Freedom, preferred), (Dominion, moderate), (Supremacy, moderate)],
        MagicSchool::Holy => &[(Justice, preferred), (Worship, moderate), (Dominion, moderate)],
        MagicSchool::Shadow => &[(Knowledge, preferred), (Freedom, moderate), (Vindication, moderate)],
        MagicSchool::Nature => &[(Legacy, preferred), (Love, moderate), (Justice, moderate)],
        MagicSchool::Necromancy => &[(Knowledge, preferred), (Worship, moderate), (Vindication, moderate)],
        MagicSchool::Arcane => &[(Knowledge, preferred), (Perfection, moderate), (Legacy, moderate)],
        _ => &[],
    };

    for &(drive, boost) in boosts {
        if let Some(entry) = weights.iter_mut().find(|(d, _)| *d == drive) {
            entry.1 += boost;
        }
    }

    weights
}

/// Trait-to-drive modifiers. Each trait boosts 1-2 drives.
fn trait_drive_modifier(t: CharacterTrait) -> &'static [(DivineDrive, f32)] {
    use CharacterTrait::*;
    use DivineDrive::*;
    match t {
        Wise => &[(Knowledge, 8.0), (Legacy, 5.0)],
        Scholarly => &[(Knowledge, 8.0), (Perfection, 5.0)],
        Cunning => &[(Freedom, 8.0), (Knowledge, 5.0)],
        PowerHungry => &[(Dominion, 8.0), (Supremacy, 5.0)],
        Ambitious => &[(Dominion, 5.0), (Supremacy, 5.0)],
        Warlike => &[(Supremacy, 8.0), (Dominion, 5.0)],
        Charismatic => &[(Worship, 8.0), (Love, 5.0)],
        Devout => &[(Worship, 5.0), (Love, 5.0)],
        Fanatical => &[(Worship, 5.0), (Justice, 8.0)],
        Just => &[(Justice, 8.0), (Legacy, 5.0)],
        Honorable => &[(Justice, 5.0), (Love, 5.0)],
        Loyal => &[(Love, 8.0), (Legacy, 5.0)],
        Peaceful => &[(Love, 5.0), (Legacy, 5.0)],
        Treacherous => &[(Freedom, 8.0), (Vindication, 5.0)],
        Paranoid => &[(Vindication, 8.0), (Dominion, 5.0)],
        Cruel => &[(Vindication, 8.0), (Supremacy, 5.0)],
        Ruthless => &[(Supremacy, 8.0), (Vindication, 5.0)],
        Brave => &[(Supremacy, 5.0), (Freedom, 5.0)],
        Greedy => &[(Dominion, 5.0), (Perfection, 5.0)],
        Corrupt => &[(Freedom, 5.0), (Dominion, 5.0)],
        Reclusive => &[(Knowledge, 5.0), (Perfection, 5.0)],
        Diplomatic => &[(Worship, 5.0), (Justice, 5.0)],
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// Flaw assignment: domain base weights + trait modifiers
// ---------------------------------------------------------------------------

/// Base flaw weights for a magic school.
fn domain_flaw_weights(domain: MagicSchool) -> Vec<(DivineFlaw, f32)> {
    use DivineFlaw::*;
    let base = 5.0;
    let preferred = 15.0;
    let moderate = 10.0;

    let mut weights = vec![
        (Hubris, base), (Jealousy, base), (Obsession, base),
        (Cruelty, base), (Blindness, base), (Isolation, base),
        (Betrayal, base), (Sacrifice, base), (Rigidity, base),
        (Hollowness, base),
    ];

    let boosts: &[(DivineFlaw, f32)] = match domain {
        MagicSchool::Fire => &[(Hubris, preferred), (Obsession, moderate), (Cruelty, moderate)],
        MagicSchool::Frost => &[(Isolation, preferred), (Rigidity, moderate), (Blindness, moderate)],
        MagicSchool::Storm => &[(Hubris, preferred), (Betrayal, moderate), (Obsession, moderate)],
        MagicSchool::Holy => &[(Rigidity, preferred), (Cruelty, moderate), (Blindness, moderate)],
        MagicSchool::Shadow => &[(Betrayal, preferred), (Isolation, moderate), (Jealousy, moderate)],
        MagicSchool::Nature => &[(Rigidity, preferred), (Blindness, moderate), (Obsession, moderate)],
        MagicSchool::Necromancy => &[(Isolation, preferred), (Hollowness, moderate), (Jealousy, moderate)],
        MagicSchool::Arcane => &[(Obsession, preferred), (Hubris, moderate), (Blindness, moderate)],
        _ => &[],
    };

    for &(flaw, boost) in boosts {
        if let Some(entry) = weights.iter_mut().find(|(f, _)| *f == flaw) {
            entry.1 += boost;
        }
    }

    weights
}

/// Trait-to-flaw modifiers.
fn trait_flaw_modifier(t: CharacterTrait) -> &'static [(DivineFlaw, f32)] {
    use CharacterTrait::*;
    use DivineFlaw::*;
    match t {
        Ambitious => &[(Hubris, 8.0), (Jealousy, 5.0)],
        PowerHungry => &[(Hubris, 8.0), (Cruelty, 5.0)],
        Brave => &[(Hubris, 5.0), (Sacrifice, 5.0)],
        Greedy => &[(Jealousy, 8.0), (Hollowness, 5.0)],
        Paranoid => &[(Jealousy, 5.0), (Isolation, 8.0)],
        Fanatical => &[(Obsession, 8.0), (Rigidity, 5.0)],
        Scholarly => &[(Obsession, 5.0), (Blindness, 5.0)],
        Devout => &[(Obsession, 5.0), (Sacrifice, 5.0)],
        Cruel => &[(Cruelty, 8.0), (Blindness, 5.0)],
        Ruthless => &[(Cruelty, 8.0), (Hubris, 5.0)],
        Warlike => &[(Cruelty, 5.0), (Hubris, 5.0)],
        Reclusive => &[(Isolation, 8.0), (Blindness, 5.0)],
        Cowardly => &[(Isolation, 5.0), (Sacrifice, 5.0)],
        Treacherous => &[(Betrayal, 8.0), (Hollowness, 5.0)],
        Cunning => &[(Betrayal, 5.0), (Hubris, 5.0)],
        Corrupt => &[(Betrayal, 5.0), (Hollowness, 5.0)],
        Loyal => &[(Sacrifice, 8.0), (Rigidity, 5.0)],
        Just => &[(Rigidity, 8.0), (Cruelty, 5.0)],
        Honorable => &[(Rigidity, 5.0), (Sacrifice, 5.0)],
        Wise => &[(Hollowness, 5.0), (Blindness, 5.0)],
        Content => &[(Hollowness, 8.0), (Blindness, 5.0)],
        Peaceful => &[(Blindness, 5.0), (Sacrifice, 5.0)],
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// Personality rolling
// ---------------------------------------------------------------------------

/// Roll a god's personality (drive + flaw) from their domain and traits.
pub fn roll_personality(
    domain: MagicSchool,
    traits: &[CharacterTrait],
    rng: &mut impl Rng,
) -> DivinePersonality {
    // Build drive weights
    let mut drive_weights = domain_drive_weights(domain);
    for &t in traits {
        for &(drive, boost) in trait_drive_modifier(t) {
            if let Some(entry) = drive_weights.iter_mut().find(|(d, _)| *d == drive) {
                entry.1 += boost;
            }
        }
    }

    // Build flaw weights
    let mut flaw_weights = domain_flaw_weights(domain);
    for &t in traits {
        for &(flaw, boost) in trait_flaw_modifier(t) {
            if let Some(entry) = flaw_weights.iter_mut().find(|(f, _)| *f == flaw) {
                entry.1 += boost;
            }
        }
    }

    let drive_table = PopTable::pick_one(drive_weights);
    let flaw_table = PopTable::pick_one(flaw_weights);

    let drive = drive_table.roll_one(rng).unwrap();
    let flaw = flaw_table.roll_one(rng).unwrap();

    DivinePersonality { drive, flaw }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn roll_personality_produces_result() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let p = roll_personality(
            MagicSchool::Fire,
            &[CharacterTrait::Warlike, CharacterTrait::Ambitious],
            &mut rng,
        );
        // Fire + Warlike + Ambitious should strongly favor Supremacy/Dominion + Hubris
        println!("Fire+Warlike+Ambitious: {:?} / {:?}", p.drive, p.flaw);
    }

    #[test]
    fn personality_deterministic() {
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(99);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(99);
        let p1 = roll_personality(MagicSchool::Shadow, &[CharacterTrait::Treacherous], &mut rng1);
        let p2 = roll_personality(MagicSchool::Shadow, &[CharacterTrait::Treacherous], &mut rng2);
        assert_eq!(p1.drive, p2.drive);
        assert_eq!(p1.flaw, p2.flaw);
    }

    #[test]
    fn domain_influences_drive() {
        // Run many rolls to check that domain-preferred drives appear more often
        let mut rng = rand::rngs::StdRng::seed_from_u64(12345);
        let mut knowledge_count = 0u32;
        let trials = 1000;
        for _ in 0..trials {
            let p = roll_personality(MagicSchool::Arcane, &[CharacterTrait::Scholarly], &mut rng);
            if p.drive == DivineDrive::Knowledge {
                knowledge_count += 1;
            }
        }
        // Arcane + Scholarly should produce Knowledge drive well above base rate (10%)
        let rate = knowledge_count as f32 / trials as f32;
        assert!(rate > 0.20, "Knowledge rate for Arcane+Scholarly was only {:.1}%", rate * 100.0);
    }

    #[test]
    fn traits_can_override_domain() {
        // A fire god with Wise+Scholarly should sometimes get Knowledge drive
        let mut rng = rand::rngs::StdRng::seed_from_u64(54321);
        let mut knowledge_count = 0u32;
        let trials = 1000;
        for _ in 0..trials {
            let p = roll_personality(
                MagicSchool::Fire,
                &[CharacterTrait::Wise, CharacterTrait::Scholarly],
                &mut rng,
            );
            if p.drive == DivineDrive::Knowledge {
                knowledge_count += 1;
            }
        }
        // Should be above base rate but below Arcane+Scholarly rate
        let rate = knowledge_count as f32 / trials as f32;
        assert!(rate > 0.05, "Knowledge rate for Fire+Wise+Scholarly was only {:.1}%", rate * 100.0);
    }

    #[test]
    fn all_domains_produce_personality() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        for domain in [
            MagicSchool::Fire, MagicSchool::Frost, MagicSchool::Storm,
            MagicSchool::Holy, MagicSchool::Shadow, MagicSchool::Nature,
            MagicSchool::Necromancy, MagicSchool::Arcane,
        ] {
            let p = roll_personality(domain, &[CharacterTrait::Ambitious], &mut rng);
            // Just verify no panic
            let _ = format!("{:?}/{:?}", p.drive, p.flaw);
        }
    }
}
