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
    world_map/         — World map gen: terrain, rivers, roads, settlements, regions
assets/                — Shader, textures, generated texture array
examples/              — Asset generation tools (gen_terrain_array, gen_atlas)
tests/                 — Integration test harness + per-system tests (~257 tests)
```

## Architecture Rules

- **No custom Bevy plugins** — everything registered directly in main.rs. Exceptions: `Material2dPlugin<TerrainMaterial>`, `HanabiPlugin` (bevy_hanabi GPU particles).
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
- **Events as Messages**: `DamageDealtEvent`, `AttackMissedEvent`, `AbilityCastEvent`, `AbilityLandedEvent`, `CastInterruptedEvent`, `ZoneTransitionEvent` use Bevy 0.18's `Message`/`MessageWriter`/`MessageReader`.
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
- [docs/vfx.md](docs/vfx.md) — Combat feedback: VFX particles, audio, micro-animations, and how to add new effects
- [docs/population.md](docs/population.md) — Population simulation: lifecycle, resources, faith, traits, happiness, migration
- [docs/narrative.md](docs/narrative.md) — 39 narrative functions (Propp/Shakespeare/Dostoevsky) for emergent storytelling
- [docs/faction-rework.md](docs/faction-rework.md) — Faction system rework: allegiance-based model, leader-driven formation, character-driven events (in progress)

## Known Issues

- **Tile occupancy**: entities can overlap on the same tile
- **Zone persistence (partial)**: tracks killed enemies + alive entity state; ground items/terrain mods not persisted
- **Diagonal speed**: ~41% faster diagonally (accepted)
- **No mob drops / loot tables**
- **Attribute bonuses from equipment not applied** (needs `EffectiveAttributes` refactor)
- **No equip/unequip from UI**
- **No save/load** (`StableId` infrastructure ready, serialization not built)
- **No camera follow**: camera is fully manual (WASD/edge scroll)
- **Terrain feature sprites**: forest has real sprites, other biomes still use placeholder colored squares
- **Terrain textures**: 5 of 9 still flat color (Sand, Snow, Swamp, Water, Mountain)
- **Terrain texture shimmer**: no mipmaps on terrain array texture — causes aliasing/shimmer when zoomed out. Fix: generate mip chain in `gen_terrain_array` + enable trilinear filtering (do once art/textures are finalized)

## CLI

```
cargo run                              # Normal
cargo run -- --debug all               # All debug visualizations
cargo run -- --debug grid,pathing      # Specific flags
cargo run -- --debug perf              # FPS + entity count profiling
cargo run --release                    # Optimized build
cargo run --features dev               # Dynamic linking (faster dev compiles)
cargo run -- --biome forest            # Spawn in specific biome (grassland/forest/mountain/desert/tundra/swamp/coast/settlement)
cargo run --example gen_terrain_array  # Regenerate terrain texture array
cargo run --example gen_blend_texture  # Regenerate terrain blend weight texture
```

Party selection: F1-F4 select party members by spawn order (Warrior, Archer, Mage, Healer)
Debug toggles (Ctrl+F key): Ctrl+F1=grid, Ctrl+F2=pathing, Ctrl+F3=aggro, Ctrl+F4=AI state, Ctrl+F5=profiling, Ctrl+F6=obstacles
