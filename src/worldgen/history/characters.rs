use rand::{Rng, RngExt};

use crate::worldgen::names::{Race, full_name};
use crate::worldgen::population_table::PopTable;

/// A persistent character in the world history.
#[derive(Clone, Debug)]
pub struct Character {
    pub id: u32,
    pub first_name: String,
    pub last_name: String,
    pub race: Race,
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub faction_id: Option<u32>,
    pub role: CharacterRole,
    pub traits: Vec<CharacterTrait>,
    pub ambition: Ambition,
    pub renown: i32,
    pub relationships: Vec<CharacterRelationship>,
    /// Epithet earned through deeds, e.g. "the Betrayer", "Ironhand".
    pub epithet: Option<String>,
}

impl Character {
    pub fn is_alive(&self, year: i32) -> bool {
        self.death_year.is_none() || self.death_year.unwrap() > year
    }

    pub fn age(&self, year: i32) -> i32 {
        year - self.birth_year
    }

    pub fn full_display_name(&self) -> String {
        match &self.epithet {
            Some(ep) => format!("{} {} {}", self.first_name, ep, self.last_name),
            None => format!("{} {}", self.first_name, self.last_name),
        }
    }

    pub fn has_trait(&self, t: CharacterTrait) -> bool {
        self.traits.contains(&t)
    }

    /// Expected lifespan based on race.
    pub fn expected_lifespan(race: Race) -> (i32, i32) {
        match race {
            Race::Human => (60, 80),
            Race::Dwarf => (150, 250),
            Race::Elf => (500, 1000),
            Race::Orc => (40, 60),
            Race::Goblin => (30, 50),
        }
    }

    /// Check if this character should die of old age this year.
    pub fn natural_death_check(&self, year: i32, rng: &mut impl Rng) -> bool {
        let age = self.age(year);
        let (min_life, max_life) = Self::expected_lifespan(self.race);
        if age < min_life {
            return false;
        }
        // Probability increases linearly from min to max lifespan
        let range = (max_life - min_life).max(1) as f32;
        let progress = (age - min_life) as f32 / range;
        let death_prob = (progress * 0.3).clamp(0.0, 0.8);
        rng.random::<f32>() < death_prob
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterRole {
    Leader,
    General,
    Hero,
    Advisor,
    Scholar,
    Villain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterTrait {
    // Aggression
    Warlike,
    Peaceful,
    Diplomatic,
    Ruthless,
    // Ambition
    Ambitious,
    Content,
    PowerHungry,
    Humble,
    // Loyalty
    Loyal,
    Treacherous,
    Pragmatic,
    Fanatical,
    // Intellect
    Cunning,
    Wise,
    Foolish,
    Scholarly,
    // Morality
    Honorable,
    Cruel,
    Just,
    Corrupt,
    // Flavor
    Paranoid,
    Charismatic,
    Reclusive,
    Devout,
    Greedy,
    Brave,
    Cowardly,
    Skeptical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ambition {
    SeizePower,
    ExpandTerritory,
    AccumulateWealth,
    AvengeWrong { target_faction: u32 },
    SeekKnowledge,
    ProtectHomeland,
    UnifyRace,
    DestroyEnemy { target_faction: u32 },
}

#[derive(Clone, Debug)]
pub struct CharacterRelationship {
    pub target_id: u32,
    pub kind: RelationshipKind,
    pub started_year: i32,
    pub sentiment: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipKind {
    Rival,
    Mentor,
    Student,
    Lover,
    Nemesis,
    Friend,
    Betrayer,
    Kin,
}

/// Generate a new character with race-appropriate traits.
pub fn generate_character(
    id: u32,
    race: Race,
    role: CharacterRole,
    faction_id: Option<u32>,
    birth_year: i32,
    rng: &mut impl Rng,
) -> Character {
    let name = full_name(race, rng);
    let parts: Vec<&str> = name.splitn(2, ' ').collect();
    let first_name = parts[0].to_string();
    let last_name = parts.get(1).unwrap_or(&"").to_string();

    let traits = roll_traits(race, role, rng);
    let ambition = pick_ambition(&traits, role, rng);

    Character {
        id,
        first_name,
        last_name,
        race,
        birth_year,
        death_year: None,
        faction_id,
        role,
        traits,
        ambition,
        renown: match role {
            CharacterRole::Leader => 20,
            CharacterRole::General => 10,
            CharacterRole::Hero => 15,
            _ => 5,
        },
        relationships: Vec::new(),
        epithet: None,
    }
}

fn roll_traits(race: Race, role: CharacterRole, rng: &mut impl Rng) -> Vec<CharacterTrait> {
    use CharacterTrait::*;

    // Build a weighted table influenced by race and role
    let mut entries: Vec<(CharacterTrait, f32)> = vec![
        (Warlike, 10.0), (Peaceful, 10.0), (Diplomatic, 10.0), (Ruthless, 5.0),
        (Ambitious, 10.0), (Content, 8.0), (PowerHungry, 5.0), (Humble, 8.0),
        (Loyal, 10.0), (Treacherous, 5.0), (Pragmatic, 10.0), (Fanatical, 3.0),
        (Cunning, 8.0), (Wise, 8.0), (Foolish, 3.0), (Scholarly, 5.0),
        (Honorable, 10.0), (Cruel, 5.0), (Just, 8.0), (Corrupt, 5.0),
        (Charismatic, 5.0), (Brave, 8.0), (Cowardly, 3.0),
        (Paranoid, 3.0), (Reclusive, 3.0), (Devout, 5.0), (Greedy, 5.0),
    ];

    // Race modifiers
    match race {
        Race::Orc => {
            boost(&mut entries, Warlike, 15.0);
            boost(&mut entries, Brave, 10.0);
            boost(&mut entries, Ruthless, 10.0);
            reduce(&mut entries, Scholarly, 3.0);
        }
        Race::Elf => {
            boost(&mut entries, Wise, 15.0);
            boost(&mut entries, Scholarly, 10.0);
            boost(&mut entries, Reclusive, 8.0);
            reduce(&mut entries, Foolish, 1.0);
        }
        Race::Dwarf => {
            boost(&mut entries, Loyal, 12.0);
            boost(&mut entries, Honorable, 10.0);
            boost(&mut entries, Greedy, 8.0);
            boost(&mut entries, Brave, 8.0);
        }
        Race::Goblin => {
            boost(&mut entries, Cunning, 12.0);
            boost(&mut entries, Treacherous, 8.0);
            boost(&mut entries, Greedy, 10.0);
            boost(&mut entries, Cowardly, 8.0);
            reduce(&mut entries, Honorable, 3.0);
        }
        Race::Human => {
            // Humans are balanced, slight boost to ambition
            boost(&mut entries, Ambitious, 5.0);
            boost(&mut entries, Pragmatic, 5.0);
        }
    }

    // Role modifiers
    match role {
        CharacterRole::Leader => {
            boost(&mut entries, Ambitious, 8.0);
            boost(&mut entries, Charismatic, 8.0);
        }
        CharacterRole::General => {
            boost(&mut entries, Warlike, 10.0);
            boost(&mut entries, Brave, 10.0);
        }
        CharacterRole::Hero => {
            boost(&mut entries, Brave, 15.0);
            boost(&mut entries, Honorable, 10.0);
            reduce(&mut entries, Cowardly, 1.0);
        }
        CharacterRole::Scholar => {
            boost(&mut entries, Scholarly, 15.0);
            boost(&mut entries, Wise, 10.0);
        }
        CharacterRole::Advisor => {
            boost(&mut entries, Cunning, 10.0);
            boost(&mut entries, Diplomatic, 8.0);
        }
        CharacterRole::Villain => {
            boost(&mut entries, Cruel, 15.0);
            boost(&mut entries, Treacherous, 10.0);
            boost(&mut entries, PowerHungry, 10.0);
            reduce(&mut entries, Honorable, 2.0);
        }
    }

    let table = PopTable::pick_n(entries, rng.random_range(2..=4));
    table.roll(rng)
}

fn boost(entries: &mut [(CharacterTrait, f32)], target: CharacterTrait, amount: f32) {
    for (t, w) in entries.iter_mut() {
        if *t == target { *w += amount; }
    }
}

fn reduce(entries: &mut [(CharacterTrait, f32)], target: CharacterTrait, new_weight: f32) {
    for (t, w) in entries.iter_mut() {
        if *t == target { *w = new_weight; }
    }
}

fn pick_ambition(traits: &[CharacterTrait], role: CharacterRole, rng: &mut impl Rng) -> Ambition {
    use CharacterTrait::*;

    // Traits strongly influence ambition
    if traits.contains(&PowerHungry) || (traits.contains(&Ambitious) && role != CharacterRole::Hero) {
        return Ambition::SeizePower;
    }
    if traits.contains(&Scholarly) {
        return Ambition::SeekKnowledge;
    }
    if traits.contains(&Greedy) {
        return Ambition::AccumulateWealth;
    }
    if traits.contains(&Warlike) && traits.contains(&Ambitious) {
        return Ambition::ExpandTerritory;
    }

    // Default based on role
    match role {
        CharacterRole::Leader => {
            let options = [Ambition::ExpandTerritory, Ambition::ProtectHomeland, Ambition::AccumulateWealth];
            options[rng.random_range(0..options.len())].clone()
        }
        CharacterRole::General => Ambition::ExpandTerritory,
        CharacterRole::Hero => Ambition::SeekKnowledge,
        CharacterRole::Scholar => Ambition::SeekKnowledge,
        CharacterRole::Advisor => Ambition::ProtectHomeland,
        CharacterRole::Villain => Ambition::SeizePower,
    }
}

/// Generate an epithet based on traits and deeds.
pub fn generate_epithet(character: &Character, rng: &mut impl Rng) -> String {
    use CharacterTrait::*;

    let trait_epithets: &[(&[CharacterTrait], &[&str])] = &[
        (&[Warlike, Brave], &["the Bold", "the Conqueror", "Ironhand", "Warbringer"]),
        (&[Cruel, Ruthless], &["the Cruel", "the Merciless", "Bloodhand", "the Dread"]),
        (&[Wise, Scholarly], &["the Wise", "the Sage", "Lorekeeper", "the Learned"]),
        (&[Treacherous], &["the Betrayer", "Oathbreaker", "the Deceiver", "Two-faced"]),
        (&[Cunning], &["the Fox", "the Schemer", "Silvertongue", "the Subtle"]),
        (&[Honorable, Just], &["the Just", "the Honorable", "Trueblade", "the Fair"]),
        (&[Devout, Fanatical], &["the Devout", "the Zealot", "the Blessed", "the Pious"]),
        (&[Greedy], &["the Gilded", "Goldclaw", "the Acquisitor", "Coinbiter"]),
        (&[Paranoid], &["the Watchful", "the Suspicious", "Shadoweye"]),
        (&[Charismatic], &["the Beloved", "the Radiant", "Brightspeech"]),
        (&[Peaceful, Diplomatic], &["the Peacemaker", "the Mediator", "Calmvoice"]),
    ];

    for (required_traits, options) in trait_epithets {
        if required_traits.iter().any(|t| character.traits.contains(t)) {
            return options[rng.random_range(0..options.len())].to_string();
        }
    }

    // Generic fallback
    let generic = ["the Elder", "the Young", "the Strong", "the Steadfast"];
    generic[rng.random_range(0..generic.len())].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn rng() -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(42)
    }

    #[test]
    fn character_has_traits() {
        let c = generate_character(1, Race::Human, CharacterRole::Leader, Some(1), 0, &mut rng());
        assert!(!c.traits.is_empty());
        assert!(c.traits.len() >= 2 && c.traits.len() <= 4);
    }

    #[test]
    fn character_has_ambition() {
        let c = generate_character(1, Race::Orc, CharacterRole::General, Some(1), 0, &mut rng());
        // Should have some ambition
        let _ = c.ambition; // just verify it exists
    }

    #[test]
    fn orc_traits_lean_warlike() {
        let mut rng = rng();
        let mut warlike_count = 0;
        for _ in 0..50 {
            let c = generate_character(1, Race::Orc, CharacterRole::General, Some(1), 0, &mut rng);
            if c.has_trait(CharacterTrait::Warlike) || c.has_trait(CharacterTrait::Brave) {
                warlike_count += 1;
            }
        }
        // Orcs should frequently get Warlike or Brave
        assert!(warlike_count > 15, "only {} warlike orcs out of 50", warlike_count);
    }

    #[test]
    fn elf_traits_lean_wise() {
        let mut rng = rng();
        let mut wise_count = 0;
        for _ in 0..50 {
            let c = generate_character(1, Race::Elf, CharacterRole::Scholar, Some(1), 0, &mut rng);
            if c.has_trait(CharacterTrait::Wise) || c.has_trait(CharacterTrait::Scholarly) {
                wise_count += 1;
            }
        }
        assert!(wise_count > 15, "only {} wise elves out of 50", wise_count);
    }

    #[test]
    fn natural_death_respects_race_lifespan() {
        let mut rng = rng();
        // Young human should not die
        let young = generate_character(1, Race::Human, CharacterRole::Leader, Some(1), 0, &mut rng);
        let mut deaths = 0;
        for _ in 0..100 {
            if young.natural_death_check(30, &mut rng) { deaths += 1; }
        }
        assert_eq!(deaths, 0);

        // Old orc should sometimes die
        let old_orc = generate_character(2, Race::Orc, CharacterRole::Leader, Some(1), 0, &mut rng);
        let mut deaths = 0;
        for _ in 0..100 {
            if old_orc.natural_death_check(55, &mut rng) { deaths += 1; }
        }
        assert!(deaths > 0, "old orc never died in 100 checks");
    }

    #[test]
    fn epithet_generation() {
        let mut rng = rng();
        let c = generate_character(1, Race::Human, CharacterRole::Hero, Some(1), 0, &mut rng);
        let epithet = generate_epithet(&c, &mut rng);
        assert!(!epithet.is_empty());
    }

    #[test]
    fn display_name_with_epithet() {
        let mut c = generate_character(1, Race::Dwarf, CharacterRole::Leader, Some(1), 0, &mut rng());
        assert!(!c.full_display_name().is_empty());
        c.epithet = Some("the Bold".into());
        assert!(c.full_display_name().contains("the Bold"));
    }

    #[test]
    fn hero_rarely_cowardly() {
        let mut rng = rng();
        let mut coward_count = 0;
        for _ in 0..100 {
            let c = generate_character(1, Race::Human, CharacterRole::Hero, Some(1), 0, &mut rng);
            if c.has_trait(CharacterTrait::Cowardly) { coward_count += 1; }
        }
        assert!(coward_count < 10, "heroes were cowardly {} times", coward_count);
    }
}
