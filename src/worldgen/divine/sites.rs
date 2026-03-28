use super::gods::GodId;
use crate::worldgen::world_map::WorldPos;

use super::terrain_scars::DivineTerrainType;

/// A divinely-created site that may persist into the mortal era.
#[derive(Clone, Debug)]
pub struct DivineSite {
    pub id: u32,
    pub name: String,
    pub kind: DivineSiteKind,
    pub world_pos: WorldPos,
    pub creator_god: GodId,
    pub created_year: i32,
    /// Whether this site persists into the mortal era.
    pub persists: bool,
    pub description: String,
    /// Terrain modification this site causes at the zone level.
    pub terrain_effect: Option<DivineTerrainType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DivineSiteKind {
    Temple,
    SacredGrove,
    Forge,
    Observatory,
    Necropolis,
    Battleground,
    SealedGate,
    Oracle,
    Wellspring,
}

impl DivineSiteKind {
    /// Map a god's magic school to the most fitting site kind for a sacred site.
    pub fn for_domain(domain: crate::resources::magic::MagicSchool) -> Self {
        use crate::resources::magic::MagicSchool;
        match domain {
            MagicSchool::Fire => Self::Forge,
            MagicSchool::Frost => Self::Wellspring,
            MagicSchool::Storm => Self::Observatory,
            MagicSchool::Holy => Self::Temple,
            MagicSchool::Shadow => Self::SealedGate,
            MagicSchool::Nature => Self::SacredGrove,
            MagicSchool::Necromancy => Self::Necropolis,
            MagicSchool::Arcane => Self::Oracle,
            _ => Self::Temple,
        }
    }
}

/// Generate a name for a divine site.
pub fn divine_site_name(kind: DivineSiteKind, god_name: &str, rng: &mut impl rand::Rng) -> String {
    use rand::RngExt;
    let templates: &[&str] = match kind {
        DivineSiteKind::Temple => &[
            "Temple of {}",
            "Shrine of {}",
            "Cathedral of {}",
            "Sanctum of {}",
        ],
        DivineSiteKind::SacredGrove => &[
            "{}'s Grove",
            "The Verdant Heart of {}",
            "Glade of {}",
            "{}'s Garden",
        ],
        DivineSiteKind::Forge => &[
            "{}'s Forge",
            "The Crucible of {}",
            "Anvil of {}",
            "Furnace of {}",
        ],
        DivineSiteKind::Observatory => &[
            "{}'s Spire",
            "The Watchtower of {}",
            "Observatory of {}",
            "Pinnacle of {}",
        ],
        DivineSiteKind::Necropolis => &[
            "Necropolis of {}",
            "{}'s Crypt",
            "The Charnel Halls of {}",
            "Ossuary of {}",
        ],
        DivineSiteKind::Battleground => &[
            "Fields of {}",
            "{}'s Scar",
            "The Ruin of {}",
            "Desolation of {}",
        ],
        DivineSiteKind::SealedGate => &[
            "{}'s Gate",
            "The Sealed Passage of {}",
            "Portal of {}",
            "Threshold of {}",
        ],
        DivineSiteKind::Oracle => &[
            "Oracle of {}",
            "{}'s Eye",
            "The Seeing Pool of {}",
            "Nexus of {}",
        ],
        DivineSiteKind::Wellspring => &[
            "Wellspring of {}",
            "{}'s Font",
            "The Frozen Spring of {}",
            "Pool of {}",
        ],
    };
    let template = templates[rng.random_range(0..templates.len())];
    template.replace("{}", god_name)
}
