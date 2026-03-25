# Terrain Rendering

## Architecture
Uses a **custom WGSL fragment shader** via `bevy_ecs_tilemap`'s `MaterialTilemap` trait. Single tilemap with `MaterialTilemapBundle<TerrainMaterial>` replaces the old base+overlay two-layer approach.

## How It Works
1. **Terrain map texture** (`Rgba8Uint`, 250x250): R=terrain type index, G/B=random UV offset per tile. Created at startup, updated on zone transitions.
2. **Terrain texture array** (`terrain_array.png`): 6-layer 2D array texture (512x512 per layer), one layer per terrain type. Loaded with `Repeat` address mode for seamless tiling.
3. **Fragment shader** (`assets/terrain_shader.wgsl`): reads terrain type from the map texture, samples the terrain texture at **world-space UVs** (seamless tiling, no visible per-tile repetition), checks all 8 neighbors for higher-priority terrain, and blends using cosine edge/corner gradients.

## World-space UV Tiling
Instead of each tile showing the same 64px crop, the texture tiles seamlessly across the entire map using world coordinates: `world_uv = (world_tile + local_uv + random_offset) / texture_scale`. The per-tile random UV offset (stored in terrain map G/B channels) breaks visible repetition patterns.

## Terrain Blend Priorities
Higher priority bleeds INTO lower priority neighbors:

| Priority | Terrain  |
|----------|----------|
| 0        | Grass    |
| 1        | Dirt     |
| 2        | Stone    |
| 3        | Forest   |
| 4        | Water    |
| 5        | Mountain |

## Blend Computation (in shader)
For each fragment, the shader checks 8 neighbors in the terrain map texture. For the highest-priority neighbor terrain, it computes an alpha gradient (cosine falloff for edges, radial for corners) and blends that terrain's texture on top. Corner blends are suppressed when adjacent cardinal edges already cover them (Wang tiling constraint).

## Key Shader UV Convention
`in.uv.zw` from bevy_ecs_tilemap: `z` = 0 at left, 1 at right. `w` = 0 at **north/top**, 1 at **south/bottom**. (Verified from `tilemap_vertex.wgsl`: bot_left vertex gets w=1, top_left gets w=0.)

## Y-sorted Depth Ordering
`sync_transforms` computes z from y-position: `z = ENTITIES + (1.0 - world_y / map_height_px) * 0.999`. Entities lower on screen (south) render in front of entities higher on screen (north). All entities stay within 3.0-3.999 z range.

## TerrainMaterial (`resources/terrain_material.rs`)
- `terrain_textures: Handle<Image>` — 2D array texture (6 layers)
- `terrain_map: Handle<Image>` — Rgba8Uint per-tile data
- `params: TerrainParams` — texture_scale, blend_depth, corner_radius, map_width

## Asset Generation
- `cargo run --example gen_terrain_array` — generates `assets/terrain_array.png` (stacked 512x3072, 6 layers). Loads source textures from `assets/textures/` (Grass.png, Dirt.png, Forest.png) with colored fallbacks for missing types.
- Source textures are seamless PBR Color maps (e.g., from AmbientCG). Only the `_Color.png` diffuse map is used.

## Render Layers (`resources/map.rs`)
```
TERRAIN: 0.0          — tilemap
TERRAIN_OVERLAY: 0.5  — (reserved, currently unused after shader migration)
TERRAIN_FEATURES: 1.0 — flat ground decorations
FLOOR_ITEMS: 2.0      — dropped items
ENTITIES: 3.0-3.999   — y-sorted entities and terrain features
PROJECTILES: 4.0
UI_OVERLAY: 5.0
```

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
- Three-phase query: entrance lookup -> Dijkstra on abstract graph -> A* refinement
- Multi-source BFS for per-tile nearest entrance precomputation
- NOT currently used in gameplay (A* with 5000 expansion limit is sufficient for 250x250)
