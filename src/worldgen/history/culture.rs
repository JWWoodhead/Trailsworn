/// Cultural traits that accumulate from a faction's history.
#[derive(Clone, Debug)]
pub struct CulturalProfile {
    pub values: Vec<CulturalValue>,
    pub taboos: Vec<CulturalTaboo>,
    pub memories: Vec<CulturalMemory>,
}

impl Default for CulturalProfile {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            taboos: Vec::new(),
            memories: Vec::new(),
        }
    }
}

impl CulturalProfile {
    pub fn add_value(&mut self, value: CulturalValue) {
        if !self.values.contains(&value) {
            self.values.push(value);
        }
    }

    pub fn add_taboo(&mut self, taboo: CulturalTaboo) {
        if !self.taboos.contains(&taboo) {
            self.taboos.push(taboo);
        }
    }

    pub fn add_memory(&mut self, memory: CulturalMemory) {
        self.memories.push(memory);
    }
}

/// Things a culture prizes — earned through history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CulturalValue {
    MilitaryProwess,
    Scholarship,
    Commerce,
    Piety,
    Independence,
    Unity,
    Expansion,
    Craftsmanship,
    Diplomacy,
    Resilience,
}

/// Things a culture despises — born from trauma.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CulturalTaboo {
    Treachery,
    Magic,
    Outsiders,
    War,
    Religion,
    Tyranny,
    Cowardice,
}

/// A specific historical event that shaped the culture.
#[derive(Clone, Debug)]
pub struct CulturalMemory {
    pub year: i32,
    pub description: String,
    /// Does this grudge/pride never fade?
    pub permanent: bool,
}

/// Analyze a faction's event history and build a cultural profile.
pub fn build_culture(
    faction_id: u32,
    events: &[super::HistoricEvent],
) -> CulturalProfile {
    use super::EventKind;

    let mut profile = CulturalProfile::default();

    let faction_events: Vec<_> = events.iter()
        .filter(|e| e.participants.contains(&faction_id))
        .collect();

    // Count event types
    let mut _wars_declared = 0u32;
    let mut wars_won = 0u32;
    let mut wars_lost = 0u32;
    let mut alliances_formed = 0u32;
    let mut alliances_broken_on_us = 0u32;
    let mut betrayals_suffered = 0u32;
    let mut trade_agreements = 0u32;
    let mut settlements_founded = 0u32;
    let mut plagues = 0u32;
    let mut heroes = 0u32;
    let mut artifacts = 0u32;
    let mut conquests_made = 0u32;
    let mut conquests_suffered = 0u32;

    for event in &faction_events {
        match &event.kind {
            EventKind::WarDeclared => _wars_declared += 1,
            EventKind::WarEnded => {
                // Check if we won or lost — winner is first participant in our description format
                if event.participants.first() == Some(&faction_id) {
                    wars_won += 1;
                } else {
                    wars_lost += 1;
                }
            }
            EventKind::AllianceFormed => alliances_formed += 1,
            EventKind::AllianceBroken => alliances_broken_on_us += 1,
            EventKind::Betrayal => {
                // Second participant is the victim
                if event.participants.get(1) == Some(&faction_id) {
                    betrayals_suffered += 1;
                }
            }
            EventKind::TradeAgreement => trade_agreements += 1,
            EventKind::SettlementFounded => settlements_founded += 1,
            EventKind::PlagueStruck => plagues += 1,
            EventKind::HeroRose => heroes += 1,
            EventKind::ArtifactDiscovered => artifacts += 1,
            EventKind::Conquest => {
                if event.participants.first() == Some(&faction_id) {
                    conquests_made += 1;
                } else {
                    conquests_suffered += 1;
                }
            }
            _ => {}
        }
    }

    // Derive cultural values from event patterns
    if wars_won >= 3 || conquests_made >= 2 {
        profile.add_value(CulturalValue::MilitaryProwess);
        profile.add_memory(CulturalMemory {
            year: 0,
            description: format!("Won {} wars and conquered {} territories", wars_won, conquests_made),
            permanent: false,
        });
    }

    if wars_won >= 2 && conquests_made >= 1 {
        profile.add_value(CulturalValue::Expansion);
    }

    if trade_agreements >= 3 {
        profile.add_value(CulturalValue::Commerce);
    }

    if alliances_formed >= 3 {
        profile.add_value(CulturalValue::Diplomacy);
    }

    if settlements_founded >= 3 {
        profile.add_value(CulturalValue::Craftsmanship);
    }

    if heroes >= 2 || artifacts >= 2 {
        profile.add_value(CulturalValue::Scholarship);
    }

    if plagues >= 2 && wars_lost < 2 {
        profile.add_value(CulturalValue::Resilience);
        profile.add_memory(CulturalMemory {
            year: 0,
            description: "Survived multiple plagues and persevered".into(),
            permanent: false,
        });
    }

    if conquests_suffered >= 1 {
        profile.add_value(CulturalValue::Independence);
        profile.add_memory(CulturalMemory {
            year: 0,
            description: "Lost territory to conquerors, kindling a desire for freedom".into(),
            permanent: true,
        });
    }

    // Derive taboos from trauma
    if betrayals_suffered >= 1 {
        profile.add_taboo(CulturalTaboo::Treachery);
        profile.add_memory(CulturalMemory {
            year: 0,
            description: "Suffered a devastating betrayal that scarred the national psyche".into(),
            permanent: true,
        });
    }

    if alliances_broken_on_us >= 2 {
        profile.add_taboo(CulturalTaboo::Outsiders);
    }

    if wars_lost >= 3 {
        profile.add_taboo(CulturalTaboo::War);
        profile.add_memory(CulturalMemory {
            year: 0,
            description: "Repeated military defeats bred a deep aversion to conflict".into(),
            permanent: false,
        });
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{HistoricEvent, EventKind};

    fn make_event(kind: EventKind, participants: Vec<u32>) -> HistoricEvent {
        HistoricEvent {
            year: 50,
            kind,
            description: "test".into(),
            participants,
        }
    }

    #[test]
    fn military_prowess_from_wars() {
        let events = vec![
            // Faction 1 wins 3 wars (first participant = winner)
            make_event(EventKind::WarEnded, vec![1, 2]),
            make_event(EventKind::WarEnded, vec![1, 3]),
            make_event(EventKind::WarEnded, vec![1, 4]),
        ];
        let culture = build_culture(1, &events);
        assert!(culture.values.contains(&CulturalValue::MilitaryProwess));
    }

    #[test]
    fn treachery_taboo_from_betrayal() {
        let events = vec![
            // Faction 1 was betrayed (second participant = victim)
            make_event(EventKind::Betrayal, vec![2, 1]),
        ];
        let culture = build_culture(1, &events);
        assert!(culture.taboos.contains(&CulturalTaboo::Treachery));
    }

    #[test]
    fn commerce_from_trade() {
        let events = vec![
            make_event(EventKind::TradeAgreement, vec![1, 2]),
            make_event(EventKind::TradeAgreement, vec![1, 3]),
            make_event(EventKind::TradeAgreement, vec![1, 4]),
        ];
        let culture = build_culture(1, &events);
        assert!(culture.values.contains(&CulturalValue::Commerce));
    }

    #[test]
    fn independence_from_conquest() {
        let events = vec![
            // Faction 1 lost territory (second participant = loser)
            make_event(EventKind::Conquest, vec![2, 1]),
        ];
        let culture = build_culture(1, &events);
        assert!(culture.values.contains(&CulturalValue::Independence));
    }

    #[test]
    fn war_taboo_from_defeats() {
        let events = vec![
            make_event(EventKind::WarEnded, vec![2, 1]),
            make_event(EventKind::WarEnded, vec![3, 1]),
            make_event(EventKind::WarEnded, vec![4, 1]),
        ];
        let culture = build_culture(1, &events);
        assert!(culture.taboos.contains(&CulturalTaboo::War));
    }

    #[test]
    fn no_values_with_no_events() {
        let culture = build_culture(1, &[]);
        assert!(culture.values.is_empty());
        assert!(culture.taboos.is_empty());
    }

    #[test]
    fn permanent_memories_for_trauma() {
        let events = vec![
            make_event(EventKind::Betrayal, vec![2, 1]),
        ];
        let culture = build_culture(1, &events);
        let permanent = culture.memories.iter().filter(|m| m.permanent).count();
        assert!(permanent > 0);
    }
}
