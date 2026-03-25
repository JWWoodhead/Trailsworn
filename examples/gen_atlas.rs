//! Generates `assets/terrain_transitions.png` — the overlay texture atlas
//! for terrain blending using the 47-tile Wang tileset.
//!
//! Run: `cargo run --example gen_atlas`
//!
//! Atlas layout: 48 columns × 6 rows (3072×384 at 64px tiles).
//! - Row 0: transparent (Grass doesn't bleed)
//! - Rows 1-5: Dirt/Stone/Forest/Water/Mountain
//! - Column = Wang 47 index (0 = transparent, 1-46 = valid blend configs)
//!
//! Each tile combines edge gradients (cardinal) and corner gradients (diagonal)
//! into a single alpha-masked terrain texture.

use image::{ImageReader, RgbaImage, Rgba};
use std::f32::consts::PI;

const TILE_SIZE: u32 = 64;
const ATLAS_COLS: u32 = 48;
const ATLAS_ROWS: u32 = 6;
const BLEND_DEPTH: f32 = 0.5; // fraction of tile for edge blending
const CORNER_RADIUS: f32 = 0.5; // fraction of tile for corner blending

// Cardinal mask bits (must match terrain_blend.rs)
const NORTH: u8 = 1;
const EAST: u8 = 2;
const SOUTH: u8 = 4;
const WEST: u8 = 8;
const CORNER_NE: u8 = 16;
const CORNER_SE: u8 = 32;
const CORNER_SW: u8 = 64;
const CORNER_NW: u8 = 128;

// Terrain texture indices in terrain.png (must match TerrainType::tile_texture_index)
// (atlas_row, source_texture_index)
const TERRAIN_ROWS: [(u32, u32); 5] = [
    (1, 1), // Dirt
    (2, 2), // Stone
    (3, 4), // Forest
    (4, 3), // Water
    (5, 5), // Mountain
];

fn main() {
    let source = ImageReader::open("assets/terrain.png")
        .expect("Failed to open assets/terrain.png")
        .decode()
        .expect("Failed to decode terrain.png")
        .to_rgba8();

    let src_h = source.height();
    let valid_masks = compute_valid_masks();
    assert_eq!(valid_masks.len(), 47);

    let atlas_w = ATLAS_COLS * TILE_SIZE;
    let atlas_h = ATLAS_ROWS * TILE_SIZE;
    let mut atlas = RgbaImage::new(atlas_w, atlas_h);

    // Row 0 is all transparent (already zeroed)

    for &(row, tex_idx) in &TERRAIN_ROWS {
        let src_x = tex_idx * TILE_SIZE;
        let src_tile = extract_tile(&source, src_x, 0, src_h);

        for (wang_index, &mask) in valid_masks.iter().enumerate() {
            if mask == 0 {
                // Index 0 = transparent, skip
                continue;
            }

            let col = wang_index as u32;
            let dest_x = col * TILE_SIZE;
            let dest_y = row * TILE_SIZE;

            for py in 0..TILE_SIZE {
                for px in 0..TILE_SIZE {
                    let alpha = compute_mask_alpha(px, py, mask);
                    let src_pixel = src_tile.get_pixel(px, py);
                    let out = Rgba([src_pixel[0], src_pixel[1], src_pixel[2], (alpha * 255.0) as u8]);
                    atlas.put_pixel(dest_x + px, dest_y + py, out);
                }
            }
        }
    }

    atlas.save("assets/terrain_transitions.png")
        .expect("Failed to save terrain_transitions.png");

    println!(
        "Generated assets/terrain_transitions.png ({atlas_w}x{atlas_h}, {} valid configs, {} terrain types)",
        valid_masks.len(),
        TERRAIN_ROWS.len(),
    );
}

/// Compute all 47 valid Wang mask values (same logic as terrain_blend.rs).
fn compute_valid_masks() -> Vec<u8> {
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

/// Extract a tile from the source image.
fn extract_tile(source: &RgbaImage, src_x: u32, src_y: u32, src_h: u32) -> RgbaImage {
    let mut tile = RgbaImage::new(TILE_SIZE, TILE_SIZE);
    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let sy = src_y + (py % src_h);
            let sx = src_x + px;
            if sx < source.width() && sy < source.height() {
                tile.put_pixel(px, py, *source.get_pixel(sx, sy));
            }
        }
    }
    tile
}

/// Compute alpha at pixel (px, py) for the combined cardinal + corner mask.
/// Note: fy is flipped because Bevy/GPU textures render PNG top as tile bottom.
fn compute_mask_alpha(px: u32, py: u32, mask: u8) -> f32 {
    let fx = px as f32 / TILE_SIZE as f32; // 0.0 = left, 1.0 = right
    let fy = py as f32 / TILE_SIZE as f32; // 0.0 = top/north, 1.0 = bottom/south (no flip needed)

    let mut alpha: f32 = 0.0;

    // Cardinal edge gradients
    if mask & NORTH != 0 {
        alpha = alpha.max(edge_gradient(fy));
    }
    if mask & SOUTH != 0 {
        alpha = alpha.max(edge_gradient(1.0 - fy));
    }
    if mask & WEST != 0 {
        alpha = alpha.max(edge_gradient(fx));
    }
    if mask & EAST != 0 {
        alpha = alpha.max(edge_gradient(1.0 - fx));
    }

    // Corner gradients (radial from corner)
    if mask & CORNER_NE != 0 {
        alpha = alpha.max(corner_gradient(1.0 - fx, fy)); // distance from top-right
    }
    if mask & CORNER_SE != 0 {
        alpha = alpha.max(corner_gradient(1.0 - fx, 1.0 - fy)); // distance from bottom-right
    }
    if mask & CORNER_SW != 0 {
        alpha = alpha.max(corner_gradient(fx, 1.0 - fy)); // distance from bottom-left
    }
    if mask & CORNER_NW != 0 {
        alpha = alpha.max(corner_gradient(fx, fy)); // distance from top-left
    }

    alpha.clamp(0.0, 1.0)
}

/// Smooth edge gradient: 1.0 at edge (t=0), fading to 0.0 at BLEND_DEPTH.
/// First BORDER pixels are forced to 1.0 to eliminate seams at tile boundaries.
fn edge_gradient(t: f32) -> f32 {
    let border = 2.0 / TILE_SIZE as f32;
    if t <= border {
        return 1.0;
    }
    let adjusted = (t - border) / (BLEND_DEPTH - border);
    if adjusted >= 1.0 {
        0.0
    } else {
        0.5 * (1.0 + (adjusted * PI).cos())
    }
}

/// Smooth corner gradient: radial from corner (dx=0, dy=0 is the corner).
/// dx and dy are distances from the corner in 0.0-1.0 tile space.
/// First BORDER pixels forced to 1.0 to eliminate seams.
fn corner_gradient(dx: f32, dy: f32) -> f32 {
    let border = 2.0 / TILE_SIZE as f32;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= border {
        return 1.0;
    }
    let adjusted = (dist - border) / (CORNER_RADIUS - border);
    if adjusted >= 1.0 {
        0.0
    } else {
        0.5 * (1.0 + (adjusted * PI).cos())
    }
}
