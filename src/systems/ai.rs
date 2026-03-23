use bevy::prelude::*;

use crate::pathfinding::astar_tile_grid;
use crate::resources::ai::{
    AiState, CombatBehavior, MovementIntent, PartyMode, PlayerCommand, RepathTimer,
};
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::combat::InCombat;
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::game_time::GameTime;
use crate::resources::map::{GridPosition, TileWorld};
use crate::resources::movement::{MovePath, PendingPath};
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::threat::ThreatTable;

/// Main AI decision system. Sets AiState and MovementIntent.
/// Skips entities with active PlayerCommands.
pub fn ai_decision(
    game_time: Res<GameTime>,
    faction_relations: Res<FactionRelations>,
    body_templates: Res<BodyTemplates>,
    status_registry: Res<StatusEffectRegistry>,
    mut ai_query: Query<
        (
            Entity,
            &GridPosition,
            &Faction,
            &CombatBehavior,
            &Body,
            &ActiveStatusEffects,
            &mut AiState,
            &mut MovementIntent,
            Option<&PartyMode>,
            Option<&ThreatTable>,
        ),
        (Without<PlayerCommand>, Without<crate::systems::spawning::PlayerControlled>),
    >,
    potential_targets: Query<(Entity, &GridPosition, &Faction, &Body), With<InCombat>>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    for (
        entity,
        grid_pos,
        faction,
        behavior,
        body,
        status_effects,
        mut ai_state,
        mut intent,
        party_mode,
        threat_table,
    ) in &mut ai_query
    {
        // Check CC
        let cc_flags = status_effects.combined_cc_flags(&status_registry);
        if cc_flags.is_incapacitated() {
            *intent = MovementIntent::None;
            continue;
        }

        // Passive party members don't engage
        if let Some(&PartyMode::Passive) = party_mode {
            *ai_state = AiState::Idle;
            *intent = MovementIntent::None;
            continue;
        }

        // Get body template for HP checks
        let template = match body_templates.get(&body.template_id) {
            Some(t) => t,
            None => continue,
        };

        // Check flee threshold
        let hp_fraction = 1.0 - body.pain_level(template);
        if behavior.flee_hp_threshold > 0.0 && hp_fraction < behavior.flee_hp_threshold {
            *ai_state = AiState::Fleeing;
            // TODO: flee from nearest threat
            *intent = MovementIntent::None;
            continue;
        }

        // Target selection
        let target = select_target(
            entity,
            grid_pos,
            faction,
            behavior.aggro_range,
            party_mode,
            threat_table,
            &faction_relations,
            &potential_targets,
        );

        match target {
            Some(target_entity) => {
                *ai_state = AiState::Engaging {
                    target: target_entity,
                };
                *intent = MovementIntent::MoveToEntity {
                    target: target_entity,
                    desired_range: behavior.attack_range,
                };
            }
            None => {
                *ai_state = AiState::Idle;
                *intent = MovementIntent::None;
            }
        }
    }
}

/// Select the best target for an entity.
fn select_target(
    self_entity: Entity,
    self_pos: &GridPosition,
    self_faction: &Faction,
    aggro_range: f32,
    party_mode: Option<&PartyMode>,
    threat_table: Option<&ThreatTable>,
    faction_relations: &FactionRelations,
    potential_targets: &Query<(Entity, &GridPosition, &Faction, &Body), With<InCombat>>,
) -> Option<Entity> {
    // If we have a threat table, use highest threat
    if let Some(table) = threat_table {
        if let Some(highest) = table.highest_threat() {
            if let Ok((_, _, target_faction, _)) = potential_targets.get(highest) {
                if faction_relations.is_hostile(self_faction.0, target_faction.0) {
                    return Some(highest);
                }
            }
        }
    }

    // Defensive party members only engage if attacked
    if let Some(&PartyMode::Defensive) = party_mode {
        if threat_table.is_none_or(|t| t.is_empty()) {
            return None;
        }
    }

    // Follow mode: don't initiate combat
    if let Some(&PartyMode::Follow) = party_mode {
        return None;
    }

    // Find nearest hostile entity within aggro range
    let aggro_range_sq = (aggro_range * aggro_range) as u32;
    let mut best: Option<(Entity, u32)> = None;
    for (target_entity, target_pos, target_faction, _) in potential_targets.iter() {
        if target_entity == self_entity {
            continue;
        }
        if !faction_relations.is_hostile(self_faction.0, target_faction.0) {
            continue;
        }

        let dx = self_pos.x.abs_diff(target_pos.x);
        let dy = self_pos.y.abs_diff(target_pos.y);
        let dist_sq = dx * dx + dy * dy;

        if dist_sq > aggro_range_sq {
            continue;
        }

        if best.is_none() || dist_sq < best.unwrap().1 {
            best = Some((target_entity, dist_sq));
        }
    }

    best.map(|(e, _)| e)
}

/// Resolve MovementIntent into actual pathfinding.
/// Respects repath timer to avoid repathing every tick.
pub fn resolve_movement_intent(
    game_time: Res<GameTime>,
    tile_world: Res<TileWorld>,
    mut query: Query<(
        Entity,
        &GridPosition,
        &MovementIntent,
        &mut RepathTimer,
        Option<&MovePath>,
        Option<&crate::systems::spawning::PlayerControlled>,
    )>,
    target_positions: Query<&GridPosition>,
    mut commands: Commands,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    for (entity, grid_pos, intent, mut repath_timer, current_path, player_controlled) in &mut query {
        // Tick the repath timer
        for _ in 0..game_time.ticks_this_frame {
            repath_timer.tick();
        }

        let goal: Option<(u32, u32)> = match intent {
            MovementIntent::None => {
                // No movement desired — clear any path
                if current_path.is_some() {
                    commands.entity(entity).remove::<MovePath>();
                    commands.entity(entity).remove::<PendingPath>();
                }
                repath_timer.reset();
                continue;
            }
            MovementIntent::MoveToEntity { target, desired_range } => {
                let Ok(target_pos) = target_positions.get(*target) else {
                    continue;
                };

                // In range check runs every tick — stop moving if close enough
                let dx = grid_pos.x as f32 - target_pos.x as f32;
                let dy = grid_pos.y as f32 - target_pos.y as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= *desired_range {
                    if current_path.is_some() {
                        commands.entity(entity).remove::<MovePath>();
                        commands.entity(entity).remove::<PendingPath>();
                    }
                    repath_timer.reset();
                    continue;
                }

                Some((target_pos.x, target_pos.y))
            }
            MovementIntent::MoveToPosition { x, y } => Some((*x, *y)),
            MovementIntent::FleeFrom { threat } => {
                // Simple flee: move away from threat
                let Ok(threat_pos) = target_positions.get(*threat) else {
                    continue;
                };
                let dx = grid_pos.x as i32 - threat_pos.x as i32;
                let dy = grid_pos.y as i32 - threat_pos.y as i32;
                // Move 10 tiles in the opposite direction
                let flee_x = (grid_pos.x as i32 + dx.signum() * 10)
                    .clamp(0, tile_world.width as i32 - 1) as u32;
                let flee_y = (grid_pos.y as i32 + dy.signum() * 10)
                    .clamp(0, tile_world.height as i32 - 1) as u32;
                Some((flee_x, flee_y))
            }
            MovementIntent::FollowEntity { leader, follow_distance } => {
                let Ok(leader_pos) = target_positions.get(*leader) else {
                    continue;
                };
                let dx = grid_pos.x as f32 - leader_pos.x as f32;
                let dy = grid_pos.y as f32 - leader_pos.y as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= *follow_distance {
                    continue;
                }
                Some((leader_pos.x, leader_pos.y))
            }
        };

        let Some(goal) = goal else { continue };

        // Player entities bypass the timer ONLY when their goal changed.
        // AI entities throttle repathing to avoid pathfinding every tick.
        let is_player = player_controlled.is_some();
        let needs_initial_path = current_path.is_none();

        if !needs_initial_path {
            if is_player {
                // Player: only repath if destination changed
                let current_dest = current_path.and_then(|p| p.destination());
                if current_dest == Some(goal) {
                    continue;
                }
            } else {
                // AI: throttle with timer
                if !repath_timer.should_repath() {
                    continue;
                }
            }
        }

        let mid_movement = current_path.is_some_and(|p| p.progress > 0.0);
        let old_progress = current_path.map(|p| p.progress).unwrap_or(0.0);

        // For player entities mid-movement: pathfind from the tile we're heading toward
        // so the entity smoothly continues to it, then follows the new path.
        let (start, prepend_current) = if is_player && mid_movement {
            let next = current_path.and_then(|p| p.next_tile());
            match next {
                Some(n) => (n, true),
                None => ((grid_pos.x, grid_pos.y), false),
            }
        } else {
            ((grid_pos.x, grid_pos.y), false)
        };

        if start == goal {
            continue;
        }

        if let Some(mut path) = astar_tile_grid(
            start,
            goal,
            tile_world.width,
            tile_world.height,
            &tile_world.walk_cost,
            5000,
        ) {
            if prepend_current {
                // Prepend GridPosition so the entity finishes its current step smoothly
                path.insert(0, (grid_pos.x, grid_pos.y));
                let mut mp = MovePath::new(path);
                mp.progress = old_progress;
                commands.entity(entity).insert(mp);
            } else if mid_movement {
                // AI entities mid-movement: use PendingPath to avoid snap
                commands.entity(entity).insert(PendingPath { waypoints: path });
            } else {
                commands.entity(entity).insert(MovePath::new(path));
            }
            repath_timer.reset();
        }
    }
}

/// Clean up completed or invalid player commands and stale movement intents.
pub fn cleanup_commands(
    mut commands: Commands,
    mut query: Query<(Entity, &GridPosition, Option<&PlayerCommand>, &mut MovementIntent, Option<&MovePath>)>,
    alive_entities: Query<Entity>,
) {
    for (entity, pos, command, mut intent, current_path) in &mut query {
        // Clean up completed/invalid player commands
        if let Some(cmd) = command {
            let should_remove = match cmd {
                PlayerCommand::MoveTo { x, y } => pos.x == *x && pos.y == *y,
                PlayerCommand::Attack(target) => alive_entities.get(*target).is_err(),
                PlayerCommand::HoldPosition => false,
                PlayerCommand::CastAbility { .. } => false,
            };
            if should_remove {
                commands.entity(entity).remove::<PlayerCommand>();
                *intent = MovementIntent::None;
            }
        }

        // Clear MoveToPosition intent when arrived and no path remaining
        if let MovementIntent::MoveToPosition { x, y } = *intent {
            if pos.x == x && pos.y == y && current_path.is_none() {
                *intent = MovementIntent::None;
            }
        }
    }
}
