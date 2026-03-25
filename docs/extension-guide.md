# Extension Guide — Adding New Content

When adding new content, the Rust compiler enforces exhaustive matches on most enums — you'll get compile errors for unhandled variants. The cases below document where to make changes and highlight the few spots where silent bugs are possible.

## Adding a New Base Item
**Just `item_defs.rs`** — add a `pub const ITEM_*: ItemId` and register it in the appropriate `register_*` function. Items are automatically available for generation and inventory. No other files need changes.

## Adding a New Affix (existing AffixEffect variant)
**Just `affix_defs.rs`** — register a new `AffixDef` with a unique `AffixId`. Declare `allowed_kinds` and tiers. Everything else (generation, bonus computation, equipment sync) works automatically.

## Adding a New AffixEffect Variant
1. `resources/affixes.rs` — add variant to `AffixEffect` enum
2. `resources/equipment_bonuses.rs` — add field to `EquipmentBonuses` struct + match arm in `compute_bonuses()` (compiler enforces this)
3. `systems/equipment.rs` — if the effect modifies weapon/armor, apply it in `sync_equipment`
4. Add tests

## Adding a New Rarity
1. `resources/items.rs` — add variant + arms in `Rarity::text_color()`, `Rarity::bg_tint()`, `max_affixes_per_slot()`, `affix_count_range()` (compiler enforces all)
2. `resources/item_gen.rs` — update `roll_rarity()` probability table

## Adding a New DamageType
1. `resources/damage.rs` — add variant to enum + update `DamageType::ALL` array + update `is_physical()` if physical
2. `resources/affix_defs.rs` — **add resistance affix entry** in `register_resistance_suffixes()` (uses hardcoded list with labels — `DamageType::ALL` is available for iteration but labels are per-type)

## Adding a New EquipSlot
1. `resources/items.rs` — add variant to `EquipSlot` enum
2. `systems/inventory.rs` — add entry to `EQUIP_DISPLAY_SLOTS` array (controls UI display)
3. `systems/equipment.rs` — if it's a weapon slot, add to the MainHand/OffHand skip check in armor sync
4. `resources/item_defs.rs` — create armor items for the new slot

## Spawning an Entity with Equipment
Use the shared helpers in `spawning.rs`:
1. `create_item_instance(base_id, &mut instance_registry)` — creates Normal-rarity instance
2. `placeholder_weapon(base_id, &item_registry)` — extracts WeaponDef for initial EquippedWeapon
3. Insert `Equipment`, `EquippedWeapon`, `EquippedArmor`, `EquipmentBonuses` components
4. `sync_equipment` system handles the rest on next frame

## Adding a New Component
When adding a new component that holds game state, you MUST categorize it for save/load:

- **Must save**: position, HP, stats, equipment, inventory, faction, abilities, combat config. Add a field to `SavedEntity` (when built) and ensure the type can derive `Serialize`.
- **Derived/reconstructable**: components computed from saved state (e.g., `EquippedWeapon` from `Equipment` via `sync_equipment`, `Transform` from `GridPosition`). Skip in save, reconstruct on load.
- **Transient**: runtime-only state that resets naturally (e.g., `CurrentTask`, `MovePath`, `ThreatTable`, `CastingState`). Skip in save.
- **Rendering/UI**: engine state reconstructed from assets (e.g., `Sprite`, UI markers). Skip in save.

If the component holds mutable game state that the player would expect to persist, it **must be saved.** When in doubt, it's a save candidate.

Currently saved via zone persistence (`ZoneSnapshot`): GridPosition, Body (part HP/destroyed), Mana, Stamina, Equipment (ItemInstanceIds). Full entity serialization will be added when quests/NPC state require it.

## Component Save Categorization Reference

**Must save (18):** StableId, EntityName, GridPosition, Body, Attributes, CharacterLevel, Mana, Stamina, ActiveStatusEffects, Faction, Equipment, Inventory, AbilitySlots, CombatBehavior, PartyMode, MovementSpeed, FacingDirection, ThreatTable

**Must save (identity/markers):** PlayerControlled, ZoneEntity, ZoneSpawnIndex

**Derived (reconstruct on load):** EquippedWeapon, EquippedArmor, EquipmentBonuses, Transform

**Transient (skip):** CurrentTask, MovePath, PendingPath, RepathTimer, InCombat, Engaging, CastingState, PathOffset

**Rendering/UI (skip):** Sprite, Name, DespawnOnExit, Selected, all UI markers

## Key Design Principles
- **Registries are open**: ItemRegistry, AffixRegistry, ItemInstanceRegistry — add entries without changing code
- **Enums are closed**: AffixEffect, Rarity, DamageType, EquipSlot — adding variants requires updating match arms (compiler catches most, see above for manual spots)
- **Rarity colors are centralized**: `Rarity::text_color()` and `Rarity::bg_tint()` on the enum — single source of truth for all UI
