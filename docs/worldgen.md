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
- **Settlements**: ~70 placed across habitable land with size distribution: ~40 hamlets, ~20 villages, ~8 towns, ~3 cities. Towns/cities get `ZoneType::Settlement`; hamlets/villages keep natural terrain. Prefer river adjacency and grassland/forest. Cities spaced furthest apart, hamlets can cluster.
- Multiple landmasses emerge naturally from elevation + continent noise

### WorldCell fields
- `zone_type`, `has_cave`, `explored` (original)
- `elevation`, `moisture`, `temperature` (0.0-1.0 noise values)
- `river` (bool), `river_entry` ([N,E,S,W] edge flags), `river_width` (0.0-1.0 progress)
- `region_id` (Option<u32>, contiguous biome region)
- `settlement_name` (Option<String>, procedurally generated)
- `settlement_size` (Option<SettlementSize>: Hamlet/Village/Town/City)
- `divine_terrain` (Option<DivineTerrainType>: overlay marking divine influence)
- `divine_owner` (Option<GodId>: which god last owned this cell)

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
- **Terrain features**: 200-800 per zone, noise-driven scatter. Data-driven via `FeatureRegistry` (`resources/feature_defs.rs`) — each feature defined once with properties, sprite path, scale, and biome weights. Blocking features update walk_cost/blocks_los. Spawned as y-sorted sprite entities on `TERRAIN_FEATURES` layer with `Anchor::BottomCenter` offset.
- **Settlement**: biome-aware theming (sand in desert, snow in tundra, raised stone paths in swamp, directional water on coast). Named settlements with procedural names.
- **Enemy camps**: 1-3 per zone, 2-5 enemies. Clear a dirt patch around camp center. Features culled within 6 tiles.
- **Wildlife**: 1-2 spawns per zone (grassland/forest/swamp/coast). Neutral faction (fights back if attacked). Biome-appropriate names.
- **Cave entrances**: visible dark placeholder sprite (2x tile size)
- Deterministic from seed

## Terrain Types (`terrain.rs`)
9 types: Grass, Dirt, Sand, Snow, Swamp, Stone, Forest, Water, Mountain

| Terrain  | walk_cost | blocks_los | flammability | blend_priority |
|----------|-----------|------------|--------------|----------------|
| Grass    | 1.0       | false      | 0.3          | 2              |
| Dirt     | 1.0       | false      | 0.0          | 0              |
| Sand     | 1.3       | false      | 0.0          | 1              |
| Snow     | 1.4       | false      | 0.0          | 3              |
| Swamp    | 2.0       | false      | 0.1          | 4              |
| Stone    | 1.0       | false      | 0.0          | 5              |
| Forest   | 1.5       | true       | 0.8          | 6              |
| Water    | 0.0       | false      | 0.0          | 7              |
| Mountain | 0.0       | true       | 0.0          | 8              |

Note: blend_priority is retained in code but NOT used by the current shader. Terrain blending uses symmetric weighted-average (see `docs/rendering.md`).

## Terrain Features (`resources/feature_defs.rs`)
Data-driven via `FeatureRegistry`. Each feature is a single `FeatureDef` registration with all properties:
- `id`, `name`, `blocks_movement`, `blocks_los`, `placeholder_color`
- `sprite: Option<&str>` — asset path (None = placeholder color square)
- `scale: f32` — display size relative to tile
- `biome_weights: &[(ZoneType, u32)]` — which biomes it spawns in and at what weight

All biome-specific — no universal features. Each biome has its own visually appropriate set:
- **Grassland**: Fieldstone, Standing Stone, Hedge Bush, Lone Tree, Tall Grass, Wildflowers
- **Forest**: Oak Tree, Pine Tree, 3x Small Tree, 4x Rock variants, 4x Bush variants (all with real sprites from scenery pack)
- **Mountain**: Rock Spire, Rubble Pile, Dead Tree Alpine, Standing Stone
- **Desert**: Cactus, Desert Scrub, Bleached Bones, Sand-Worn Rock
- **Tundra**: Snow Pine, Ice Chunk, Frozen Dead Tree
- **Swamp**: Swamp Tree, Reed Cluster, Hanging Moss
- **Coast**: Driftwood, Beach Grass, Tidal Rock

Spawned as `TerrainFeatureEntity` + `ZoneEntity` on `TERRAIN_FEATURES` layer (z=1.0) with y-sorting. Features with real sprites use `Anchor::BottomCenter` (Y offset) and per-type scale. No persistence — deterministic from seed. Trimmed source sprites in `assets/features/`.

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

## History Generation (`worldgen/history/` + `worldgen/divine_era/`)

Unified simulation where gods and mortal factions coexist in the same 100-year timeline. Gods influence mortal events through worship, drives, and flaws. Called from `main.rs` with world map and pantheon.

### Core Architecture
- `generate_history(config, world_map, god_pool, pantheon, seed) -> WorldHistory`
- 10 phases per year, processing both divine and mortal actors
- `WorldHistory` output contains factions, characters, settlements, events, gods, divine sites/artifacts/races, terrain scars

### God Personality System (`divine_era/personality.rs`)
Each drawn god has a **drive** (what they want) and **flaw** (how it breaks them), rolled from domain weights + trait modifiers:
- **10 Drives**: Knowledge, Dominion, Worship, Perfection, Justice, Love, Freedom, Legacy, Vindication, Supremacy
- **10 Flaws**: Hubris, Jealousy, Obsession, Cruelty, Blindness, Isolation, Betrayal, Sacrifice, Rigidity, Hollowness
- Domain sets base weights (e.g., Fire favors Supremacy/Perfection), traits shift them (e.g., Warlike boosts Supremacy)
- Same archetype can tell different stories across runs depending on rolled traits

### Year Phases
```
Phase 0:  Character aging & death (mortals)
Phase 1:  God power update — power = f(worshippers), fade check
Phase 2:  Territory expansion (BFS, ~80 cells/god/year) & terrain shaping
Phase 3:  Worship competition — gods compete for settlement patronage
Phase 4:  Drive-based divine actions — what each god does depends on their drive
Phase 5-8: Mortal simulation — faction upkeep, settlement economy, characters, events
Phase 9:  Divine conflict — god wars, pacts, pact-breaking
Phase 10: Flaw pressure & triggers — reactive narrative events from god flaws
Phase 11: Sentiment drift — both faction and divine relations
```

### God Behaviors (`divine_era/behavior.rs`)
- **Territory expansion**: BFS from frontier, terrain-weighted, gods claim ~50% of map
- **Terrain shaping**: gods reshape zone_type to match their domain, 10% chance of divine terrain scars
- **Worship**: settlements in god territory may begin worshipping; drive affects conversion rate
- **Drive actions**: drive determines what gods build (Knowledge→observatories, Perfection→artifacts, Worship→temples, etc.)
- **Flaw triggers**: pressure accumulates (Jealousy from others' success, Hubris from victories, etc.), triggers at 80+ create narrative events (obsessed god neglects worshippers, jealous god turns on rival)
- **Divine wars**: hostility-driven, terrain scars from battles
- **Gods fade, not die**: zero worshippers for 20+ years → faded (can't act). Regaining worshippers un-fades them.

### Divine Creatures (`divine_era/creatures.rs`)
4 creatures per domain (32 total), each with a role:
- **Guardian**: guards sacred sites (Salamander, Ice Wyrm, Solar Lion, etc.)
- **Warrior**: fights in divine wars (Forge Golem, Frost Giant, Griffin, etc.)
- **Emissary**: appears to mortals as signs (Phoenix, Thunderbird, Night Stalker, etc.)
- **Companion**: accompanies the god (Fire Drake, Crystal Spirit, Dark Raven, etc.)

### Mortal Simulation
- **Faction gauges**: military_strength, wealth, stability (1-100). Patron god influences behavior.
- **Prerequisite-based events**: 17 mortal event types + 14 divine event types (31 total)
- **Character system**: 27 traits, 8 ambitions, roles, epithets, race-specific lifespans
- **Settlements**: ~70 on map with size tiers, patron god, devotion level
- **Cultural accumulation**: values/taboos from event patterns

### Divine Terrain Overlay (`divine_era/terrain_scars.rs`)
8 overlay types (not new TerrainType variants — metadata for future rendering):
Lava, Ice, ScorchedEarth, HallowedGround, Shadowlands, DeepWild, Blight, Crystal

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

## God Pool (`worldgen/gods.rs`) + God Systems (`worldgen/divine_era/`)

### Archetypes (`gods.rs`)
8 god archetypes (Fire, Frost, Storm, Holy, Shadow, Nature, Necromancy, Arcane), growing to ~25.
- **Fixed per archetype**: domain (MagicSchool), terrain influence, gift to mortals, 5 spells (data only), Propp tendencies
- **GodPool** resource: holds all archetypes, `draw_pantheon(6, rng)` selects and randomizes
- **DrawnPantheon** resource: drawn god IDs, names, traits, and emergent relationships

### Per-Run Randomization
- **Name**: syllable generation (30 prefixes x 20 mid-syllables x 30 suffixes, 40% mid chance)
- **Traits**: 2-4 `CharacterTrait` values, domain-weighted with thematic blocklists
- **Drive + Flaw**: rolled from domain weights + trait modifiers (see personality system above)
- **Relationships**: emergent from domain overlap, trait axis alignment, forbidden school dynamics

### God Lifecycle
Gods never truly die — they fade when they have no worshippers:
- **Power = worshippers**: god power scales directly with how many settlements worship them
- **Fading**: 20+ consecutive years without worshippers → god fades (can't act)
- **Revival**: if a settlement starts worshipping a faded god, they un-fade
- **Territory**: gods claim territory via BFS expansion on the world map, ~80 cells/year
- **Worship competition**: gods compete for settlement patronage based on territory and drive

### God-Created Content
- **Divine artifacts** (`divine_era/artifacts.rs`): named weapons/armor/implements/keys/vessels with domain-themed name generation
- **Divine sites** (`divine_era/sites.rs`): temples, forges, observatories, necropolises, sacred groves, etc. per domain
- **Created races** (`divine_era/races.rs`): god-flavored race templates (e.g., Fire→Forgeborn Dwarves, Nature→Rootborn Elves)
- **Mythical creatures** (`divine_era/creatures.rs`): 4 per domain with roles (guardian/warrior/emissary/companion)

## Noise Utilities (`worldgen/noise_util.rs`)
- `NoiseLayer`: wrapper around `Fbm<Perlin>` with configurable frequency/octaves
- `sample(x, y) -> f64`: raw noise (-1.0 to 1.0)
- `sample_normalized(x, y) -> f64`: mapped to 0.0-1.0, clamped
