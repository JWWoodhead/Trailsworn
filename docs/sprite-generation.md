# Sprite Generation System

Procedural sprite generation tool and pipeline for terrain features.

## Tool

`tools/sprite-generator.html` — open in a browser. Single-page app with all generators.

### Generator Tabs

| Tab | Canvas Size | Features Covered |
|-----|------------|-----------------|
| Rock / Boulder | 64x64 (1x1 tile) | Fieldstone, Standing Stone, Rock Spire, Rubble Pile, Sand-Worn Rock, Ice Chunk, Tidal Rock |
| Tree | 128x128 (2x2 tiles) | Oak Tree, Pine Tree, Lone Tree, Snow Pine, Frozen Dead Tree, Dead Tree Alpine, Swamp Tree, Hanging Moss |
| Bush / Shrub | 64x64 (1x1 tile) | Hedge Bush, Desert Scrub, Beach Grass, Reed Cluster |
| Ground Scatter | 64x64 (1x1 tile) | Tall Grass, Wildflowers |
| Cactus | 64x128 (1x2 tiles) | Cactus |
| Debris | 64x64 (1x1 tile) | Bleached Bones, Driftwood |

### Workflow

1. Select a generator tab
2. Pick a **mode** (biome/feature preset) from the dropdown
3. Tweak sliders, hit **Regenerate** until you like variants
4. Toggle **Terrain bg** to preview against actual terrain textures
5. Toggle **Outline** to add a 1px black edge (baked into export)
6. **Click** individual sprites to add them to the export queue
7. **Export Queue** downloads a single spritesheet PNG — one row per mode

### Preview Options

- **Tile guide** — dashed 64px grid from the anchor point, shows tile boundaries
- **Outline** — 1px black silhouette border, baked into exported sprites
- **Terrain bg** — tiles the selected terrain texture behind sprites (Forest/Grass/Dirt/Stone). Preview only, not exported.

### Canvas Sizes

All sprites use **tile-size multiples** (64px base):
- Ground features (rocks, bushes, grass, debris): **64x64** — sits within one tile
- Tall features (cacti, standing stones): **64x128** — occupies 1 wide, 2 tall
- Trees: **128x128** — canopy extends 2x2 tiles

Canvas sizes are defined per-generator in the `GENERATORS` registry (`canvasW`, `canvasH`).

## How Sprites Are Rendered In-Game

### Positioning

All feature sprites use `Anchor::BOTTOM_CENTER`. The sprite's bottom-center pixel is placed at the tile's world coordinate. This means:
- Content should have its "foot" at the **bottom-center** of the image
- The sprite extends **upward** from the tile position
- Trees grow up from their root tile, rocks sit on their tile

### No Scaling

Sprites render at **native PNG pixel size**. A 128x128 PNG appears as 128x128 pixels in-game (2x2 tiles at 64px/tile). There is no `custom_size` override — the image dimensions ARE the rendered dimensions.

The `scale` field in `FeatureDef` only affects **placeholder squares** (features with `sprite: None`). It's ignored for features with real sprites.

### Spritesheets

For features with multiple visual variants (e.g., 11 oak tree variants), sprites are packed into a **horizontal spritesheet** with 1px gaps between frames.

`FeatureDef` fields:
- `sprite_count: u32` — number of frames (1 = single image, >1 = spritesheet)
- `sprite_frame: [u32; 2]` — individual frame size `[width, height]` in pixels

At spawn time, the game creates a `TextureAtlasLayout` and picks a random frame index. Same sprite asset, different visual each time.

Example (oak trees):
```rust
FeatureDef {
    sprite: Some("zones/forest/trees/forest_oak_trees.png"),
    sprite_count: 11,
    sprite_frame: [128, 128],
    ...
}
```
Spritesheet: 1418x128 (11 frames * 128px + 10 * 1px padding).

### Drop Shadows

Every terrain feature gets a drop shadow — a small dark ellipse spawned just behind it (z - 0.0001). The shadow is offset slightly right and down to match the top-left light direction. Defined in `systems/zone.rs`.

### Y-Sorting

Features share the `WORLD_OBJECTS` render layer with characters. `y_sorted_z()` ensures lower-screen entities render in front of higher-screen ones, giving correct depth overlap.

## Color Palettes

Leaf/foliage palettes are intentionally **brighter and warmer** than the terrain textures to create visual separation. The terrain is dark desaturated olive; sprites pop by being more vivid yellow-green.

Each generator has per-biome palette presets:
- **Forest** — warm mid-green leaves, brown bark (derived from Forest.png but shifted brighter)
- **Grassland** — vivid green (shifted from Grass.png)
- **Tundra** — cool muted blue-green, frost overlays
- **Swamp** — murky yellow-green, dark trunks
- **Mountain** — no leaves (dead trees), weathered grey-brown bark
- **Desert** — dry olive scrub, warm sandstone rocks
- **Coast** — sandy green, wet grey rocks

## Key Files

| File | Purpose |
|------|---------|
| `tools/sprite-generator.html` | All generators, queue, export |
| `src/resources/feature_defs.rs` | Feature definitions: sprite path, count, frame size, biome weights |
| `src/systems/zone.rs` | Sprite spawning: anchor, atlas, shadow, y-sort |
| `src/resources/map.rs` | Tile size (64px), render layer constants, y_sorted_z() |
| `assets/textures/` | Terrain textures (Forest.png, Grass.png, Dirt.png, Stone.png) |
| `assets/zones/` | Exported spritesheets organized by biome |

## Adding a New Feature Type

1. Add a `FeatureId` constant in `feature_defs.rs`
2. Register the `FeatureDef` with biome weights, terrain weights, and sprite info
3. Generate sprites using the tool (or add a new generator mode if the shape is novel)
4. Export spritesheet, place in `assets/zones/<biome>/`
5. Set `sprite`, `sprite_count`, `sprite_frame` on the def
6. `rebuild_biome_tables()` is called at startup — no further wiring needed

## Potential Extensions

### More Biome Scatter

Features currently missing real sprites (using placeholder squares):
- **Grassland**: Fieldstone, Standing Stone, Hedge Bush, Lone Tree, Tall Grass, Wildflowers
- **Mountain**: Rock Spire, Rubble Pile, Dead Tree (Alpine)
- **Desert**: Cactus, Desert Scrub, Bleached Bones, Sand-Worn Rock
- **Tundra**: Snow Pine, Ice Chunk, Frozen Dead Tree
- **Swamp**: Swamp Tree, Reed Cluster, Hanging Moss
- **Coast**: Driftwood, Beach Grass, Tidal Rock

### Flowers and Ground Cover

The ground scatter generator supports wildflowers (grass blades + colored flower dots) but could be extended:
- **Flower clusters** — standalone flower patches without grass, just colored blooms
- **Mushrooms** — small cap-and-stem shapes, forest/swamp biomes
- **Fallen leaves** — scattered leaf shapes for autumn forest variants
- **Pebbles/gravel** — tiny rock scatter for paths and mountain edges
- **Puddles** — semi-transparent blue-grey ovals for swamp/rain

### Animated Sprites

Currently all features are static. Potential animation targets:
- **Grass/reeds swaying** — spritesheet with 3-4 sway frames, cycle with a timer system
- **Water shimmer** — for puddles or coast features
- **Torch/campfire** — for settlement POIs (not terrain features, but same pipeline)

### Seasonal Variants

The generator could output seasonal palette variants:
- **Spring** — brighter greens, flower scatter
- **Autumn** — orange/red/brown leaf ramps
- **Winter** — snow overlay pass, bare branches

Would require a season system in the game and per-season sprite paths in `FeatureDef`.

### Biome Transition Features

Features that blend between biomes at zone edges:
- Half-snowy trees at tundra/forest borders
- Dried-out bushes at desert/grassland borders
- Already partially supported by terrain_weights (features can spawn on multiple terrain types)

### Generator Improvements

- **Undo** — restore previous generation when tweaking goes wrong
- **Lock individual sprites** — regenerate the rest without losing a good one
- **Color picker** — override palette colors per-generation for fine-tuning
- **Import reference** — load an existing sprite as a background reference layer
- **Batch generate** — auto-generate N sprites and pick top candidates by diversity metric
