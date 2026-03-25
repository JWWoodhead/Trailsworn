use bevy::prelude::*;

use crate::resources::events::{AbilityCastEvent, AttackMissedEvent, CastInterruptedEvent, DamageDealtEvent};
use crate::resources::game_time::GameTime;
use crate::resources::abilities::AbilityRegistry;
use crate::resources::theme::Theme;
use crate::systems::spawning::EntityName;

// ── Pause / Speed Indicator ──

#[derive(Component)]
pub struct SpeedIndicator;

pub fn setup_hud(mut commands: Commands, theme: Res<Theme>) {
    // Speed indicator — top right
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(16.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(theme.surface),
    )).with_child((
        Text::new("1x"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(theme.primary),
        SpeedIndicator,
    ));

    // Combat log panel — bottom left
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            width: Val::Px(400.0),
            height: Val::Px(200.0),
            flex_direction: FlexDirection::ColumnReverse,
            overflow: Overflow::clip(),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.075, 0.075, 0.075, 0.85)),
        CombatLogPanel,
    ));
}

pub fn update_speed_indicator(
    game_time: Res<GameTime>,
    mut query: Query<(&mut Text, &mut TextColor), With<SpeedIndicator>>,
    theme: Res<Theme>,
) {
    let Ok((mut text, mut color)) = query.single_mut() else {
        return;
    };

    if game_time.paused {
        **text = "PAUSED".to_string();
        color.0 = theme.secondary;
    } else {
        **text = format!("{}x", game_time.speed as u32);
        color.0 = theme.primary;
    }
}

// ── Combat Log ──

#[derive(Component)]
pub struct CombatLogPanel;

const MAX_LOG_ENTRIES: usize = 50;

pub fn combat_log_damage(
    mut damage_events: MessageReader<DamageDealtEvent>,
    mut miss_events: MessageReader<AttackMissedEvent>,
    mut cast_events: MessageReader<AbilityCastEvent>,
    mut interrupt_events: MessageReader<CastInterruptedEvent>,
    ability_registry: Res<AbilityRegistry>,
    theme: Res<Theme>,
    names: Query<&EntityName>,
    panel_query: Query<Entity, With<CombatLogPanel>>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    let Ok(panel) = panel_query.single() else {
        return;
    };

    let name_of = |entity: Entity| -> &str {
        names.get(entity).map(|n| n.0.as_str()).unwrap_or("???")
    };

    // Cast start events
    for event in cast_events.read() {
        let caster = name_of(event.caster);
        let msg = format!("{} casts {} on {}", caster, event.ability_name, event.target_description);
        spawn_log_entry(&mut commands, panel, &msg, theme.primary);
    }

    // Cast interrupt events
    for event in interrupt_events.read() {
        let caster = name_of(event.caster);
        let ability_name = ability_registry
            .get(event.ability_id)
            .map(|a| a.name.as_str())
            .unwrap_or("???");
        let msg = format!("{}'s {} was interrupted!", caster, ability_name);
        spawn_log_entry(&mut commands, panel, &msg, Color::srgba(0.9, 0.6, 0.2, 1.0));
    }

    // Damage events
    for event in damage_events.read() {
        let attacker = name_of(event.attacker);
        let target = name_of(event.target);

        let msg = if event.target_killed {
            format!("{} killed {}!", attacker, target)
        } else if event.part_destroyed {
            if let Some(ref ability) = event.ability_name {
                format!("{}'s {} on {}: {} destroyed! ({:.0})", attacker, ability, target, event.body_part_name, event.amount)
            } else {
                format!("{} hit {}: {} destroyed! ({:.0})", attacker, target, event.body_part_name, event.amount)
            }
        } else if let Some(ref ability) = event.ability_name {
            format!("{}'s {} hit {}: {:.0} to {}", attacker, ability, target, event.amount, event.body_part_name)
        } else {
            format!("{} hit {}: {:.0} to {}", attacker, target, event.amount, event.body_part_name)
        };

        let color = if event.target_killed || event.part_destroyed {
            theme.secondary
        } else {
            theme.text_parchment
        };

        spawn_log_entry(&mut commands, panel, &msg, color);
    }

    // Miss events
    for event in miss_events.read() {
        let attacker = name_of(event.attacker);
        let target = name_of(event.target);
        let msg = format!("{} missed {}", attacker, target);
        spawn_log_entry(&mut commands, panel, &msg, Color::srgba(0.5, 0.5, 0.5, 0.7));
    }

    // Trim old entries
    if let Ok(children) = children_query.get(panel) {
        let overflow = children.len().saturating_sub(MAX_LOG_ENTRIES);
        for child in children.iter().take(overflow) {
            commands.entity(child).despawn();
        }
    }
}

fn spawn_log_entry(commands: &mut Commands, panel: Entity, msg: &str, color: Color) {
    let entry = commands
        .spawn((
            Text::new(msg),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(color),
        ))
        .id();

    commands.entity(panel).add_child(entry);
}
