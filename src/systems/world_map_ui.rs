use bevy::prelude::*;
use bevy::image::{Image, ImageSampler};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::resources::input::{Action, ActionState};
use crate::resources::theme::Theme;
use crate::resources::world::CurrentZone;
use crate::worldgen::world_map::WorldMap;
use crate::worldgen::zone::ZoneType;

// ---------------------------------------------------------------------------
// Resources & Components
// ---------------------------------------------------------------------------

/// Tracks whether the world map overlay is visible.
#[derive(Resource, Default)]
pub struct WorldMapVisible(pub bool);

/// Marker for the root overlay node.
#[derive(Component)]
pub struct WorldMapOverlay;

/// Marker for the map image node.
#[derive(Component)]
pub struct WorldMapImage;

/// Marker for the player position indicator.
#[derive(Component)]
pub struct WorldMapPlayerMarker;

/// Marker for zone info text.
#[derive(Component)]
pub struct WorldMapInfoText;

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

// ---------------------------------------------------------------------------
// Setup — create the map texture and UI overlay (hidden by default)
// ---------------------------------------------------------------------------

pub fn setup_world_map_ui(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    world_map: Res<WorldMap>,
    theme: Res<Theme>,
) {
    // Generate the map texture
    let map_image = generate_map_image(&world_map);
    let map_handle = images.add(map_image);

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
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            GlobalZIndex(20),
        ))
        .with_children(|root| {
            // Title
            root.spawn((
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

            // Map image container (square, sized to fit screen)
            root.spawn(Node {
                width: Val::Px(768.0),
                height: Val::Px(768.0),
                position_type: PositionType::Relative,
                ..default()
            })
            .with_children(|container| {
                // The map image
                container.spawn((
                    WorldMapImage,
                    ImageNode::new(map_handle),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                ));

                // Player position marker (small bright dot, positioned absolutely)
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
            });

            // Zone info text
            root.spawn((
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
}

/// Generate an RGBA image from the world map data.
fn generate_map_image(world_map: &WorldMap) -> Image {
    let w = world_map.width;
    let h = world_map.height;
    let mut data = vec![0u8; (w * h * 4) as usize];

    for y in 0..h {
        for x in 0..w {
            let cell_idx = (y * w + x) as usize;
            let cell = &world_map.cells[cell_idx];

            let mut color = biome_color(cell.zone_type);

            // River overlay
            if cell.river && cell.zone_type != ZoneType::Ocean {
                color = [40, 90, 170, 255];
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
    }
}

// ---------------------------------------------------------------------------
// Update — move player marker to current zone position
// ---------------------------------------------------------------------------

pub fn update_world_map_marker(
    visible: Res<WorldMapVisible>,
    current_zone: Res<CurrentZone>,
    world_map: Res<WorldMap>,
    mut marker_query: Query<&mut Node, With<WorldMapPlayerMarker>>,
    mut info_query: Query<&mut Text, With<WorldMapInfoText>>,
) {
    if !visible.0 {
        return;
    }

    let Ok(mut marker_node) = marker_query.single_mut() else {
        return;
    };

    // Map world position to pixel position in the 768x768 display
    let map_display_size = 768.0;
    let px = current_zone.world_pos.x as f32 / world_map.width as f32 * map_display_size;
    // Flip y: world y=0 is bottom, UI y=0 is top
    let py = (1.0 - current_zone.world_pos.y as f32 / world_map.height as f32) * map_display_size;

    marker_node.left = Val::Px(px - 4.0); // center the 8px marker
    marker_node.top = Val::Px(py - 4.0);

    // Update info text
    if let Ok(mut text) = info_query.single_mut() {
        if let Some(cell) = world_map.get(current_zone.world_pos) {
            let biome_name = match cell.zone_type {
                ZoneType::Grassland => "Grassland",
                ZoneType::Forest => "Forest",
                ZoneType::Mountain => "Mountains",
                ZoneType::Settlement => "Settlement",
                ZoneType::Desert => "Desert",
                ZoneType::Tundra => "Tundra",
                ZoneType::Swamp => "Swamp",
                ZoneType::Coast => "Coast",
                ZoneType::Ocean => "Ocean",
            };
            **text = format!(
                "{} ({}, {}) | Elev: {:.0}% | Moist: {:.0}% | Temp: {:.0}%{}",
                biome_name,
                current_zone.world_pos.x,
                current_zone.world_pos.y,
                cell.elevation * 100.0,
                cell.moisture * 100.0,
                cell.temperature * 100.0,
                if cell.river { " | River" } else { "" },
            );
        }
    }
}
