//! Generates `assets/blend_texture.png` — an RGBA image controlling terrain
//! edge blending weights. Tiles across the world map.
//!
//! Channels:
//!   R = horizontal neighbor weight (high near left/right tile edges)
//!   G = vertical neighbor weight (high near top/bottom tile edges)
//!   B = corner neighbor weight (high near tile corners)
//!   A = self weight (high at tile centers)
//!
//! All four channels sum to ~1.0 at every pixel.
//!
//! Run: `cargo run --example gen_blend_texture`

use image::{RgbaImage, Rgba};
use noise::{NoiseFn, Perlin};

const TILES: u32 = 8;          // blend texture covers 8x8 tiles
const PX_PER_TILE: u32 = 64;   // pixels per tile in the blend texture
const SIZE: u32 = TILES * PX_PER_TILE; // 512x512 total

const EDGE_WIDTH: f64 = 0.38;  // how far blend extends from edge (0-0.5)
const NOISE_STRENGTH: f64 = 0.5; // how much noise distorts the blend boundary
const EDGE_PCT: f64 = 0.50;    // max blend % at tile edge (50% self + 50% neighbor)
const CORNER_PCT: f64 = 0.25;  // max blend % at tile corner (25% each of 4 tiles)
const NOISE_FREQ: f64 = 0.04;  // noise frequency in pixels

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn main() {
    let perlin = Perlin::new(42);

    let mut img = RgbaImage::new(SIZE, SIZE);

    for py in 0..SIZE {
        for px in 0..SIZE {
            // Tile-local coordinates (0.0 to 1.0)
            let local_x = (px % PX_PER_TILE) as f64 / PX_PER_TILE as f64;
            let local_y = (py % PX_PER_TILE) as f64 / PX_PER_TILE as f64;

            // Distance from nearest horizontal edge (0 at edge, 0.5 at center)
            let dist_h = local_x.min(1.0 - local_x);
            // Distance from nearest vertical edge
            let dist_v = local_y.min(1.0 - local_y);

            // Noise modulation — use different offsets for each axis for variety
            // Use tileable noise by sampling at world pixel coordinates
            let n_h = perlin.get([px as f64 * NOISE_FREQ, py as f64 * NOISE_FREQ, 0.0]);
            let n_v = perlin.get([px as f64 * NOISE_FREQ, py as f64 * NOISE_FREQ, 100.0]);
            let n_c = perlin.get([px as f64 * NOISE_FREQ, py as f64 * NOISE_FREQ, 200.0]);

            // Effective edge width varies with noise
            let ew_h = (EDGE_WIDTH + NOISE_STRENGTH * n_h * EDGE_WIDTH).clamp(0.05, 0.48);
            let ew_v = (EDGE_WIDTH + NOISE_STRENGTH * n_v * EDGE_WIDTH).clamp(0.05, 0.48);
            let ew_c = (EDGE_WIDTH * 0.8 + NOISE_STRENGTH * n_c * EDGE_WIDTH * 0.6).clamp(0.05, 0.40);

            // Raw gradients: 1.0 at edge, falling to 0.0 at edge_width distance
            let raw_h = 1.0 - smoothstep(0.0, ew_h, dist_h);
            let raw_v = 1.0 - smoothstep(0.0, ew_v, dist_v);
            let dist_corner = (dist_h * dist_h + dist_v * dist_v).sqrt();
            let raw_c = 1.0 - smoothstep(0.0, ew_c, dist_corner);

            // Scale to max percentages (tutorial: edge=50%, corner=25%)
            let w_c = raw_c * CORNER_PCT;
            // Subtract corner from edges to avoid over-blending at corners
            let w_h = (raw_h * EDGE_PCT - w_c).max(0.0);
            let w_v = (raw_v * EDGE_PCT - w_c).max(0.0);

            // Self fills the remainder — always sums to 100%
            let w_self = (1.0 - w_h - w_v - w_c).max(0.0);

            // Scale weights so that (channel * 255 / 100) in the shader produces ~1.0 total.
            // channel_value = weight * (100/255) * 255 = weight * 100
            // So store weight * 100 as u8 (max ~100 per channel, sum ~100).
            img.put_pixel(px, py, Rgba([
                (w_h * 100.0).round().min(255.0) as u8,
                (w_v * 100.0).round().min(255.0) as u8,
                (w_c * 100.0).round().min(255.0) as u8,
                (w_self * 100.0).round().min(255.0) as u8,
            ]));
        }
    }

    img.save("assets/blend_texture.png")
        .expect("Failed to save blend_texture.png");

    println!("Generated assets/blend_texture.png ({SIZE}x{SIZE}, {TILES}x{TILES} tiles at {PX_PER_TILE}px)");
}
