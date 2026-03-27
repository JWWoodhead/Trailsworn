use crate::worldgen::gods::GodId;

/// A divine-era historic event.
#[derive(Clone, Debug)]
pub struct DivineEvent {
    pub year: i32,
    pub kind: DivineEventKind,
    pub description: String,
    pub participants: Vec<GodId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DivineEventKind {
    // Territory
    TerritoryClaimed,
    TerritoryContested,
    TerrainShaped,

    // Mortal interaction
    GiftBestowed,
    TempleEstablished,
    ChampionChosen,
    RaceCreated,

    // Conflict
    DivineWarDeclared,
    DivineWarEnded,
    GodVanquished,
    DomainAbsorbed,

    // Creation
    ArtifactForged,
    SacredSiteCreated,
    CursedSiteCreated,

    // Cooperation
    PactFormed,
    PactBroken,
    JointCreation,

    // Narrative
    NarrativeAdvanced,
    Prophesy,
}
