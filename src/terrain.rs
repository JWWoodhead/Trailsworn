#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainType {
    Grass,
    Dirt,
    Stone,
    Water,
    Forest,
    Mountain,
}

impl TerrainType {
    /// Default movement cost. 0.0 = impassable.
    pub fn default_walk_cost(self) -> f32 {
        match self {
            Self::Grass => 1.0,
            Self::Dirt => 1.0,
            Self::Stone => 1.0,
            Self::Water => 0.0,
            Self::Forest => 1.5,
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
            _ => 0.0,
        }
    }

    /// Texture atlas index for this terrain type.
    pub fn tile_texture_index(self) -> u32 {
        match self {
            Self::Grass => 0,
            Self::Dirt => 1,
            Self::Stone => 2,
            Self::Water => 3,
            Self::Forest => 4,
            Self::Mountain => 5,
        }
    }
}
