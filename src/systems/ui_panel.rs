use bevy::prelude::*;

use crate::resources::{
    input::{Action, ActionState},
    selection::Selected,
    theme::Theme,
};
use crate::systems::character_sheet;
use crate::systems::inventory;
use crate::systems::spawning::EntityName;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Which tab is currently active.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum UiTab {
    #[default]
    Character,
    Inventory,
}

/// Tracks whether the panel is open and which tab is shown.
/// `None` = panel closed, `Some(tab)` = panel open on that tab.
#[derive(Resource, Default)]
pub struct ActiveUiTab(pub Option<UiTab>);

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct UiPanelRoot;

#[derive(Component)]
pub struct UiTabButton(pub UiTab);

/// Container for a tab's content. Display toggled by active tab.
#[derive(Component)]
pub struct UiTabContent(pub UiTab);

#[derive(Component)]
pub struct UiNoSelectionOverlay;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PANEL_WIDTH: f32 = 720.0;
const PANEL_HEIGHT: f32 = 540.0;
const TAB_HEIGHT: f32 = 28.0;
const TAB_GAP: f32 = 2.0;

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

pub fn setup_ui_panel(mut commands: Commands, theme: Res<Theme>) {
    let label_color = Color::srgba(0.961, 0.961, 0.863, 0.5);

    commands
        .spawn((
            UiPanelRoot,
            Interaction::default(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-(PANEL_WIDTH / 2.0)),
                    top: Val::Px(-(PANEL_HEIGHT / 2.0)),
                    ..default()
                },
                width: Val::Px(PANEL_WIDTH),
                height: Val::Px(PANEL_HEIGHT),
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgb(0.075, 0.075, 0.075)),
            GlobalZIndex(10),
        ))
        .with_children(|root| {
            // ── Tab bar ──────────────────────────────────────────────
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(TAB_GAP),
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|tabs| {
                spawn_tab_button(tabs, &theme, UiTab::Character, "Character");
                spawn_tab_button(tabs, &theme, UiTab::Inventory, "Inventory");
            });

            // ── Content area (below tabs) ────────────────────────────
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            })
            .with_children(|content_area| {
                // "No character selected" overlay (shared across tabs)
                content_area.spawn((
                    UiNoSelectionOverlay,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        display: Display::None,
                        ..default()
                    },
                    GlobalZIndex(11),
                ))
                .with_child((
                    Text::new("No character selected"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(label_color),
                ));

                // ── Character tab content ────────────────────────────
                content_area.spawn((
                    UiTabContent(UiTab::Character),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        display: Display::None,
                        ..default()
                    },
                ))
                .with_children(|container| {
                    character_sheet::spawn_character_sheet_content(container, &theme);
                });

                // ── Inventory tab content ────────────────────────────
                content_area.spawn((
                    UiTabContent(UiTab::Inventory),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        display: Display::None,
                        ..default()
                    },
                ))
                .with_children(|container| {
                    inventory::spawn_inventory_content(container, &theme);
                });
            });
        });
}

fn spawn_tab_button(
    parent: &mut bevy::ecs::hierarchy::ChildSpawnerCommands,
    theme: &Theme,
    tab: UiTab,
    label: &str,
) {
    parent.spawn((
        UiTabButton(tab),
        Interaction::default(),
        Node {
            padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
            height: Val::Px(TAB_HEIGHT),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    ))
    .with_child((
        Text::new(label),
        TextFont { font_size: 13.0, ..default() },
        TextColor(theme.text_parchment),
    ));
}

// ---------------------------------------------------------------------------
// Toggle (Input set)
// ---------------------------------------------------------------------------

pub fn toggle_ui_panel(
    actions: Res<ActionState>,
    mut active_tab: ResMut<ActiveUiTab>,
    mut root_query: Query<&mut Node, With<UiPanelRoot>>,
    mut tab_contents: Query<(&UiTabContent, &mut Node), Without<UiPanelRoot>>,
) {
    let pressed_tab = if actions.just_pressed(Action::ToggleCharacterSheet) {
        Some(UiTab::Character)
    } else if actions.just_pressed(Action::ToggleInventory) {
        Some(UiTab::Inventory)
    } else {
        None
    };

    let Some(pressed) = pressed_tab else {
        return;
    };

    // Determine new state
    let new_state = match active_tab.0 {
        Some(current) if current == pressed => None,       // Same key → close
        Some(_) => Some(pressed),                          // Different key → switch
        None => Some(pressed),                             // Closed → open
    };

    active_tab.0 = new_state;

    // Update root visibility
    let Ok(mut root_node) = root_query.single_mut() else {
        return;
    };
    root_node.display = if new_state.is_some() {
        Display::Flex
    } else {
        Display::None
    };

    // Update tab content visibility
    for (tab_content, mut node) in &mut tab_contents {
        node.display = if new_state == Some(tab_content.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
}

// ---------------------------------------------------------------------------
// Tab button click handling + visual update (Ui set)
// ---------------------------------------------------------------------------

pub fn update_tab_visuals(
    theme: Res<Theme>,
    mut active_tab: ResMut<ActiveUiTab>,
    mut tab_buttons: Query<(&UiTabButton, &Interaction, &mut BackgroundColor, &Children)>,
    mut tab_contents: Query<(&UiTabContent, &mut Node)>,
    mut tab_texts: Query<&mut TextColor>,
) {
    let label_color = Color::srgba(0.961, 0.961, 0.863, 0.5);

    // Handle tab clicks
    let mut clicked_tab = None;
    for (tab_btn, interaction, _, _) in &tab_buttons {
        if *interaction == Interaction::Pressed && active_tab.0.is_some() {
            clicked_tab = Some(tab_btn.0);
        }
    }

    if let Some(clicked) = clicked_tab {
        if active_tab.0 != Some(clicked) {
            active_tab.0 = Some(clicked);
            for (tab_content, mut node) in &mut tab_contents {
                node.display = if Some(tab_content.0) == active_tab.0 {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }

    // Update button visuals
    for (tab_btn, _interaction, mut bg, children) in &mut tab_buttons {
        let is_active = active_tab.0 == Some(tab_btn.0);
        bg.0 = if is_active {
            Color::srgb(0.15, 0.15, 0.15)
        } else {
            Color::srgb(0.1, 0.1, 0.1)
        };

        for child in children.iter() {
            if let Ok(mut text_color) = tab_texts.get_mut(child) {
                text_color.0 = if is_active {
                    theme.primary
                } else {
                    label_color
                };
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared overlay logic (Ui set)
// ---------------------------------------------------------------------------

pub fn update_ui_panel_overlay(
    active_tab: Res<ActiveUiTab>,
    selected: Query<&EntityName, With<Selected>>,
    mut overlay: Query<&mut Node, (With<UiNoSelectionOverlay>, Without<UiPanelRoot>)>,
) {
    // Only run when panel is open
    if active_tab.0.is_none() {
        return;
    }

    let has_selection = selected.iter().next().is_some();

    if let Ok(mut overlay_node) = overlay.single_mut() {
        overlay_node.display = if has_selection {
            Display::None
        } else {
            Display::Flex
        };
    }
}
