# Art Requirements

Tracking all visual assets needed. Organized by category with status.

Art style target: gritty, grounded fantasy. Think weathered, earthy tones. Design system colors in `resources/theme.rs`.

## Terrain Tile Textures (512x512, seamless tiling)

These are layers in the terrain texture array (`assets/terrain_array.png`). Each must tile seamlessly since the shader samples them with world-space UVs (repeating every 4 tiles). Place source PNGs in `assets/textures/<Name>.png` and re-run `cargo run --example gen_terrain_array`.

| Layer | Terrain  | Status | Fallback Color | Notes |
|-------|----------|--------|----------------|-------|
| 0     | Grass    | **HAS TEXTURE** | `[80,130,50]` | Real texture in place |
| 1     | Dirt     | **HAS TEXTURE** | `[120,85,55]` | Real texture in place |
| 2     | Sand     | FLAT COLOR | `[210,190,130]` | Needs: warm sandy texture, fine grain |
| 3     | Snow     | FLAT COLOR | `[230,235,240]` | Needs: packed snow with subtle drift patterns |
| 4     | Swamp    | FLAT COLOR | `[60,80,50]` | Needs: muddy waterlogged ground, dark greens/browns |
| 5     | Stone    | FLAT COLOR | `[140,140,135]` | Needs: rough natural stone, cracked/weathered |
| 6     | Forest   | **HAS TEXTURE** | `[40,85,30]` | Real texture in place (forest floor/undergrowth) |
| 7     | Water    | FLAT COLOR | `[40,80,140]` | Needs: dark water surface. Consider animated later. |
| 8     | Mountain | FLAT COLOR | `[90,85,80]` | Needs: dark craggy rock, rougher than Stone |

**6 terrain textures needed:** Sand, Snow, Swamp, Stone, Water, Mountain

## Terrain Feature Sprites (y-sorted entities)

Sparse entities spawned on the `TERRAIN_FEATURES` render layer (z=1.0). These need transparency and should be designed for y-sort depth (character walks behind the trunk, in front of the canopy base). Rough target: 200-800 features per 250x250 zone.

### Universal (any biome)
| Sprite | Variants | Size | Notes |
|--------|----------|------|-------|
| Boulder (small) | 2-3 | ~1x1 tile | Low rock, doesn't block LOS |
| Boulder (large) | 2-3 | ~2x2 tiles | Blocks LOS, impassable |
| Bush/shrub | 2-3 | ~1x1 tile | Decorative, walkable |

### Grassland
| Sprite | Variants | Size | Notes |
|--------|----------|------|-------|
| Lone tree | 2-3 | 1w x 2-3h tiles | Scattered shade trees, blocks LOS |
| Tall grass clump | 2-3 | ~1x1 tile | Decorative, walkable |
| Wildflower patch | 2-3 | ~1x1 tile | Decorative, walkable |

### Forest
| Sprite | Variants | Size | Notes |
|--------|----------|------|-------|
| Deciduous tree | 3-4 | 1w x 2-3h tiles | Dense placement, blocks LOS |
| Conifer/pine | 2-3 | 1w x 3h tiles | Taller, narrower silhouette |
| Fallen log | 2 | 2w x 1h tile | Impassable obstacle |
| Tree stump | 2 | ~1x1 tile | Walkable decoration |
| Mushroom cluster | 2 | ~1x1 tile | Decorative |

### Mountain
| Sprite | Variants | Size | Notes |
|--------|----------|------|-------|
| Rock spire | 2-3 | 1w x 2h tiles | Tall, blocks LOS |
| Rubble pile | 2-3 | ~1x1 tile | Slows movement |
| Dead tree (alpine) | 2 | 1w x 2h tiles | Gnarled, bare branches |

### Desert
| Sprite | Variants | Size | Notes |
|--------|----------|------|-------|
| Cactus | 2-3 | 1w x 1-2h tiles | Iconic desert feature |
| Desert scrub | 2-3 | ~1x1 tile | Low dry bush |
| Bleached bones | 2 | ~1x1 tile | Decorative, flavor |
| Sand-worn rock | 2 | ~1x1 tile | Rounded, eroded |

### Tundra
| Sprite | Variants | Size | Notes |
|--------|----------|------|-------|
| Snow-covered pine | 2-3 | 1w x 2-3h tiles | Sparse, icy |
| Ice chunk | 2-3 | ~1x1 tile | Translucent blue-white |
| Frozen dead tree | 2 | 1w x 2h tiles | Bare, frosted |

### Swamp
| Sprite | Variants | Size | Notes |
|--------|----------|------|-------|
| Swamp tree (cypress) | 2-3 | 1w x 2-3h tiles | Gnarled, moss-draped |
| Reed cluster | 2-3 | ~1x1 tile | Along water edges |
| Lily pads | 2 | ~1x1 tile | On water tiles only |
| Hanging moss | 2 | 1w x 2h tiles | Atmospheric |

### Coast
| Sprite | Variants | Size | Notes |
|--------|----------|------|-------|
| Driftwood | 2-3 | 1-2w x 1h tile | Along shoreline |
| Seashell cluster | 2 | ~1x1 tile | Decorative |
| Beach grass | 2-3 | ~1x1 tile | Transition to inland |
| Tidal rock | 2 | ~1x1 tile | Near waterline |

### Estimated totals: ~75-90 individual sprite images across all biomes

## POI Sprites (interactive entities)

These are placed at points of interest and have gameplay interaction.

| Sprite | Status | Notes |
|--------|--------|-------|
| Cave entrance | MISSING | Rocky opening, ~3x3 tiles, interactable |
| Campfire | MISSING | For enemy camps, warm glow |
| Tent/lean-to | MISSING | 2-3 variants, enemy camp structures |
| Treasure chest | MISSING | Lootable |
| Shrine/altar | MISSING | Future: tied to god domains |
| Signpost/marker | MISSING | Settlement/road markers |
| Mine entrance | MISSING | Variant of cave entrance for mountains |

## Character/Entity Sprites

| Sprite | Status | Current | Notes |
|--------|--------|---------|-------|
| Player character | PLACEHOLDER | White square (pawn.png), no tint | Needs real sprite |
| Melee bandit | PLACEHOLDER | pawn.png, red tint | Needs real sprite |
| Ranged bandit | PLACEHOLDER | pawn.png, green tint | Needs real sprite |
| Caster bandit | PLACEHOLDER | pawn.png, purple tint | Needs real sprite |
| Wildlife (wolf, boar, etc.) | MISSING | WildlifeSpawn uses enemy code | Needs real sprites + spawn code |

## Settlement Sprites

Not yet designed — settlements currently use terrain tiles only (stone blocks for buildings, dirt for roads). Future needs:

| Sprite | Notes |
|--------|-------|
| House/building wall tiles | For structured building outlines |
| Door | Interactable building entry |
| Market stall | Settlement commerce |
| Well/fountain | Town center feature |
| Wall/gate segments | Settlement perimeter |
| NPC sprites | Merchants, quest givers, townsfolk |

## Priority Order

1. **6 terrain textures** (Sand, Snow, Swamp, Stone, Water, Mountain) — biggest visual bang, everything stops looking flat
2. **Core feature sprites** (~30): trees (3-4 types), boulders, bushes — zones stop looking empty
3. **POI sprites** (~7): cave entrance, campfire, tent, chest — POIs become visible
4. **Character sprites** (~5): player + 3 enemy roles + 1 wildlife — entities stop being squares
5. **Biome-specific features** (~45): remaining per-biome decorations
6. **Settlement sprites** — deferred until settlement overhaul
