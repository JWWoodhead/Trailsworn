# Trailsworn

Party-based fantasy RPG built with Bevy 0.18 (Rust). Rimworld-style tactical combat + Kenshi-style open-world exploration. MMO-paced combat with pause-and-command.

**Crate name:** `trailsworn`

## Project Structure

```
src/
  main.rs              — App setup, resource init, system registration
  lib.rs               — Module declarations
  terrain.rs           — TerrainType enum + blend_priority
  generation.rs        — Legacy test map generator (replaced by worldgen)
  systems/             — All Bevy systems (one file per system or system group)
    task/              — Task execution + AI evaluator systems
  resources/           — All Components, Resources, and data types
  pathfinding/         — A* and HPA* (pure algorithms, no Bevy)
  worldgen/            — World/zone/history generation (pure Rust, no Bevy)
assets/                — Shader, textures, generated texture array
examples/              — Asset generation tools (gen_terrain_array, gen_atlas)
tests/                 — Integration test harness + per-system tests (~257 tests)
```

## Architecture Rules

- **No custom Bevy plugins** — everything registered directly in main.rs. Exception: `MaterialTilemapPlugin<TerrainMaterial>`.
- **Organize by ECS role**: `systems/` for systems, `resources/` for components/resources
- **Pure logic at top level**: terrain.rs, generation.rs, pathfinding/, worldgen/
- All keybindings centralized in `resources/input.rs`
- All colors centralized in `resources/theme.rs`

## System Pipeline

```
Input → Tick → Ai → Combat → Movement → Ui → Render
```

All gated by `run_if(in_state(GameState::Playing))`. See [docs/systems.md](docs/systems.md) for per-system details.

## Best Practices

- **GameState**: Loading → Playing. Systems `run_if(in_state(Playing))`.
- **SystemSets**: `GameSet` enum for coarse ordering. Individual systems use `.in_set()`.
- **DespawnOnExit(GameState::Playing)**: on all spawned gameplay entities.
- **StableId**: on all persistent entities. `StableIdRegistry` for lookups. Ready for save/load.
- **Name component**: Bevy's built-in `Name` alongside `EntityName` on all characters.
- **Events as Messages**: `DamageDealtEvent`, `AttackMissedEvent`, `ZoneTransitionEvent` use Bevy 0.18's `Message`/`MessageWriter`/`MessageReader`.
- **CurrentTask on all agents**: every entity that acts gets a `CurrentTask`. NPC entities additionally get `AiBrain::enemy()`. Player entities have no `AiBrain` — driven by player commands.

## Design System ("The Gritty Chronicle")

Colors in `resources/theme.rs`:
- Surface: `#131313` (dark iron) | Text: `#F5F5DC` (parchment)
- Primary: `#f2ca50` → `#d4af37` (gold) | Secondary: `#ffb4a8` / `#920703` (blood)
- 0px border radius everywhere. No 1px borders — use tonal background shifts. Shadows tinted warm.

## Detailed Reference Docs

- [docs/systems.md](docs/systems.md) — Full system execution order, task/brain system, movement pipeline, input, UI panel
- [docs/combat.md](docs/combat.md) — Hit resolution, body parts, damage types, magic schools, abilities, status effects, threat
- [docs/items.md](docs/items.md) — Item/affix definitions, rarity model, generation pipeline, equipment integration
- [docs/worldgen.md](docs/worldgen.md) — World map, zone/cave generation, history simulation, population tables, names
- [docs/rendering.md](docs/rendering.md) — Terrain shader, blending, UV conventions, render layers, pathfinding algorithms
- [docs/extension-guide.md](docs/extension-guide.md) — How to add items, affixes, damage types, equip slots, components (with save categorization)

## Known Issues

- **Tile occupancy**: entities can overlap on the same tile
- **Zone persistence (partial)**: tracks killed enemies + alive entity state; ground items/terrain mods not persisted
- **Diagonal speed**: ~41% faster diagonally (accepted)
- **No mob drops / loot tables**
- **Attribute bonuses from equipment not applied** (needs `EffectiveAttributes` refactor)
- **No equip/unequip from UI**
- **No save/load** (`StableId` infrastructure ready, serialization not built)
- **UseCondition gaps**: `AllyHpBelow` and `EnemiesInRange` evaluator conditions are stubbed
- **Terrain feature sprites**: no y-sortable sprite entities yet

## CLI

```
cargo run                              # Normal
cargo run -- --debug all               # All debug visualizations
cargo run -- --debug grid,pathing      # Specific flags
cargo run -- --debug perf              # FPS + entity count profiling
cargo run --release                    # Optimized build
cargo run --features dev               # Dynamic linking (faster dev compiles)
cargo run -- --biome forest            # Spawn in a specific biome (grassland, forest, mountain, desert, tundra, swamp, coast, settlement)
cargo run --example gen_terrain_array  # Regenerate terrain texture array
```

Runtime debug toggles: F1=grid, F2=pathing, F3=aggro, F4=AI state, F5=profiling
