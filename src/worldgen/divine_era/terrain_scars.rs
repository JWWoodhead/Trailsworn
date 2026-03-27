use crate::worldgen::gods::GodId;
use crate::worldgen::world_map::WorldPos;

/// Divine terrain overlay type. Not a TerrainType variant — purely worldgen
/// metadata stored on WorldCell. Tracks what kind of divine influence exists
/// so zone generation and future rendering can use it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DivineTerrainType {
    Lava,           // Fire god — volcanic activity
    Ice,            // Frost god — permanent frost
    ScorchedEarth,  // Storm god — lightning-blasted
    HallowedGround, // Holy god — blessed land
    Shadowlands,    // Shadow god — perpetual darkness
    DeepWild,       // Nature god — primordial overgrowth
    Blight,         // Death god — corrupted land
    Crystal,        // Arcane god — crystalline formations
}

impl DivineTerrainType {
    /// Map a future_terrain string from GodDef to a DivineTerrainType.
    pub fn from_future_terrain(s: &str) -> Option<Self> {
        match s {
            "Lava" => Some(Self::Lava),
            "Ice" => Some(Self::Ice),
            "ScorchedEarth" => Some(Self::ScorchedEarth),
            "HallowedGround" => Some(Self::HallowedGround),
            "Shadowlands" => Some(Self::Shadowlands),
            "DeepWild" => Some(Self::DeepWild),
            "Blight" | "Blighted" => Some(Self::Blight),
            "Crystal" => Some(Self::Crystal),
            _ => None,
        }
    }
}

/// How a divine terrain scar was caused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainScarCause {
    /// Normal god activity reshaping their domain.
    TerritoryShaping,
    /// Collateral from divine combat.
    DivineWarBattle,
    /// Massive scar from a god being destroyed.
    GodVanquished,
    /// Scars from the Fall event itself.
    TheFall,
    /// Forging a powerful artifact damaged the land.
    ArtifactCreation,
}

/// A record of a permanent divine terrain modification.
#[derive(Clone, Debug)]
pub struct TerrainScar {
    pub id: u32,
    pub world_pos: WorldPos,
    pub terrain_type: DivineTerrainType,
    pub cause: TerrainScarCause,
    pub caused_year: i32,
    pub caused_by: Vec<GodId>,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_future_terrain_maps_all() {
        assert_eq!(DivineTerrainType::from_future_terrain("Lava"), Some(DivineTerrainType::Lava));
        assert_eq!(DivineTerrainType::from_future_terrain("Ice"), Some(DivineTerrainType::Ice));
        assert_eq!(DivineTerrainType::from_future_terrain("ScorchedEarth"), Some(DivineTerrainType::ScorchedEarth));
        assert_eq!(DivineTerrainType::from_future_terrain("HallowedGround"), Some(DivineTerrainType::HallowedGround));
        assert_eq!(DivineTerrainType::from_future_terrain("Shadowlands"), Some(DivineTerrainType::Shadowlands));
        assert_eq!(DivineTerrainType::from_future_terrain("DeepWild"), Some(DivineTerrainType::DeepWild));
        assert_eq!(DivineTerrainType::from_future_terrain("Blight"), Some(DivineTerrainType::Blight));
        assert_eq!(DivineTerrainType::from_future_terrain("Crystal"), Some(DivineTerrainType::Crystal));
        assert_eq!(DivineTerrainType::from_future_terrain("Nonsense"), None);
    }
}
