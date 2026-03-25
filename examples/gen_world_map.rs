//! Generates `assets/debug_world_map.png` — a color-coded visualization of the
//! procedurally generated world map showing biomes, rivers, settlements, and regions.
//!
//! Run: `cargo run --example gen_world_map`
//! Run with custom seed: `cargo run --example gen_world_map -- 12345`
//! Run with custom size: `cargo run --example gen_world_map -- 42 128`

use image::{Rgba, RgbaImage};
use trailsworn::worldgen::world_map::generate_world_map;
use trailsworn::worldgen::zone::ZoneType;

const DEFAULT_SIZE: u32 = 256;
const DEFAULT_SEED: u64 = 42;
const PIXEL_SCALE: u32 = 4; // each cell = 4x4 pixels for visibility

fn biome_color(zone_type: ZoneType) -> [u8; 4] {
    match zone_type {
        ZoneType::Ocean => [30, 60, 120, 255],
        ZoneType::Grassland => [80, 140, 50, 255],
        ZoneType::Forest => [30, 80, 25, 255],
        ZoneType::Mountain => [130, 125, 115, 255],
        ZoneType::Desert => [210, 190, 120, 255],
        ZoneType::Tundra => [200, 210, 220, 255],
        ZoneType::Swamp => [55, 75, 45, 255],
        ZoneType::Coast => [170, 180, 130, 255],
        ZoneType::Settlement => [220, 180, 50, 255],
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_SEED);
    let size = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_SIZE);

    println!("Generating {size}x{size} world map with seed {seed}...");
    let map = generate_world_map(size, size, seed);

    // Collect stats
    let mut biome_counts = std::collections::HashMap::new();
    let mut river_count = 0u32;
    let mut region_ids = std::collections::HashSet::new();
    for cell in &map.cells {
        *biome_counts.entry(cell.zone_type).or_insert(0u32) += 1;
        if cell.river {
            river_count += 1;
        }
        if let Some(rid) = cell.region_id {
            region_ids.insert(rid);
        }
    }

    println!("\nBiome distribution:");
    let mut sorted_biomes: Vec<_> = biome_counts.iter().collect();
    sorted_biomes.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    let total = map.cells.len() as f32;
    for (biome, count) in &sorted_biomes {
        let pct = **count as f32 / total * 100.0;
        println!("  {:?}: {} ({:.1}%)", biome, count, pct);
    }
    println!("\nRivers: {river_count} cells");
    println!("Regions: {}", region_ids.len());
    println!("Spawn: ({}, {})", map.spawn_pos.x, map.spawn_pos.y);

    // Render image
    let img_w = size * PIXEL_SCALE;
    let img_h = size * PIXEL_SCALE;
    let mut img = RgbaImage::new(img_w, img_h);

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) as usize;
            let cell = &map.cells[idx];

            let mut color = biome_color(cell.zone_type);

            // River overlay: blue tint
            if cell.river && cell.zone_type != ZoneType::Ocean {
                color = [40, 90, 170, 255];
            }

            // Settlement: bright gold dot
            if cell.zone_type == ZoneType::Settlement {
                color = [220, 180, 50, 255];
            }

            let pixel = Rgba(color);
            // Image y=0 is top, world y=0 is bottom, so flip
            let img_y = (size - 1 - y) * PIXEL_SCALE;
            let img_x = x * PIXEL_SCALE;
            for py in 0..PIXEL_SCALE {
                for px in 0..PIXEL_SCALE {
                    img.put_pixel(img_x + px, img_y + py, pixel);
                }
            }
        }
    }

    // Draw spawn marker (red cross)
    let sx = map.spawn_pos.x as u32 * PIXEL_SCALE + PIXEL_SCALE / 2;
    let sy = (size - 1 - map.spawn_pos.y as u32) * PIXEL_SCALE + PIXEL_SCALE / 2;
    let red = Rgba([255, 50, 50, 255]);
    for d in 0..8 {
        if sx + d < img_w { img.put_pixel(sx + d, sy, red); }
        if sx >= d { img.put_pixel(sx - d, sy, red); }
        if sy + d < img_h { img.put_pixel(sx, sy + d, red); }
        if sy >= d { img.put_pixel(sx, sy - d, red); }
    }

    let output_path = "assets/debug_world_map.png";
    img.save(output_path).expect("Failed to save world map image");
    println!("\nSaved to {output_path} ({img_w}x{img_h})");

    // Also render elevation, moisture, temperature as separate greyscale images
    render_layer(&map.cells, size, |c| c.elevation, "assets/debug_elevation.png");
    render_layer(&map.cells, size, |c| c.moisture, "assets/debug_moisture.png");
    render_layer(&map.cells, size, |c| c.temperature, "assets/debug_temperature.png");
    println!("Saved elevation/moisture/temperature maps");
}

fn render_layer(
    cells: &[trailsworn::worldgen::world_map::WorldCell],
    size: u32,
    f: impl Fn(&trailsworn::worldgen::world_map::WorldCell) -> f32,
    path: &str,
) {
    let img_size = size * PIXEL_SCALE;
    let mut img = RgbaImage::new(img_size, img_size);

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) as usize;
            let v = (f(&cells[idx]).clamp(0.0, 1.0) * 255.0) as u8;
            let pixel = Rgba([v, v, v, 255]);
            let img_y = (size - 1 - y) * PIXEL_SCALE;
            let img_x = x * PIXEL_SCALE;
            for py in 0..PIXEL_SCALE {
                for px in 0..PIXEL_SCALE {
                    img.put_pixel(img_x + px, img_y + py, pixel);
                }
            }
        }
    }

    img.save(path).expect("Failed to save debug image");
}
