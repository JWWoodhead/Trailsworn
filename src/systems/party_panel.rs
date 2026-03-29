use bevy::prelude::*;

use crate::resources::abilities::Mana;
use crate::resources::body::Health;
use crate::resources::combat::Dead;
use crate::resources::selection::Selected;
use crate::resources::theme::Theme;
use crate::systems::spawning::{EntityName, PlayerControlled};

/// Root container for the party portraits panel.
#[derive(Component)]
pub struct PartyPanelRoot;

/// Links a portrait UI node to its game entity.
#[derive(Component)]
pub struct PartyPortrait {
    pub entity: Entity,
}

/// Marker for the HP bar fill within a portrait.
#[derive(Component)]
pub struct PortraitHpFill {
    pub entity: Entity,
}

/// Marker for the mana bar fill within a portrait.
#[derive(Component)]
pub struct PortraitManaFill {
    pub entity: Entity,
}

/// Marker for the name text within a portrait.
#[derive(Component)]
pub struct PortraitNameText {
    pub entity: Entity,
}

/// Marker for the accent stripe, stores the default (non-selected) color.
#[derive(Component)]
pub struct PortraitAccentStripe {
    pub entity: Entity,
    pub default_color: Color,
}

const PORTRAIT_WIDTH: f32 = 120.0;
const PORTRAIT_HEIGHT: f32 = 48.0;
const PORTRAIT_GAP: f32 = 6.0;
const BAR_HEIGHT: f32 = 6.0;

/// Spawn the party panel UI. Runs once at startup.
pub fn setup_party_panel(mut commands: Commands, _theme: Res<Theme>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(50.0), // Below speed indicator
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(PORTRAIT_GAP),
            ..default()
        },
        PartyPanelRoot,
    ));
}

/// Spawn portrait nodes for new party members, despawn for removed ones.
pub fn sync_party_portraits(
    mut commands: Commands,
    theme: Res<Theme>,
    panel_root: Query<Entity, With<PartyPanelRoot>>,
    party_members: Query<(Entity, &EntityName, &Sprite), (With<PlayerControlled>, Without<Dead>)>,
    existing_portraits: Query<(Entity, &PartyPortrait)>,
) {
    let Ok(root) = panel_root.single() else { return };

    // Check which entities already have portraits
    let existing: Vec<Entity> = existing_portraits.iter().map(|(_, p)| p.entity).collect();

    for (member_entity, name, sprite) in &party_members {
        if existing.contains(&member_entity) {
            continue;
        }

        let tint = sprite.color.to_srgba();
        let accent = Color::srgba(tint.red, tint.green, tint.blue, 0.6);

        // Portrait container
        let portrait = commands.spawn((
            Node {
                width: Val::Px(PORTRAIT_WIDTH),
                height: Val::Px(PORTRAIT_HEIGHT),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(4.0)),
                row_gap: Val::Px(2.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(theme.surface),
            Interaction::default(),
            PartyPortrait { entity: member_entity },
        )).id();

        // Name label
        let name_text = commands.spawn((
            Text::new(name.0.clone()),
            TextFont { font_size: 12.0, ..default() },
            TextColor(theme.text_parchment),
            Node::default(),
            PortraitNameText { entity: member_entity },
        )).id();

        // HP bar background
        let hp_bg = commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(BAR_HEIGHT),
                ..default()
            },
            BackgroundColor(theme.hp_bar_bg),
        )).id();

        // HP bar fill
        let hp_fill = commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(theme.hp_full),
            PortraitHpFill { entity: member_entity },
        )).id();

        // Mana bar background
        let mana_bg = commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(BAR_HEIGHT - 2.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.15, 0.85)),
        )).id();

        // Mana bar fill
        let mana_fill = commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.3, 0.8)),
            PortraitManaFill { entity: member_entity },
        )).id();

        // Build hierarchy
        commands.entity(hp_bg).add_child(hp_fill);
        commands.entity(mana_bg).add_child(mana_fill);
        commands.entity(portrait).add_children(&[name_text, hp_bg, mana_bg]);

        // Left accent stripe (uses entity tint color, turns gold when selected)
        let accent_stripe = commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(3.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(accent),
            PortraitAccentStripe { entity: member_entity, default_color: accent },
        )).id();
        commands.entity(portrait).add_child(accent_stripe);

        commands.entity(root).add_child(portrait);
    }

    // Remove portraits for dead/despawned entities
    for (portrait_entity, portrait) in &existing_portraits {
        if party_members.get(portrait.entity).is_err() {
            commands.entity(portrait_entity).despawn();
        }
    }
}

/// Update HP and mana bars each frame.
pub fn update_party_portraits(
    theme: Res<Theme>,
    health_query: Query<&Health>,
    mana_query: Query<&Mana>,
    selected_query: Query<(), With<Selected>>,
    mut hp_fills: Query<(&PortraitHpFill, &mut Node, &mut BackgroundColor), Without<PortraitManaFill>>,
    mut mana_fills: Query<(&PortraitManaFill, &mut Node, &mut BackgroundColor), Without<PortraitHpFill>>,
    mut portraits: Query<(&PartyPortrait, &mut BackgroundColor), (Without<PortraitHpFill>, Without<PortraitManaFill>, Without<PortraitAccentStripe>)>,
    mut accent_stripes: Query<(&PortraitAccentStripe, Entity, &mut BackgroundColor), (Without<PartyPortrait>, Without<PortraitHpFill>, Without<PortraitManaFill>)>,
) {
    // Update HP bars
    for (hp, mut node, mut bg) in &mut hp_fills {
        let Ok(health) = health_query.get(hp.entity) else { continue };
        let fraction = health.fraction();
        node.width = Val::Percent(fraction * 100.0);
        bg.0 = theme.hp_color(fraction);
    }

    // Update mana bars
    for (mp, mut node, _bg) in &mut mana_fills {
        let Ok(mana) = mana_query.get(mp.entity) else { continue };
        let fraction = if mana.max > 0.0 { mana.current / mana.max } else { 0.0 };
        node.width = Val::Percent(fraction * 100.0);
    }

    // Highlight selected portraits with brighter background + gold left border
    for (portrait, mut bg) in &mut portraits {
        if selected_query.get(portrait.entity).is_ok() {
            bg.0 = Color::srgb(0.15, 0.14, 0.10);
        } else {
            bg.0 = theme.surface;
        }
    }

    // Update accent stripe: gold when selected, tinted when not
    for (stripe, _stripe_entity, mut bg) in &mut accent_stripes {
        if selected_query.get(stripe.entity).is_ok() {
            bg.0 = theme.primary;
        } else {
            bg.0 = stripe.default_color;
        }
    }
}

/// Handle clicks on party portraits to select the corresponding entity.
pub fn click_party_portrait(
    mut commands: Commands,
    portraits: Query<(&PartyPortrait, &Interaction), Changed<Interaction>>,
    currently_selected: Query<Entity, With<Selected>>,
) {
    for (portrait, interaction) in &portraits {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Deselect all currently selected entities
        for entity in &currently_selected {
            commands.entity(entity).remove::<Selected>();
            // Selection ring cleanup handled by update_selection_visuals
        }

        // Select the portrait's entity
        commands.entity(portrait.entity).insert(Selected);
    }
}
