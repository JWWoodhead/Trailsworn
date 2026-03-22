use bevy::prelude::*;

/// Core character attributes. Range 1-20, starting values typically 3-8.
#[derive(Component, Clone, Debug)]
pub struct Attributes {
    /// Melee damage, carry weight.
    pub strength: u32,
    /// Dodge, move speed, attack speed.
    pub agility: u32,
    /// Magic power, spell learning.
    pub intellect: u32,
    /// HP per body part, pain resistance.
    pub toughness: u32,
    /// Resist CC, mental break threshold.
    pub willpower: u32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            strength: 5,
            agility: 5,
            intellect: 5,
            toughness: 5,
            willpower: 5,
        }
    }
}

/// Character progression via experience and leveling.
#[derive(Component, Clone, Debug)]
pub struct CharacterLevel {
    pub level: u32,
    pub current_xp: u32,
    pub xp_to_next: u32,
    /// Unspent attribute points from leveling up.
    pub unspent_points: u32,
}

impl Default for CharacterLevel {
    fn default() -> Self {
        Self {
            level: 1,
            current_xp: 0,
            xp_to_next: 100,
            unspent_points: 0,
        }
    }
}

impl CharacterLevel {
    /// Add XP and handle level-ups. Returns number of levels gained.
    pub fn add_xp(&mut self, xp: u32) -> u32 {
        self.current_xp += xp;
        let mut levels_gained = 0;

        while self.current_xp >= self.xp_to_next {
            self.current_xp -= self.xp_to_next;
            self.level += 1;
            self.unspent_points += 1;
            levels_gained += 1;
            // Each level requires more XP
            self.xp_to_next = xp_for_level(self.level + 1);
        }

        levels_gained
    }
}

/// XP required to reach a given level.
fn xp_for_level(level: u32) -> u32 {
    // Roughly quadratic scaling: 100, 150, 210, 280, 360...
    let base = 100;
    base + (level - 1) * 50 + (level - 1) * (level - 1) * 5
}

/// Which attribute to increase when spending a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeChoice {
    Strength,
    Agility,
    Intellect,
    Toughness,
    Willpower,
}

/// Spend an unspent attribute point on the chosen attribute.
/// Returns true if successful.
pub fn spend_attribute_point(
    level: &mut CharacterLevel,
    attributes: &mut Attributes,
    choice: AttributeChoice,
) -> bool {
    if level.unspent_points == 0 {
        return false;
    }

    let attr = match choice {
        AttributeChoice::Strength => &mut attributes.strength,
        AttributeChoice::Agility => &mut attributes.agility,
        AttributeChoice::Intellect => &mut attributes.intellect,
        AttributeChoice::Toughness => &mut attributes.toughness,
        AttributeChoice::Willpower => &mut attributes.willpower,
    };

    if *attr >= 20 {
        return false;
    }

    *attr += 1;
    level.unspent_points -= 1;
    true
}
