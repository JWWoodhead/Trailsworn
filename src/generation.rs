use crate::resources::map::TileWorld;
use crate::terrain::TerrainType;

/// Generate a simple test map with some terrain variety.
pub fn generate_test_map(width: u32, height: u32) -> TileWorld {
    let mut world = TileWorld::filled(width, height, TerrainType::Grass);

    // Scatter some dirt patches
    for x in 20..40 {
        for y in 20..35 {
            world.set_terrain(x, y, TerrainType::Dirt);
        }
    }

    // A lake
    for x in 80..110 {
        for y in 60..85 {
            let dx = x as f32 - 95.0;
            let dy = y as f32 - 72.5;
            if dx * dx + dy * dy < 225.0 {
                world.set_terrain(x, y, TerrainType::Water);
            }
        }
    }

    // Forest band
    for x in 0..width {
        for y in 140..165 {
            if (x + y) % 3 != 0 {
                world.set_terrain(x, y, TerrainType::Forest);
            }
        }
    }

    // Mountain range
    for x in 180..210 {
        for y in 30..60 {
            let dx = x as f32 - 195.0;
            let dy = y as f32 - 45.0;
            if dx * dx / 225.0 + dy * dy / 100.0 < 1.0 {
                world.set_terrain(x, y, TerrainType::Mountain);
            }
        }
    }

    // Stone patches
    for x in 50..65 {
        for y in 100..115 {
            world.set_terrain(x, y, TerrainType::Stone);
        }
    }

    world
}
