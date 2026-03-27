use rand::{Rng, RngExt};

use crate::resources::magic::MagicSchool;
use crate::worldgen::gods::{DrawnPantheon, GodId};
use crate::worldgen::world_map::{WorldMap, WorldPos};

use super::events::{DivineEvent, DivineEventKind};
use super::personality::{DivineDrive, DivineFlaw};
use super::state::{DivineWorldState, GodState};
use super::terrain_scars::{DivineTerrainType, TerrainScar, TerrainScarCause};

/// The Fall — the event that ended the divine era.
#[derive(Clone, Debug)]
pub struct TheFall {
    pub cause: FallCause,
    /// Year the Fall occurred (last year of divine era).
    pub year: i32,
    pub description: String,
    pub consequences: Vec<FallConsequence>,
    /// The god most responsible, if any.
    pub instigator: Option<GodId>,
}

/// What caused all the gods to disappear.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FallCause {
    /// One god's hubris drove them to overreach, and reality broke.
    Hubris,
    /// A jealous god destroyed what they coveted, and the chain reaction consumed all.
    Jealousy,
    /// A god's obsession with their drive consumed them and everything around them.
    Obsession,
    /// A god's cruelty turned mortals against all divinity.
    Cruelty,
    /// Mutual destruction — gods who couldn't stop fighting destroyed each other.
    MutualDestruction,
    /// A betrayal so profound it shattered the divine order.
    Betrayal,
    /// The gods exhausted themselves — too much sacrifice, too many wars.
    Exhaustion,
    /// A god achieved everything and found it hollow — their nihilism unmade the world.
    Hollowness,
    /// The gods' rigid order became a cage — mortals or reality itself broke free.
    Rebellion,
    /// A god withdrew so completely they pulled the divine from the world.
    Withdrawal,
}

/// A consequence of the Fall.
#[derive(Clone, Debug)]
pub enum FallConsequence {
    GodBanished(GodId),
    GodWeakened(GodId),
    GodFragmented(GodId),
    MagicDiminished(MagicSchool),
    TerrainScarred(Vec<WorldPos>, DivineTerrainType),
    RaceCursed(u32),
    ArtifactScattered,
    BarrierRaised,
}

/// Score how well a particular FallCause fits the divine era that played out.
struct CauseCandidate {
    cause: FallCause,
    score: f32,
    instigator: Option<GodId>,
    description: String,
}

/// Analyze the divine era's final state and derive the Fall from the actual story.
#[allow(clippy::too_many_arguments)]
pub fn derive_the_fall(
    gods: &[GodState],
    world_state: &DivineWorldState,
    events: &[DivineEvent],
    _artifacts: &[super::artifacts::DivineArtifact],
    terrain_scars: &mut Vec<TerrainScar>,
    world_map: &mut WorldMap,
    pantheon: &DrawnPantheon,
    fall_year: i32,
    rng: &mut impl Rng,
) -> TheFall {
    let mut candidates: Vec<CauseCandidate> = Vec::new();

    // --- Score each possible cause based on what actually happened ---

    // Hubris: a god with Hubris flaw who had many flaw triggers (NarrativeAdvanced events)
    // Also fits when one god became vastly more powerful than others
    if let Some(hubris_god) = find_god_with_flaw(gods, DivineFlaw::Hubris) {
        let flaw_events = count_narrative_events_for(events, hubris_god.god_id);
        let power_gap = hubris_god.power as f32 - avg_power(gods);
        let name = pantheon.name(hubris_god.god_id).unwrap_or("A god");
        let mut score = flaw_events as f32 * 10.0 + power_gap;
        if hubris_god.wars_won > 0 { score += 15.0; }
        if hubris_god.artifacts_created > 3 { score += 10.0; }
        candidates.push(CauseCandidate {
            cause: FallCause::Hubris,
            score,
            instigator: Some(hubris_god.god_id),
            description: format!(
                "{}, drunk on victory and power, reached beyond the limits of divinity itself. \
                 The world could not contain what they tried to become, and reality shattered",
                name
            ),
        });
    }

    // Jealousy: a god with Jealousy flaw who attacked the most successful god
    if let Some(jealous_god) = find_god_with_flaw(gods, DivineFlaw::Jealousy) {
        let flaw_events = count_narrative_events_for(events, jealous_god.god_id);
        let most_worshipped = gods.iter()
            .filter(|g| g.god_id != jealous_god.god_id && g.is_active())
            .max_by_key(|g| g.worshipper_settlements.len());
        let name = pantheon.name(jealous_god.god_id).unwrap_or("A god");
        let mut score = flaw_events as f32 * 12.0;
        if jealous_god.wars_fought > 0 { score += 15.0; }
        let target_name = most_worshipped
            .and_then(|g| pantheon.name(g.god_id))
            .unwrap_or("the others");
        candidates.push(CauseCandidate {
            cause: FallCause::Jealousy,
            score,
            instigator: Some(jealous_god.god_id),
            description: format!(
                "{}, consumed by jealousy of {}, sought to destroy what they could never have. \
                 Their spite poisoned the divine and dragged all gods into the abyss",
                name, target_name
            ),
        });
    }

    // Obsession: a god with Obsession flaw who neglected everything in pursuit of their drive
    if let Some(obsessed_god) = find_god_with_flaw(gods, DivineFlaw::Obsession) {
        let flaw_events = count_narrative_events_for(events, obsessed_god.god_id);
        let low_devotion = world_state.settlements.iter()
            .filter(|s| s.patron_god == Some(obsessed_god.god_id) && s.devotion < 30)
            .count();
        let name = pantheon.name(obsessed_god.god_id).unwrap_or("A god");
        let drive_name = drive_description(obsessed_god.drive());
        let mut score = flaw_events as f32 * 10.0 + low_devotion as f32 * 5.0;
        if obsessed_god.artifacts_created > 4 { score += 10.0; }
        candidates.push(CauseCandidate {
            cause: FallCause::Obsession,
            score,
            instigator: Some(obsessed_god.god_id),
            description: format!(
                "{}, obsessed with {}, poured everything into their pursuit until \
                 nothing remained — not their followers, not their allies, not the world itself",
                name, drive_name
            ),
        });
    }

    // Cruelty: a god with Cruelty flaw whose worshippers lost devotion
    if let Some(cruel_god) = find_god_with_flaw(gods, DivineFlaw::Cruelty) {
        let flaw_events = count_narrative_events_for(events, cruel_god.god_id);
        let name = pantheon.name(cruel_god.god_id).unwrap_or("A god");
        let score = flaw_events as f32 * 12.0;
        candidates.push(CauseCandidate {
            cause: FallCause::Cruelty,
            score,
            instigator: Some(cruel_god.god_id),
            description: format!(
                "{} ruled through fear and punishment, until the mortals — \
                 broken but united in desperation — rejected all gods rather than suffer any longer",
                name
            ),
        });
    }

    // Mutual Destruction: wars happened, gods were vanquished
    let total_wars = events.iter().filter(|e| e.kind == DivineEventKind::DivineWarDeclared).count();
    let vanquished = gods.iter().filter(|g| !g.is_active()).count();
    let active_wars = world_state.active_wars.len();
    {
        let mut score = total_wars as f32 * 8.0 + vanquished as f32 * 15.0 + active_wars as f32 * 20.0;
        // Find the god who fought the most
        let warrior = gods.iter().max_by_key(|g| g.wars_fought);
        let instigator = warrior.filter(|g| g.wars_fought >= 2).map(|g| g.god_id);
        if vanquished >= 2 { score += 20.0; }
        let desc = if let Some(id) = instigator {
            let name = pantheon.name(id).unwrap_or("A god");
            format!(
                "The divine wars, fueled by {}'s aggression, escalated until the gods \
                 destroyed each other. Their final battle tore reality apart",
                name
            )
        } else {
            "The gods could not stop fighting. War after war eroded their power \
             until the final clash consumed them all and shattered the world".into()
        };
        candidates.push(CauseCandidate {
            cause: FallCause::MutualDestruction,
            score,
            instigator,
            description: desc,
        });
    }

    // Betrayal: pacts were broken, a god with Betrayal flaw
    let betrayals = events.iter().filter(|e| e.kind == DivineEventKind::PactBroken).count();
    if let Some(betrayer) = find_god_with_flaw(gods, DivineFlaw::Betrayal) {
        let name = pantheon.name(betrayer.god_id).unwrap_or("A god");
        let flaw_events = count_narrative_events_for(events, betrayer.god_id);
        let score = betrayals as f32 * 15.0 + flaw_events as f32 * 10.0;
        candidates.push(CauseCandidate {
            cause: FallCause::Betrayal,
            score,
            instigator: Some(betrayer.god_id),
            description: format!(
                "{} betrayed those who trusted them. The broken oaths cascaded — \
                 trust between gods collapsed entirely, and without trust, divinity could not endure",
                name
            ),
        });
    } else if betrayals > 0 {
        candidates.push(CauseCandidate {
            cause: FallCause::Betrayal,
            score: betrayals as f32 * 12.0,
            instigator: None,
            description: "Too many oaths were broken. The gods' word was once unbreakable, \
                         but betrayal after betrayal eroded the foundation of the divine order".into(),
        });
    }

    // Exhaustion: gods with Sacrifice flaw, low average power
    let avg_pow = avg_power(gods);
    if let Some(sacrifice_god) = find_god_with_flaw(gods, DivineFlaw::Sacrifice) {
        let name = pantheon.name(sacrifice_god.god_id).unwrap_or("A god");
        let flaw_events = count_narrative_events_for(events, sacrifice_god.god_id);
        let low_power_bonus = if avg_pow < 50.0 { 20.0 } else { 0.0 };
        let score = flaw_events as f32 * 10.0 + low_power_bonus + vanquished as f32 * 10.0;
        candidates.push(CauseCandidate {
            cause: FallCause::Exhaustion,
            score,
            instigator: Some(sacrifice_god.god_id),
            description: format!(
                "{} gave too much of themselves. And they were not alone — the gods \
                 had spent their power in wars, creations, and sacrifice until nothing remained",
                name
            ),
        });
    }

    // Hollowness: a god with Hollowness flaw who achieved their drive
    if let Some(hollow_god) = find_god_with_flaw(gods, DivineFlaw::Hollowness) {
        let name = pantheon.name(hollow_god.god_id).unwrap_or("A god");
        let flaw_events = count_narrative_events_for(events, hollow_god.god_id);
        let drive_name = drive_description(hollow_god.drive());
        let score = flaw_events as f32 * 12.0 + hollow_god.artifacts_created as f32 * 3.0;
        candidates.push(CauseCandidate {
            cause: FallCause::Hollowness,
            score,
            instigator: Some(hollow_god.god_id),
            description: format!(
                "{} achieved {} and found it empty. Their despair became a void \
                 that consumed the meaning from all divine works, and the gods faded",
                name, drive_name
            ),
        });
    }

    // Rebellion: a god with Rigidity flaw, or Cruelty flaw + low mortal devotion
    if let Some(rigid_god) = find_god_with_flaw(gods, DivineFlaw::Rigidity) {
        let name = pantheon.name(rigid_god.god_id).unwrap_or("A god");
        let flaw_events = count_narrative_events_for(events, rigid_god.god_id);
        let avg_devotion = avg_settlement_devotion(world_state);
        let low_devotion_bonus = if avg_devotion < 40.0 { 15.0 } else { 0.0 };
        let score = flaw_events as f32 * 10.0 + low_devotion_bonus;
        candidates.push(CauseCandidate {
            cause: FallCause::Rebellion,
            score,
            instigator: Some(rigid_god.god_id),
            description: format!(
                "{}'s unyielding order became a cage. The mortals, suffocated by divine law, \
                 rose up — and when they rejected one god, they rejected them all",
                name
            ),
        });
    }

    // Withdrawal: a god with Isolation flaw who withdrew
    if let Some(isolated_god) = find_god_with_flaw(gods, DivineFlaw::Isolation) {
        let name = pantheon.name(isolated_god.god_id).unwrap_or("A god");
        let flaw_events = count_narrative_events_for(events, isolated_god.god_id);
        let few_worshippers = isolated_god.worshipper_settlements.is_empty();
        let score = flaw_events as f32 * 10.0 + if few_worshippers { 15.0 } else { 0.0 };
        candidates.push(CauseCandidate {
            cause: FallCause::Withdrawal,
            score,
            instigator: Some(isolated_god.god_id),
            description: format!(
                "{} withdrew from the world entirely. The absence of one god \
                 destabilized the others — divinity is a web, and when one thread is pulled, \
                 the whole tapestry unravels",
                name
            ),
        });
    }

    // --- Pick the highest-scoring cause ---
    // Add some randomness so the same seed doesn't always pick the same marginal winner
    for c in candidates.iter_mut() {
        c.score += rng.random::<f32>() * 10.0;
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let chosen = candidates.into_iter().next().unwrap_or(CauseCandidate {
        cause: FallCause::MutualDestruction,
        score: 0.0,
        instigator: None,
        description: "The divine era ended. The cause is lost to time".into(),
    });

    // --- Generate consequences ---
    let mut consequences: Vec<FallConsequence> = Vec::new();

    // The instigator gets the harshest consequence
    if let Some(id) = chosen.instigator {
        match chosen.cause {
            FallCause::Hubris | FallCause::Obsession | FallCause::Jealousy => {
                consequences.push(FallConsequence::GodFragmented(id));
            }
            FallCause::Betrayal | FallCause::Cruelty => {
                consequences.push(FallConsequence::GodBanished(id));
            }
            _ => {
                consequences.push(FallConsequence::GodWeakened(id));
            }
        }
    }

    // Other active gods get weaker consequences
    for god in gods.iter().filter(|g| g.is_active()) {
        if Some(god.god_id) == chosen.instigator { continue; }
        consequences.push(FallConsequence::GodBanished(god.god_id));
    }

    consequences.push(FallConsequence::ArtifactScattered);
    consequences.push(FallConsequence::BarrierRaised);

    // Apply Fall terrain scars around seats of power
    for god in gods {
        if let Some(seat) = god.seat_of_power {
            let dt = DivineTerrainType::Blight;
            let radius = rng.random_range(3..8i32);
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if (dx.abs() + dy.abs()) > radius { continue; }
                    if rng.random::<f32>() > 0.5 { continue; }
                    let pos = WorldPos::new(seat.x + dx, seat.y + dy);
                    if let Some(cell) = world_map.get_mut(pos) {
                        cell.divine_terrain = Some(dt);
                        terrain_scars.push(TerrainScar {
                            id: 0,
                            world_pos: pos,
                            terrain_type: dt,
                            cause: TerrainScarCause::TheFall,
                            caused_year: fall_year,
                            caused_by: vec![],
                            description: "Scarred by the Fall of the gods".into(),
                        });
                    }
                }
            }
        }
    }

    TheFall {
        cause: chosen.cause,
        year: fall_year,
        description: chosen.description,
        consequences,
        instigator: chosen.instigator,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_god_with_flaw(gods: &[GodState], flaw: DivineFlaw) -> Option<&GodState> {
    gods.iter()
        .filter(|g| g.is_active() && g.flaw() == flaw)
        .max_by_key(|g| g.flaw_pressure)
}

fn count_narrative_events_for(events: &[DivineEvent], god_id: GodId) -> usize {
    events.iter()
        .filter(|e| e.kind == DivineEventKind::NarrativeAdvanced && e.participants.contains(&god_id))
        .count()
}

fn avg_power(gods: &[GodState]) -> f32 {
    let active: Vec<&GodState> = gods.iter().filter(|g| g.is_active()).collect();
    if active.is_empty() { return 0.0; }
    active.iter().map(|g| g.power as f32).sum::<f32>() / active.len() as f32
}

fn avg_settlement_devotion(world_state: &DivineWorldState) -> f32 {
    let patronized: Vec<&super::state::DivineSettlement> = world_state.settlements.iter()
        .filter(|s| s.patron_god.is_some())
        .collect();
    if patronized.is_empty() { return 50.0; }
    patronized.iter().map(|s| s.devotion as f32).sum::<f32>() / patronized.len() as f32
}

fn drive_description(drive: DivineDrive) -> &'static str {
    match drive {
        DivineDrive::Knowledge => "understanding the secret of creation",
        DivineDrive::Dominion => "absolute dominion over all",
        DivineDrive::Worship => "the adoration of mortals",
        DivineDrive::Perfection => "the perfect creation",
        DivineDrive::Justice => "a world of perfect order",
        DivineDrive::Love => "protecting what they loved",
        DivineDrive::Freedom => "breaking every chain",
        DivineDrive::Legacy => "a legacy that would outlast eternity",
        DivineDrive::Vindication => "proving they were right all along",
        DivineDrive::Supremacy => "being the strongest of all",
    }
}
