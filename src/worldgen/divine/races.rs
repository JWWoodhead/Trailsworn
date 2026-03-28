use rand::{Rng, RngExt};

use crate::resources::magic::MagicSchool;
use crate::terrain::TerrainType;
use super::gods::{GodDef, GodId};
use crate::worldgen::history::characters::CharacterTrait;
use crate::worldgen::names::Race;
use crate::worldgen::world_map::WorldPos;

/// A race created by a god during the divine era.
#[derive(Clone, Debug)]
pub struct CreatedRace {
    pub id: u32,
    pub name: String,
    pub creator_god: GodId,
    pub created_year: i32,
    /// Which core race this is modeled on (for lifespan/name generation).
    pub base_race: Race,
    pub description: String,
    /// Trait modifiers applied to characters of this race.
    pub trait_modifiers: Vec<(CharacterTrait, f32)>,
    /// Terrain this race prefers.
    pub preferred_terrain: TerrainType,
    /// Magic school this race has affinity for.
    pub magic_affinity: MagicSchool,
    /// Lifespan multiplier (1.0 = same as base race).
    pub lifespan_modifier: f32,
    /// Home region on the world map.
    pub home_region: Vec<WorldPos>,
}

/// Which base race a god archetype creates.
fn base_race_for_domain(domain: MagicSchool) -> Race {
    match domain {
        MagicSchool::Fire => Race::Dwarf,
        MagicSchool::Frost => Race::Elf,
        MagicSchool::Storm => Race::Human,
        MagicSchool::Holy => Race::Human,
        MagicSchool::Shadow => Race::Goblin,
        MagicSchool::Nature => Race::Elf,
        MagicSchool::Necromancy => Race::Orc,
        MagicSchool::Arcane => Race::Elf,
        _ => Race::Human,
    }
}

fn lifespan_modifier_for_domain(domain: MagicSchool) -> f32 {
    match domain {
        MagicSchool::Necromancy => 1.5,
        MagicSchool::Nature => 1.3,
        MagicSchool::Fire => 0.8,
        _ => 1.0,
    }
}

fn descriptors_for_domain(domain: MagicSchool) -> &'static [&'static str] {
    match domain {
        MagicSchool::Fire => &["Forgeborn", "Ashen", "Emberkin"],
        MagicSchool::Frost => &["Frostwalkers", "Rimborn", "Glacials"],
        MagicSchool::Storm => &["Stormtouched", "Thunderborn", "Galecallers"],
        MagicSchool::Holy => &["Lightsworn", "Blessed", "Radiant"],
        MagicSchool::Shadow => &["Shadowkin", "Duskborn", "Veilwalkers"],
        MagicSchool::Nature => &["Wildkin", "Rootborn", "Thornblood"],
        MagicSchool::Necromancy => &["Deathbound", "Hollowed", "Graveborn"],
        MagicSchool::Arcane => &["Spellwoven", "Aetherborn", "Crystalkin"],
        _ => &["Touched", "Marked", "Chosen"],
    }
}

fn description_for_domain(domain: MagicSchool) -> &'static str {
    match domain {
        MagicSchool::Fire => "Stout and heat-resistant, shaped in the forge-god's image.",
        MagicSchool::Frost => "Pale and enduring, born of eternal winter.",
        MagicSchool::Storm => "Restless and keen-sighted, attuned to wind and lightning.",
        MagicSchool::Holy => "Radiant and long-lived, bearing a fraction of divine light.",
        MagicSchool::Shadow => "Slight and nocturnal, most comfortable in darkness.",
        MagicSchool::Nature => "Wild and instinctive, deeply connected to growing things.",
        MagicSchool::Necromancy => "Hardy and unsettling, touched by the knowledge of death.",
        MagicSchool::Arcane => "Slender and cerebral, with an innate sense for magic.",
        _ => "A people shaped by forces beyond mortal understanding.",
    }
}

/// Generate a created race from a god archetype template.
pub fn race_template(
    id: u32,
    god: &GodDef,
    god_name: &str,
    year: i32,
    core_territory: &[WorldPos],
    rng: &mut impl Rng,
) -> CreatedRace {
    let base_race = base_race_for_domain(god.domain);
    let lifespan_modifier = lifespan_modifier_for_domain(god.domain);

    // Inherit half-strength trait modifiers from the god
    let trait_modifiers: Vec<(CharacterTrait, f32)> = god
        .trait_modifiers
        .iter()
        .map(|(t, w)| (*t, w * 0.5))
        .collect();

    let descriptors = descriptors_for_domain(god.domain);
    let descriptor = descriptors[rng.random_range(0..descriptors.len())];
    let name = format!("{}'s {}", god_name, descriptor);

    let description = description_for_domain(god.domain).to_string();

    CreatedRace {
        id,
        name,
        creator_god: god.id,
        created_year: year,
        base_race,
        description,
        trait_modifiers,
        preferred_terrain: god.terrain_influence.primary_terrain,
        magic_affinity: god.domain,
        lifespan_modifier,
        home_region: core_territory.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn base_race_coverage() {
        // Every domain maps to a base race
        for domain in [
            MagicSchool::Fire, MagicSchool::Frost, MagicSchool::Storm,
            MagicSchool::Holy, MagicSchool::Shadow, MagicSchool::Nature,
            MagicSchool::Necromancy, MagicSchool::Arcane,
        ] {
            let _race = base_race_for_domain(domain);
        }
    }

    #[test]
    fn descriptor_coverage() {
        for domain in [
            MagicSchool::Fire, MagicSchool::Frost, MagicSchool::Storm,
            MagicSchool::Holy, MagicSchool::Shadow, MagicSchool::Nature,
            MagicSchool::Necromancy, MagicSchool::Arcane,
        ] {
            let descs = descriptors_for_domain(domain);
            assert!(!descs.is_empty());
        }
    }

    #[test]
    fn race_template_name_includes_god() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        // Minimal GodDef for testing
        let god = GodDef {
            id: 1,
            title: "God of Fire".into(),
            domain: MagicSchool::Fire,
            trait_modifiers: vec![(CharacterTrait::Warlike, 15.0)],
            trait_blocklist: vec![],
            aspect_description: String::new(),
            terrain_influence: crate::worldgen::divine::gods::TerrainInfluence {
                primary_terrain: TerrainType::Stone,
                secondary_terrain: Some(TerrainType::Sand),
                future_terrain: Some("Lava".into()),
                flavor: String::new(),
            },
            gift_to_mortals: String::new(),
            spells: vec![],
            propp_tendencies: vec![],
        };
        let race = race_template(100, &god, "Vorthak", -50, &[], &mut rng);
        assert!(race.name.starts_with("Vorthak's"));
        assert_eq!(race.base_race, Race::Dwarf);
        assert_eq!(race.magic_affinity, MagicSchool::Fire);
        assert!((race.lifespan_modifier - 0.8).abs() < f32::EPSILON);
    }
}
