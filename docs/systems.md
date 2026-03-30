# System Execution Order

Systems are organized into `GameSet` groups that run in sequence, all gated by `run_if(in_state(GameState::Playing))`:

```
Input → Tick → Ai → Combat → Movement → Ui → Render
```

## Input (GameSet::Input)
- `input::process_input` — reads raw keyboard/mouse, populates `ActionState` (MUST run first)
- `game_time::game_speed_input` — pause (Space), speed (1/2/3)
- `camera::camera_pan` — WASD/arrows + edge scroll
- `camera::camera_zoom` — scroll wheel (reads raw events, not action-mapped)
- `selection::update_hovered_target` — reads Bevy's `HoverMap` (bevy_picking) to find the topmost `Pickable` entity under the cursor, stores in `HoveredTarget` resource
- `selection::selection_input` — left-click select (uses picking for single-click, drag-box for multi-select), AND targeting mode resolution. Guards against UI click-through.
- `selection::right_click_command` — creates move or attack `Task` on selected entities. Uses `HoveredTarget` for sprite-based attack targeting. Guards against UI click-through.
- `selection::ability_input` — ability hotkeys (Q/E/R/T/F/G), creates cast `Task` or enters targeting mode
- `selection::party_hotkey_select` — F1-F4 selects individual party members by spawn order
- `ui_panel::toggle_ui_panel` — `C` opens Character tab, `I` opens Inventory tab. Same key closes, other key switches.

## Tick (GameSet::Tick)
- `game_time::advance_game_time` — accumulates real time into simulation ticks (60Hz fixed timestep)

## AI / Task Execution (GameSet::Ai)
- `task::advance_eval_timers` — decrements evaluator cooldowns on `AiBrain`, clears stale proposals each frame
- `task::flee` — proposes `FleeFrom`/`Wait` when HP below threshold (priority 90)
- `task::use_ability` — proposes `CastAbility` for first valid ability matching conditions (priority 70)
- `task::engage_combat` — proposes `EngageTarget` for nearest hostile within aggro range (priority 60)
- `task::defend_self` — proposes `EngageTarget` only when attacked (threat table non-empty, priority 60)
- `task::follow_leader` — proposes `FollowEntity` for configured leader (priority 20)
- `task::assign_task` — picks highest-priority proposal from `AiBrain.proposals`, assigns as `CurrentTask` (respects player task protection, interruptibility, CC)
- `task::execute_actions` — advances the current `Action` in each entity's `CurrentTask`. Handles completion/failure detection, cast initiation (inserts `CastingState`), and action sequencing. Manages `Engaging` marker component. Runs for ALL entities. **Not tick-gated** — runs even while paused so orders can be given during pause.
- `movement::resolve_movement` — reads the current action from `CurrentTask`, extracts a movement goal, and runs A* pathfinding -> `MovePath`. Handles repath throttling (`RepathTimer`, 30 ticks for AI), mid-movement repathing (`PendingPath` for AI, progress-preserving prepend for players). **Not tick-gated** — pathfinding computes during pause so move previews show immediately.

## Combat (GameSet::Combat)
- `equipment::sync_equipment` — derives `EquippedWeapon`/`EquippedArmor`/`EquipmentBonuses` from `Equipment` component's `ItemInstanceId` references. Bakes weapon affixes into `WeaponDef`, builds armor pieces, syncs `Mana.max`/`Stamina.max` from bonuses. Runs on `Changed<Equipment>`.
- `combat::tick_weapon_cooldowns` — decrements weapon cooldown per tick
- `combat::auto_attack` — entities with an `Engaging` marker attack their target when in range and weapon ready. Ranged weapons require line of sight (Bresenham check via `has_line_of_sight`). Fires `DamageDealtEvent` / `AttackMissedEvent`.
- `casting::tick_ability_cooldowns` — decrements per-slot ability cooldowns
- `casting::regenerate_resources` — mana/stamina regen per tick
- `casting::begin_cast` — processes newly-added `CastingState`: spends resources, starts cooldowns, resolves instant casts. Fires `AbilityLandedEvent` for VFX.
- `casting::tick_casting` — counts down cast timers, resolves effects on completion. Fires `AbilityLandedEvent` for VFX.
- `casting::interrupt_casting` — removes `CastingState` if caster takes damage and ability is interruptible
- `combat::tick_status_effects` — decrements status effect durations
- `combat::cleanup_dead` — turns dead entities into corpses: inserts `Dead` marker, rotates sprite 90°, greys out, lowers z to floor layer. Removes `InCombat`/`Engaging`/`CurrentTask`/`CastingState`/`MovePath`/`HitFlash`. `Without<Dead>` guards on all targeting queries prevent interacting with corpses.

## Movement (GameSet::Movement)
- `movement::movement` — advances `MovePath.progress` each tick. Updates `GridPosition` when arriving at next tile. Swaps in `PendingPath` at tile boundaries (AI only). Applies ease-in/ease-out speed multiplier over the whole path (first 1.5 tiles accelerate, last 1.5 tiles decelerate).
- `zone::detect_zone_edge` — fires `ZoneTransitionEvent` when player reaches map edge
- `zone::handle_zone_transition` — snapshots alive zone entities to `ZoneStateCache`, despawns zone entities, generates new zone, respawns enemies (skipping dead, restoring alive state from snapshot), repositions player

## UI (GameSet::Ui)
- `health_bars::spawn_health_bars` — attaches health bar sprites to entities with `Body`. Adds `HealthBarBackground` marker to the parent entity to prevent re-spawning.
- `health_bars::update_health_bars` — scales/colors bars based on HP fraction (gold->red from theme)
- `health_bars::cleanup_orphaned_health_bars` — removes bars for dead entities
- `floating_text::spawn_damage_numbers` — reads `DamageDealtEvent`/`AttackMissedEvent`, spawns Text2d
- `floating_text::spawn_heal_numbers` — reads `HealEvent`, spawns green "+N" floating text
- `floating_text::animate_floating_text` — drifts text up, fades alpha, despawns on expiry
- `hover_info::update_hover_tooltip` — shows entity stats when mouse hovers (reads `HoveredTarget` from picking)
- `party_panel::sync_party_portraits` — spawns/removes portrait UI nodes for party members
- `party_panel::update_party_portraits` — updates HP/mana bars and selection highlight on portraits
- `party_panel::click_party_portrait` — clicking a portrait selects that party member
- `selection::update_selection_visuals` — cleans up legacy selection ring sprites on deselection
- `selection::draw_selection_indicators` — draws gold gizmo circles under selected entities
- `selection::draw_move_preview` — draws path lines and destination circles for selected moving entities
- `selection::draw_engage_lines` — draws pulsing red line from selected attackers to their targets
- `selection::draw_hover_highlight` — draws subtle white circle on entity under cursor (from picking)
- `selection::draw_drag_box` — draws selection rectangle with gizmos
- `hud::update_speed_indicator` — shows "PAUSED" or "1x/2x/3x" top-right
- `hud::combat_log_damage` — appends combat events to bottom-left panel (capped at 50 entries)
- `ui_panel::update_tab_visuals` — highlights active tab button, handles tab click switching
- `ui_panel::update_ui_panel_overlay` — shows "No character selected" overlay when no selection
- `character_sheet::update_character_sheet` — updates body/stats/resources on Character tab (only when active)
- `inventory::update_inventory_panel` — updates equipment/grid/weight on Inventory tab (only when active)
- `ability_bar::update_ability_bar` — hides bar when nothing selected OR multiple selected, shows slots when exactly one entity selected
- `vfx::spawn_combat_effects` — reads `DamageDealtEvent`/`AttackMissedEvent`: inserts `AttackLunge` on attacker, `HitFlash` on target, spawns per-hit particle impact, adds screen trauma, plays audio one-shots. For ranged attacks, spawns cosmetic `Projectile` sprite from attacker to target.
- `vfx::spawn_cast_effects` — reads `AbilityCastEvent`: plays cast audio (data-driven per ability via `cast_sfx`)
- `vfx::spawn_interrupt_effects` — reads `CastInterruptedEvent`: plays interrupt audio
- `vfx::spawn_ability_landed_effects` — reads `AbilityLandedEvent`: spawns big particle burst at ability impact position (AoE center), scaled by `AbilityDef.impact_vfx_scale`
- `vfx::spawn_heal_effects` — reads `HealEvent`: spawns heal particles (`VfxKind::ImpactHeal`) at target, plays `SfxKind::HealLand` audio
- `vfx::tick_attack_lunge` — advances `AttackLunge.progress`, removes when done
- `vfx::tick_hit_flash` — ticks `HitFlash.timer`, overrides sprite to white, restores on expiry
- `vfx::tick_projectiles` — advances cosmetic `Projectile` entities from start to end position, despawns on arrival
- `vfx::cleanup_despawn_timers` — ticks `DespawnTimer` on effect entities, despawns when expired

## Render (GameSet::Render)
- `movement::sync_transforms` — sets entity `Transform` from `GridPosition` + `MovePath.progress` + `PathOffset` + `AttackLunge` offset. Computes y-sorted z-depth via `render_layers::y_sorted_z()` so entities further north render behind entities further south.
- `movement::sync_facing_sprites` — updates sprite sheet atlas index when `FacingDirection` changes (entities with `DirectionalSprites`)
- `vfx::tick_screen_trauma` — decays `ScreenTrauma.trauma` exponentially, applies camera shake offset (runs after sync_transforms)
- `rendering::update_terrain_map` — updates the terrain map GPU texture (R=terrain type, G/B=random UV offset) when `TileWorld` changes (zone transitions)

## Always-running (not state-gated)
- `identity::register_stable_ids` — indexes new `StableId` components
- `identity::cleanup_stable_ids` — removes despawned entities from registry

## Debug (only when `--debug` is passed)
- `debug::debug_key_toggles` — F1-F6 toggle individual debug visualizations
- `debug::draw_grid` — tile grid lines
- `debug::draw_pathing` — blue lines showing entity movement paths
- `debug::draw_aggro_radius` — red circles showing detection range
- `debug::draw_ai_state` — colored dots showing current task action (idle/engaging/fleeing)
- `debug::draw_obstacles` — red outlines on impassable tiles (water, mountains, blocking features)
- `profiling::frame_profiler` — FPS and frame time (F5 to toggle)
- `profiling::entity_counter` — total entity count breakdown

## Task/Action Brain System

Inspired by Rimworld's Job/Toil system. Split into two components:
- **`CurrentTask`** (Component) — on ALL entities. Holds the active `Task` with its action sequence.
- **`AiBrain`** (Component) — on NPC entities ONLY. Holds evaluator list, cooldowns, and proposals vec.

### Core types (`resources/task.rs`):
- **`CurrentTask`** (Component): wraps the active `Task`. Method `should_replace()` determines if a proposal beats the current task (player tasks only replaced by player tasks, respects interruptibility).
- **`AiBrain`** (Component): holds `evaluators` list, per-category cooldowns (combat ~5 ticks, routine ~60 ticks), and `proposals` vec that evaluators push into each frame.
- **`Task`**: label, priority, source (Evaluator/Player), action sequence, current index, interruptible flag.
- **`Action`** (enum): `MoveToEntity`, `MoveToPosition`, `FleeFrom`, `FollowEntity`, `EngageTarget`, `Wait`, `CastAbility`.
- **`Engaging`** (marker Component): automatically added/removed by `execute_actions` when current action is `EngageTarget`.
- **`TaskEvaluator`** (enum): `EngageCombat`, `Flee`, `UseAbility`, `DefendSelf`, `FollowLeader`, `Idle`. Each is a separate system that pushes proposals into `AiBrain.proposals`.
- **`TaskSource`**: `Evaluator` (AI-generated) or `Player` (from input). Player tasks can only be replaced by other player tasks.

### How it flows:
1. **`advance_eval_timers`**: decrements evaluator cooldowns, clears stale proposals.
2. **Individual evaluator systems** (NPC-only): each evaluator (`flee`, `use_ability`, `engage_combat`, `defend_self`, `follow_leader`) runs on its own cooldown and pushes an `Option<Task>` proposal into `AiBrain.proposals`.
3. **`assign_task`**: picks the highest-priority proposal. If it beats the current task (via `CurrentTask::should_replace()`), it replaces it. Respects CC (incapacitated entities don't change tasks).
4. **`execute_actions`** (all entities): advances the current action. Checks completion (in range? arrived? cast finished? target dead?). Advances to next action on Done, clears task on Failed. Manages `Engaging` marker.
5. **`resolve_movement`** (all entities): reads the current action from `CurrentTask`, extracts a movement goal, and pathfinds. Movement actions produce pathfinding goals. Non-movement actions (`Wait`, `CastAbility`) clear paths.
6. **Existing systems do the rest**: `movement` follows `MovePath`, `auto_attack` reads the `Engaging` marker, casting pipeline processes `CastingState`.

### Player commands:
Player input (right-click, ability hotkeys) injects a `Task` directly into the entity's `CurrentTask` with `TaskSource::Player` and priority 100. No special component — player commands are just high-priority tasks.
- Right-click ground -> `Task { [MoveToPosition] }` (multiple selected entities spread into formation around target tile)
- Right-click enemy -> `Task { [EngageTarget] }`
- Ability hotkey -> `Task { [CastAbility] }` (or enters targeting mode)

### Evaluator loadouts:
- **AI enemies**: `[Flee, UseAbility, EngageCombat, Idle]` via `AiBrain::enemy()`
- **Player party** (4 members: Warrior, Archer, Mage, Healer): no `AiBrain` component — driven entirely by player commands. All `PlayerControlled`, no auto-abilities.

### Evaluator priority mapping:
| Evaluator | Priority | Trigger |
|-----------|----------|---------|
| Flee | 90 | HP below `flee_hp_threshold` |
| UseAbility | 70 | `auto_use_abilities` + valid ability + engaging target |
| EngageCombat | 60 | Hostile within `aggro_range` or threat table entry |
| DefendSelf | 60 | Threat table non-empty (counter-attack only) |
| FollowLeader | 20 | Always (when configured) |
| Idle | — | Returns None (fallback) |

### CombatBehavior component (`resources/combat_behavior.rs`):
- `role`: Tank, MeleeDps, RangedDps, Healer, Caster
- `aggro_range`: detection distance in tiles (25 for melee, 30 for ranged)
- `attack_range`: how close to get for attacking (1.5 for melee)
- `flee_hp_threshold`: flee when HP fraction drops below this (0.0 = never flee)
- `auto_use_abilities`: true for enemies, false for party members
- `ability_priorities`: ordered list of abilities with conditions (`UseCondition` enum)

## Movement Pipeline

### How movement works:
1. The current `Action` on the entity's `CurrentTask` determines the movement goal (via `resolve_movement`)
2. `resolve_movement` extracts a goal tile from the action, runs A* pathfinding -> `MovePath`
3. `movement` system advances `progress` 0->1 for each tile-to-tile segment
4. When `progress >= 1.0`: updates `GridPosition`, calls `advance()` (resets progress, increments waypoint index)
5. `sync_transforms` lerps visual position between `GridPosition` and `next_tile()` using `progress`

### Player vs AI movement:
- **AI:** evaluators set tasks with movement actions. `resolve_movement` uses `RepathTimer` (30 ticks) to throttle. Mid-movement repathing creates `PendingPath` which swaps in at tile boundary.
- **Player:** `right_click_command`/`ability_input` inject tasks directly. `resolve_movement` bypasses timer but only repaths when destination changes. Mid-movement repathing pathfinds from the NEXT tile (where entity is heading), prepends `GridPosition` to the path, and preserves progress — so the entity smoothly finishes its current step then follows the new path. No `PendingPath` for players.

### PathOffset:
Each entity has a random +/-20% tile offset (`PathOffset` component) applied in `sync_transforms`. Makes movement look less robotic — entities don't all walk through exact tile centers.

### Ease-in/ease-out:
`MovePath.ease_speed_multiplier()` returns a speed factor based on distance from path start/end. Uses `tiles_traveled` (cumulative across path swaps) for ease-in so AI entities don't stutter when repathing mid-movement. First 1.5 tiles: speed ramps 0.5->1.0. Last 1.5 tiles: speed ramps 1.0->0.5. Middle: full speed. Applied in the `movement` system as a multiplier on `progress_per_tick`.

### Known issue — diagonal speed:
Diagonal movement is ~41% faster visually because progress 0->1 takes the same time regardless of direction, but diagonal tiles are sqrt(2) further apart. Accepted for now — fixing it caused speed oscillation on mixed cardinal/diagonal paths.

## Input System (`resources/input.rs`)

All keybindings centralized. Systems read `ActionState`, never raw `ButtonInput<KeyCode>`.

- `InputMap`: maps `InputBinding` (key or mouse button) -> `Action`
- `ActionState`: populated each frame by `process_input` system
- `Action` enum: CameraPan(Up/Down/Left/Right), Pause, Speed(1/2/3), Select, Command, Cancel, AbilitySlot(1-6), ToggleCharacterSheet, ToggleInventory, Debug toggles
- `CursorPosition`: resource computed once per frame by `selection::update_cursor_position` (early Input set). Holds screen, world, and tile coordinates. All downstream systems read this instead of querying raw cursor position.
- Scroll wheel for zoom reads raw `MessageReader<MouseWheel>` (not action-mapped)
- To rebind: `input_map.bind(InputBinding::Key(KeyCode::KeyQ), Action::Pause)`

## UI Panel System (`systems/ui_panel.rs`)

Unified tabbed panel replacing separate character sheet and inventory windows. One panel, multiple tabs.

### Architecture:
- `UiPanelRoot` — single root node (720x540, centered, opaque dark iron bg, z-index 10)
- `ActiveUiTab` resource — `Option<UiTab>`: `None` = closed, `Some(Character|Inventory)` = open
- `UiTabButton` — clickable tab buttons with `Interaction` component
- `UiTabContent(UiTab)` — container per tab, toggled `Display::None/Flex` by active tab
- `UiNoSelectionOverlay` — shared overlay across all tabs

### Tab content (spawned by panel, updated by own systems):
- **Character tab** (`systems/character_sheet.rs`): body part HP bars, attributes, combat stats, mana/stamina, status effects. Uses `CsText`/`CsBar` enum markers.
- **Inventory tab** (`systems/inventory.rs`): equipment slots (7 slots), item grid (6x4 = 24 slots), detail panel. Uses `InvText`/`InvEquipSlotBg`/`InvGridSlotBg` markers.

### Toggle behavior:
- `C` -> open to Character tab / close if already on Character / switch if on another tab
- `I` -> same for Inventory tab
- Tab buttons clickable to switch while panel is open

### UI click-through prevention:
All interactive UI nodes (`UiPanelRoot`, `AbilitySlotUi`, `UiTabButton`) have `Interaction::default()`. `selection_input` and `right_click_command` query all `Interaction` components and skip world input if any are `Hovered` or `Pressed`. Future UI elements just need `Interaction` to automatically block world clicks.
