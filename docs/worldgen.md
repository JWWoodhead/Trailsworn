# World Generation (pure Rust, no Bevy)

## World Map (`worldgen/world_map.rs`)
- 256x256 grid of zones (65,536 cells), each zone 250x250 tiles
- Noise-driven geography using three Fbm<Perlin> layers:
  - **Elevation** (freq 0.012, 6 octaves): continent shapes, ocean < 0.35, mountain > 0.80
  - **Moisture** (freq 0.015, 5 octaves): wet/dry regions, boosted near ocean via BFS falloff
  - **Temperature** (latitude gradient + freq 0.02 noise): warm south, cold north, reduced by elevation
- **Biome classification** from (elevation, moisture, temperature):
  - Ocean, Mountain, Tundra, Desert, Swamp, Forest, Grassland, Coast
- **Rivers**: 3-20 sources at high elevation, walk downhill to ocean. Rivers boost adjacent moisture.
- **Region identification**: flood-fill contiguous same-type zones → `region_id` (foundation for divine domains)
- **Settlements**: 2-12 placed on Grassland/Forest, preferring river adjacency, minimum spacing
- Multiple landmasses emerge naturally from elevation noise

### WorldCell fields
- `zone_type`, `has_cave`, `explored` (original)
- `elevation`, `moisture`, `temperature` (0.0-1.0 noise values)
- `river` (bool)
- `region_id` (Option<u32>, contiguous biome region)

## Zone Generation (`worldgen/zone.rs`)
- **Noise-based terrain**: 3 noise layers per zone (detail, wetness, rocky) drive per-tile terrain selection
- **Context-aware**: `ZoneGenContext` carries world-level elevation/moisture/temperature + neighbor zone types
- **Biome recipes**: each `ZoneType` has a terrain selection function mapping noise values to terrain types:
  - Grassland: grass base, dirt/forest/stone/water from noise
  - Forest: forest base, grass clearings, dirt paths, swamp patches
  - Mountain: stone base, grass valleys, impassable peaks, alpine water
  - Desert: sand base, rare oasis water, stone outcrops
  - Tundra: snow base, stone/dirt/mountain from noise
  - Swamp: swamp base, water pools, grass/dirt islands
  - Coast: sand base, water/grass/dirt blending
- **Edge blending**: within 30 tiles of zone borders, terrain blends toward neighbor's base terrain via noise threshold
- **River carving**: meandering 3-wide water channel across zones marked as river
- **Settlement**: hand-crafted (dirt square, stone buildings, roads)
- POIs: cave entrances, enemy camps (1-3, 2-5 enemies), wildlife spawns
- Deterministic from seed

## Terrain Types (`terrain.rs`)
9 types: Grass, Dirt, Sand, Snow, Swamp, Stone, Forest, Water, Mountain

| Terrain  | walk_cost | blocks_los | flammability | blend_priority |
|----------|-----------|------------|--------------|----------------|
| Grass    | 1.0       | false      | 0.3          | 0              |
| Dirt     | 1.0       | false      | 0.0          | 1              |
| Sand     | 1.3       | false      | 0.0          | 2              |
| Snow     | 1.4       | false      | 0.0          | 3              |
| Swamp    | 2.0       | false      | 0.1          | 4              |
| Stone    | 1.0       | false      | 0.0          | 5              |
| Forest   | 1.5       | true       | 0.8          | 6              |
| Water    | 0.0       | false      | 0.0          | 7              |
| Mountain | 0.0       | true       | 0.0          | 8              |

## Cave Generation (`worldgen/cave.rs`)
- Cellular automata: 45% random fill -> 5 iterations of 4-5 smoothing rule
- Produces natural-looking cavern systems with corridors
- Entrance area cleared, enemy groups placed in open chambers

## Zone Transitions (`systems/zone.rs`)
- `detect_zone_edge`: fires event when player reaches map boundary, checks `is_passable()` (blocks ocean)
- `handle_zone_transition`: builds `ZoneGenContext` from world map, generates zone via `generate_zone_with_context`, snapshots entities, repositions player
- `rendering::update_terrain_map`: detects `TileWorld` change and rebuilds the terrain map GPU texture

## History Generation (`worldgen/history/`)
- 100-year simulation producing factions, characters, settlements, events, artifacts, cultures
- **State-driven**: `WorldState` tracks pairwise faction sentiment (`RelationMatrix`), active wars/alliances/treaties
- **Faction gauges**: military_strength, wealth, stability (1-100). Wars drain, treaties add, settlements produce income.
- **Prerequisite-based events**: wars require hostility (sentiment < -20), alliances require friendship (> 30), etc.
- **Character system**: persistent characters with 27 traits across 5 personality axes, race-weighted. Characters have roles (Leader, General, Hero, Scholar, Villain), ambitions, epithets.
- **Character-driven probability**: Warlike leaders increase war chance (+20%), Peaceful leaders decrease it (-25%), Treacherous characters enable betrayal events.
- **Race lifespans**: Orc 40-60yr, Elf 500-1000yr, Human 60-80, Dwarf 150-250, Goblin 30-50.
- **Cultural accumulation**: faction history produces cultural values and taboos based on event patterns.
- **Artifacts**: persistent named items held by characters, discovered through events.
- **Not yet integrated into gameplay** (see `docs/worldgen-vision.md`)

## Population Tables (`worldgen/population_table.rs`)
- Weighted random selection tool (Caves of Qud pattern)
- `PickOne`: weighted single selection
- `PickEach`: independent probability per entry
- `PickN`: pick N unique entries without replacement

## Name Generation (`worldgen/names.rs`)
- Race-specific character names (Human, Dwarf, Elf, Orc, Goblin)
- Settlement names (prefix+root: "Ironhold", "Deepcrossing")
- Faction names (8 types x pattern templates)
- Region names

## Noise Utilities (`worldgen/noise_util.rs`)
- `NoiseLayer`: wrapper around `Fbm<Perlin>` with configurable frequency/octaves
- `sample(x, y) -> f64`: raw noise (-1.0 to 1.0)
- `sample_normalized(x, y) -> f64`: mapped to 0.0-1.0, clamped
