# Item & Affix System

## Item Definitions (`resources/items.rs`, `resources/item_defs.rs`)
- `ItemDef` — static template: id, name, description, kind, rarity, weight, max_stack, icon, properties, base_tier, item_level_req, weapon_class, armor_class
- `ItemKind`: Weapon, Armor, Consumable, Material, Quest
- `EquipSlot`: Head, Chest, Hands, Legs, Feet, MainHand, OffHand
- `BaseTier`: Crude (ilvl 1+), Tempered (ilvl 15+), Runic (ilvl 30+) — craftsmanship quality
- `WeaponClass`: Sword, Mace, Dagger, Bow, Staff (5 classes x 3 tiers = 15 weapons)
- `ArmorClass`: Light, Medium, Heavy, Shield (6 slots x 3 tiers = 18 armor pieces)
- `ItemRegistry` — HashMap storage with `all()` iterator

## Rarity Model (D2-inspired)
| Rarity | Affixes | Max Prefix | Max Suffix |
|--------|---------|------------|------------|
| Normal | 0 | 0 | 0 |
| Magic | 1-2 | 1 | 1 |
| Rare | 3-6 | 3 | 3 |
| Unique | fixed (future) | — | — |

## Affix System (`resources/affixes.rs`, `resources/affix_defs.rs`)
- `AffixDef` — id, name, slot_type (Prefix/Suffix), allowed_kinds, tiered values
- `AffixTier` — min_item_level gate + effect value + display label
- `AffixEffect` enum: FlatDamage, PercentDamage, Resistance, Attribute, AttackSpeed, MaxMana, MaxStamina
- `RolledAffix` — specific affix instance on a rolled item
- `AffixRegistry` — with `candidates(kind, slot_type, ilvl)` filtering
- 20 starter affixes: 2 weapon prefixes, 2 armor prefixes, 1 weapon suffix, 10 resistance suffixes (all damage types), 5 attribute suffixes

## Item Instances (`resources/items.rs`)
- `ItemInstance` — rolled item: base_item_id + rarity + item_level + prefixes + suffixes
- `ItemInstanceId(u64)` — unique per rolled item
- `ItemInstanceRegistry` Resource — HashMap storage + id counter
- `display_name()` — "Prefix BaseType Suffix" (e.g., "Keen Oathbrand of the Colossus")

## Generation Pipeline (`resources/item_gen.rs`)
- `roll_rarity(item_level, rng)` — ilvl-scaled: 88% Normal at ilvl 1, 50% Normal at ilvl 40+
- `pick_base_type(item_level, kind, registry, rng)` — weighted toward tier-appropriate items
- `roll_affixes(rarity, item_level, kind, affix_registry, rng)` — respects prefix/suffix caps, no duplicate affixes, falls back to other slot type
- `generate_item(params, registries, rng)` -> `ItemInstanceId`

## Equipment Integration
- **Inventory** holds both `Vec<ItemStack>` (stackables) and `Vec<ItemInstanceId>` (equipment). Shared capacity pool.
- **Equipment** stores `HashMap<EquipSlot, ItemInstanceId>` — references into `ItemInstanceRegistry`.
- **`sync_equipment` system** (GameSet::Combat): Derives `EquippedWeapon`/`EquippedArmor` from equipped `ItemInstance` data. Bakes weapon affixes (FlatDamage, PercentDamage, AttackSpeed) into `WeaponDef`. Builds `ArmorPiece` entries with base + affix resistances. Computes `EquipmentBonuses` for attribute/resource bonuses.
- **`EquipmentBonuses` component**: Aggregates all affix effects. Weapon bonuses baked into `EquippedWeapon`, resource bonuses applied to `Mana.max`/`Stamina.max`. Attribute bonuses accumulated but not yet applied to combat.
- **Spawning** creates `ItemInstance` (Normal rarity) from `ItemRegistry` for player (`ITEM_CHIPPED_BLADE`) and enemies.
- **UI** shows rolled item names, rarity colors from instance.

## Not Yet Implemented
- Mob drops / loot tables
- Attribute bonuses applied to combat (needs `EffectiveAttributes` refactor)
- Equip/unequip from UI (drag-and-drop or click-to-equip)
- Inventory detail panel hover (affix details)
