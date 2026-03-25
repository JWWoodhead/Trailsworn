use crate::resources::map::TileWorld;
use crate::terrain::TerrainType;

// Cardinal direction bits (lower 4 bits).
pub const NORTH: u8 = 1;
pub const EAST: u8 = 2;
pub const SOUTH: u8 = 4;
pub const WEST: u8 = 8;

// Corner direction bits (upper 4 bits).
// A corner bit is only valid when NEITHER adjacent cardinal is set.
pub const CORNER_NE: u8 = 16;
pub const CORNER_SE: u8 = 32;
pub const CORNER_SW: u8 = 64;
pub const CORNER_NW: u8 = 128;

/// Number of columns per terrain row in the atlas (47 valid configs + 1 padding).
pub const ATLAS_COLS: u32 = 48;

/// Lookup table mapping 8-bit mask → Wang 47 atlas column index (0-46).
/// Index 0 = no blend (transparent). Indices 1-46 = the 46 non-zero valid configs.
/// Invalid mask combinations (corner set when adjacent cardinal is set) map to 0.
const WANG47_TABLE: [u8; 256] = build_wang47_table();

const fn build_wang47_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut index: u8 = 0;
    let mut mask: u16 = 0;
    while mask < 256 {
        let m = mask as u8;
        let n = m & NORTH != 0;
        let e = m & EAST != 0;
        let s = m & SOUTH != 0;
        let w = m & WEST != 0;
        let ne = m & CORNER_NE != 0;
        let se = m & CORNER_SE != 0;
        let sw = m & CORNER_SW != 0;
        let nw = m & CORNER_NW != 0;

        // Validate: corners only when neither adjacent cardinal is set
        let valid = !(ne && (n || e))
            && !(se && (s || e))
            && !(sw && (s || w))
            && !(nw && (n || w));

        if valid {
            table[mask as usize] = index;
            index += 1;
        }
        mask += 1;
    }
    // index should be 47 at this point
    table
}

/// Returns the number of valid Wang 47 configurations (should be 47).
pub const fn wang47_count() -> u8 {
    let mut count: u8 = 0;
    let mut mask: u16 = 0;
    while mask < 256 {
        let m = mask as u8;
        let n = m & NORTH != 0;
        let e = m & EAST != 0;
        let s = m & SOUTH != 0;
        let w = m & WEST != 0;
        let ne = m & CORNER_NE != 0;
        let se = m & CORNER_SE != 0;
        let sw = m & CORNER_SW != 0;
        let nw = m & CORNER_NW != 0;

        let valid = !(ne && (n || e))
            && !(se && (s || e))
            && !(sw && (s || w))
            && !(nw && (n || w));

        if valid {
            count += 1;
        }
        mask += 1;
    }
    count
}

// Compile-time assertion that we have exactly 47 valid configs.
const _: () = assert!(wang47_count() == 47);

/// Returns the list of all 47 valid mask values (for atlas generation).
pub fn valid_wang47_masks() -> Vec<u8> {
    (0u16..256)
        .filter(|&mask| {
            let m = mask as u8;
            let n = m & NORTH != 0;
            let e = m & EAST != 0;
            let s = m & SOUTH != 0;
            let w = m & WEST != 0;
            let ne = m & CORNER_NE != 0;
            let se = m & CORNER_SE != 0;
            let sw = m & CORNER_SW != 0;
            let nw = m & CORNER_NW != 0;
            !(ne && (n || e))
                && !(se && (s || e))
                && !(sw && (s || w))
                && !(nw && (n || w))
        })
        .map(|m| m as u8)
        .collect()
}

/// Per-tile overlay information for terrain blending.
#[derive(Clone, Copy, Debug)]
pub struct OverlayTile {
    /// Which terrain type is bleeding into this tile.
    pub terrain: TerrainType,
    /// Combined 8-bit mask: cardinal (bits 0-3) + corners (bits 4-7).
    pub mask: u8,
}

impl OverlayTile {
    /// Compute the texture atlas index for this overlay in the transition atlas.
    /// Layout: 48 columns × 6 rows. Row = terrain blend_priority. Column = Wang 47 index.
    pub fn atlas_index(&self) -> u32 {
        let terrain_row = self.terrain.blend_priority() as u32;
        let wang_col = WANG47_TABLE[self.mask as usize] as u32;
        terrain_row * ATLAS_COLS + wang_col
    }
}

/// Computed terrain transition overlay data for the entire map.
/// Recomputed on zone transitions when TileWorld changes.
pub struct TerrainTransitions {
    pub width: u32,
    pub height: u32,
    /// One entry per tile. None = no overlay needed.
    pub overlays: Vec<Option<OverlayTile>>,
}

/// Compute terrain transitions for every tile in the world.
///
/// For each tile, checks all 8 neighbors. Cardinal neighbors (N/E/S/W) produce edge
/// blends. Diagonal neighbors (NE/SE/SW/NW) produce corner blends only when neither
/// adjacent cardinal already covers them.
pub fn compute_transitions(tile_world: &TileWorld) -> TerrainTransitions {
    let w = tile_world.width;
    let h = tile_world.height;
    let n = (w * h) as usize;
    let mut overlays = vec![None; n];

    for y in 0..h {
        for x in 0..w {
            let i = tile_world.idx(x, y);
            let my_priority = tile_world.terrain[i].blend_priority();

            let mut best_priority: u8 = 0;
            let mut best_terrain = tile_world.terrain[i];
            let mut mask: u8 = 0;

            // Check all 8 neighbors
            let neighbors: [(i32, i32, u8); 8] = [
                (0, 1, NORTH),
                (1, 0, EAST),
                (0, -1, SOUTH),
                (-1, 0, WEST),
                (1, 1, CORNER_NE),
                (1, -1, CORNER_SE),
                (-1, -1, CORNER_SW),
                (-1, 1, CORNER_NW),
            ];

            for (dx, dy, dir) in neighbors {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }

                let ni = tile_world.idx(nx as u32, ny as u32);
                let neighbor_priority = tile_world.terrain[ni].blend_priority();

                if neighbor_priority > my_priority && neighbor_priority > best_priority {
                    best_priority = neighbor_priority;
                    best_terrain = tile_world.terrain[ni];
                    mask = dir; // reset mask for new best
                } else if neighbor_priority == best_priority && neighbor_priority > my_priority {
                    mask |= dir; // accumulate mask for same terrain
                }
            }

            if best_priority > my_priority {
                // Strip corner bits that are redundant (adjacent cardinal already set)
                if mask & NORTH != 0 || mask & EAST != 0 {
                    mask &= !CORNER_NE;
                }
                if mask & SOUTH != 0 || mask & EAST != 0 {
                    mask &= !CORNER_SE;
                }
                if mask & SOUTH != 0 || mask & WEST != 0 {
                    mask &= !CORNER_SW;
                }
                if mask & NORTH != 0 || mask & WEST != 0 {
                    mask &= !CORNER_NW;
                }

                overlays[i] = Some(OverlayTile {
                    terrain: best_terrain,
                    mask,
                });
            }
        }
    }

    TerrainTransitions {
        width: w,
        height: h,
        overlays,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wang47_table_has_47_entries() {
        // Compile-time assert already checks this, but verify at runtime too
        let count = (0u16..256)
            .filter(|&m| WANG47_TABLE[m as usize] != 0 || m == 0)
            .count();
        // Index 0 maps to 0 (valid), and 46 non-zero entries also map to unique indices
        assert_eq!(wang47_count(), 47);
        // Verify all 47 indices are assigned
        let max_index = WANG47_TABLE.iter().copied().max().unwrap();
        assert_eq!(max_index, 46);
        let _ = count; // silence unused
    }

    #[test]
    fn no_overlay_for_uniform_terrain() {
        let world = TileWorld::filled(3, 3, TerrainType::Grass);
        let transitions = compute_transitions(&world);
        assert!(transitions.overlays.iter().all(|o| o.is_none()));
    }

    #[test]
    fn grass_next_to_stone_gets_overlay() {
        let mut world = TileWorld::filled(3, 3, TerrainType::Grass);
        world.set_terrain(1, 2, TerrainType::Stone);
        let transitions = compute_transitions(&world);

        let overlay = transitions.overlays[world.idx(1, 1)].unwrap();
        assert_eq!(overlay.terrain, TerrainType::Stone);
        assert_eq!(overlay.mask & NORTH, NORTH);
    }

    #[test]
    fn highest_priority_neighbor_wins() {
        let mut world = TileWorld::filled(3, 3, TerrainType::Grass);
        world.set_terrain(2, 1, TerrainType::Stone);
        world.set_terrain(0, 1, TerrainType::Mountain);
        let transitions = compute_transitions(&world);

        let overlay = transitions.overlays[world.idx(1, 1)].unwrap();
        assert_eq!(overlay.terrain, TerrainType::Mountain);
        assert_eq!(overlay.mask & WEST, WEST);
        assert_eq!(overlay.mask & EAST, 0);
    }

    #[test]
    fn same_terrain_accumulates_mask() {
        let mut world = TileWorld::filled(3, 3, TerrainType::Grass);
        world.set_terrain(1, 2, TerrainType::Stone);
        world.set_terrain(2, 1, TerrainType::Stone);
        let transitions = compute_transitions(&world);

        let overlay = transitions.overlays[world.idx(1, 1)].unwrap();
        assert_eq!(overlay.terrain, TerrainType::Stone);
        assert_eq!(overlay.mask & (NORTH | EAST), NORTH | EAST);
    }

    #[test]
    fn higher_priority_tile_gets_no_overlay_from_lower() {
        let mut world = TileWorld::filled(3, 3, TerrainType::Stone);
        world.set_terrain(1, 2, TerrainType::Grass);
        let transitions = compute_transitions(&world);
        assert!(transitions.overlays[world.idx(1, 1)].is_none());
    }

    #[test]
    fn diagonal_only_produces_corner_overlay() {
        let mut world = TileWorld::filled(3, 3, TerrainType::Grass);
        // Stone only at NE corner (2, 2)
        world.set_terrain(2, 2, TerrainType::Stone);
        let transitions = compute_transitions(&world);

        let overlay = transitions.overlays[world.idx(1, 1)].unwrap();
        assert_eq!(overlay.terrain, TerrainType::Stone);
        // Should have NE corner bit, no cardinal bits
        assert_eq!(overlay.mask & 0x0F, 0); // no cardinal
        assert_ne!(overlay.mask & CORNER_NE, 0); // NE corner set
    }

    #[test]
    fn corner_suppressed_when_adjacent_cardinal_set() {
        let mut world = TileWorld::filled(3, 3, TerrainType::Grass);
        // Stone to north (1, 2) AND northeast (2, 2)
        world.set_terrain(1, 2, TerrainType::Stone);
        world.set_terrain(2, 2, TerrainType::Stone);
        let transitions = compute_transitions(&world);

        let overlay = transitions.overlays[world.idx(1, 1)].unwrap();
        assert_eq!(overlay.terrain, TerrainType::Stone);
        assert_ne!(overlay.mask & NORTH, 0);      // north cardinal set
        assert_eq!(overlay.mask & CORNER_NE, 0);   // NE corner suppressed (N is set)
    }

    #[test]
    fn edge_tiles_handle_bounds() {
        let mut world = TileWorld::filled(3, 3, TerrainType::Grass);
        world.set_terrain(0, 0, TerrainType::Stone);
        let transitions = compute_transitions(&world);
        assert!(transitions.overlays[world.idx(0, 0)].is_none());

        let overlay = transitions.overlays[world.idx(1, 0)].unwrap();
        assert_eq!(overlay.terrain, TerrainType::Stone);
        assert_ne!(overlay.mask & WEST, 0);
    }

    #[test]
    fn atlas_index_uses_wang47_lookup() {
        // Mask with just NORTH cardinal = should map to a valid wang47 index
        let overlay = OverlayTile {
            terrain: TerrainType::Water,
            mask: NORTH,
        };
        let idx = overlay.atlas_index();
        let water_row = TerrainType::Water.blend_priority() as u32;
        assert_eq!(idx, water_row * ATLAS_COLS + WANG47_TABLE[NORTH as usize] as u32);
        assert!(idx > 0); // not transparent
    }
}
