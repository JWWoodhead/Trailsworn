use bevy::prelude::*;

use crate::resources::abilities::{AbilityRegistry, CastingState};
use crate::resources::game_time::GameTime;
use crate::resources::map::GridPosition;
use crate::resources::movement::{MovePath, PendingPath};
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::task::{Action, ActionStatus, CurrentTask, Engaging};

/// Advance the current action for every entity with a CurrentTask.
/// Maintains derived marker components (Engaging).
/// Removes CurrentTask when the task completes or fails.
pub fn execute_actions(
    mut commands: Commands,
    game_time: Res<GameTime>,
    status_registry: Res<StatusEffectRegistry>,
    ability_registry: Res<AbilityRegistry>,
    mut query: Query<(
        Entity,
        &GridPosition,
        &ActiveStatusEffects,
        &mut CurrentTask,
    )>,
    target_positions: Query<&GridPosition>,
    casting_query: Query<&CastingState>,
) {

    for (entity, grid_pos, status_effects, mut current_task) in &mut query {
        let cc_flags = status_effects.combined_cc_flags(&status_registry);
        if cc_flags.is_incapacitated() {
            continue;
        }

        let status = process_current_action(
            entity, grid_pos, &mut current_task, game_time.ticks_this_frame,
            &target_positions, &casting_query, &ability_registry, &mut commands,
        );

        match status {
            Some(ActionStatus::Done) => {
                if !current_task.0.advance() {
                    commands.entity(entity).remove::<CurrentTask>();
                    commands.entity(entity).remove::<Engaging>();
                    commands.entity(entity).remove::<MovePath>();
                    commands.entity(entity).remove::<PendingPath>();
                    continue;
                }
            }
            Some(ActionStatus::Failed) => {
                commands.entity(entity).remove::<CurrentTask>();
                commands.entity(entity).remove::<Engaging>();
                commands.entity(entity).remove::<MovePath>();
                commands.entity(entity).remove::<PendingPath>();
                continue;
            }
            _ => {}
        }

        // Maintain Engaging derived marker
        match current_task.current_action() {
            Some(Action::EngageTarget { target, .. }) => {
                commands.entity(entity).insert(Engaging { target: *target });
            }
            _ => {
                commands.entity(entity).remove::<Engaging>();
            }
        }
    }
}

fn process_current_action(
    entity: Entity,
    grid_pos: &GridPosition,
    current_task: &mut CurrentTask,
    ticks: u32,
    target_positions: &Query<&GridPosition>,
    casting_query: &Query<&CastingState>,
    ability_registry: &AbilityRegistry,
    commands: &mut Commands,
) -> Option<ActionStatus> {
    let task = &mut current_task.0;
    let action = task.actions.get_mut(task.current_action)?;

    Some(match action {
        Action::MoveToEntity { target, range } => {
            match target_positions.get(*target) {
                Ok(target_pos) => {
                    let dx = grid_pos.x as f32 - target_pos.x as f32;
                    let dy = grid_pos.y as f32 - target_pos.y as f32;
                    if (dx * dx + dy * dy).sqrt() <= *range {
                        ActionStatus::Done
                    } else {
                        ActionStatus::Active
                    }
                }
                Err(_) => ActionStatus::Failed,
            }
        }

        Action::MoveToPosition { x, y } => {
            if grid_pos.x == *x && grid_pos.y == *y {
                ActionStatus::Done
            } else {
                ActionStatus::Active
            }
        }

        Action::FleeFrom { threat } => {
            if target_positions.get(*threat).is_err() {
                ActionStatus::Done
            } else {
                ActionStatus::Active
            }
        }

        Action::FollowEntity { leader, .. } => {
            if target_positions.get(*leader).is_err() {
                ActionStatus::Failed
            } else {
                ActionStatus::Active
            }
        }

        Action::EngageTarget { target, .. } => {
            if target_positions.get(*target).is_err() {
                ActionStatus::Failed
            } else {
                ActionStatus::Active
            }
        }

        Action::Wait { ticks: total, elapsed } => {
            *elapsed += ticks;
            if *elapsed >= *total {
                ActionStatus::Done
            } else {
                ActionStatus::Active
            }
        }

        Action::CastAbility { ability_id, slot_index, target, initiated } => {
            if !*initiated {
                let cast_time = ability_registry.get(*ability_id).map_or(0, |a| a.cast_time_ticks);
                commands.entity(entity).insert(CastingState {
                    ability_id: *ability_id,
                    slot_index: *slot_index,
                    remaining_ticks: cast_time,
                    target: target.clone(),
                });
                *initiated = true;
                ActionStatus::Active
            } else if casting_query.get(entity).is_ok() {
                ActionStatus::Active
            } else {
                ActionStatus::Done
            }
        }
    })
}
