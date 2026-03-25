#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainType {
    Grass,
    Dirt,
    Sand,
    Snow,
    Swamp,
    Stone,
    Forest,
    Water,
    Mountain,
}

impl TerrainType {
    /// Default movement cost. 0.0 = impassable.
    pub fn default_walk_cost(self) -> f32 {
        match self {
            Self::Grass => 1.0,
            Self::Dirt => 1.0,
            Self::Sand => 1.3,
            Self::Snow => 1.4,
            Self::Swamp => 2.0,
            Self::Stone => 1.0,
            Self::Forest => 1.5,
            Self::Water => 0.0,
            Self::Mountain => 0.0,
        }
    }

    /// Whether this terrain blocks line of sight by default.
    pub fn default_blocks_los(self) -> bool {
        match self {
            Self::Mountain => true,
            Self::Forest => true,
            _ => false,
        }
    }

    /// Default flammability. 0.0 = fireproof, 1.0 = highly flammable.
    pub fn default_flammability(self) -> f32 {
        match self {
            Self::Grass => 0.3,
            Self::Forest => 0.8,
            Self::Swamp => 0.1,
            _ => 0.0,
        }
    }

    /// Render priority for terrain blending. Higher priority bleeds onto lower.
    pub fn blend_priority(self) -> u8 {
        match self {
            Self::Grass => 0,
            Self::Dirt => 1,
            Self::Sand => 2,
            Self::Snow => 3,
            Self::Swamp => 4,
            Self::Stone => 5,
            Self::Forest => 6,
            Self::Water => 7,
            Self::Mountain => 8,
        }
    }

    /// Texture atlas index for this terrain type.
    pub fn tile_texture_index(self) -> u32 {
        match self {
            Self::Grass => 0,
            Self::Dirt => 1,
            Self::Sand => 2,
            Self::Snow => 3,
            Self::Swamp => 4,
            Self::Stone => 5,
            Self::Forest => 6,
            Self::Water => 7,
            Self::Mountain => 8,
        }
    }
}
