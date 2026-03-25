use bevy::prelude::*;

use bevy::ecs::hierarchy::ChildSpawnerCommands;

use crate::resources::{
    abilities::{Mana, Stamina},
    body::{Body, BodyTemplates},
    damage::EquippedWeapon,
    selection::Selected,
    stats::{Attributes, CharacterLevel},
    status_effects::ActiveStatusEffects,
    theme::Theme,
};
use crate::systems::spawning::EntityName;
use crate::systems::ui_panel::{ActiveUiTab, UiTab};

// ---------------------------------------------------------------------------
// Marker components — enum-based to keep query count low
// ---------------------------------------------------------------------------

/// Identifies which text element this entity represents.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum CsText {
    Name,
    Level,
    BodyPartHp(usize),
    Vitals,
    Pain,
    Attributes,
    Combat,
    Mana,
    Stamina,
    Status,
}

/// Identifies which bar fill element this entity represents.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum CsBar {
    Xp,
    BodyPart(usize),
    Mana,
    Stamina,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SECTION_GAP: f32 = 12.0;
const BAR_HEIGHT: f32 = 10.0;
const BAR_WIDTH: f32 = 100.0;
const BODY_PART_ROW_HEIGHT: f32 = 18.0;
const RESOURCE_BAR_WIDTH: f32 = 160.0;
const RESOURCE_BAR_HEIGHT: f32 = 10.0;

/// The 6 major body regions to display, by part index in the humanoid template.
const BODY_DISPLAY_PARTS: [(usize, &str); 6] = [
    (0, "Head"),
    (5, "Torso"),
    (10, "L.Arm"),
    (12, "R.Arm"),
    (14, "L.Leg"),
    (16, "R.Leg"),
];

const VITAL_BRAIN: usize = 1;
const VITAL_HEART: usize = 6;

// ---------------------------------------------------------------------------
// Content setup — called by ui_panel::setup_ui_panel
// ---------------------------------------------------------------------------

pub fn spawn_character_sheet_content(parent: &mut ChildSpawnerCommands, theme: &Theme) {
    let label_color = Color::srgba(0.961, 0.961, 0.863, 0.5);

    // ── Header row ──────────────────────────────────────────
    parent.spawn(Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::End,
        width: Val::Percent(100.0),
        margin: UiRect::bottom(Val::Px(SECTION_GAP)),
        ..default()
    })
    .with_children(|header| {
        header.spawn((
            CsText::Name,
            Text::new("—"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(theme.primary),
        ));
        header.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::End,
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|level_col| {
            level_col.spawn((
                CsText::Level,
                Text::new("Level 1"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(theme.text_parchment),
            ));
            level_col.spawn(Node {
                width: Val::Px(160.0),
                height: Val::Px(6.0),
                ..default()
            })
            .insert(BackgroundColor(Color::srgb(0.05, 0.05, 0.05)))
            .with_children(|xp_bg| {
                xp_bg.spawn((
                    CsBar::Xp,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(theme.primary_container),
                ));
            });
        });
    });

    // ── Divider ─────────────────────────────────────────────
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::bottom(Val::Px(SECTION_GAP)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.961, 0.961, 0.863, 0.1)),
    ));

    // ── Content columns (40/60 split) ───────────────────────
    parent.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_grow: 1.0,
        column_gap: Val::Px(20.0),
        width: Val::Percent(100.0),
        ..default()
    })
    .with_children(|content| {
        // ── Left column (body) ──────────────────────────────
        content
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(40.0),
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|left| {
                left.spawn((
                    Text::new("BODY"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(label_color),
                    Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
                ));

                for &(part_idx, part_name) in &BODY_DISPLAY_PARTS {
                    left.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        height: Val::Px(BODY_PART_ROW_HEIGHT),
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(part_name),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(theme.text_parchment),
                            Node { width: Val::Px(48.0), ..default() },
                        ));
                        row.spawn((
                            Node {
                                width: Val::Px(BAR_WIDTH),
                                height: Val::Px(BAR_HEIGHT),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                        ))
                        .with_children(|bar_bg| {
                            bar_bg.spawn((
                                CsBar::BodyPart(part_idx),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(theme.hp_full),
                            ));
                        });
                        row.spawn((
                            CsText::BodyPartHp(part_idx),
                            Text::new("—"),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(theme.text_parchment),
                        ));
                    });
                }

                left.spawn((
                    CsText::Vitals,
                    Text::new(""),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(label_color),
                    Node { margin: UiRect::top(Val::Px(6.0)), ..default() },
                ));
                left.spawn((
                    CsText::Pain,
                    Text::new(""),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(theme.text_parchment),
                    Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
                ));
            });

        // ── Right column (stats + resources) ────────────────
        content
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(60.0),
                row_gap: Val::Px(4.0),
                overflow: Overflow::clip(),
                ..default()
            })
            .with_children(|right| {
                right.spawn((
                    Text::new("ATTRIBUTES"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(label_color),
                    Node { margin: UiRect::bottom(Val::Px(2.0)), ..default() },
                ));
                right.spawn((
                    CsText::Attributes,
                    Text::new(""),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(theme.text_parchment),
                    Node { margin: UiRect::bottom(Val::Px(SECTION_GAP)), ..default() },
                ));

                right.spawn((
                    Text::new("COMBAT"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(label_color),
                    Node { margin: UiRect::bottom(Val::Px(2.0)), ..default() },
                ));
                right.spawn((
                    CsText::Combat,
                    Text::new(""),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(theme.text_parchment),
                    Node { margin: UiRect::bottom(Val::Px(SECTION_GAP)), ..default() },
                ));

                right.spawn((
                    Text::new("RESOURCES"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(label_color),
                    Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
                ));
                spawn_resource_bar(right, theme, "Mana", true);
                spawn_resource_bar(right, theme, "Stamina", false);

                right.spawn((
                    Text::new("STATUS EFFECTS"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(label_color),
                    Node {
                        margin: UiRect { top: Val::Px(SECTION_GAP), bottom: Val::Px(2.0), ..default() },
                        ..default()
                    },
                ));
                right.spawn((
                    CsText::Status,
                    Text::new("None"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(theme.text_parchment),
                ));
            });
    });
}

fn spawn_resource_bar(parent: &mut ChildSpawnerCommands, theme: &Theme, label: &str, is_mana: bool) {
    let bar_color = if is_mana {
        Color::srgb(0.2, 0.4, 0.8)
    } else {
        Color::srgb(0.3, 0.7, 0.3)
    };
    let bar_marker = if is_mana { CsBar::Mana } else { CsBar::Stamina };
    let text_marker = if is_mana { CsText::Mana } else { CsText::Stamina };

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            margin: UiRect::bottom(Val::Px(4.0)),
            ..default()
        })
        .with_children(|row: &mut ChildSpawnerCommands| {
            row.spawn((
                Text::new(label),
                TextFont { font_size: 12.0, ..default() },
                TextColor(theme.text_parchment),
                Node { width: Val::Px(56.0), ..default() },
            ));
            row.spawn((
                Node {
                    width: Val::Px(RESOURCE_BAR_WIDTH),
                    height: Val::Px(RESOURCE_BAR_HEIGHT),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
            ))
            .with_children(|bar_bg: &mut ChildSpawnerCommands| {
                bar_bg.spawn((
                    bar_marker,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(bar_color),
                ));
            });
            row.spawn((
                text_marker,
                Text::new("—"),
                TextFont { font_size: 11.0, ..default() },
                TextColor(theme.text_parchment),
            ));
        });
}

// ---------------------------------------------------------------------------
// Update content (Ui set)
// ---------------------------------------------------------------------------

pub fn update_character_sheet(
    theme: Res<Theme>,
    body_templates: Res<BodyTemplates>,
    active_tab: Res<ActiveUiTab>,
    selected: Query<
        (
            &EntityName,
            &CharacterLevel,
            &Attributes,
            &Body,
            Option<&EquippedWeapon>,
            Option<&Mana>,
            Option<&Stamina>,
            Option<&ActiveStatusEffects>,
        ),
        With<Selected>,
    >,
    mut texts: Query<(&CsText, &mut Text)>,
    mut bars: Query<(&CsBar, &mut Node, &mut BackgroundColor)>,
) {
    // Only update when this tab is active
    if active_tab.0 != Some(UiTab::Character) {
        return;
    }

    let selected_data = selected.iter().next();

    let Some((entity_name, char_level, attrs, body, weapon, mana, stamina, status_effects)) =
        selected_data
    else {
        // Clear all content so nothing bleeds through behind the overlay
        for (_marker, mut text) in &mut texts {
            **text = String::new();
        }
        for (_marker, mut node, _bg) in &mut bars {
            node.width = Val::Percent(0.0);
        }
        return;
    };

    let template = body_templates.get(&body.template_id);

    // ── Update all text elements ────────────────────────────────
    for (marker, mut text) in &mut texts {
        match marker {
            CsText::Name => {
                **text = entity_name.0.clone();
            }
            CsText::Level => {
                **text = if char_level.unspent_points > 0 {
                    format!("Level {}  (+{} pts)", char_level.level, char_level.unspent_points)
                } else {
                    format!("Level {}", char_level.level)
                };
            }
            CsText::BodyPartHp(idx) => {
                if let Some(tmpl) = template {
                    if *idx < body.parts.len() && *idx < tmpl.parts.len() {
                        if body.parts[*idx].destroyed {
                            **text = "DESTROYED".to_string();
                        } else {
                            **text = format!(
                                "{:.0}/{:.0}",
                                body.parts[*idx].current_hp,
                                tmpl.parts[*idx].max_hp,
                            );
                        }
                    }
                }
            }
            CsText::Vitals => {
                let brain_ok = !body.parts[VITAL_BRAIN].destroyed;
                let heart_ok = !body.parts[VITAL_HEART].destroyed;
                **text = format!(
                    "Brain: {}  Heart: {}",
                    if brain_ok { "OK" } else { "DESTROYED" },
                    if heart_ok { "OK" } else { "DESTROYED" },
                );
            }
            CsText::Pain => {
                if let Some(tmpl) = template {
                    let pain = body.pain_level(tmpl);
                    let label = match pain {
                        p if p < 0.05 => "None",
                        p if p < 0.2 => "Low",
                        p if p < 0.5 => "Moderate",
                        p if p < 0.8 => "Severe",
                        _ => "Extreme",
                    };
                    **text = format!("Pain: {}", label);
                }
            }
            CsText::Attributes => {
                **text = format!(
                    "STR  {:>2}    AGI  {:>2}    INT  {:>2}\nTOU  {:>2}    WIL  {:>2}",
                    attrs.strength, attrs.agility, attrs.intellect,
                    attrs.toughness, attrs.willpower,
                );
            }
            CsText::Combat => {
                **text = if let Some(wpn) = weapon {
                    format!(
                        "{}\n{:.0} {:?}  |  Speed: {} ticks  |  Range: {:.1}",
                        wpn.weapon.name, wpn.weapon.base_damage, wpn.weapon.damage_type,
                        wpn.weapon.attack_speed_ticks, wpn.weapon.range,
                    )
                } else {
                    "Unarmed".to_string()
                };
            }
            CsText::Mana => {
                if let Some(m) = mana {
                    **text = format!("{:.0}/{:.0}", m.current, m.max);
                }
            }
            CsText::Stamina => {
                if let Some(s) = stamina {
                    **text = format!("{:.0}/{:.0}", s.current, s.max);
                }
            }
            CsText::Status => {
                **text = if let Some(active) = status_effects {
                    if active.effects.is_empty() {
                        "None".to_string()
                    } else {
                        active
                            .effects
                            .iter()
                            .map(|e| {
                                let secs = e.remaining_ticks as f32 / 60.0;
                                if e.stacks > 1 {
                                    format!("{:?} x{} ({:.1}s)", e.status_id, e.stacks, secs)
                                } else {
                                    format!("{:?} ({:.1}s)", e.status_id, secs)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                } else {
                    "None".to_string()
                };
            }
        }
    }

    // ── Update all bar elements ─────────────────────────────────
    for (marker, mut node, mut bg) in &mut bars {
        match marker {
            CsBar::Xp => {
                let frac = if char_level.xp_to_next > 0 {
                    char_level.current_xp as f32 / char_level.xp_to_next as f32
                } else {
                    0.0
                };
                node.width = Val::Percent(frac * 100.0);
            }
            CsBar::BodyPart(idx) => {
                if let Some(tmpl) = template {
                    if *idx < body.parts.len() && *idx < tmpl.parts.len() {
                        let hp = body.parts[*idx].current_hp;
                        let max_hp = tmpl.parts[*idx].max_hp;
                        let frac = if max_hp > 0.0 { hp / max_hp } else { 0.0 };
                        node.width = Val::Percent(frac * 100.0);
                        bg.0 = theme.hp_color(frac);
                    }
                }
            }
            CsBar::Mana => {
                if let Some(m) = mana {
                    let frac = if m.max > 0.0 { m.current / m.max } else { 0.0 };
                    node.width = Val::Percent(frac * 100.0);
                }
            }
            CsBar::Stamina => {
                if let Some(s) = stamina {
                    let frac = if s.max > 0.0 { s.current / s.max } else { 0.0 };
                    node.width = Val::Percent(frac * 100.0);
                }
            }
        }
    }
}
