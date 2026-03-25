use bevy::prelude::*;

use bevy::ecs::hierarchy::ChildSpawnerCommands;

use crate::resources::{
    damage::EquippedWeapon,
    items::{Equipment, EquipSlot, Inventory, ItemInstanceRegistry, ItemRegistry},
    selection::Selected,
    theme::Theme,
};
use crate::systems::spawning::EntityName;
use crate::systems::ui_panel::{ActiveUiTab, UiTab};

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

/// Identifies text elements in the inventory panel.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum InvText {
    CharacterName,
    EquipSlotLabel(usize),  // index into EQUIP_DISPLAY_SLOTS
    EquipSlotItem(usize),
    ItemName(usize),        // inventory grid slot index
    ItemCount(usize),
    WeightTotal,
    SlotCount,
    SelectedItemName,
    SelectedItemDesc,
    SelectedItemStats,
}

/// Background of an equipment slot (for rarity coloring).
#[derive(Component, Clone, Copy)]
pub struct InvEquipSlotBg(pub usize);

/// Background of an inventory grid slot.
#[derive(Component, Clone, Copy)]
pub struct InvGridSlotBg(pub usize);

#[derive(Component)]
pub struct InvDetailPanel;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const GRID_COLS: usize = 6;
const GRID_ROWS: usize = 4;
const GRID_SLOTS: usize = GRID_COLS * GRID_ROWS;
const GRID_CELL: f32 = 52.0;
const GRID_GAP: f32 = 4.0;
const EQUIP_SLOT_W: f32 = 64.0;
const EQUIP_SLOT_H: f32 = 28.0;

/// Equipment slots displayed in the paper-doll column.
const EQUIP_DISPLAY_SLOTS: [(EquipSlot, &str); 7] = [
    (EquipSlot::Head, "Head"),
    (EquipSlot::Chest, "Chest"),
    (EquipSlot::Hands, "Hands"),
    (EquipSlot::Legs, "Legs"),
    (EquipSlot::Feet, "Feet"),
    (EquipSlot::MainHand, "Weapon"),
    (EquipSlot::OffHand, "Off-Hand"),
];

// ---------------------------------------------------------------------------
// Content setup — called by ui_panel::setup_ui_panel
// ---------------------------------------------------------------------------

pub fn spawn_inventory_content(parent: &mut ChildSpawnerCommands, theme: &Theme) {
    let label_color = Color::srgba(0.961, 0.961, 0.863, 0.5);

    // ── Header ───────────────────────────────────────────────
    parent.spawn(Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        width: Val::Percent(100.0),
        margin: UiRect::bottom(Val::Px(8.0)),
        ..default()
    })
    .with_children(|header| {
        header.spawn((
            InvText::CharacterName,
            Text::new("INVENTORY"),
            TextFont { font_size: 20.0, ..default() },
            TextColor(theme.primary),
        ));
        header.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|stats| {
            stats.spawn((
                InvText::WeightTotal,
                Text::new("Weight: 0.0"),
                TextFont { font_size: 12.0, ..default() },
                TextColor(theme.text_parchment),
            ));
            stats.spawn((
                InvText::SlotCount,
                Text::new("0/0"),
                TextFont { font_size: 12.0, ..default() },
                TextColor(label_color),
            ));
        });
    });

    // ── Divider ──────────────────────────────────────────────
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.961, 0.961, 0.863, 0.1)),
    ));

    // ── Content columns (30/70 split) ────────────────────────
    parent.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_grow: 1.0,
        column_gap: Val::Px(16.0),
        width: Val::Percent(100.0),
        ..default()
    })
    .with_children(|content| {
        // ── Left: equipment slots ─────────────────────────────
        spawn_equipment_column(content, theme, label_color);

        // ── Right: grid + detail ──────────────────────────────
        content
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(70.0),
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|right| {
                spawn_inventory_grid(right, theme, label_color);
                spawn_detail_panel(right, theme, label_color);
            });
    });
}

fn spawn_equipment_column(
    parent: &mut ChildSpawnerCommands,
    _theme: &Theme,
    label_color: Color,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(30.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new("EQUIPPED"),
                TextFont { font_size: 11.0, ..default() },
                TextColor(label_color),
                Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
            ));

            for (i, &(_, slot_label)) in EQUIP_DISPLAY_SLOTS.iter().enumerate() {
                col.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(1.0),
                    margin: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                })
                .with_children(|slot_row| {
                    slot_row.spawn((
                        InvText::EquipSlotLabel(i),
                        Text::new(slot_label),
                        TextFont { font_size: 10.0, ..default() },
                        TextColor(label_color),
                    ));
                    slot_row.spawn((
                        InvEquipSlotBg(i),
                        Node {
                            width: Val::Px(EQUIP_SLOT_W * 2.5),
                            height: Val::Px(EQUIP_SLOT_H),
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                    ))
                    .with_child((
                        InvText::EquipSlotItem(i),
                        Text::new("— empty —"),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(Color::srgba(0.961, 0.961, 0.863, 0.3)),
                    ));
                });
            }
        });
}

fn spawn_inventory_grid(
    parent: &mut ChildSpawnerCommands,
    theme: &Theme,
    label_color: Color,
) {
    parent.spawn((
        Text::new("BAG"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(label_color),
    ));

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(GRID_GAP),
            row_gap: Val::Px(GRID_GAP),
            ..default()
        })
        .with_children(|grid| {
            for i in 0..GRID_SLOTS {
                grid.spawn((
                    InvGridSlotBg(i),
                    Node {
                        width: Val::Px(GRID_CELL),
                        height: Val::Px(GRID_CELL),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .with_children(|cell| {
                    cell.spawn((
                        InvText::ItemName(i),
                        Text::new(""),
                        TextFont { font_size: 9.0, ..default() },
                        TextColor(theme.text_parchment),
                        Node {
                            overflow: Overflow::clip(),
                            ..default()
                        },
                    ));
                    cell.spawn((
                        InvText::ItemCount(i),
                        Text::new(""),
                        TextFont { font_size: 9.0, ..default() },
                        TextColor(Color::srgba(0.961, 0.961, 0.863, 0.5)),
                    ));
                });
            }
        });
}

fn spawn_detail_panel(
    parent: &mut ChildSpawnerCommands,
    theme: &Theme,
    label_color: Color,
) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.961, 0.961, 0.863, 0.1)),
    ));

    parent
        .spawn((
            InvDetailPanel,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.8)),
        ))
        .with_children(|detail| {
            detail.spawn((
                InvText::SelectedItemName,
                Text::new(""),
                TextFont { font_size: 14.0, ..default() },
                TextColor(theme.primary),
            ));
            detail.spawn((
                InvText::SelectedItemDesc,
                Text::new("Hover over an item to see details."),
                TextFont { font_size: 12.0, ..default() },
                TextColor(label_color),
            ));
            detail.spawn((
                InvText::SelectedItemStats,
                Text::new(""),
                TextFont { font_size: 11.0, ..default() },
                TextColor(theme.text_parchment),
            ));
        });
}

// ---------------------------------------------------------------------------
// Update content (Ui set)
// ---------------------------------------------------------------------------

pub fn update_inventory_panel(
    item_registry: Res<ItemRegistry>,
    instance_registry: Res<ItemInstanceRegistry>,
    theme: Res<Theme>,
    active_tab: Res<ActiveUiTab>,
    selected: Query<
        (
            &EntityName,
            &Inventory,
            Option<&Equipment>,
            Option<&EquippedWeapon>,
        ),
        With<Selected>,
    >,
    mut texts: Query<(&InvText, &mut Text, &mut TextColor)>,
    mut equip_bgs: Query<(&InvEquipSlotBg, &mut BackgroundColor), Without<InvGridSlotBg>>,
    mut grid_bgs: Query<(&InvGridSlotBg, &mut BackgroundColor), Without<InvEquipSlotBg>>,
) {
    // Only update when this tab is active
    if active_tab.0 != Some(UiTab::Inventory) {
        return;
    }

    let selected_data = selected.iter().next();

    let Some((entity_name, inventory, equipment, weapon)) = selected_data else {
        // Clear all content so nothing bleeds through behind the overlay
        for (_marker, mut text, _color) in &mut texts {
            **text = String::new();
        }
        for (_slot_bg, mut bg) in &mut equip_bgs {
            bg.0 = Color::srgb(0.1, 0.1, 0.1);
        }
        for (_slot_bg, mut bg) in &mut grid_bgs {
            bg.0 = Color::srgb(0.1, 0.1, 0.1);
        }
        return;
    };

    for (marker, mut text, mut color) in &mut texts {
        match marker {
            InvText::CharacterName => {
                **text = format!("{} — Inventory", entity_name.0);
            }
            InvText::WeightTotal => {
                let w = inventory.total_weight(&item_registry, &instance_registry);
                **text = format!("Weight: {:.1}", w);
            }
            InvText::SlotCount => {
                **text = format!("{}/{}", inventory.occupied_slots(), inventory.capacity);
            }
            InvText::EquipSlotLabel(_) => {}
            InvText::EquipSlotItem(idx) => {
                let slot = EQUIP_DISPLAY_SLOTS[*idx].0;
                let instance = equipment
                    .and_then(|eq| eq.in_slot(slot))
                    .and_then(|id| instance_registry.get(id));

                if let Some(inst) = instance {
                    // Show rolled display name (includes affix labels)
                    **text = inst.display_name(&item_registry);
                    color.0 = inst.rarity.text_color(&theme);
                } else if slot == EquipSlot::MainHand {
                    // Fallback to EquippedWeapon name if no Equipment entry
                    if let Some(w) = weapon {
                        **text = w.weapon.name.clone();
                        color.0 = theme.text_parchment;
                    } else {
                        **text = "— empty —".to_string();
                        color.0 = Color::srgba(0.961, 0.961, 0.863, 0.3);
                    }
                } else {
                    **text = "— empty —".to_string();
                    color.0 = Color::srgba(0.961, 0.961, 0.863, 0.3);
                }
            }
            InvText::ItemName(idx) => {
                let stack_count = inventory.items.len();
                if *idx < stack_count {
                    // Stackable item
                    if let Some(stack) = inventory.items.get(*idx) {
                        if let Some(def) = item_registry.get(stack.item_id) {
                            **text = truncate_name(&def.name, 8);
                            color.0 = def.rarity.text_color(&theme);
                        } else {
                            **text = "???".to_string();
                            color.0 = theme.text_parchment;
                        }
                    }
                } else {
                    // Instance item (equipment in bag)
                    let inst_idx = *idx - stack_count;
                    if let Some(&inst_id) = inventory.instances.get(inst_idx) {
                        if let Some(inst) = instance_registry.get(inst_id) {
                            if let Some(def) = item_registry.get(inst.base_item_id) {
                                **text = truncate_name(&def.name, 8);
                                color.0 = inst.rarity.text_color(&theme);
                            } else {
                                **text = "???".to_string();
                                color.0 = theme.text_parchment;
                            }
                        } else {
                            **text = String::new();
                        }
                    } else {
                        **text = String::new();
                    }
                }
            }
            InvText::ItemCount(idx) => {
                let stack_count = inventory.items.len();
                if *idx < stack_count {
                    if let Some(stack) = inventory.items.get(*idx) {
                        if stack.count > 1 {
                            **text = format!("x{}", stack.count);
                        } else {
                            **text = String::new();
                        }
                    } else {
                        **text = String::new();
                    }
                } else {
                    // Instance items never stack
                    **text = String::new();
                }
            }
            InvText::SelectedItemName => {}
            InvText::SelectedItemDesc => {}
            InvText::SelectedItemStats => {}
        }
    }

    // Equipment slot backgrounds — rarity tint
    for (slot_bg, mut bg) in &mut equip_bgs {
        let slot = EQUIP_DISPLAY_SLOTS[slot_bg.0].0;
        let rarity = equipment
            .and_then(|eq| eq.in_slot(slot))
            .and_then(|id| instance_registry.get(id))
            .map(|inst| &inst.rarity);

        bg.0 = if let Some(r) = rarity {
            r.bg_tint()
        } else {
            Color::srgb(0.1, 0.1, 0.1)
        };
    }

    // Grid slot backgrounds — rarity tint
    let stack_count = inventory.items.len();
    for (slot_bg, mut bg) in &mut grid_bgs {
        let idx = slot_bg.0;
        let rarity = if idx < stack_count {
            // Stackable item
            inventory.items.get(idx)
                .and_then(|stack| item_registry.get(stack.item_id))
                .map(|def| def.rarity)
        } else {
            // Instance item
            let inst_idx = idx - stack_count;
            inventory.instances.get(inst_idx)
                .and_then(|&id| instance_registry.get(id))
                .map(|inst| inst.rarity)
        };

        bg.0 = if let Some(r) = rarity {
            r.bg_tint()
        } else {
            Color::srgb(0.1, 0.1, 0.1)
        };
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}…", &name[..max_len])
    }
}

