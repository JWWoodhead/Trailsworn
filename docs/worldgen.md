# World Generation (pure Rust, no Bevy)

## World Map (`worldgen/world_map.rs`)
- 5x5 grid of zones, each 250x250 tiles
- Zone types: Grassland, Forest, Mountain, Settlement
- Caves can spawn in any non-settlement zone (30% chance)
- Settlement placed near center, player spawns adjacent

## Zone Generation (`worldgen/zone.rs`)
- Biome-appropriate terrain: grassland (grass + patches), forest (trees + clearings), mountain (stone + valleys), settlement (dirt roads + stone buildings)
- Organic shapes via jittered-radius patches
- POIs: cave entrances, enemy camps (1-3 per zone, 2-5 enemies), wildlife spawns
- Deterministic from seed

## Cave Generation (`worldgen/cave.rs`)
- Cellular automata: 45% random fill -> 5 iterations of 4-5 smoothing rule
- Produces natural-looking cavern systems with corridors
- Entrance area cleared, enemy groups placed in open chambers

## Zone Transitions (`systems/zone.rs`)
- `detect_zone_edge`: fires event when player reaches map boundary
- `handle_zone_transition`: snapshots alive entities to `ZoneStateCache`, despawns `ZoneEntity` marked entities, generates new zone, applies snapshot (skips dead, restores alive state), replaces `TileWorld` resource, repositions player at opposite edge
- `rendering::update_terrain_map`: detects `TileWorld` change and rebuilds the terrain map GPU texture

## History Generation (`worldgen/history/`)
- 100-year simulation producing factions, characters, settlements, events, artifacts, cultures
- **State-driven**: `WorldState` tracks pairwise faction sentiment (`RelationMatrix`), active wars/alliances/treaties
- **Faction gauges**: military_strength, wealth, stability (1-100). Wars drain, treaties add, settlements produce income.
- **Prerequisite-based events**: wars require hostility (sentiment < -20), alliances require friendship (> 30), etc.
- **Character system**: persistent characters with 27 traits across 5 personality axes, race-weighted (Orcs lean Warlike, Elves lean Wise). Characters have roles (Leader, General, Hero, Scholar, Villain), ambitions, epithets ("the Bold", "Oathbreaker").
- **Character-driven probability**: Warlike leaders increase war chance (+20%), Peaceful leaders decrease it (-25%), Treacherous characters enable betrayal events.
- **Race lifespans**: Orc 40-60yr (fast leader churn), Elf 500-1000yr (ancient rulers), Human 60-80, Dwarf 150-250, Goblin 30-50.
- **Cultural accumulation**: faction history produces cultural values (MilitaryProwess, Commerce, Scholarship) and taboos (Treachery, War, Outsiders) based on event patterns.
- **Artifacts**: persistent named items (The Eternal Gauntlets of Whispers) held by characters, discovered through events.
- **Simulation loop per year**: Aging/Death -> Faction Upkeep -> Settlement Upkeep -> New Characters -> Friction -> Event Evaluation -> Sentiment Drift

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
