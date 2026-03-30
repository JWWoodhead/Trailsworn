use super::*;

#[test]
fn world_map_has_correct_size() {
    let map = generate_world_map(64, 64, 42);
    assert_eq!(map.cells.len(), 64 * 64);
    assert_eq!(map.width, 64);
    assert_eq!(map.height, 64);
}

#[test]
fn world_map_has_settlement() {
    let map = generate_world_map(64, 64, 42);
    let has_settlement = map.cells.iter().any(|c| c.settlement_name.is_some());
    assert!(has_settlement);
}

#[test]
fn large_map_has_settlement_variety() {
    let map = generate_world_map(256, 256, 42);
    let hamlets = map.cells.iter().filter(|c| c.settlement_size == Some(SettlementSize::Hamlet)).count();
    let villages = map.cells.iter().filter(|c| c.settlement_size == Some(SettlementSize::Village)).count();
    let towns = map.cells.iter().filter(|c| c.settlement_size == Some(SettlementSize::Town)).count();
    let cities = map.cells.iter().filter(|c| c.settlement_size == Some(SettlementSize::City)).count();
    assert!(hamlets > 10, "Expected >10 hamlets, got {}", hamlets);
    assert!(villages > 5, "Expected >5 villages, got {}", villages);
    assert!(towns > 2, "Expected >2 towns, got {}", towns);
    assert!(cities >= 1, "Expected >=1 city, got {}", cities);
    // Towns and cities should have ZoneType::Settlement
    let zone_settlements = map.cells.iter().filter(|c| c.zone_type == ZoneType::Settlement).count();
    assert_eq!(zone_settlements, towns + cities);
}

#[test]
fn world_map_has_caves() {
    let map = generate_world_map(64, 64, 42);
    let caves = map.cells.iter().filter(|c| c.has_cave).count();
    assert!(caves > 0);
}

#[test]
fn settlement_has_no_cave() {
    let map = generate_world_map(64, 64, 42);
    for cell in &map.cells {
        if cell.zone_type == ZoneType::Settlement {
            assert!(!cell.has_cave);
        }
    }
}

#[test]
fn spawn_is_in_bounds() {
    let map = generate_world_map(64, 64, 42);
    assert!(map.in_bounds(map.spawn_pos));
}

#[test]
fn spawn_zone_is_explored() {
    let map = generate_world_map(64, 64, 42);
    let cell = map.get(map.spawn_pos).unwrap();
    assert!(cell.explored);
}

#[test]
fn spawn_is_not_ocean() {
    let map = generate_world_map(64, 64, 42);
    let cell = map.get(map.spawn_pos).unwrap();
    assert_ne!(cell.zone_type, ZoneType::Ocean);
}

#[test]
fn has_multiple_biomes() {
    let map = generate_world_map(128, 128, 42);
    let mut types = std::collections::HashSet::new();
    for cell in &map.cells {
        types.insert(cell.zone_type);
    }
    // Should have at least Ocean + 3 land biomes
    assert!(types.len() >= 4, "Only found {} biome types: {:?}", types.len(), types);
}

#[test]
fn has_ocean_cells() {
    let map = generate_world_map(128, 128, 42);
    let ocean_count = map.cells.iter().filter(|c| c.zone_type == ZoneType::Ocean).count();
    assert!(ocean_count > 0, "No ocean cells found");
}

#[test]
fn has_land_cells() {
    let map = generate_world_map(128, 128, 42);
    let land_count = map.cells.iter().filter(|c| c.zone_type != ZoneType::Ocean).count();
    assert!(land_count > 100, "Too few land cells: {land_count}");
}

#[test]
fn has_rivers() {
    let map = generate_world_map(128, 128, 42);
    let river_count = map.cells.iter().filter(|c| c.river).count();
    assert!(river_count > 0, "No river cells found");
}

#[test]
fn regions_are_assigned() {
    let map = generate_world_map(64, 64, 42);
    let assigned = map
        .cells
        .iter()
        .filter(|c| c.zone_type != ZoneType::Ocean && c.region_id.is_some())
        .count();
    let land = map
        .cells
        .iter()
        .filter(|c| c.zone_type != ZoneType::Ocean)
        .count();
    assert_eq!(assigned, land, "All land cells should have a region_id");
}

#[test]
fn deterministic() {
    let a = generate_world_map(64, 64, 123);
    let b = generate_world_map(64, 64, 123);
    for (ca, cb) in a.cells.iter().zip(b.cells.iter()) {
        assert_eq!(ca.zone_type, cb.zone_type);
        assert_eq!(ca.elevation, cb.elevation);
        assert_eq!(ca.river, cb.river);
    }
}

#[test]
fn is_passable_checks_ocean() {
    let map = generate_world_map(64, 64, 42);
    // Find an ocean cell
    for (i, cell) in map.cells.iter().enumerate() {
        let pos = WorldPos::new((i as u32 % map.width) as i32, (i as u32 / map.width) as i32);
        if cell.zone_type == ZoneType::Ocean {
            assert!(!map.is_passable(pos));
        } else {
            assert!(map.is_passable(pos));
        }
    }
}

#[test]
fn biome_balance_across_seeds() {
    // Verify biome distribution is reasonable across multiple seeds
    for seed in [1, 42, 123, 999, 2025, 7777, 31415, 54321, 100_000, 999_999] {
        let map = generate_world_map(128, 128, seed);
        let total = map.cells.len() as f32;

        let mut counts = std::collections::HashMap::new();
        for cell in &map.cells {
            *counts.entry(cell.zone_type).or_insert(0u32) += 1;
        }

        let ocean_pct = *counts.get(&ZoneType::Ocean).unwrap_or(&0) as f32 / total;
        let forest_pct = *counts.get(&ZoneType::Forest).unwrap_or(&0) as f32 / total;

        // Ocean should be between 15% and 65%
        assert!(
            ocean_pct > 0.15 && ocean_pct < 0.65,
            "Seed {seed}: ocean {:.1}% out of range (expected 15-65%)",
            ocean_pct * 100.0
        );

        // Forest should not dominate (< 30%)
        assert!(
            forest_pct < 0.30,
            "Seed {seed}: forest {:.1}% too high (expected < 30%)",
            forest_pct * 100.0
        );

        // Should have at least 5 distinct biome types
        assert!(
            counts.len() >= 5,
            "Seed {seed}: only {} biome types: {:?}",
            counts.len(),
            counts.keys().collect::<Vec<_>>()
        );

        // No single land biome should exceed 35%
        for (biome, count) in &counts {
            if *biome == ZoneType::Ocean {
                continue;
            }
            let pct = *count as f32 / total;
            assert!(
                pct < 0.35,
                "Seed {seed}: {:?} at {:.1}% exceeds 35% cap",
                biome,
                pct * 100.0
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Road tests
// ---------------------------------------------------------------------------

#[test]
fn roads_connect_settlements() {
    let map = generate_world_map(128, 128, 42);
    let road_cells = map.cells.iter().filter(|c| c.road).count();
    assert!(road_cells > 0, "Map should have road cells");
}

#[test]
fn road_cells_have_entry_edges() {
    let map = generate_world_map(128, 128, 42);
    for cell in &map.cells {
        if cell.road {
            let entries = cell.road_entry.iter().filter(|e| **e).count();
            assert!(entries >= 1, "Road cell should have at least 1 entry edge");
        }
    }
}

#[test]
fn road_edges_are_symmetric() {
    let map = generate_world_map(128, 128, 42);
    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let pos = WorldPos::new(x, y);
            let Some(cell) = map.get(pos) else { continue };
            if !cell.road { continue; }

            // If this cell exits north, the north neighbor should enter south
            if cell.road_entry[0] {
                let n = WorldPos::new(x, y + 1);
                if let Some(nc) = map.get(n) {
                    assert!(nc.road_entry[2], "Road edge N/S mismatch at ({x},{y})");
                }
            }
            // East
            if cell.road_entry[1] {
                let e = WorldPos::new(x + 1, y);
                if let Some(ec) = map.get(e) {
                    assert!(ec.road_entry[3], "Road edge E/W mismatch at ({x},{y})");
                }
            }
            // South
            if cell.road_entry[2] {
                let s = WorldPos::new(x, y - 1);
                if let Some(sc) = map.get(s) {
                    assert!(sc.road_entry[0], "Road edge S/N mismatch at ({x},{y})");
                }
            }
            // West
            if cell.road_entry[3] {
                let w_pos = WorldPos::new(x - 1, y);
                if let Some(wc) = map.get(w_pos) {
                    assert!(wc.road_entry[1], "Road edge W/E mismatch at ({x},{y})");
                }
            }
        }
    }
}

#[test]
fn no_roads_in_ocean() {
    let map = generate_world_map(128, 128, 42);
    for cell in &map.cells {
        if cell.zone_type == ZoneType::Ocean {
            assert!(!cell.road, "Ocean cell should not have road");
        }
    }
}

#[test]
fn roads_deterministic() {
    let a = generate_world_map(128, 128, 42);
    let b = generate_world_map(128, 128, 42);
    for (ca, cb) in a.cells.iter().zip(b.cells.iter()) {
        assert_eq!(ca.road, cb.road);
        assert_eq!(ca.road_entry, cb.road_entry);
        assert_eq!(ca.road_class, cb.road_class);
    }
}

#[test]
fn roads_have_class() {
    let map = generate_world_map(128, 128, 42);
    let minor = map.cells.iter().filter(|c| c.road_class == 1).count();
    let major = map.cells.iter().filter(|c| c.road_class == 2).count();
    assert!(minor + major > 0, "Should have classified road cells");
}
