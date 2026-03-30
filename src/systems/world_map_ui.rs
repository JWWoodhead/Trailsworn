use bevy::prelude::*;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::image::{Image, ImageSampler};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::resources::input::{Action, ActionState};
use crate::resources::theme::Theme;
use crate::resources::world::CurrentZone;
use crate::worldgen::world_map::{SettlementSize, WorldCell, WorldMap, WorldPos};
use crate::worldgen::zone::ZoneType;

// ---------------------------------------------------------------------------
// Resources & Components
// ---------------------------------------------------------------------------

/// Tracks whether the world map overlay is visible.
#[derive(Resource, Default)]
pub struct WorldMapVisible(pub bool);

/// View state for zoom and pan.
#[derive(Resource)]
pub struct WorldMapViewState {
    pub zoom: f32,
    pub offset: Vec2, // offset in map-pixel space (0..256)
}

impl Default for WorldMapViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: Vec2::ZERO,
        }
    }
}

/// Marker for the root overlay node.
#[derive(Component)]
pub struct WorldMapOverlay;

/// Marker for the map image container (clipping parent).
#[derive(Component)]
pub struct WorldMapContainer;

/// Marker for the map image node.
#[derive(Component)]
pub struct WorldMapImage;

/// Marker for the player position indicator.
#[derive(Component)]
pub struct WorldMapPlayerMarker;

/// Marker for zone info text.
#[derive(Component)]
pub struct WorldMapInfoText;

/// Marker for settlement label text nodes.
#[derive(Component)]
pub struct WorldMapSettlementLabel;

// ---------------------------------------------------------------------------
// Colors (matching the gen_world_map example)
// ---------------------------------------------------------------------------

fn biome_color(zone_type: ZoneType) -> [u8; 4] {
    match zone_type {
        ZoneType::Ocean => [30, 60, 120, 255],
        ZoneType::Grassland => [80, 140, 50, 255],
        ZoneType::Forest => [30, 80, 25, 255],
        ZoneType::Mountain => [130, 125, 115, 255],
        ZoneType::Desert => [210, 190, 120, 255],
        ZoneType::Tundra => [200, 210, 220, 255],
        ZoneType::Swamp => [55, 75, 45, 255],
        ZoneType::Coast => [170, 180, 130, 255],
        ZoneType::Settlement => [220, 180, 50, 255],
    }
}

fn biome_label(zone_type: ZoneType) -> &'static str {
    match zone_type {
        ZoneType::Ocean => "Ocean",
        ZoneType::Grassland => "Grassland",
        ZoneType::Forest => "Forest",
        ZoneType::Mountain => "Mountains",
        ZoneType::Desert => "Desert",
        ZoneType::Tundra => "Tundra",
        ZoneType::Swamp => "Swamp",
        ZoneType::Coast => "Coast",
        ZoneType::Settlement => "Settlement",
    }
}

/// Format the info text for a world map cell.
fn cell_info_text(pos: WorldPos, cell: &WorldCell) -> String {
    let label = biome_label(cell.zone_type);

    let mut parts = vec![format!(
        "{} ({}, {}) | Elev: {:.0}% | Moist: {:.0}% | Temp: {:.0}%",
        label, pos.x, pos.y,
        cell.elevation * 100.0,
        cell.moisture * 100.0,
        cell.temperature * 100.0,
    )];

    if cell.river { parts.push("River".into()); }

    if cell.road {
        let class = match cell.road_class {
            2 => "Major Road",
            1 => "Road",
            _ => "Road",
        };
        parts.push(class.into());
    }

    if let Some(name) = &cell.settlement_name {
        let size_label = match cell.settlement_size {
            Some(SettlementSize::City) => "City",
            Some(SettlementSize::Town) => "Town",
            Some(SettlementSize::Village) => "Village",
            Some(SettlementSize::Hamlet) => "Hamlet",
            None => "Settlement",
        };
        parts.push(format!("{size_label}: {name}"));
    }

    parts.join(" | ")
}

const RIVER_COLOR: [u8; 4] = [40, 90, 170, 255];
const MAP_DISPLAY_SIZE: f32 = 768.0;

// ---------------------------------------------------------------------------
// Setup — create the map texture and UI overlay (hidden by default)
// ---------------------------------------------------------------------------

pub fn setup_world_map_ui(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    world_map: Res<WorldMap>,
    theme: Res<Theme>,
) {
    let map_image = generate_map_image(&world_map);
    let map_handle = images.add(map_image);

    // Collect settlement positions for labels (deduplicate multi-cell settlements)
    let mut seen_names = std::collections::HashSet::new();
    let settlements: Vec<(i32, i32, String)> = world_map
        .cells
        .iter()
        .enumerate()
        .filter_map(|(i, cell)| {
            let name = cell.settlement_name.as_ref()?;
            if !seen_names.insert(name.clone()) {
                return None; // already added this settlement
            }
            let x = (i as u32 % world_map.width) as i32;
            let y = (i as u32 / world_map.width) as i32;
            Some((x, y, name.clone()))
        })
        .collect();

    // Root overlay — covers entire screen, hidden by default
    commands
        .spawn((
            WorldMapOverlay,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Row,
                display: Display::None,
                column_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            GlobalZIndex(20),
        ))
        .with_children(|root| {
            // Left column: title + map + info
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|col| {
                // Title
                col.spawn((
                    Text::new("World Map"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(theme.primary),
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                ));

                // Map image container with clipping for zoom/pan
                col.spawn((
                    WorldMapContainer,
                    Node {
                        width: Val::Px(MAP_DISPLAY_SIZE),
                        height: Val::Px(MAP_DISPLAY_SIZE),
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .with_children(|container| {
                    // The map image (sized dynamically by zoom)
                    container.spawn((
                        WorldMapImage,
                        ImageNode::new(map_handle),
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(MAP_DISPLAY_SIZE),
                            height: Val::Px(MAP_DISPLAY_SIZE),
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            ..default()
                        },
                    ));

                    // Player position marker
                    container.spawn((
                        WorldMapPlayerMarker,
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(8.0),
                            height: Val::Px(8.0),
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(1.0, 0.2, 0.2)),
                    ));

                    // Settlement labels
                    for (wx, wy, name) in &settlements {
                        let px = *wx as f32 / world_map.width as f32 * MAP_DISPLAY_SIZE;
                        let py =
                            (1.0 - *wy as f32 / world_map.height as f32) * MAP_DISPLAY_SIZE;
                        container.spawn((
                            WorldMapSettlementLabel,
                            Text::new(name.clone()),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(theme.primary),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(px + 6.0),
                                top: Val::Px(py - 5.0),
                                ..default()
                            },
                        ));
                    }
                });

                // Zone info text
                col.spawn((
                    WorldMapInfoText,
                    Text::new(""),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(theme.text_parchment),
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                ));
            });

            // Right column: legend
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|legend| {
                // Legend title
                legend.spawn((
                    Text::new("Legend"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(theme.primary),
                    Node {
                        margin: UiRect::bottom(Val::Px(4.0)),
                        ..default()
                    },
                ));

                // Biome entries
                let biome_entries = [
                    ZoneType::Ocean,
                    ZoneType::Coast,
                    ZoneType::Grassland,
                    ZoneType::Forest,
                    ZoneType::Mountain,
                    ZoneType::Desert,
                    ZoneType::Tundra,
                    ZoneType::Swamp,
                    ZoneType::Settlement,
                ];
                for biome in biome_entries {
                    let color = biome_color(biome);
                    spawn_legend_row(legend, color, biome_label(biome), &theme);
                }
                // River entry
                spawn_legend_row(legend, RIVER_COLOR, "River", &theme);
            });
        });
}

fn spawn_legend_row(parent: &mut ChildSpawnerCommands, color: [u8; 4], label: &str, theme: &Theme) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(14.0),
                    height: Val::Px(14.0),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(color[0], color[1], color[2], color[3])),
            ));
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(theme.text_parchment),
            ));
        });
}

/// Generate an RGBA image from the world map data, including settlement icons.
fn generate_map_image(world_map: &WorldMap) -> Image {
    let w = world_map.width;
    let h = world_map.height;
    let mut data = vec![0u8; (w * h * 4) as usize];

    for y in 0..h {
        for x in 0..w {
            let cell_idx = (y * w + x) as usize;
            let cell = &world_map.cells[cell_idx];

            let mut color = biome_color(cell.zone_type);

            // Road overlay (under rivers so rivers take visual priority)
            if cell.road && !cell.river && cell.zone_type != ZoneType::Ocean && cell.zone_type != ZoneType::Settlement {
                color = [139, 119, 80, 255]; // tan/brown
            }

            // River overlay
            if cell.river && cell.zone_type != ZoneType::Ocean {
                color = RIVER_COLOR;
            }

            // Image y=0 is top, world y=0 is bottom — flip
            let img_y = h - 1 - y;
            let pixel_idx = ((img_y * w + x) * 4) as usize;
            data[pixel_idx] = color[0];
            data[pixel_idx + 1] = color[1];
            data[pixel_idx + 2] = color[2];
            data[pixel_idx + 3] = color[3];
        }
    }

    // Draw settlement markers — deduplicated, sized by settlement tier.
    // Each settlement gets a colored block with a dark outline for visibility.
    let city_color: [u8; 4] = [212, 175, 55, 255]; // bright gold
    let town_color: [u8; 4] = [212, 175, 55, 255]; // gold
    let village_color: [u8; 4] = [190, 155, 50, 255]; // slightly dimmer
    let hamlet_color: [u8; 4] = [160, 130, 40, 255]; // dim gold
    let outline_color: [u8; 4] = [20, 15, 5, 255]; // near-black outline

    // Helper to set a pixel in the image data
    let set_pixel = |data: &mut [u8], px: i32, img_y: i32, color: [u8; 4]| {
        if px < 0 || img_y < 0 || px >= w as i32 || img_y >= h as i32 { return; }
        let pi = ((img_y as u32 * w + px as u32) * 4) as usize;
        data[pi] = color[0];
        data[pi + 1] = color[1];
        data[pi + 2] = color[2];
        data[pi + 3] = color[3];
    };

    let mut seen_settlement_names = std::collections::HashSet::new();
    for (i, cell) in world_map.cells.iter().enumerate() {
        let Some(name) = &cell.settlement_name else { continue };
        if !seen_settlement_names.insert(name.clone()) { continue; }
        // Footprint matches world map: City 2x2, Town 2x1, Village/Hamlet 1x1
        let (color, footprint): (_, &[(i32, i32)]) = match cell.settlement_size {
            Some(SettlementSize::City) => (city_color, &[(0,0),(1,0),(0,1),(1,1)]),
            Some(SettlementSize::Town) => (town_color, &[(0,0),(1,0)]),
            Some(SettlementSize::Village) => (village_color, &[(0,0)]),
            Some(SettlementSize::Hamlet) => (hamlet_color, &[(0,0)]),
            None => continue,
        };
        let cx = (i as u32 % w) as i32;
        let cy = (i as u32 / w) as i32;
        // Pass 1: dark outline (1px border around each footprint cell)
        for &(dx, dy) in footprint {
            for ox in -1i32..=1 {
                for oy in -1i32..=1 {
                    if ox == 0 && oy == 0 { continue; }
                    let px = cx + dx + ox;
                    let img_y = h as i32 - 1 - (cy + dy + oy);
                    set_pixel(&mut data, px, img_y, outline_color);
                }
            }
        }
        // Pass 2: fill (overwrites outline pixels inside the footprint)
        for &(dx, dy) in footprint {
            let px = cx + dx;
            let img_y = h as i32 - 1 - (cy + dy);
            set_pixel(&mut data, px, img_y, color);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

// ---------------------------------------------------------------------------
// Toggle — press M to show/hide
// ---------------------------------------------------------------------------

pub fn toggle_world_map(
    actions: Res<ActionState>,
    mut visible: ResMut<WorldMapVisible>,
    mut view_state: ResMut<WorldMapViewState>,
    mut overlay_query: Query<&mut Node, With<WorldMapOverlay>>,
) {
    if actions.just_pressed(Action::ToggleWorldMap) {
        visible.0 = !visible.0;
        let Ok(mut node) = overlay_query.single_mut() else {
            return;
        };
        node.display = if visible.0 {
            Display::Flex
        } else {
            Display::None
        };
        // Reset zoom/pan when closing
        if !visible.0 {
            *view_state = WorldMapViewState::default();
        }
    }
}

// ---------------------------------------------------------------------------
// Zoom — scroll wheel when map is open
// ---------------------------------------------------------------------------

pub fn world_map_zoom(
    visible: Res<WorldMapVisible>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut view_state: ResMut<WorldMapViewState>,
    world_map: Res<WorldMap>,
) {
    if !visible.0 {
        return;
    }

    for event in scroll_events.read() {
        let delta = match event.unit {
            MouseScrollUnit::Line => event.y * 0.25,
            MouseScrollUnit::Pixel => event.y * 0.005,
        };
        let old_zoom = view_state.zoom;
        view_state.zoom = (view_state.zoom + delta).clamp(1.0, 4.0);

        // Adjust offset to keep center stable during zoom
        if (view_state.zoom - old_zoom).abs() > 0.001 {
            let map_w = world_map.width as f32;
            let map_h = world_map.height as f32;
            let visible_w = map_w / view_state.zoom;
            let visible_h = map_h / view_state.zoom;
            view_state.offset.x = view_state.offset.x.clamp(0.0, map_w - visible_w);
            view_state.offset.y = view_state.offset.y.clamp(0.0, map_h - visible_h);
        }
    }
}

// ---------------------------------------------------------------------------
// Pan — arrow keys or camera pan keys when map is open
// ---------------------------------------------------------------------------

pub fn world_map_pan(
    visible: Res<WorldMapVisible>,
    actions: Res<ActionState>,
    mut view_state: ResMut<WorldMapViewState>,
    world_map: Res<WorldMap>,
    time: Res<Time>,
) {
    if !visible.0 || view_state.zoom <= 1.0 {
        return;
    }

    let pan_speed = 80.0 * time.delta_secs(); // map-pixels per second
    let mut delta = Vec2::ZERO;

    if actions.pressed(Action::CameraPanUp) {
        delta.y -= pan_speed;
    }
    if actions.pressed(Action::CameraPanDown) {
        delta.y += pan_speed;
    }
    if actions.pressed(Action::CameraPanLeft) {
        delta.x -= pan_speed;
    }
    if actions.pressed(Action::CameraPanRight) {
        delta.x += pan_speed;
    }

    if delta != Vec2::ZERO {
        let map_w = world_map.width as f32;
        let map_h = world_map.height as f32;
        let visible_w = map_w / view_state.zoom;
        let visible_h = map_h / view_state.zoom;
        view_state.offset.x = (view_state.offset.x + delta.x).clamp(0.0, map_w - visible_w);
        view_state.offset.y = (view_state.offset.y + delta.y).clamp(0.0, map_h - visible_h);
    }
}

// ---------------------------------------------------------------------------
// Click — click on map to inspect a zone
// ---------------------------------------------------------------------------

pub fn world_map_click(
    visible: Res<WorldMapVisible>,
    view_state: Res<WorldMapViewState>,
    world_map: Res<WorldMap>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    container_query: Query<&GlobalTransform, With<WorldMapContainer>>,
    mut info_query: Query<&mut Text, With<WorldMapInfoText>>,
) {
    if !visible.0 || !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };
    let cursor_pos = match window.cursor_position() {
        Some(p) => p,
        None => return,
    };
    let Ok(container_transform) = container_query.single() else {
        return;
    };

    // Container is MAP_DISPLAY_SIZE x MAP_DISPLAY_SIZE, centered at transform position
    let container_global = container_transform.translation().truncate();
    let container_min = container_global - Vec2::splat(MAP_DISPLAY_SIZE / 2.0);

    // Cursor relative to container
    let rel_x = cursor_pos.x - container_min.x;
    let rel_y = cursor_pos.y - container_min.y;
    if rel_x < 0.0 || rel_y < 0.0 || rel_x >= MAP_DISPLAY_SIZE || rel_y >= MAP_DISPLAY_SIZE {
        return;
    }

    // Convert to map cell coordinates accounting for zoom/pan
    let map_w = world_map.width as f32;
    let map_h = world_map.height as f32;
    let cell_x = (view_state.offset.x + rel_x / MAP_DISPLAY_SIZE * map_w / view_state.zoom) as i32;
    // Flip y: UI top = world north (high y)
    let cell_y_flipped =
        (view_state.offset.y + rel_y / MAP_DISPLAY_SIZE * map_h / view_state.zoom) as i32;
    let cell_y = (map_h as i32 - 1) - cell_y_flipped;

    let pos = WorldPos::new(cell_x, cell_y);
    if let Some(cell) = world_map.get(pos) {
        if let Ok(mut text) = info_query.single_mut() {
            **text = cell_info_text(pos, cell);
        }
    }
}

// ---------------------------------------------------------------------------
// Update — move player marker, apply zoom/pan to image node and labels
// ---------------------------------------------------------------------------

pub fn update_world_map_marker(
    visible: Res<WorldMapVisible>,
    view_state: Res<WorldMapViewState>,
    current_zone: Res<CurrentZone>,
    world_map: Res<WorldMap>,
    mut image_query: Query<&mut Node, (With<WorldMapImage>, Without<WorldMapPlayerMarker>, Without<WorldMapSettlementLabel>)>,
    mut marker_query: Query<&mut Node, (With<WorldMapPlayerMarker>, Without<WorldMapImage>, Without<WorldMapSettlementLabel>)>,
    mut label_query: Query<&mut Node, (With<WorldMapSettlementLabel>, Without<WorldMapImage>, Without<WorldMapPlayerMarker>)>,
    mut info_query: Query<&mut Text, With<WorldMapInfoText>>,
) {
    if !visible.0 {
        return;
    }

    let zoom = view_state.zoom;
    let offset = view_state.offset;
    let map_w = world_map.width as f32;
    let map_h = world_map.height as f32;
    let scaled_size = MAP_DISPLAY_SIZE * zoom;

    // Update image node position/size for zoom and pan
    if let Ok(mut img_node) = image_query.single_mut() {
        img_node.width = Val::Px(scaled_size);
        img_node.height = Val::Px(scaled_size);
        img_node.left = Val::Px(-offset.x / map_w * scaled_size);
        img_node.top = Val::Px(-offset.y / map_h * scaled_size);
    }

    // Update player marker position
    if let Ok(mut marker_node) = marker_query.single_mut() {
        let px = current_zone.world_pos.x as f32 / map_w * scaled_size
            - offset.x / map_w * scaled_size;
        let py = (1.0 - current_zone.world_pos.y as f32 / map_h) * scaled_size
            - offset.y / map_h * scaled_size;
        marker_node.left = Val::Px(px - 4.0);
        marker_node.top = Val::Px(py - 4.0);
    }

    // Update settlement label positions for zoom/pan (deduplicate multi-cell settlements)
    let mut settlement_idx = 0;
    let mut seen_label_names = std::collections::HashSet::new();
    for (i, cell) in world_map.cells.iter().enumerate() {
        let Some(name) = &cell.settlement_name else { continue };
        if !seen_label_names.insert(name.clone()) {
            continue;
        }
        let wx = (i as u32 % world_map.width) as f32;
        let wy = (i as u32 / world_map.width) as f32;
        let px = wx / map_w * scaled_size - offset.x / map_w * scaled_size;
        let py = (1.0 - wy / map_h) * scaled_size - offset.y / map_h * scaled_size;

        if let Some(mut label_node) = label_query.iter_mut().nth(settlement_idx) {
            label_node.left = Val::Px(px + 6.0 * zoom);
            label_node.top = Val::Px(py - 5.0 * zoom);
        }
        settlement_idx += 1;
    }

    // Update info text with current zone info
    if let Ok(mut text) = info_query.single_mut() {
        if let Some(cell) = world_map.get(current_zone.world_pos) {
            **text = cell_info_text(current_zone.world_pos, cell
            );
        }
    }
}
