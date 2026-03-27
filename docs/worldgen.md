# World Generation (pure Rust, no Bevy)

## World Map (`worldgen/world_map.rs`)
- 256x256 grid of zones (65,536 cells), each zone 250x250 tiles
- Noise-driven geography using four Fbm<Perlin> layers:
  - **Elevation** (freq 0.008, 6 octaves): continent shapes, ocean < 0.42, mountain > 0.78
  - **Continent modulation** (freq 0.003, 2 octaves): large-scale elevation variation for ocean variety
  - **Moisture** (freq 0.015, 5 octaves): wet/dry regions, boosted near ocean via BFS falloff (factor 0.20)
  - **Temperature** (latitude gradient + freq 0.02 noise): warm south, cold north, reduced by elevation
- **Ridge noise** for mountain ranges: domain-warped (freq 0.025, 4 octaves) with seed-driven directional stretch, power curve for sharp narrow ridges
- **Biome classification** from (elevation, moisture, temperature):
  - Ocean, Mountain, Tundra, Desert (temp>0.60, moist<0.35), Swamp (moist>0.70), Forest (moist>0.55), Grassland, Coast
- **Mountain smoothing**: post-classification pass connects isolated mountain cells into ranges
- **Ocean balance**: adaptive threshold ensures 15-60% ocean coverage across all seeds
- **Rivers**: 5-40 sources at high elevation, walk downhill to ocean with momentum + meander. Track entry/exit edges and width per cell.
- **Region identification**: flood-fill contiguous same-type zones → `region_id` (foundation for divine domains)
- **Settlements**: 2-12 placed on Grassland/Forest, preferring river adjacency, minimum spacing. Each gets a procedural name.
- Multiple landmasses emerge naturally from elevation + continent noise

### WorldCell fields
- `zone_type`, `has_cave`, `explored` (original)
- `elevation`, `moisture`, `temperature` (0.0-1.0 noise values)
- `river` (bool), `river_entry` ([N,E,S,W] edge flags), `river_width` (0.0-1.0 progress)
- `region_id` (Option<u32>, contiguous biome region)
- `settlement_name` (Option<String>, procedurally generated)

## Zone Generation (`worldgen/zone.rs`)
- **Noise-based terrain**: 3 noise layers per zone (detail, wetness, rocky) drive per-tile terrain selection
- **Context-aware**: `ZoneGenContext` carries world-level elevation/moisture/temperature + neighbor zone types + ocean edge directions
- **Biome recipes**: each `ZoneType` has a terrain selection function mapping noise values to terrain types:
  - Grassland: grass base, dirt/forest/stone/water from noise
  - Forest: forest base, grass clearings, dirt paths, swamp patches
  - Mountain: stone base, grass valleys, impassable peaks, alpine water
  - Desert: sand base, rare oasis water, stone outcrops
  - Tundra: snow base, stone/dirt/mountain from noise
  - Swamp: swamp base, water pools, grass/dirt islands
  - Coast: sand base, **directional water toward ocean** (noise-modulated shoreline), sand transition band
- **Edge blending**: within 30 tiles of zone borders, terrain blends toward neighbor's base terrain via noise threshold
- **Coast direction**: `ocean_edges: [bool; 4]` on `ZoneGenContext` tracks which edges face ocean. Coast zones place water on the correct side(s).
- **River carving**: uses world-level entry/exit edges and width. Noise-driven curved path with variable width (2-10 tiles) and dirt riverbanks.
- **Terrain features**: 200-800 per zone, noise-driven scatter with per-biome feature tables. 28 feature kinds (trees, rocks, bushes, etc.). Blocking features update walk_cost/blocks_los. Spawned as y-sorted sprite entities on `TERRAIN_FEATURES` layer.
- **Settlement**: biome-aware theming (sand in desert, snow in tundra, raised stone paths in swamp, directional water on coast). Named settlements with procedural names.
- **Enemy camps**: 1-3 per zone, 2-5 enemies. Clear a dirt patch around camp center. Features culled within 6 tiles.
- **Wildlife**: 1-2 spawns per zone (grassland/forest/swamp/coast). Neutral faction (fights back if attacked). Biome-appropriate names.
- **Cave entrances**: visible dark placeholder sprite (2x tile size)
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

## Terrain Features (`terrain.rs`)
28 feature kinds across 7 biomes + universal. Each has `blocks_movement()`, `blocks_los()`, and `placeholder_color()`.
- **Universal**: BoulderSmall, BoulderLarge, Bush
- **Grassland**: LoneTree, TallGrass, Wildflowers
- **Forest**: DeciduousTree, ConiferTree, FallenLog, TreeStump, MushroomCluster
- **Mountain**: RockSpire, RubblePile, DeadTreeAlpine
- **Desert**: Cactus, DesertScrub, BleachedBones, SandWornRock
- **Tundra**: SnowPine, IceChunk, FrozenDeadTree
- **Swamp**: SwampTree, ReedCluster, HangingMoss
- **Coast**: Driftwood, BeachGrass, TidalRock

Spawned as `TerrainFeatureEntity` + `ZoneEntity` on `TERRAIN_FEATURES` layer (z=1.0) with y-sorting. No persistence — deterministic from seed.

## Cave Generation (`worldgen/cave.rs`)
- Cellular automata: 45% random fill -> 5 iterations of 4-5 smoothing rule
- Produces natural-looking cavern systems with corridors
- Entrance area cleared, enemy groups placed in open chambers

## World Map UI (`systems/world_map_ui.rs`)
- Toggle with M key, full-screen overlay with semi-transparent background
- 256x256 pixel texture with nearest-neighbor scaling, settlement icons (gold outlined circles)
- **Legend**: color-coded biome key with river entry
- **Settlement labels**: procedural names displayed near icons
- **Zoom/Pan**: scroll wheel zoom (1x-4x), arrow key panning, clipping container
- **Clickable zones**: left-click to inspect any zone's biome/elevation/moisture/temperature
- Camera pan disabled while map overlay is open

## Zone Transitions (`systems/zone.rs`)
- `detect_zone_edge`: fires event when player reaches map boundary, checks `is_passable()` (blocks ocean)
- `handle_zone_transition`: builds `ZoneGenContext` from world map (includes river entry/width), generates zone via `generate_zone_with_context`, snapshots entities, repositions player
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

## God Pool (`worldgen/gods.rs`)
- 8 god archetypes (Fire, Frost, Storm, Holy, Shadow, Nature, Necromancy, Arcane), growing to ~25
- **Fixed per archetype:** domain (MagicSchool), terrain influence, gift to mortals, 5 spells (data only), Propp tendencies
- **Randomized per run:** name (syllable generation), 2-4 personality traits (CharacterTrait, shared with mortal characters)
- **GodPool** resource: holds all archetypes, `draw_pantheon(6, rng)` selects and randomizes
- **DrawnPantheon** resource: drawn god IDs, names, traits, and emergent relationships
- **Trait rolling:** domain-specific weight modifiers (e.g., Fire +Warlike+15, +Ruthless+12) with thematic blocklists (e.g., Holy blocks Treacherous/Corrupt/Cowardly)
- **Relationships:** emergent from domain category overlap, trait axis alignment (aggression/peace/darkness/virtue/intellect/ambition/fear/zeal), and forbidden school dynamics
- **Name generation:** 30 prefixes x 20 mid-syllables x 30 suffixes, 40% chance of mid-syllable for length variation

## Noise Utilities (`worldgen/noise_util.rs`)
- `NoiseLayer`: wrapper around `Fbm<Perlin>` with configurable frequency/octaves
- `sample(x, y) -> f64`: raw noise (-1.0 to 1.0)
- `sample_normalized(x, y) -> f64`: mapped to 0.0-1.0, clamped
