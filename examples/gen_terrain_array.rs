//! Generates `assets/terrain_array.png` — a vertically stacked texture array
//! with one 512×512 layer per terrain type. Loaded at runtime with
//! `reinterpret_stacked_2d_as_array` to create a texture_2d_array.
//!
//! Run: `cargo run --example gen_terrain_array`
//!
//! Layer order matches TerrainType::tile_texture_index():
//!   0: Grass, 1: Dirt, 2: Stone, 3: Water, 4: Forest, 5: Mountain

use image::{ImageReader, RgbaImage, Rgba};

const LAYER_SIZE: u32 = 512;
const NUM_LAYERS: u32 = 9;

struct TerrainSource {
    name: &'static str,
    path: Option<&'static str>,
    fallback_color: [u8; 4],
}

// Layer order must match TerrainType::tile_texture_index() in src/terrain.rs
const SOURCES: [TerrainSource; 9] = [
    TerrainSource { name: "Grass",    path: None, fallback_color: [80, 130, 50, 255] },
    TerrainSource { name: "Dirt",     path: None, fallback_color: [120, 85, 55, 255] },
    TerrainSource { name: "Sand",     path: None, fallback_color: [210, 190, 130, 255] },
    TerrainSource { name: "Snow",     path: None, fallback_color: [230, 235, 240, 255] },
    TerrainSource { name: "Swamp",    path: None, fallback_color: [60, 80, 50, 255] },
    TerrainSource { name: "Stone",    path: None, fallback_color: [140, 140, 135, 255] },
    TerrainSource { name: "Forest",   path: None, fallback_color: [40, 85, 30, 255] },
    TerrainSource { name: "Water",    path: None, fallback_color: [40, 80, 140, 255] },
    TerrainSource { name: "Mountain", path: None, fallback_color: [90, 85, 80, 255] },
];

/// Try to load a source texture from common locations.
fn try_load_texture(name: &str) -> Option<RgbaImage> {
    // Check for textures placed in assets/textures/
    let asset_path = format!("assets/textures/{name}.png");
    if let Ok(reader) = ImageReader::open(&asset_path) {
        if let Ok(img) = reader.decode() {
            println!("  Loaded {asset_path}");
            return Some(img.to_rgba8());
        }
    }
    None
}

fn make_fallback(color: [u8; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(LAYER_SIZE, LAYER_SIZE);
    let pixel = Rgba(color);
    for py in 0..LAYER_SIZE {
        for px in 0..LAYER_SIZE {
            img.put_pixel(px, py, pixel);
        }
    }
    img
}

fn resize_to_layer(img: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(LAYER_SIZE, LAYER_SIZE);
    let sx = img.width() as f32 / LAYER_SIZE as f32;
    let sy = img.height() as f32 / LAYER_SIZE as f32;
    for py in 0..LAYER_SIZE {
        for px in 0..LAYER_SIZE {
            let src_x = ((px as f32 * sx) as u32).min(img.width() - 1);
            let src_y = ((py as f32 * sy) as u32).min(img.height() - 1);
            out.put_pixel(px, py, *img.get_pixel(src_x, src_y));
        }
    }
    out
}

fn main() {
    let atlas_w = LAYER_SIZE;
    let atlas_h = LAYER_SIZE * NUM_LAYERS;
    let mut atlas = RgbaImage::new(atlas_w, atlas_h);

    for (i, source) in SOURCES.iter().enumerate() {
        println!("Layer {i} ({}):", source.name);

        let layer = if let Some(path) = source.path {
            match ImageReader::open(path).and_then(|r| Ok(r.decode())) {
                Ok(Ok(img)) => {
                    println!("  Loaded {path}");
                    resize_to_layer(&img.to_rgba8())
                }
                _ => {
                    println!("  Failed to load {path}, using fallback");
                    make_fallback(source.fallback_color)
                }
            }
        } else if let Some(img) = try_load_texture(source.name) {
            resize_to_layer(&img)
        } else {
            println!("  Using fallback color");
            make_fallback(source.fallback_color)
        };

        let dest_y = i as u32 * LAYER_SIZE;
        for py in 0..LAYER_SIZE {
            for px in 0..LAYER_SIZE {
                atlas.put_pixel(px, dest_y + py, *layer.get_pixel(px, py));
            }
        }
    }

    atlas.save("assets/terrain_array.png")
        .expect("Failed to save terrain_array.png");

    println!("\nGenerated assets/terrain_array.png ({atlas_w}x{atlas_h}, {NUM_LAYERS} layers at {LAYER_SIZE}x{LAYER_SIZE})");
}
