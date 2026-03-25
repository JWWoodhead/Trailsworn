use bevy::prelude::*;

use crate::terrain::TerrainType;

/// Cursor position in all coordinate spaces. Computed once per frame by `update_cursor_position`.
#[derive(Resource, Default)]
pub struct CursorPosition {
    /// Screen-space position (for UI placement like tooltips).
    pub screen: Option<Vec2>,
    /// World-space position.
    pub world: Option<Vec2>,
    /// Tile coordinates (i32 to allow out-of-bounds detection).
    pub tile: Option<(i32, i32)>,
}

pub const DEFAULT_TILE_SIZE: f32 = 64.0;
pub const DEFAULT_MAP_WIDTH: u32 = 250;
pub const DEFAULT_MAP_HEIGHT: u32 = 250;

#[derive(Resource)]
pub struct MapSettings {
    pub tile_size: f32,
    pub width: u32,
    pub height: u32,
}

impl Default for MapSettings {
    fn default() -> Self {
        Self {
            tile_size: DEFAULT_TILE_SIZE,
            width: DEFAULT_MAP_WIDTH,
            height: DEFAULT_MAP_HEIGHT,
        }
    }
}

/// Core tile data stored as struct-of-arrays for cache-friendly access.
/// Each Vec is indexed by `y * width + x`.
#[derive(Resource)]
pub struct TileWorld {
    pub width: u32,
    pub height: u32,
    pub terrain: Vec<TerrainType>,
    pub walk_cost: Vec<f32>,
    pub blocks_los: Vec<bool>,
    pub flammability: Vec<f32>,
}

impl TileWorld {
    /// Create a new TileWorld filled with a single terrain type.
    pub fn filled(width: u32, height: u32, terrain: TerrainType) -> Self {
        let n = (width * height) as usize;
        Self {
            width,
            height,
            terrain: vec![terrain; n],
            walk_cost: vec![terrain.default_walk_cost(); n],
            blocks_los: vec![terrain.default_blocks_los(); n],
            flammability: vec![terrain.default_flammability(); n],
        }
    }

    pub fn idx(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    /// Set a tile's terrain and update all derived properties to defaults.
    pub fn set_terrain(&mut self, x: u32, y: u32, terrain: TerrainType) {
        let i = self.idx(x, y);
        self.terrain[i] = terrain;
        self.walk_cost[i] = terrain.default_walk_cost();
        self.blocks_los[i] = terrain.default_blocks_los();
        self.flammability[i] = terrain.default_flammability();
    }
}

/// Authoritative tile position for simulation. Entities on the map get this.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub x: u32,
    pub y: u32,
}

impl GridPosition {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    /// Convert to world-space pixel coordinates (center of the tile).
    /// Matches bevy_ecs_tilemap's square grid positioning.
    pub fn to_world(self, tile_size: f32) -> Vec2 {
        Vec2::new(
            self.x as f32 * tile_size,
            self.y as f32 * tile_size,
        )
    }
}

/// Render layer z-values for consistent ordering.
pub mod render_layers {
    pub const TERRAIN: f32 = 0.0;
    pub const TERRAIN_OVERLAY: f32 = 0.5;
    pub const TERRAIN_FEATURES: f32 = 1.0;
    pub const FLOOR_ITEMS: f32 = 2.0;
    pub const ENTITIES: f32 = 3.0;
    pub const PROJECTILES: f32 = 4.0;
    pub const UI_OVERLAY: f32 = 5.0;

    /// Compute z for y-sorted depth ordering within a layer.
    /// Entities with lower world_y (bottom of screen) get higher z (rendered in front).
    pub fn y_sorted_z(world_y: f32, map_height_px: f32, base: f32) -> f32 {
        base + (1.0 - (world_y / map_height_px).clamp(0.0, 1.0)) * 0.999
    }
}
