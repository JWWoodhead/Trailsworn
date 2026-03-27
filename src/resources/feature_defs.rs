use std::collections::HashMap;

use bevy::prelude::*;

use crate::worldgen::zone::ZoneType;

// ---------------------------------------------------------------------------
// ID type
// ---------------------------------------------------------------------------

pub type FeatureId = u32;

// ---------------------------------------------------------------------------
// Feature IDs — organized by category
// ---------------------------------------------------------------------------

// Universal (1-9)
pub const FEATURE_BOULDER_SMALL: FeatureId = 1;
pub const FEATURE_BOULDER_LARGE: FeatureId = 2;
pub const FEATURE_BUSH: FeatureId = 3;

// Grassland (10-19)
pub const FEATURE_LONE_TREE: FeatureId = 10;
pub const FEATURE_TALL_GRASS: FeatureId = 11;
pub const FEATURE_WILDFLOWERS: FeatureId = 12;

// Forest (20-39)
pub const FEATURE_OAK_TREE: FeatureId = 20;
pub const FEATURE_PINE_TREE: FeatureId = 21;
pub const FEATURE_FOREST_SMALL_TREE_A: FeatureId = 22;
pub const FEATURE_FOREST_SMALL_TREE_B: FeatureId = 23;
pub const FEATURE_FOREST_SMALL_TREE_C: FeatureId = 24;
pub const FEATURE_FOREST_ROCK_A: FeatureId = 25;
pub const FEATURE_FOREST_ROCK_B: FeatureId = 26;
pub const FEATURE_FOREST_ROCK_C: FeatureId = 27;
pub const FEATURE_FOREST_ROCK_D: FeatureId = 28;
pub const FEATURE_FOREST_BUSH_A: FeatureId = 29;
pub const FEATURE_FOREST_BUSH_B: FeatureId = 33;
pub const FEATURE_FOREST_BUSH_C: FeatureId = 34;
pub const FEATURE_FOREST_BUSH_D: FeatureId = 35;

// Mountain (30-39)
pub const FEATURE_ROCK_SPIRE: FeatureId = 30;
pub const FEATURE_RUBBLE_PILE: FeatureId = 31;
pub const FEATURE_DEAD_TREE_ALPINE: FeatureId = 32;

// Desert (40-49)
pub const FEATURE_CACTUS: FeatureId = 40;
pub const FEATURE_DESERT_SCRUB: FeatureId = 41;
pub const FEATURE_BLEACHED_BONES: FeatureId = 42;
pub const FEATURE_SAND_WORN_ROCK: FeatureId = 43;

// Tundra (50-59)
pub const FEATURE_SNOW_PINE: FeatureId = 50;
pub const FEATURE_ICE_CHUNK: FeatureId = 51;
pub const FEATURE_FROZEN_DEAD_TREE: FeatureId = 52;

// Swamp (60-69)
pub const FEATURE_SWAMP_TREE: FeatureId = 60;
pub const FEATURE_REED_CLUSTER: FeatureId = 61;
pub const FEATURE_HANGING_MOSS: FeatureId = 62;

// Coast (70-79)
pub const FEATURE_DRIFTWOOD: FeatureId = 70;
pub const FEATURE_BEACH_GRASS: FeatureId = 71;
pub const FEATURE_TIDAL_ROCK: FeatureId = 72;

// ---------------------------------------------------------------------------
// Definition
// ---------------------------------------------------------------------------

/// Everything about a terrain feature in one place.
pub struct FeatureDef {
    pub id: FeatureId,
    pub name: &'static str,
    pub blocks_movement: bool,
    pub blocks_los: bool,
    pub placeholder_color: [f32; 4],
    /// Asset path for the sprite. None = use placeholder_color square.
    pub sprite: Option<&'static str>,
    /// Display scale relative to tile size (1.0 = one tile wide).
    pub scale: f32,
    /// Which biomes this feature spawns in, with relative weight per biome.
    pub biome_weights: &'static [(ZoneType, u32)],
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Global registry of terrain feature definitions.
#[derive(Resource, Default)]
pub struct FeatureRegistry {
    features: HashMap<FeatureId, FeatureDef>,
    /// Pre-built per-biome lookup: Vec of (FeatureId, weight).
    biome_tables: HashMap<ZoneType, Vec<(FeatureId, u32)>>,
    /// Target feature density per zone type.
    biome_densities: HashMap<ZoneType, u32>,
}

impl FeatureRegistry {
    pub fn register(&mut self, def: FeatureDef) {
        self.features.insert(def.id, def);
    }

    pub fn set_density(&mut self, zone_type: ZoneType, density: u32) {
        self.biome_densities.insert(zone_type, density);
    }

    pub fn get(&self, id: FeatureId) -> Option<&FeatureDef> {
        self.features.get(&id)
    }

    /// Pre-built biome table: (FeatureId, weight) pairs for a zone type.
    pub fn biome_table(&self, zone_type: ZoneType) -> &[(FeatureId, u32)] {
        self.biome_tables
            .get(&zone_type)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Target feature count for a zone type.
    pub fn biome_density(&self, zone_type: ZoneType) -> u32 {
        self.biome_densities.get(&zone_type).copied().unwrap_or(0)
    }

    /// Rebuild the per-biome lookup tables from registered features.
    /// Call once after all registrations are done.
    pub fn rebuild_biome_tables(&mut self) {
        self.biome_tables.clear();
        for def in self.features.values() {
            for &(zone_type, weight) in def.biome_weights {
                self.biome_tables
                    .entry(zone_type)
                    .or_default()
                    .push((def.id, weight));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Populate the feature registry with all terrain features.
pub fn register_features(reg: &mut FeatureRegistry) {
    use ZoneType::*;

    // ---- Grassland ----
    reg.register(FeatureDef {
        id: FEATURE_BOULDER_SMALL, name: "Fieldstone",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.50, 0.50, 0.48, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Grassland, 10)],
    });
    reg.register(FeatureDef {
        id: FEATURE_BOULDER_LARGE, name: "Standing Stone",
        blocks_movement: true, blocks_los: true,
        placeholder_color: [0.40, 0.40, 0.38, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Grassland, 5), (Mountain, 20)],
    });
    reg.register(FeatureDef {
        id: FEATURE_BUSH, name: "Hedge Bush",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.30, 0.50, 0.25, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Grassland, 20)],
    });
    reg.register(FeatureDef {
        id: FEATURE_LONE_TREE, name: "Lone Tree",
        blocks_movement: false, blocks_los: true,
        placeholder_color: [0.30, 0.55, 0.20, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Grassland, 15)],
    });
    reg.register(FeatureDef {
        id: FEATURE_TALL_GRASS, name: "Tall Grass",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.45, 0.60, 0.30, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Grassland, 30)],
    });
    reg.register(FeatureDef {
        id: FEATURE_WILDFLOWERS, name: "Wildflowers",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.60, 0.50, 0.70, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Grassland, 20)],
    });

    // ---- Forest ----
    // Big trees (block LOS)
    reg.register(FeatureDef {
        id: FEATURE_OAK_TREE, name: "Oak Tree",
        blocks_movement: false, blocks_los: true,
        placeholder_color: [0.20, 0.50, 0.15, 1.0],
        sprite: Some("features/scenery1.png"), scale: 1.5,
        biome_weights: &[(Forest, 18)],
    });
    reg.register(FeatureDef {
        id: FEATURE_PINE_TREE, name: "Pine Tree",
        blocks_movement: false, blocks_los: true,
        placeholder_color: [0.15, 0.40, 0.15, 1.0],
        sprite: Some("features/scenery2.png"), scale: 1.5,
        biome_weights: &[(Forest, 14)],
    });
    // Small trees
    reg.register(FeatureDef {
        id: FEATURE_FOREST_SMALL_TREE_A, name: "Small Tree",
        blocks_movement: false, blocks_los: true,
        placeholder_color: [0.25, 0.48, 0.18, 1.0],
        sprite: Some("features/scenery26.png"), scale: 1.0,
        biome_weights: &[(Forest, 12)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FOREST_SMALL_TREE_B, name: "Small Tree",
        blocks_movement: false, blocks_los: true,
        placeholder_color: [0.22, 0.45, 0.16, 1.0],
        sprite: Some("features/scenery27.png"), scale: 1.0,
        biome_weights: &[(Forest, 10)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FOREST_SMALL_TREE_C, name: "Small Tree",
        blocks_movement: false, blocks_los: true,
        placeholder_color: [0.28, 0.50, 0.20, 1.0],
        sprite: Some("features/scenery28.png"), scale: 1.0,
        biome_weights: &[(Forest, 8)],
    });
    // Rocks
    reg.register(FeatureDef {
        id: FEATURE_FOREST_ROCK_A, name: "Forest Rock",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.50, 0.48, 0.45, 1.0],
        sprite: Some("features/scenery44.png"), scale: 0.7,
        biome_weights: &[(Forest, 4)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FOREST_ROCK_B, name: "Rock Pile",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.48, 0.45, 0.42, 1.0],
        sprite: Some("features/scenery45.png"), scale: 0.7,
        biome_weights: &[(Forest, 3)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FOREST_ROCK_C, name: "Boulder",
        blocks_movement: true, blocks_los: true,
        placeholder_color: [0.42, 0.40, 0.38, 1.0],
        sprite: Some("features/scenery46.png"), scale: 0.8,
        biome_weights: &[(Forest, 3)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FOREST_ROCK_D, name: "Flat Rock",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.50, 0.47, 0.44, 1.0],
        sprite: Some("features/scenery47.png"), scale: 0.6,
        biome_weights: &[(Forest, 3)],
    });
    // Bushes
    reg.register(FeatureDef {
        id: FEATURE_FOREST_BUSH_A, name: "Bush",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.30, 0.50, 0.25, 1.0],
        sprite: Some("features/scenery57.png"), scale: 0.8,
        biome_weights: &[(Forest, 8)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FOREST_BUSH_B, name: "Bush Cluster",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.28, 0.48, 0.22, 1.0],
        sprite: Some("features/scenery58.png"), scale: 0.8,
        biome_weights: &[(Forest, 6)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FOREST_BUSH_C, name: "Small Bush",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.32, 0.52, 0.26, 1.0],
        sprite: Some("features/scenery59.png"), scale: 0.7,
        biome_weights: &[(Forest, 6)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FOREST_BUSH_D, name: "Low Hedge",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.26, 0.46, 0.20, 1.0],
        sprite: Some("features/scenery60.png"), scale: 0.7,
        biome_weights: &[(Forest, 5)],
    });

    // ---- Mountain ----
    reg.register(FeatureDef {
        id: FEATURE_ROCK_SPIRE, name: "Rock Spire",
        blocks_movement: true, blocks_los: true,
        placeholder_color: [0.45, 0.43, 0.40, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Mountain, 20)],
    });
    reg.register(FeatureDef {
        id: FEATURE_RUBBLE_PILE, name: "Rubble Pile",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.55, 0.52, 0.50, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Mountain, 35)],
    });
    reg.register(FeatureDef {
        id: FEATURE_DEAD_TREE_ALPINE, name: "Dead Tree (Alpine)",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.35, 0.30, 0.20, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Mountain, 15)],
    });

    // ---- Desert ----
    reg.register(FeatureDef {
        id: FEATURE_CACTUS, name: "Cactus",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.30, 0.50, 0.20, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Desert, 25)],
    });
    reg.register(FeatureDef {
        id: FEATURE_DESERT_SCRUB, name: "Desert Scrub",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.55, 0.50, 0.30, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Desert, 30)],
    });
    reg.register(FeatureDef {
        id: FEATURE_BLEACHED_BONES, name: "Bleached Bones",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.85, 0.82, 0.75, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Desert, 15)],
    });
    reg.register(FeatureDef {
        id: FEATURE_SAND_WORN_ROCK, name: "Sand-Worn Rock",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.60, 0.55, 0.45, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Desert, 20)],
    });

    // ---- Tundra ----
    reg.register(FeatureDef {
        id: FEATURE_SNOW_PINE, name: "Snow Pine",
        blocks_movement: false, blocks_los: true,
        placeholder_color: [0.25, 0.45, 0.30, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Tundra, 25)],
    });
    reg.register(FeatureDef {
        id: FEATURE_ICE_CHUNK, name: "Ice Chunk",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.70, 0.80, 0.90, 0.85],
        sprite: None, scale: 1.0,
        biome_weights: &[(Tundra, 30)],
    });
    reg.register(FeatureDef {
        id: FEATURE_FROZEN_DEAD_TREE, name: "Frozen Dead Tree",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.35, 0.30, 0.20, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Tundra, 20)],
    });

    // ---- Swamp ----
    reg.register(FeatureDef {
        id: FEATURE_SWAMP_TREE, name: "Swamp Tree",
        blocks_movement: false, blocks_los: true,
        placeholder_color: [0.25, 0.35, 0.15, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Swamp, 25)],
    });
    reg.register(FeatureDef {
        id: FEATURE_REED_CLUSTER, name: "Reed Cluster",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.40, 0.50, 0.30, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Swamp, 25)],
    });
    reg.register(FeatureDef {
        id: FEATURE_HANGING_MOSS, name: "Hanging Moss",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.30, 0.45, 0.25, 0.9],
        sprite: None, scale: 1.0,
        biome_weights: &[(Swamp, 15)],
    });

    // ---- Coast ----
    reg.register(FeatureDef {
        id: FEATURE_DRIFTWOOD, name: "Driftwood",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.55, 0.48, 0.38, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Coast, 25)],
    });
    reg.register(FeatureDef {
        id: FEATURE_BEACH_GRASS, name: "Beach Grass",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.50, 0.60, 0.35, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Coast, 25)],
    });
    reg.register(FeatureDef {
        id: FEATURE_TIDAL_ROCK, name: "Tidal Rock",
        blocks_movement: false, blocks_los: false,
        placeholder_color: [0.45, 0.47, 0.50, 1.0],
        sprite: None, scale: 1.0,
        biome_weights: &[(Coast, 20)],
    });

    // ---- Biome densities (features per 250x250 zone) ----
    reg.set_density(Forest, 700);
    reg.set_density(Swamp, 500);
    reg.set_density(Grassland, 400);
    reg.set_density(Mountain, 350);
    reg.set_density(Tundra, 300);
    reg.set_density(Coast, 300);
    reg.set_density(Desert, 200);

    reg.rebuild_biome_tables();
}

/// Create a fully populated feature registry (convenience for tests and startup).
pub fn default_feature_registry() -> FeatureRegistry {
    let mut reg = FeatureRegistry::default();
    register_features(&mut reg);
    reg
}
