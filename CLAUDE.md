# Trailsworn — Architecture & Systems Documentation

## Overview

Trailsworn is a party-based fantasy RPG built with Bevy 0.18 (Rust). Rimworld-style tactical combat with Kenshi-style open-world exploration. MMO-paced combat with pause-and-command.

**Crate name:** `trailsworn`
**Repo:** https://github.com/JWWoodhead/Trailsworn

## Project Structure

```
src/
  main.rs              — App setup, resource init, system registration
  lib.rs               — Module declarations
  terrain.rs           — TerrainType enum (Grass, Dirt, Stone, Water, Forest, Mountain)
  generation.rs        — Legacy test map generator (replaced by worldgen)

  systems/             — All Bevy systems (ECS logic that runs each frame)
  resources/           — All Components, Resources, and data types
  pathfinding/         — A* and HPA* (pure algorithms, no Bevy)
  worldgen/            — World/zone/history generation (pure Rust, no Bevy)
```

### Architecture Rules (from memory/feedback_architecture.md)
- **No Bevy plugins** — everything registered directly in main.rs
- **Organize by ECS role**: `systems/` for systems, `resources/` for components/resources
- **Pure logic at top level**: terrain.rs, generation.rs, pathfinding/, worldgen/
- All keybindings centralized in `resources/input.rs`
- All colors centralized in `resources/theme.rs`

## System Execution Order

Systems are organized into `GameSet` groups that run in sequence, all gated by `run_if(in_state(GameState::Playing))`:

```
Input → Tick → Ai → Combat → Movement → Ui → Render
```

### Input (GameSet::Input)
- `input::process_input` — reads raw keyboard/mouse, populates `ActionState` (MUST run first)
- `game_time::game_speed_input` — pause (Space), speed (1/2/3)
- `camera::camera_pan` — WASD/arrows + edge scroll
- `camera::camera_zoom` — scroll wheel (reads raw events, not action-mapped)
- `selection::selection_input` — left-click select, drag-box multi-select
- `selection::right_click_command` — sets `MovementIntent` (move) or `PlayerCommand` + `MovementIntent` (attack)

### Tick (GameSet::Tick)
- `game_time::advance_game_time` — accumulates real time into simulation ticks (60Hz fixed timestep)

### AI (GameSet::Ai)
- `ai::ai_decision` — target selection, intent setting. **Skips PlayerControlled entities.** Uses `aggro_range` from `CombatBehavior` to limit detection distance.
- `ai::resolve_movement_intent` — converts `MovementIntent` → A* pathfinding → `MovePath`. Handles ALL entities. AI uses `RepathTimer` (every 30 ticks). Player entities bypass timer but only repath when destination changes.

### Combat (GameSet::Combat)
- `combat::tick_weapon_cooldowns` — decrements weapon cooldown per tick
- `combat::auto_attack` — entities with `AiState::Engaging` attack their target when in range and weapon ready. Fires `DamageDealtEvent` / `AttackMissedEvent`.
- `combat::tick_status_effects` — decrements status effect durations
- `combat::cleanup_dead` — despawns entities whose vital body parts are destroyed
- `ai::cleanup_commands` — removes completed/invalid `PlayerCommand`, clears stale `MovementIntent`

### Movement (GameSet::Movement)
- `movement::movement` — advances `MovePath.progress` each tick. Updates `GridPosition` when arriving at next tile. Swaps in `PendingPath` at tile boundaries (AI only). Applies ease-in/ease-out speed multiplier over the whole path (first 1.5 tiles accelerate, last 1.5 tiles decelerate).
- `zone::detect_zone_edge` — fires `ZoneTransitionEvent` when player reaches map edge
- `zone::handle_zone_transition` — despawns zone entities, generates new zone, respawns enemies, repositions player

### UI (GameSet::Ui)
- `health_bars::spawn_health_bars` — attaches health bar sprites to entities with `Body`. Adds `HealthBarBackground` marker to the parent entity to prevent re-spawning.
- `health_bars::update_health_bars` — scales/colors bars based on HP fraction (gold→red from theme)
- `health_bars::cleanup_orphaned_health_bars` — removes bars for dead entities
- `floating_text::spawn_damage_numbers` — reads `DamageDealtEvent`/`AttackMissedEvent`, spawns Text2d
- `floating_text::animate_floating_text` — drifts text up, fades alpha, despawns on expiry
- `hover_info::update_hover_tooltip` — shows entity stats when mouse hovers over them
- `selection::update_selection_visuals` — spawns/despawns gold ring sprites on selected entities
- `selection::draw_drag_box` — draws selection rectangle with gizmos
- `hud::update_speed_indicator` — shows "PAUSED" or "1x/2x/3x" top-right
- `hud::combat_log_damage` — appends combat events to bottom-left panel (capped at 50 entries)

### Render (GameSet::Render)
- `movement::sync_transforms` — sets entity `Transform` from `GridPosition` + `MovePath.progress` + `PathOffset`
- `rendering::sync_tilemap` — updates tile texture indices when `TileWorld` resource changes (zone transitions)

### Always-running (not state-gated)
- `identity::register_stable_ids` — indexes new `StableId` components
- `identity::cleanup_stable_ids` — removes despawned entities from registry

### Debug (only when `--debug` is passed)
- `debug::debug_key_toggles` — F1-F5 toggle individual debug visualizations
- `debug::draw_grid` — tile grid lines
- `debug::draw_pathing` — blue lines showing entity movement paths
- `debug::draw_aggro_radius` — red circles showing detection range
- `debug::draw_ai_state` — colored dots showing AI state (idle/engaging/fleeing)
- `profiling::frame_profiler` — FPS and frame time (F5 to toggle)
- `profiling::entity_counter` — total entity count breakdown

## Movement Pipeline (Critical — Complex Area)

### How movement works:
1. Something sets `MovementIntent` on an entity (AI decision or player right-click)
2. `resolve_movement_intent` converts intent → A* pathfinding → `MovePath`
3. `movement` system advances `progress` 0→1 for each tile-to-tile segment
4. When `progress >= 1.0`: updates `GridPosition`, calls `advance()` (resets progress, increments waypoint index)
5. `sync_transforms` lerps visual position between `GridPosition` and `next_tile()` using `progress`

### Player vs AI movement:
- **AI:** `ai_decision` sets `MovementIntent`. `resolve_movement_intent` uses `RepathTimer` (30 ticks) to throttle. Mid-movement repathing creates `PendingPath` which swaps in at tile boundary.
- **Player:** `right_click_command` sets `MovementIntent` directly. `ai_decision` skips `PlayerControlled` entities. `resolve_movement_intent` bypasses timer but only repaths when destination changes. Mid-movement repathing pathfinds from the NEXT tile (where entity is heading), prepends `GridPosition` to the path, and preserves progress — so the entity smoothly finishes its current step then follows the new path. No `PendingPath` for players.

### PathOffset:
Each entity has a random ±20% tile offset (`PathOffset` component) applied in `sync_transforms`. Makes movement look less robotic — entities don't all walk through exact tile centers.

### Ease-in/ease-out:
`MovePath.ease_speed_multiplier()` returns a speed factor based on absolute distance from path start/end (not percentage). First 1.5 tiles: speed ramps 0.5→1.0. Last 1.5 tiles: speed ramps 1.0→0.5. Middle: full speed. Applied in the `movement` system as a multiplier on `progress_per_tick`.

### Known issue — diagonal speed:
Diagonal movement is ~41% faster visually because progress 0→1 takes the same time regardless of direction, but diagonal tiles are √2 further apart. Accepted for now — fixing it caused speed oscillation on mixed cardinal/diagonal paths.

## Combat System

### Hit resolution chain:
1. `accuracy_check(accuracy, dodge, roll)` — hit chance clamped 5%-95%
2. `select_body_part(template, coverage_roll)` — weighted by body part coverage
3. `armor.reduce_damage(part_index, damage_type, raw_damage)` — per-part armor resistances
4. `body.damage_part(index, damage, template)` — reduces HP, cascades destruction to children

### Body part system:
- Tree structure: Head → (Brain, Eyes, Jaw), Torso → (Heart, Lungs, Arms → Hands, etc.)
- Each part has: max_hp, coverage weight, vital flag, capabilities (Sight, Movement, etc.)
- Destroying a part destroys all children
- Destroying a vital part (Brain, Heart) kills the entity
- `BodyTemplate` loaded from data (currently `humanoid_template()`)
- `Body` component stores per-part runtime HP state

### Damage types (10):
- Physical: Slashing, Piercing, Blunt
- Magical: Fire, Frost, Storm, Arcane, Holy, Shadow, Nature
- `Resistances` uses `HashMap<DamageType, f32>` — adding new types doesn't require struct changes

### Magic schools (10):
- Elemental: Fire, Frost, Storm
- Divine: Holy, Shadow
- Arcane: Arcane, Enchantment
- Primal: Nature, Blood (forbidden)
- Death: Necromancy (forbidden)
- Schools define WHAT magic does, not HOW it's practiced

### Status effects:
- Duration (ticks), stacking with max stacks, tick effects (DoT/HoT)
- CC flags: Stunned, Rooted, Silenced, Feared, Sleeping
- Stat modifiers: MoveSpeedMul, AttackSpeedMul, AttributeFlat
- `ActiveStatusEffects` component tracks all active effects per entity

### Abilities (data structures exist, NOT yet wired to casting systems):
- `AbilityDef`: cast time, cooldown, mana/stamina cost, range, target type (Single/Circle/Cone/Line), effects chain
- `AbilitySlots`: per-entity known abilities with cooldown state
- `Mana` + `Stamina`: separate resource pools
- `CastingState`: tracks active casting (interruptible flag)

### Threat:
- `ThreatTable` per entity — tracks threat from each attacker
- Damage generates threat
- AI uses highest-threat target when available

## AI System

### CombatBehavior component:
- `role`: Tank, MeleeDps, RangedDps, Healer, Caster
- `aggro_range`: detection distance in tiles (25 for melee, 30 for ranged)
- `attack_range`: how close to get for attacking (1.5 for melee)
- `flee_hp_threshold`: flee when HP fraction drops below this
- `auto_use_abilities`: true for enemies, false for party members
- `ability_priorities`: ordered list of abilities with conditions

### AI decision flow (ai_decision system):
1. Check CC — incapacitated entities can't act
2. Check PartyMode — Passive entities don't engage
3. Check flee threshold
4. Select target: threat table → nearest hostile within `aggro_range`
5. Set `AiState::Engaging` + `MovementIntent::MoveToEntity`

### PlayerCommand:
- `MoveTo`, `Attack`, `HoldPosition`, `CastAbility`
- While present, `ai_decision` skips the entity
- `cleanup_commands` removes when completed or target dies

### MovementIntent:
- `None`, `MoveToEntity`, `MoveToPosition`, `FleeFrom`, `FollowEntity`
- Set by AI (`ai_decision`) or player (`right_click_command`)
- Consumed by `resolve_movement_intent` which produces `MovePath`

## World Generation (pure Rust, no Bevy)

### World map (`worldgen/world_map.rs`):
- 5x5 grid of zones, each 250x250 tiles
- Zone types: Grassland, Forest, Mountain, Settlement
- Caves can spawn in any non-settlement zone (30% chance)
- Settlement placed near center, player spawns adjacent

### Zone generation (`worldgen/zone.rs`):
- Biome-appropriate terrain: grassland (grass + patches), forest (trees + clearings), mountain (stone + valleys), settlement (dirt roads + stone buildings)
- Organic shapes via jittered-radius patches
- POIs: cave entrances, enemy camps (1-3 per zone, 2-5 enemies), wildlife spawns
- Deterministic from seed

### Cave generation (`worldgen/cave.rs`):
- Cellular automata: 45% random fill → 5 iterations of 4-5 smoothing rule
- Produces natural-looking cavern systems with corridors
- Entrance area cleared, enemy groups placed in open chambers

### Zone transitions (`systems/zone.rs`):
- `detect_zone_edge`: fires event when player reaches map boundary
- `handle_zone_transition`: despawns `ZoneEntity` marked entities, generates new zone, replaces `TileWorld` resource, spawns new enemies, repositions player at opposite edge
- `rendering::sync_tilemap`: detects `TileWorld` change and updates all 62,500 tile texture indices

### History generation (`worldgen/history/`):
- 100-year simulation producing factions, characters, settlements, events, artifacts, cultures
- **State-driven**: `WorldState` tracks pairwise faction sentiment (`RelationMatrix`), active wars/alliances/treaties
- **Faction gauges**: military_strength, wealth, stability (1-100). Wars drain, treaties add, settlements produce income.
- **Prerequisite-based events**: wars require hostility (sentiment < -20), alliances require friendship (> 30), etc.
- **Character system**: persistent characters with 27 traits across 5 personality axes, race-weighted (Orcs lean Warlike, Elves lean Wise). Characters have roles (Leader, General, Hero, Scholar, Villain), ambitions, epithets ("the Bold", "Oathbreaker").
- **Character-driven probability**: Warlike leaders increase war chance (+20%), Peaceful leaders decrease it (-25%), Treacherous characters enable betrayal events.
- **Race lifespans**: Orc 40-60yr (fast leader churn), Elf 500-1000yr (ancient rulers), Human 60-80, Dwarf 150-250, Goblin 30-50.
- **Cultural accumulation**: faction history produces cultural values (MilitaryProwess, Commerce, Scholarship) and taboos (Treachery, War, Outsiders) based on event patterns.
- **Artifacts**: persistent named items (The Eternal Gauntlets of Whispers) held by characters, discovered through events.
- **Simulation loop per year**: Aging/Death → Faction Upkeep → Settlement Upkeep → New Characters → Friction → Event Evaluation → Sentiment Drift

### Population tables (`worldgen/population_table.rs`):
- Weighted random selection tool (Caves of Qud pattern)
- `PickOne`: weighted single selection
- `PickEach`: independent probability per entry
- `PickN`: pick N unique entries without replacement

### Name generation (`worldgen/names.rs`):
- Race-specific character names (Human, Dwarf, Elf, Orc, Goblin)
- Settlement names (prefix+root: "Ironhold", "Deepcrossing")
- Faction names (8 types × pattern templates)
- Region names

## Pathfinding (`pathfinding/`)

Ported from Metropolis project. Pure algorithms, no Bevy.

### A* (`astar.rs`):
- 8-directional with octile heuristic
- No corner-cutting (diagonals blocked if either adjacent cardinal is impassable)
- Per-tile walk cost (0.0 = impassable, >0 = multiplier)
- Expansion limit (5000 default)
- Bounded variant for HPA* intra-cluster paths

### HPA* (`hpa.rs`):
- Hierarchical Pathfinding A* with configurable cluster size (default 10)
- Builder pattern: `HpaGraphBuilder::new(&costs, w, h).cluster_size(10).build()`
- Three-phase query: entrance lookup → Dijkstra on abstract graph → A* refinement
- Multi-source BFS for per-tile nearest entrance precomputation
- NOT currently used in gameplay (A* with 5000 expansion limit is sufficient for 250x250)

## Input System (`resources/input.rs`)

All keybindings centralized. Systems read `ActionState`, never raw `ButtonInput<KeyCode>`.

- `InputMap`: maps `InputBinding` (key or mouse button) → `Action`
- `ActionState`: populated each frame by `process_input` system
- `Action` enum: CameraPan(Up/Down/Left/Right), Pause, Speed(1/2/3), Select, Command, Debug(Grid/Pathing/Aggro/AiState/Profiling)
- Scroll wheel for zoom reads raw `MessageReader<MouseWheel>` (not action-mapped)
- To rebind: `input_map.bind(InputBinding::Key(KeyCode::KeyQ), Action::Pause)`

## Best Practices Enforced

- **GameState**: Loading → Playing. Systems run_if(in_state(Playing)).
- **SystemSets**: GameSet enum for coarse ordering. Individual systems use `.in_set()`.
- **DespawnOnExit(GameState::Playing)**: on all spawned gameplay entities.
- **StableId**: on all persistent entities. `StableIdRegistry` for lookups. Ready for save/load.
- **Name component**: Bevy's built-in `Name` alongside `EntityName` on all characters.
- **Events as Messages**: `DamageDealtEvent`, `AttackMissedEvent`, `ZoneTransitionEvent` use Bevy 0.18's `Message`/`MessageWriter`/`MessageReader`.

## Design System ("The Gritty Chronicle")

Colors in `resources/theme.rs`:
- Surface: `#131313` (dark iron)
- Text: `#F5F5DC` (parchment)
- Primary: `#f2ca50` → `#d4af37` (gold)
- Secondary: `#ffb4a8` / `#920703` (blood)
- 0px border radius everywhere
- No 1px borders — use tonal background shifts
- Shadows tinted warm, not pure grey

## Tests

153 tests covering:
- Pathfinding (A*, HPA*, bounded A*)
- Body parts, stats, leveling
- Combat resolution, damage, armor
- Abilities, mana/stamina
- Status effects, CC flags
- Factions, threat tables
- Game time, movement
- World map, zone generation, cave generation
- Name generation, population tables
- History generation (factions, characters, events, cultures, artifacts)

## Known Issues / Not Yet Implemented

- **Tile occupancy**: entities can overlap on the same tile. Needs pathfinding-level solution.
- **Ability casting**: data structures exist but no systems wire them to gameplay.
- **Zone persistence**: killed enemies respawn on re-entry (zones regenerate from seed).
- **History → gameplay integration**: generated history not yet connected to zone generation or NPC dialogue.
- **Diagonal speed**: entities move ~41% faster diagonally (accepted for now).
- **Cover system**: planned but not implemented.
- **Items/equipment/loot**: not implemented.
- **Save/load**: `StableId` infrastructure ready but serialization not built.

## CLI

```
cargo run                           # Normal
cargo run -- --debug all            # All debug visualizations
cargo run -- --debug grid,pathing   # Specific flags
cargo run -- --debug perf           # FPS + entity count profiling
cargo run --release                 # Optimized build
cargo run --features dev            # Dynamic linking (faster dev compiles)
```

Runtime debug toggles: F1=grid, F2=pathing, F3=aggro, F4=AI state, F5=profiling
