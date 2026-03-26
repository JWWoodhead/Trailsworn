use std::collections::HashMap;

use rand::{Rng, RngExt};

use crate::resources::abilities::TargetType;
use crate::resources::damage::DamageType;
use crate::resources::magic::{MagicCategory, MagicSchool};
use crate::terrain::TerrainType;
use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::population_table::PopTable;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

pub type GodId = u32;

/// Narrative functions from Propp's morphology, adapted for divine arcs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProppFunction {
    InitialSituation,
    Interdiction,
    Violation,
    Villainy,
    Departure,
    Testing,
    HelperGift,
    Struggle,
    Transformation,
    Resolution,
}

/// How a god shaped terrain.
#[derive(Clone, Debug)]
pub struct TerrainInfluence {
    pub primary_terrain: TerrainType,
    pub secondary_terrain: Option<TerrainType>,
    /// Future terrain type name (not yet in TerrainType enum).
    pub future_terrain: Option<String>,
    pub flavor: String,
}

/// A spell belonging to a god — pure data, not registered in AbilityRegistry.
#[derive(Clone, Debug)]
pub struct GodSpellDef {
    pub name: String,
    pub description: String,
    pub damage_type: Option<DamageType>,
    pub mana_cost: u32,
    pub cast_time_ticks: u32,
    pub cooldown_ticks: u32,
    pub range: f32,
    pub target_type: TargetType,
    pub base_damage: f32,
}

/// Definition of a god archetype. Domain, spells, terrain are fixed.
/// Name and personality traits are randomized per run during `draw_pantheon`.
#[derive(Clone, Debug)]
pub struct GodDef {
    pub id: GodId,
    /// Short label like "God of Fire" — the archetype, not the per-run name.
    pub title: String,
    pub domain: MagicSchool,
    /// Domain-specific trait weight modifiers applied on top of a base table.
    /// Format: (trait, weight_to_add). These boost/reduce weighted trait rolling.
    pub trait_modifiers: Vec<(CharacterTrait, f32)>,
    /// Traits that can never appear on this archetype — hard thematic blocklist.
    pub trait_blocklist: Vec<CharacterTrait>,
    pub aspect_description: String,
    pub terrain_influence: TerrainInfluence,
    pub gift_to_mortals: String,
    pub spells: Vec<GodSpellDef>,
    pub propp_tendencies: Vec<ProppFunction>,
}

impl GodDef {
    pub fn is_forbidden(&self) -> bool {
        self.domain.is_forbidden()
    }
}

/// A computed relationship between two drawn gods.
#[derive(Clone, Debug)]
pub struct GodRelationship {
    pub god_a: GodId,
    pub god_b: GodId,
    /// -100 (mortal enemies) to +100 (perfect affinity).
    pub affinity: i32,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// God Pool
// ---------------------------------------------------------------------------

/// The full pool of available god archetypes.
#[derive(Clone, Debug, Default, bevy::prelude::Resource)]
pub struct GodPool {
    pub gods: HashMap<GodId, GodDef>,
}

impl GodPool {
    pub fn register(&mut self, god: GodDef) {
        self.gods.insert(god.id, god);
    }

    pub fn get(&self, id: GodId) -> Option<&GodDef> {
        self.gods.get(&id)
    }

    /// Draw a pantheon of `count` gods from the pool.
    /// Randomizes names and personality traits for each drawn god.
    pub fn draw_pantheon(&self, count: usize, rng: &mut impl Rng) -> DrawnPantheon {
        let mut ids: Vec<GodId> = self.gods.keys().copied().collect();
        ids.sort(); // deterministic starting order

        // Fisher-Yates shuffle
        for i in (1..ids.len()).rev() {
            let j = rng.random_range(0..=i);
            ids.swap(i, j);
        }

        let drawn: Vec<GodId> = ids.into_iter().take(count.min(self.gods.len())).collect();

        // Roll names and traits for each drawn god
        let mut drawn_names = HashMap::new();
        let mut drawn_traits = HashMap::new();
        let mut used_names: Vec<String> = Vec::new();
        for &id in &drawn {
            if let Some(god) = self.get(id) {
                let name = generate_unique_god_name(&used_names, rng);
                used_names.push(name.clone());
                drawn_names.insert(id, name);

                let traits = roll_god_traits(&god.trait_modifiers, &god.trait_blocklist, rng);
                drawn_traits.insert(id, traits);
            }
        }

        let relationships = compute_relationships(&drawn, self, &drawn_traits);

        DrawnPantheon {
            god_ids: drawn,
            drawn_names,
            drawn_traits,
            relationships,
        }
    }
}

/// The gods active in this run, with their randomized identities.
#[derive(Clone, Debug, Default, bevy::prelude::Resource)]
pub struct DrawnPantheon {
    pub god_ids: Vec<GodId>,
    /// Randomized name for each drawn god.
    pub drawn_names: HashMap<GodId, String>,
    /// Personality traits rolled for each drawn god (2-4 per god).
    pub drawn_traits: HashMap<GodId, Vec<CharacterTrait>>,
    pub relationships: Vec<GodRelationship>,
}

impl DrawnPantheon {
    pub fn contains(&self, id: GodId) -> bool {
        self.god_ids.contains(&id)
    }

    /// Get the randomized name for a drawn god.
    pub fn name(&self, id: GodId) -> Option<&str> {
        self.drawn_names.get(&id).map(|s| s.as_str())
    }

    /// Get the rolled personality traits for a drawn god.
    pub fn traits(&self, id: GodId) -> &[CharacterTrait] {
        self.drawn_traits.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all magic schools present in this run.
    pub fn available_schools(&self, pool: &GodPool) -> Vec<MagicSchool> {
        self.god_ids
            .iter()
            .filter_map(|id| pool.get(*id))
            .map(|g| g.domain)
            .collect()
    }

    /// Get the relationship between two gods, if both are drawn.
    pub fn relationship(&self, a: GodId, b: GodId) -> Option<&GodRelationship> {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        self.relationships
            .iter()
            .find(|r| r.god_a == lo && r.god_b == hi)
    }
}

// ---------------------------------------------------------------------------
// God Name Generation (syllable-based)
// ---------------------------------------------------------------------------

const GOD_PREFIXES: &[&str] = &[
    "Vor", "Ser", "Kael", "Lum", "Mor", "Ael", "Thar", "Zyr",
    "Oth", "Ven", "Dra", "Nar", "Ash", "Eld", "Gal", "Ith",
    "Xar", "Bal", "Cor", "Fen", "Hel", "Mal", "Nyx", "Pyr",
    "Rha", "Syl", "Tor", "Ura", "Val", "Yth",
];

const GOD_MIDS: &[&str] = &[
    "a", "e", "i", "o", "u", "ae", "ei", "ou", "ra", "la",
    "na", "ri", "li", "en", "an", "ir", "or", "ar", "al", "el",
];

const GOD_SUFFIXES: &[&str] = &[
    "thak", "phel", "thos", "nael", "vrith", "this", "don", "rak",
    "iel", "oth", "nar", "ros", "wyn", "mael", "dris", "gorn",
    "vael", "shar", "keth", "moth", "zar", "rion", "leth", "xis",
    "than", "gael", "phis", "trel", "vorn", "kael",
];

fn generate_god_name(rng: &mut impl Rng) -> String {
    let prefix = GOD_PREFIXES[rng.random_range(0..GOD_PREFIXES.len())];
    let suffix = GOD_SUFFIXES[rng.random_range(0..GOD_SUFFIXES.len())];

    // 40% chance to include a middle syllable for longer names
    if rng.random::<f32>() < 0.4 {
        let mid = GOD_MIDS[rng.random_range(0..GOD_MIDS.len())];
        format!("{prefix}{mid}{suffix}")
    } else {
        format!("{prefix}{suffix}")
    }
}

fn generate_unique_god_name(used: &[String], rng: &mut impl Rng) -> String {
    for _ in 0..100 {
        let name = generate_god_name(rng);
        if !used.iter().any(|u| u == &name) {
            return name;
        }
    }
    generate_god_name(rng) // fallback, extremely unlikely to collide
}

// ---------------------------------------------------------------------------
// Trait Rolling
// ---------------------------------------------------------------------------

/// Roll 2-4 personality traits for a god, using the archetype's weight modifiers
/// and blocklist to prevent thematically broken combinations.
fn roll_god_traits(
    modifiers: &[(CharacterTrait, f32)],
    blocklist: &[CharacterTrait],
    rng: &mut impl Rng,
) -> Vec<CharacterTrait> {
    use CharacterTrait::*;

    // Base weights — same pool as mortal characters, but tuned for divine personalities.
    // Gods are larger-than-life, so extreme traits are more common.
    let mut entries: Vec<(CharacterTrait, f32)> = vec![
        (Warlike, 8.0), (Peaceful, 8.0), (Diplomatic, 5.0), (Ruthless, 8.0),
        (Ambitious, 10.0), (Content, 3.0), (PowerHungry, 8.0), (Humble, 3.0),
        (Loyal, 5.0), (Treacherous, 8.0), (Pragmatic, 5.0), (Fanatical, 10.0),
        (Cunning, 8.0), (Wise, 8.0), (Foolish, 2.0), (Scholarly, 5.0),
        (Honorable, 8.0), (Cruel, 8.0), (Just, 8.0), (Corrupt, 5.0),
        (Charismatic, 5.0), (Brave, 5.0), (Cowardly, 2.0),
        (Paranoid, 5.0), (Reclusive, 5.0), (Devout, 3.0), (Greedy, 5.0),
    ];

    // Remove blocklisted traits entirely
    entries.retain(|(t, _)| !blocklist.contains(t));

    // Apply archetype-specific modifiers
    for &(trait_target, boost_amount) in modifiers {
        for (t, w) in entries.iter_mut() {
            if *t == trait_target {
                *w = (*w + boost_amount).max(0.0);
            }
        }
    }

    let count = rng.random_range(2..=4);
    let table = PopTable::pick_n(entries, count);
    table.roll(rng)
}

// ---------------------------------------------------------------------------
// Emergent Relationship Computation
// ---------------------------------------------------------------------------

/// Trait axes for relationship computation. Traits within the same axis are
/// related and can create affinity or conflict between gods.
fn trait_axis(t: CharacterTrait) -> Option<&'static str> {
    use CharacterTrait::*;
    match t {
        Warlike | Ruthless | Brave => Some("aggression"),
        Peaceful | Diplomatic | Humble => Some("peace"),
        Cruel | Corrupt | Treacherous => Some("darkness"),
        Honorable | Just | Loyal => Some("virtue"),
        Wise | Scholarly | Cunning => Some("intellect"),
        Ambitious | PowerHungry | Greedy => Some("ambition"),
        Paranoid | Reclusive | Cowardly => Some("fear"),
        Fanatical | Devout => Some("zeal"),
        _ => None,
    }
}

/// Axes that oppose each other — gods on opposing axes have tension.
fn axes_oppose(a: &str, b: &str) -> bool {
    matches!(
        (a, b),
        ("aggression", "peace") | ("peace", "aggression")
        | ("darkness", "virtue") | ("virtue", "darkness")
        | ("ambition", "peace") | ("peace", "ambition")
        | ("fear", "aggression") | ("aggression", "fear")
    )
}

fn compute_relationships(
    drawn: &[GodId],
    pool: &GodPool,
    drawn_traits: &HashMap<GodId, Vec<CharacterTrait>>,
) -> Vec<GodRelationship> {
    let mut results = Vec::new();

    for i in 0..drawn.len() {
        for j in (i + 1)..drawn.len() {
            let (id_a, id_b) = (drawn[i].min(drawn[j]), drawn[i].max(drawn[j]));
            let Some(god_a) = pool.get(id_a) else { continue };
            let Some(god_b) = pool.get(id_b) else { continue };

            let mut affinity = 0i32;
            let mut reasons: Vec<&str> = Vec::new();

            let cat_a = god_a.domain.category();
            let cat_b = god_b.domain.category();

            // Same magic category (but not if they have elemental opposition)
            let domains = (god_a.domain, god_b.domain);
            let elemental_clash = matches!(
                domains,
                (MagicSchool::Fire, MagicSchool::Frost) | (MagicSchool::Frost, MagicSchool::Fire)
            );
            if cat_a == cat_b && !elemental_clash {
                affinity += 20;
                reasons.push("shared magical tradition");
            }

            // Opposing categories
            let cats = (cat_a, cat_b);
            let opposing = matches!(
                cats,
                (MagicCategory::Divine, MagicCategory::Death)
                    | (MagicCategory::Death, MagicCategory::Divine)
                    | (MagicCategory::Divine, MagicCategory::Primal)
                    | (MagicCategory::Primal, MagicCategory::Divine)
            );
            if opposing {
                affinity -= 30;
                reasons.push("opposed magical traditions");
            }

            // Elemental opposition (fire vs frost)
            if elemental_clash {
                affinity -= 20;
                reasons.push("elemental opposition");
            }

            // Trait-based interactions (using rolled traits)
            let traits_a = drawn_traits.get(&id_a).map(|v| v.as_slice()).unwrap_or(&[]);
            let traits_b = drawn_traits.get(&id_b).map(|v| v.as_slice()).unwrap_or(&[]);

            // Shared traits create affinity
            let mut shared = 0;
            for ta in traits_a {
                if traits_b.contains(ta) {
                    shared += 1;
                }
            }
            if shared > 0 {
                affinity += shared as i32 * 10;
                reasons.push("shared personality");
            }

            // Same trait axis = some affinity, opposing axes = tension
            let mut axis_affinity = 0i32;
            for ta in traits_a {
                if let Some(axis_a) = trait_axis(*ta) {
                    for tb in traits_b {
                        if let Some(axis_b) = trait_axis(*tb) {
                            if axis_a == axis_b {
                                axis_affinity += 5;
                            } else if axes_oppose(axis_a, axis_b) {
                                axis_affinity -= 8;
                            }
                        }
                    }
                }
            }
            if axis_affinity > 0 {
                affinity += axis_affinity.min(15);
                reasons.push("kindred nature");
            } else if axis_affinity < 0 {
                affinity += axis_affinity.max(-15);
                reasons.push("opposing natures");
            }

            // Forbidden school dynamics
            let a_forbidden = god_a.is_forbidden();
            let b_forbidden = god_b.is_forbidden();
            if a_forbidden && b_forbidden {
                affinity += 15;
                reasons.push("fellow outcasts");
            } else if a_forbidden || b_forbidden {
                affinity -= 20;
                reasons.push("forbidden knowledge");
            }

            let reason = if reasons.is_empty() {
                "distant".to_string()
            } else {
                reasons.join(", ")
            };

            results.push(GodRelationship {
                god_a: id_a,
                god_b: id_b,
                affinity: affinity.clamp(-100, 100),
                reason,
            });
        }
    }

    results
}

// ---------------------------------------------------------------------------
// God Pool Construction
// ---------------------------------------------------------------------------

pub fn build_god_pool() -> GodPool {
    use CharacterTrait::*;

    let mut pool = GodPool::default();

    // 1. God of Fire
    pool.register(GodDef {
        id: 1,
        title: "God of Fire".into(),
        domain: MagicSchool::Fire,
        trait_modifiers: vec![
            (Warlike, 15.0), (Ruthless, 12.0), (Ambitious, 10.0), (Brave, 8.0),
            (Peaceful, -5.0), (Cowardly, -5.0),
        ],
        trait_blocklist: vec![Cowardly], // fire gods are never cowards
        aspect_description: "Destruction, purification, the forge, righteous fury".into(),
        terrain_influence: TerrainInfluence {
            primary_terrain: TerrainType::Stone,
            secondary_terrain: Some(TerrainType::Sand),
            future_terrain: Some("Lava".into()),
            flavor: "Volcanic fields of cooled basalt and ash plains".into(),
        },
        gift_to_mortals: "Forge-craft — the ability to smelt metal and shape weapons".into(),
        spells: vec![
            GodSpellDef {
                name: "Immolate".into(),
                description: "Engulf a target in divine fire, burning them over time".into(),
                damage_type: Some(DamageType::Fire),
                mana_cost: 20, cast_time_ticks: 60, cooldown_ticks: 240, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 14.0,
            },
            GodSpellDef {
                name: "Eruption".into(),
                description: "The ground erupts in a geyser of molten rock".into(),
                damage_type: Some(DamageType::Fire),
                mana_cost: 30, cast_time_ticks: 90, cooldown_ticks: 360, range: 6.0,
                target_type: TargetType::CircleAoE, base_damage: 18.0,
            },
            GodSpellDef {
                name: "Flame Shield".into(),
                description: "Wreath yourself in protective fire".into(),
                damage_type: None,
                mana_cost: 15, cast_time_ticks: 0, cooldown_ticks: 300, range: 0.0,
                target_type: TargetType::SelfOnly, base_damage: 0.0,
            },
            GodSpellDef {
                name: "Pyroclasm".into(),
                description: "Unleash a cone of searing flame".into(),
                damage_type: Some(DamageType::Fire),
                mana_cost: 25, cast_time_ticks: 60, cooldown_ticks: 300, range: 5.0,
                target_type: TargetType::ConeAoE, base_damage: 12.0,
            },
            GodSpellDef {
                name: "Wrath of the Forge".into(),
                description: "Channel the god's rage into a devastating blast".into(),
                damage_type: Some(DamageType::Fire),
                mana_cost: 40, cast_time_ticks: 120, cooldown_ticks: 600, range: 10.0,
                target_type: TargetType::SingleEnemy, base_damage: 30.0,
            },
        ],
        propp_tendencies: vec![ProppFunction::Villainy, ProppFunction::Struggle, ProppFunction::Transformation],
    });

    // 2. Goddess of Frost
    pool.register(GodDef {
        id: 2,
        title: "Goddess of Frost".into(),
        domain: MagicSchool::Frost,
        trait_modifiers: vec![
            (Reclusive, 15.0), (Wise, 10.0), (Peaceful, 8.0), (Paranoid, 8.0),
            (Warlike, -5.0), (Ambitious, -5.0),
        ],
        trait_blocklist: vec![Foolish], // frost preserves wisdom, never foolish
        aspect_description: "Preservation, stillness, mourning, endurance".into(),
        terrain_influence: TerrainInfluence {
            primary_terrain: TerrainType::Snow,
            secondary_terrain: Some(TerrainType::Water),
            future_terrain: Some("Ice".into()),
            flavor: "Frozen wastelands of eternal winter and still lakes".into(),
        },
        gift_to_mortals: "Preservation — enduring harsh climates, storing food, honoring the dead".into(),
        spells: vec![
            GodSpellDef {
                name: "Frozen Lance".into(),
                description: "Hurl a shard of divine ice that slows on impact".into(),
                damage_type: Some(DamageType::Frost),
                mana_cost: 15, cast_time_ticks: 60, cooldown_ticks: 180, range: 10.0,
                target_type: TargetType::SingleEnemy, base_damage: 12.0,
            },
            GodSpellDef {
                name: "Glacial Wave".into(),
                description: "A wave of frost rolls outward in a line".into(),
                damage_type: Some(DamageType::Frost),
                mana_cost: 25, cast_time_ticks: 90, cooldown_ticks: 300, range: 8.0,
                target_type: TargetType::LineAoE, base_damage: 10.0,
            },
            GodSpellDef {
                name: "Permafrost".into(),
                description: "Freeze the ground, slowing all who stand upon it".into(),
                damage_type: Some(DamageType::Frost),
                mana_cost: 20, cast_time_ticks: 0, cooldown_ticks: 360, range: 6.0,
                target_type: TargetType::CircleAoE, base_damage: 5.0,
            },
            GodSpellDef {
                name: "Frozen Tears".into(),
                description: "Weep healing frost that mends wounds and soothes pain".into(),
                damage_type: None,
                mana_cost: 20, cast_time_ticks: 90, cooldown_ticks: 240, range: 8.0,
                target_type: TargetType::SingleAlly, base_damage: 0.0,
            },
            GodSpellDef {
                name: "Crystalline Tomb".into(),
                description: "Encase a foe in ice, freezing them solid".into(),
                damage_type: Some(DamageType::Frost),
                mana_cost: 30, cast_time_ticks: 120, cooldown_ticks: 480, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 8.0,
            },
        ],
        propp_tendencies: vec![ProppFunction::Departure, ProppFunction::Testing, ProppFunction::HelperGift],
    });

    // 3. God of Storm
    pool.register(GodDef {
        id: 3,
        title: "God of Storm".into(),
        domain: MagicSchool::Storm,
        trait_modifiers: vec![
            (Cunning, 12.0), (Ambitious, 10.0), (Scholarly, 8.0), (Treacherous, 8.0),
            (Content, -5.0), (Humble, -5.0),
        ],
        trait_blocklist: vec![Content, Devout], // storms are never content or pious
        aspect_description: "Chaos, experimentation, the thrill of discovery and destruction".into(),
        terrain_influence: TerrainInfluence {
            primary_terrain: TerrainType::Grass,
            secondary_terrain: Some(TerrainType::Stone),
            future_terrain: Some("ScorchedEarth".into()),
            flavor: "Windswept highlands and lightning-scarred ridges".into(),
        },
        gift_to_mortals: "Navigation — reading weather, sailing seas, predicting omens".into(),
        spells: vec![
            GodSpellDef {
                name: "Lightning Bolt".into(),
                description: "Call down a bolt of divine lightning".into(),
                damage_type: Some(DamageType::Storm),
                mana_cost: 15, cast_time_ticks: 0, cooldown_ticks: 180, range: 10.0,
                target_type: TargetType::SingleEnemy, base_damage: 14.0,
            },
            GodSpellDef {
                name: "Tempest".into(),
                description: "Summon a localized storm that shocks all within".into(),
                damage_type: Some(DamageType::Storm),
                mana_cost: 30, cast_time_ticks: 90, cooldown_ticks: 360, range: 7.0,
                target_type: TargetType::CircleAoE, base_damage: 12.0,
            },
            GodSpellDef {
                name: "Chain Spark".into(),
                description: "Lightning arcs between targets".into(),
                damage_type: Some(DamageType::Storm),
                mana_cost: 20, cast_time_ticks: 60, cooldown_ticks: 240, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 18.0,
            },
            GodSpellDef {
                name: "Gale Force".into(),
                description: "A blast of wind hurls enemies backward".into(),
                damage_type: Some(DamageType::Storm),
                mana_cost: 20, cast_time_ticks: 0, cooldown_ticks: 300, range: 4.0,
                target_type: TargetType::ConeAoE, base_damage: 6.0,
            },
            GodSpellDef {
                name: "Eye of the Storm".into(),
                description: "Enter a state of perfect calm amid chaos".into(),
                damage_type: None,
                mana_cost: 15, cast_time_ticks: 0, cooldown_ticks: 480, range: 0.0,
                target_type: TargetType::SelfOnly, base_damage: 0.0,
            },
        ],
        propp_tendencies: vec![ProppFunction::Violation, ProppFunction::Testing, ProppFunction::Transformation],
    });

    // 4. God of Holy Light
    pool.register(GodDef {
        id: 4,
        title: "God of Holy Light".into(),
        domain: MagicSchool::Holy,
        trait_modifiers: vec![
            (Just, 15.0), (Honorable, 12.0), (Fanatical, 10.0), (Brave, 8.0),
            (Treacherous, -5.0), (Corrupt, -5.0), (Cruel, -5.0),
        ],
        trait_blocklist: vec![Treacherous, Corrupt, Cowardly], // holy gods cannot be dishonest or craven
        aspect_description: "Divine justice, the burden of judgment, absolute certainty".into(),
        terrain_influence: TerrainInfluence {
            primary_terrain: TerrainType::Grass,
            secondary_terrain: Some(TerrainType::Stone),
            future_terrain: Some("HallowedGround".into()),
            flavor: "Blessed meadows and white-stone ruins".into(),
        },
        gift_to_mortals: "Law and healing — codes of conduct, organized religion, mending wounds through faith".into(),
        spells: vec![
            GodSpellDef {
                name: "Smite".into(),
                description: "Strike a foe with holy radiance".into(),
                damage_type: Some(DamageType::Holy),
                mana_cost: 20, cast_time_ticks: 0, cooldown_ticks: 240, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 16.0,
            },
            GodSpellDef {
                name: "Divine Ward".into(),
                description: "Shield an ally with holy light, mending their wounds".into(),
                damage_type: None,
                mana_cost: 25, cast_time_ticks: 90, cooldown_ticks: 300, range: 8.0,
                target_type: TargetType::SingleAlly, base_damage: 0.0,
            },
            GodSpellDef {
                name: "Consecrate".into(),
                description: "Sanctify the ground, searing enemies who stand upon it".into(),
                damage_type: Some(DamageType::Holy),
                mana_cost: 30, cast_time_ticks: 90, cooldown_ticks: 360, range: 6.0,
                target_type: TargetType::CircleAoE, base_damage: 10.0,
            },
            GodSpellDef {
                name: "Judgment".into(),
                description: "Pass divine judgment, stunning the unworthy".into(),
                damage_type: Some(DamageType::Holy),
                mana_cost: 35, cast_time_ticks: 120, cooldown_ticks: 480, range: 10.0,
                target_type: TargetType::SingleEnemy, base_damage: 22.0,
            },
            GodSpellDef {
                name: "Radiant Aura".into(),
                description: "Emanate healing light that mends nearby wounds".into(),
                damage_type: None,
                mana_cost: 25, cast_time_ticks: 0, cooldown_ticks: 600, range: 0.0,
                target_type: TargetType::SelfOnly, base_damage: 0.0,
            },
        ],
        propp_tendencies: vec![ProppFunction::Interdiction, ProppFunction::HelperGift, ProppFunction::Struggle],
    });

    // 5. God of Shadow
    pool.register(GodDef {
        id: 5,
        title: "God of Shadow".into(),
        domain: MagicSchool::Shadow,
        trait_modifiers: vec![
            (Cunning, 15.0), (Treacherous, 10.0), (Paranoid, 12.0), (Reclusive, 10.0),
            (Honorable, -5.0), (Brave, -5.0),
        ],
        trait_blocklist: vec![Foolish, Charismatic], // shadow gods are never stupid or openly charming
        aspect_description: "Hidden truths, the fear of the unseen, secrets and deception".into(),
        terrain_influence: TerrainInfluence {
            primary_terrain: TerrainType::Forest,
            secondary_terrain: Some(TerrainType::Swamp),
            future_terrain: Some("Shadowlands".into()),
            flavor: "Shadow-choked groves and lightless bogs".into(),
        },
        gift_to_mortals: "Stealth and writing — hiding, deception, and recording secrets".into(),
        spells: vec![
            GodSpellDef {
                name: "Shadow Bolt".into(),
                description: "Hurl a bolt of condensed darkness".into(),
                damage_type: Some(DamageType::Shadow),
                mana_cost: 15, cast_time_ticks: 60, cooldown_ticks: 180, range: 10.0,
                target_type: TargetType::SingleEnemy, base_damage: 12.0,
            },
            GodSpellDef {
                name: "Veil of Shadows".into(),
                description: "Cloak yourself in shadow, becoming harder to see".into(),
                damage_type: None,
                mana_cost: 15, cast_time_ticks: 0, cooldown_ticks: 360, range: 0.0,
                target_type: TargetType::SelfOnly, base_damage: 0.0,
            },
            GodSpellDef {
                name: "Creeping Dread".into(),
                description: "Fill a target's mind with paralyzing fear".into(),
                damage_type: Some(DamageType::Shadow),
                mana_cost: 20, cast_time_ticks: 90, cooldown_ticks: 300, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 8.0,
            },
            GodSpellDef {
                name: "Umbral Grasp".into(),
                description: "Tendrils of shadow root a target in place".into(),
                damage_type: Some(DamageType::Shadow),
                mana_cost: 20, cast_time_ticks: 0, cooldown_ticks: 300, range: 6.0,
                target_type: TargetType::SingleEnemy, base_damage: 10.0,
            },
            GodSpellDef {
                name: "Eclipse".into(),
                description: "Darkness descends on an area, damaging all within".into(),
                damage_type: Some(DamageType::Shadow),
                mana_cost: 35, cast_time_ticks: 120, cooldown_ticks: 480, range: 7.0,
                target_type: TargetType::CircleAoE, base_damage: 15.0,
            },
        ],
        propp_tendencies: vec![ProppFunction::Departure, ProppFunction::Villainy, ProppFunction::Testing],
    });

    // 6. Goddess of Nature
    pool.register(GodDef {
        id: 6,
        title: "Goddess of Nature".into(),
        domain: MagicSchool::Nature,
        trait_modifiers: vec![
            (Wise, 15.0), (Peaceful, 10.0), (Humble, 10.0), (Devout, 8.0),
            (Greedy, -5.0), (PowerHungry, -5.0), (Cruel, -5.0),
        ],
        trait_blocklist: vec![Greedy, PowerHungry, Foolish], // nature doesn't accumulate, dominate, or stumble blindly
        aspect_description: "Growth, decay, renewal, the long view, patience of ages".into(),
        terrain_influence: TerrainInfluence {
            primary_terrain: TerrainType::Forest,
            secondary_terrain: Some(TerrainType::Swamp),
            future_terrain: Some("DeepWild".into()),
            flavor: "Ancient forests and thick swamps teeming with life".into(),
        },
        gift_to_mortals: "Agriculture — growing food, taming animals, understanding seasons".into(),
        spells: vec![
            GodSpellDef {
                name: "Entangling Roots".into(),
                description: "Roots burst from the earth to hold a target fast".into(),
                damage_type: Some(DamageType::Nature),
                mana_cost: 15, cast_time_ticks: 0, cooldown_ticks: 240, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 8.0,
            },
            GodSpellDef {
                name: "Rejuvenation".into(),
                description: "Infuse an ally with life energy that heals over time".into(),
                damage_type: None,
                mana_cost: 20, cast_time_ticks: 0, cooldown_ticks: 300, range: 8.0,
                target_type: TargetType::SingleAlly, base_damage: 0.0,
            },
            GodSpellDef {
                name: "Thorn Burst".into(),
                description: "Thorns erupt from the ground in an area".into(),
                damage_type: Some(DamageType::Nature),
                mana_cost: 25, cast_time_ticks: 60, cooldown_ticks: 300, range: 6.0,
                target_type: TargetType::CircleAoE, base_damage: 12.0,
            },
            GodSpellDef {
                name: "Barkskin".into(),
                description: "Harden your skin like ancient bark".into(),
                damage_type: None,
                mana_cost: 15, cast_time_ticks: 0, cooldown_ticks: 360, range: 0.0,
                target_type: TargetType::SelfOnly, base_damage: 0.0,
            },
            GodSpellDef {
                name: "Wrath of the Wild".into(),
                description: "Nature itself lashes out in a cone of fury".into(),
                damage_type: Some(DamageType::Nature),
                mana_cost: 30, cast_time_ticks: 90, cooldown_ticks: 420, range: 5.0,
                target_type: TargetType::ConeAoE, base_damage: 14.0,
            },
        ],
        propp_tendencies: vec![ProppFunction::InitialSituation, ProppFunction::HelperGift, ProppFunction::Transformation],
    });

    // 7. God of Death (Necromancy — FORBIDDEN)
    pool.register(GodDef {
        id: 7,
        title: "God of Death".into(),
        domain: MagicSchool::Necromancy,
        trait_modifiers: vec![
            (Cruel, 15.0), (PowerHungry, 12.0), (Greedy, 10.0), (Ruthless, 10.0),
            (Honorable, -5.0), (Humble, -5.0), (Peaceful, -5.0),
        ],
        trait_blocklist: vec![Humble, Devout], // death gods are never humble or pious
        aspect_description: "Consumption, the refusal to accept endings, forbidden arts of undeath".into(),
        terrain_influence: TerrainInfluence {
            primary_terrain: TerrainType::Dirt,
            secondary_terrain: Some(TerrainType::Stone),
            future_terrain: Some("Blight".into()),
            flavor: "Barren lifeless earth and bone-white rock".into(),
        },
        gift_to_mortals: "Knowledge of death — understanding mortality, speaking with the dead, and the forbidden arts of undeath".into(),
        spells: vec![
            GodSpellDef {
                name: "Drain Life".into(),
                description: "Siphon life force from a target, healing yourself".into(),
                damage_type: Some(DamageType::Shadow),
                mana_cost: 20, cast_time_ticks: 60, cooldown_ticks: 240, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 12.0,
            },
            GodSpellDef {
                name: "Bone Spike".into(),
                description: "A spike of bone erupts beneath a target".into(),
                damage_type: Some(DamageType::Piercing),
                mana_cost: 15, cast_time_ticks: 0, cooldown_ticks: 180, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 15.0,
            },
            GodSpellDef {
                name: "Plague Touch".into(),
                description: "Infect a target with a wasting plague".into(),
                damage_type: Some(DamageType::Shadow),
                mana_cost: 20, cast_time_ticks: 0, cooldown_ticks: 300, range: 1.5,
                target_type: TargetType::SingleEnemy, base_damage: 6.0,
            },
            GodSpellDef {
                name: "Death's Embrace".into(),
                description: "Necrotic energy floods an area, weakening the living".into(),
                damage_type: Some(DamageType::Shadow),
                mana_cost: 30, cast_time_ticks: 90, cooldown_ticks: 420, range: 6.0,
                target_type: TargetType::CircleAoE, base_damage: 14.0,
            },
            GodSpellDef {
                name: "Unholy Resurrection".into(),
                description: "Defy death itself through forbidden power".into(),
                damage_type: None,
                mana_cost: 40, cast_time_ticks: 120, cooldown_ticks: 900, range: 0.0,
                target_type: TargetType::SelfOnly, base_damage: 0.0,
            },
        ],
        propp_tendencies: vec![ProppFunction::Villainy, ProppFunction::Departure, ProppFunction::Struggle],
    });

    // 8. God of Arcane Knowledge
    pool.register(GodDef {
        id: 8,
        title: "God of Arcane Knowledge".into(),
        domain: MagicSchool::Arcane,
        trait_modifiers: vec![
            (Scholarly, 15.0), (Wise, 10.0), (Cunning, 10.0), (Ambitious, 8.0),
            (Foolish, -5.0), (Warlike, -5.0),
        ],
        trait_blocklist: vec![Foolish, Warlike], // arcane gods are never stupid or brutish
        aspect_description: "Endless pursuit of understanding, knowledge as an end in itself, the danger of knowing too much".into(),
        terrain_influence: TerrainInfluence {
            primary_terrain: TerrainType::Stone,
            secondary_terrain: Some(TerrainType::Mountain),
            future_terrain: Some("Crystal".into()),
            flavor: "Crystal-veined rock and spires of pure arcane energy".into(),
        },
        gift_to_mortals: "Magic itself — the ability to channel raw arcane energy".into(),
        spells: vec![
            GodSpellDef {
                name: "Arcane Missile".into(),
                description: "A bolt of pure arcane energy that never misses".into(),
                damage_type: Some(DamageType::Arcane),
                mana_cost: 10, cast_time_ticks: 0, cooldown_ticks: 120, range: 12.0,
                target_type: TargetType::SingleEnemy, base_damage: 10.0,
            },
            GodSpellDef {
                name: "Mana Burn".into(),
                description: "Assault a target's magical reserves".into(),
                damage_type: Some(DamageType::Arcane),
                mana_cost: 20, cast_time_ticks: 60, cooldown_ticks: 300, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 12.0,
            },
            GodSpellDef {
                name: "Arcane Blast".into(),
                description: "An explosion of raw magical force".into(),
                damage_type: Some(DamageType::Arcane),
                mana_cost: 30, cast_time_ticks: 90, cooldown_ticks: 360, range: 8.0,
                target_type: TargetType::CircleAoE, base_damage: 16.0,
            },
            GodSpellDef {
                name: "Counterspell".into(),
                description: "Disrupt a target's ability to cast magic".into(),
                damage_type: Some(DamageType::Arcane),
                mana_cost: 15, cast_time_ticks: 0, cooldown_ticks: 360, range: 8.0,
                target_type: TargetType::SingleEnemy, base_damage: 4.0,
            },
            GodSpellDef {
                name: "Infinite Insight".into(),
                description: "Open your mind to arcane understanding".into(),
                damage_type: None,
                mana_cost: 20, cast_time_ticks: 0, cooldown_ticks: 480, range: 0.0,
                target_type: TargetType::SelfOnly, base_damage: 0.0,
            },
        ],
        propp_tendencies: vec![ProppFunction::InitialSituation, ProppFunction::Violation, ProppFunction::Transformation],
    });

    pool
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn pool_has_all_gods() {
        let pool = build_god_pool();
        assert_eq!(pool.gods.len(), 8);
        for id in 1..=8 {
            assert!(pool.get(id).is_some(), "Missing god ID {id}");
        }
    }

    #[test]
    fn all_gods_have_spells() {
        let pool = build_god_pool();
        for god in pool.gods.values() {
            assert!(
                god.spells.len() >= 4 && god.spells.len() <= 6,
                "{} has {} spells, expected 4-6",
                god.title,
                god.spells.len()
            );
        }
    }

    #[test]
    fn all_gods_have_unique_domains() {
        let pool = build_god_pool();
        let mut domains = std::collections::HashSet::new();
        for god in pool.gods.values() {
            assert!(
                domains.insert(god.domain),
                "Duplicate domain {:?} on {}",
                god.domain,
                god.title
            );
        }
    }

    #[test]
    fn all_gods_have_propp_tendencies() {
        let pool = build_god_pool();
        for god in pool.gods.values() {
            assert!(
                !god.propp_tendencies.is_empty(),
                "{} has no Propp tendencies",
                god.title
            );
        }
    }

    #[test]
    fn draw_returns_correct_count() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(6, &mut rng);
        assert_eq!(pantheon.god_ids.len(), 6);
    }

    #[test]
    fn draw_no_duplicates() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(6, &mut rng);
        let mut seen = std::collections::HashSet::new();
        for id in &pantheon.god_ids {
            assert!(seen.insert(id), "Duplicate god ID {id} in draw");
        }
    }

    #[test]
    fn draw_deterministic() {
        let pool = build_god_pool();
        let mut rng_a = rand::rngs::StdRng::seed_from_u64(123);
        let mut rng_b = rand::rngs::StdRng::seed_from_u64(123);
        let a = pool.draw_pantheon(6, &mut rng_a);
        let b = pool.draw_pantheon(6, &mut rng_b);
        assert_eq!(a.god_ids, b.god_ids);
        // Names should also match with same seed
        for id in &a.god_ids {
            assert_eq!(a.drawn_names[id], b.drawn_names[id]);
        }
    }

    #[test]
    fn draw_capped_at_pool_size() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(20, &mut rng);
        assert_eq!(pantheon.god_ids.len(), 8);
    }

    #[test]
    fn relationships_computed_for_drawn() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(6, &mut rng);
        // 6 gods = 6*5/2 = 15 relationships
        assert_eq!(pantheon.relationships.len(), 15);
    }

    #[test]
    fn relationships_symmetric() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(6, &mut rng);
        for rel in &pantheon.relationships {
            assert!(rel.god_a < rel.god_b);
            let fwd = pantheon.relationship(rel.god_a, rel.god_b);
            let rev = pantheon.relationship(rel.god_b, rel.god_a);
            assert_eq!(fwd.map(|r| r.affinity), rev.map(|r| r.affinity));
        }
    }

    #[test]
    fn forbidden_god_has_negative_affinities() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(8, &mut rng);
        let morvrith_rels: Vec<_> = pantheon
            .relationships
            .iter()
            .filter(|r| r.god_a == 7 || r.god_b == 7)
            .collect();
        let negative_count = morvrith_rels.iter().filter(|r| r.affinity < 0).count();
        assert!(
            negative_count >= 4,
            "Death god should have mostly negative affinities, got {negative_count}/{} negative",
            morvrith_rels.len()
        );
    }

    #[test]
    fn available_schools_matches_drawn() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(6, &mut rng);
        let schools = pantheon.available_schools(&pool);
        assert_eq!(schools.len(), 6);
        let unique: std::collections::HashSet<_> = schools.iter().collect();
        assert_eq!(unique.len(), 6);
    }

    #[test]
    fn fire_frost_opposition() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(8, &mut rng);
        let rel = pantheon.relationship(1, 2).expect("should have relationship");
        assert!(
            rel.affinity < 0,
            "Fire-Frost gods should have negative affinity, got {}",
            rel.affinity
        );
    }

    #[test]
    fn drawn_names_unique() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(8, &mut rng);
        let names: Vec<_> = pantheon.drawn_names.values().collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len(), "God names should be unique");
    }

    #[test]
    fn drawn_names_vary_across_seeds() {
        let pool = build_god_pool();
        let mut rng_a = rand::rngs::StdRng::seed_from_u64(1);
        let mut rng_b = rand::rngs::StdRng::seed_from_u64(999);
        let a = pool.draw_pantheon(8, &mut rng_a);
        let b = pool.draw_pantheon(8, &mut rng_b);
        // At least some names should differ (extremely unlikely all 8 match)
        let mut diffs = 0;
        for id in 1..=8u32 {
            if a.drawn_names.get(&id) != b.drawn_names.get(&id) {
                diffs += 1;
            }
        }
        assert!(diffs > 0, "Names should vary across seeds");
    }

    #[test]
    fn drawn_traits_populated() {
        let pool = build_god_pool();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pantheon = pool.draw_pantheon(6, &mut rng);
        for &id in &pantheon.god_ids {
            let traits = pantheon.traits(id);
            assert!(
                traits.len() >= 2 && traits.len() <= 4,
                "God {id} should have 2-4 traits, got {}",
                traits.len()
            );
        }
    }

    #[test]
    fn traits_vary_across_seeds() {
        let pool = build_god_pool();
        let mut seen_variation = false;
        let mut first_traits = None;
        for seed in 0..20 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let pantheon = pool.draw_pantheon(8, &mut rng);
            let god1_traits = pantheon.traits(1).to_vec();
            if let Some(ref first) = first_traits {
                if *first != god1_traits {
                    seen_variation = true;
                    break;
                }
            } else {
                first_traits = Some(god1_traits);
            }
        }
        assert!(seen_variation, "Traits should vary across seeds");
    }
}
