use bevy::prelude::*;
use trailsworn::resources::abilities::AbilityRegistry;
use trailsworn::resources::game_time::GameTime;
use trailsworn::resources::map::GridPosition;
use trailsworn::resources::movement::{MovePath, PendingPath};
use trailsworn::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use trailsworn::resources::task::{Action, CurrentTask, Engaging, Task, TaskSource};
use trailsworn::systems::task::execute_actions;

/// Create a minimal App with tick_actions and the required resources.
fn test_app() -> App {
    let mut app = App::new();
    app.insert_resource(game_time_one_tick());
    app.insert_resource(StatusEffectRegistry::default());
    app.insert_resource(AbilityRegistry::default());
    app.add_systems(Update, execute_actions);
    app
}

/// GameTime that produces exactly 1 tick this frame.
fn game_time_one_tick() -> GameTime {
    let mut gt = GameTime::default();
    gt.ticks_this_frame = 1;
    gt
}

/// Spawn a minimal entity that tick_actions can process.
/// Returns the entity id.
fn spawn_entity(app: &mut App, pos: (u32, u32), task: Task) -> Entity {
    app.world_mut()
        .spawn((
            GridPosition::new(pos.0, pos.1),
            ActiveStatusEffects::default(),
            CurrentTask::new(task),
        ))
        .id()
}

// ---------------------------------------------------------------------------
// MoveToPosition
// ---------------------------------------------------------------------------

#[test]
fn move_to_position_active_when_not_arrived() {
    let mut app = test_app();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "move", 100, TaskSource::Player,
        vec![Action::MoveToPosition { x: 5, y: 5 }],
    ));

    app.update();

    // Entity hasn't arrived — CurrentTask should still be present
    assert!(app.world().get::<CurrentTask>(entity).is_some());
}

#[test]
fn move_to_position_done_when_arrived() {
    let mut app = test_app();
    let entity = spawn_entity(&mut app, (5, 5), Task::new(
        "move", 100, TaskSource::Player,
        vec![Action::MoveToPosition { x: 5, y: 5 }],
    ));

    app.update();

    // Entity is at destination — task should be removed
    assert!(app.world().get::<CurrentTask>(entity).is_none());
}

// ---------------------------------------------------------------------------
// MoveToEntity
// ---------------------------------------------------------------------------

#[test]
fn move_to_entity_active_when_out_of_range() {
    let mut app = test_app();

    let target = app.world_mut().spawn(GridPosition::new(10, 10)).id();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "approach", 100, TaskSource::Player,
        vec![Action::MoveToEntity { target, range: 1.5 }],
    ));

    app.update();

    assert!(app.world().get::<CurrentTask>(entity).is_some());
}

#[test]
fn move_to_entity_done_when_in_range() {
    let mut app = test_app();

    let target = app.world_mut().spawn(GridPosition::new(10, 10)).id();
    let entity = spawn_entity(&mut app, (10, 11), Task::new(
        "approach", 100, TaskSource::Player,
        vec![Action::MoveToEntity { target, range: 1.5 }],
    ));

    app.update();

    // Distance is 1.0 which is <= 1.5 — task should complete
    assert!(app.world().get::<CurrentTask>(entity).is_none());
}

#[test]
fn move_to_entity_fails_when_target_despawned() {
    let mut app = test_app();

    let target = app.world_mut().spawn(GridPosition::new(10, 10)).id();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "approach", 100, TaskSource::Player,
        vec![Action::MoveToEntity { target, range: 1.5 }],
    ));

    // Despawn the target before running
    app.world_mut().despawn(target);
    app.update();

    // Target gone — task should fail and be removed
    assert!(app.world().get::<CurrentTask>(entity).is_none());
}

// ---------------------------------------------------------------------------
// EngageTarget + Engaging marker
// ---------------------------------------------------------------------------

#[test]
fn engage_target_inserts_engaging_marker() {
    let mut app = test_app();

    let target = app.world_mut().spawn(GridPosition::new(5, 5)).id();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "attack", 100, TaskSource::Player,
        vec![Action::EngageTarget { target, attack_range: 1.5 }],
    ));

    app.update();

    // Should have Engaging marker pointing at target
    let engaging = app.world().get::<Engaging>(entity).expect("Engaging marker should be present");
    assert_eq!(engaging.target, target);
    // Task stays active (persistent)
    assert!(app.world().get::<CurrentTask>(entity).is_some());
}

#[test]
fn engage_target_fails_when_target_despawned() {
    let mut app = test_app();

    let target = app.world_mut().spawn(GridPosition::new(5, 5)).id();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "attack", 100, TaskSource::Player,
        vec![Action::EngageTarget { target, attack_range: 1.5 }],
    ));

    app.world_mut().despawn(target);
    app.update();

    // Target dead — task removed, Engaging removed
    assert!(app.world().get::<CurrentTask>(entity).is_none());
    assert!(app.world().get::<Engaging>(entity).is_none());
}

#[test]
fn engaging_marker_removed_when_task_changes_to_non_engage() {
    let mut app = test_app();

    let target = app.world_mut().spawn(GridPosition::new(5, 5)).id();
    let entity = spawn_entity(&mut app, (5, 5), Task::new(
        "sequence", 100, TaskSource::Player,
        vec![
            Action::MoveToPosition { x: 5, y: 5 }, // already there → Done immediately
            Action::EngageTarget { target, attack_range: 1.5 },
        ],
    ));

    // First update: MoveToPosition completes, advances to EngageTarget
    app.update();

    // Should now have Engaging from the EngageTarget action
    assert!(app.world().get::<Engaging>(entity).is_some());

    // Now replace the task with a non-engage task
    app.world_mut().entity_mut(entity).insert(CurrentTask::new(Task::new(
        "move", 100, TaskSource::Player,
        vec![Action::MoveToPosition { x: 10, y: 10 }],
    )));

    app.update();

    // Engaging should be gone since current action is MoveToPosition
    assert!(app.world().get::<Engaging>(entity).is_none());
}

// ---------------------------------------------------------------------------
// Task sequencing
// ---------------------------------------------------------------------------

#[test]
fn task_advances_through_action_sequence() {
    let mut app = test_app();

    let target = app.world_mut().spawn(GridPosition::new(5, 5)).id();
    let entity = spawn_entity(&mut app, (5, 5), Task::new(
        "sequence", 100, TaskSource::Player,
        vec![
            Action::MoveToPosition { x: 5, y: 5 }, // already there → Done
            Action::EngageTarget { target, attack_range: 1.5 },
        ],
    ));

    app.update();

    // MoveToPosition completed, should have advanced to EngageTarget
    let ct = app.world().get::<CurrentTask>(entity).expect("Task should still exist");
    assert_eq!(ct.0.current_action, 1);
    assert!(app.world().get::<Engaging>(entity).is_some());
}

#[test]
fn task_removed_when_all_actions_complete() {
    let mut app = test_app();

    // Both MoveToPosition actions will complete immediately (entity already at both positions)
    let entity = spawn_entity(&mut app, (5, 5), Task::new(
        "double_move", 100, TaskSource::Player,
        vec![
            Action::MoveToPosition { x: 5, y: 5 },
        ],
    ));

    app.update();

    assert!(app.world().get::<CurrentTask>(entity).is_none());
}

// ---------------------------------------------------------------------------
// Wait
// ---------------------------------------------------------------------------

#[test]
fn wait_action_counts_ticks() {
    let mut app = test_app();

    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "wait", 100, TaskSource::Player,
        vec![Action::Wait { ticks: 5, elapsed: 0 }],
    ));

    // After 1 tick, should still be waiting
    app.update();
    assert!(app.world().get::<CurrentTask>(entity).is_some());

    // After 4 more ticks (5 total), should complete
    for _ in 0..4 {
        app.update();
    }
    assert!(app.world().get::<CurrentTask>(entity).is_none());
}

// ---------------------------------------------------------------------------
// FleeFrom
// ---------------------------------------------------------------------------

#[test]
fn flee_from_active_while_threat_alive() {
    let mut app = test_app();

    let threat = app.world_mut().spawn(GridPosition::new(5, 5)).id();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "flee", 90, TaskSource::Evaluator,
        vec![Action::FleeFrom { threat }],
    ));

    app.update();

    assert!(app.world().get::<CurrentTask>(entity).is_some());
}

#[test]
fn flee_from_done_when_threat_dead() {
    let mut app = test_app();

    let threat = app.world_mut().spawn(GridPosition::new(5, 5)).id();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "flee", 90, TaskSource::Evaluator,
        vec![Action::FleeFrom { threat }],
    ));

    app.world_mut().despawn(threat);
    app.update();

    assert!(app.world().get::<CurrentTask>(entity).is_none());
}

// ---------------------------------------------------------------------------
// FollowEntity
// ---------------------------------------------------------------------------

#[test]
fn follow_entity_stays_active() {
    let mut app = test_app();

    let leader = app.world_mut().spawn(GridPosition::new(5, 5)).id();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "follow", 20, TaskSource::Evaluator,
        vec![Action::FollowEntity { leader, distance: 5.0 }],
    ));

    app.update();

    // FollowEntity is persistent — never completes on its own
    assert!(app.world().get::<CurrentTask>(entity).is_some());
}

#[test]
fn follow_entity_fails_when_leader_despawned() {
    let mut app = test_app();

    let leader = app.world_mut().spawn(GridPosition::new(5, 5)).id();
    let entity = spawn_entity(&mut app, (0, 0), Task::new(
        "follow", 20, TaskSource::Evaluator,
        vec![Action::FollowEntity { leader, distance: 5.0 }],
    ));

    app.world_mut().despawn(leader);
    app.update();

    assert!(app.world().get::<CurrentTask>(entity).is_none());
}

// ---------------------------------------------------------------------------
// Zero ticks — systems should skip
// ---------------------------------------------------------------------------

#[test]
fn no_processing_when_zero_ticks() {
    let mut app = App::new();
    let mut gt = GameTime::default();
    gt.ticks_this_frame = 0;
    app.insert_resource(gt);
    app.insert_resource(StatusEffectRegistry::default());
    app.insert_resource(AbilityRegistry::default());
    app.add_systems(Update, execute_actions);

    // Entity at destination — would normally complete
    let entity = app.world_mut().spawn((
        GridPosition::new(5, 5),
        ActiveStatusEffects::default(),
        CurrentTask::new(Task::new(
            "move", 100, TaskSource::Player,
            vec![Action::MoveToPosition { x: 5, y: 5 }],
        )),
    )).id();

    app.update();

    // With zero ticks, nothing should happen — task still present
    assert!(app.world().get::<CurrentTask>(entity).is_some());
}

// ---------------------------------------------------------------------------
// CC incapacitation — pauses action processing
// ---------------------------------------------------------------------------

#[test]
fn incapacitated_entity_does_not_advance() {
    let mut app = test_app();

    // Register a stun effect
    let mut registry = StatusEffectRegistry::default();
    registry.register(trailsworn::resources::status_effects::StatusEffectDef {
        id: 1,
        name: "Stun".into(),
        max_stacks: 1,
        tick_interval_ticks: 0,
        tick_effect: None,
        stat_modifiers: vec![],
        cc_flags: trailsworn::resources::status_effects::CcFlags {
            stunned: true,
            ..Default::default()
        },
        is_buff: false,
    });
    app.insert_resource(registry);

    // Create entity at destination (would normally complete) but stunned
    let mut status_effects = ActiveStatusEffects::default();
    status_effects.apply(1, 100, None, app.world().resource::<StatusEffectRegistry>());

    let entity = app.world_mut().spawn((
        GridPosition::new(5, 5),
        status_effects,
        CurrentTask::new(Task::new(
            "move", 100, TaskSource::Player,
            vec![Action::MoveToPosition { x: 5, y: 5 }],
        )),
    )).id();

    app.update();

    // Stunned — task should NOT complete even though entity is at destination
    assert!(app.world().get::<CurrentTask>(entity).is_some());
}

// ---------------------------------------------------------------------------
// Cleanup on completion/failure
// ---------------------------------------------------------------------------

#[test]
fn movepath_removed_on_task_failure() {
    let mut app = test_app();

    let target = app.world_mut().spawn(GridPosition::new(10, 10)).id();
    let entity = app.world_mut().spawn((
        GridPosition::new(0, 0),
        ActiveStatusEffects::default(),
        CurrentTask::new(Task::new(
            "attack", 100, TaskSource::Player,
            vec![Action::EngageTarget { target, attack_range: 1.5 }],
        )),
        MovePath::new(vec![(0, 0), (1, 0), (2, 0)]),
    )).id();

    // Kill the target to trigger failure
    app.world_mut().despawn(target);
    app.update();

    assert!(app.world().get::<CurrentTask>(entity).is_none());
    assert!(app.world().get::<MovePath>(entity).is_none());
    assert!(app.world().get::<PendingPath>(entity).is_none());
    assert!(app.world().get::<Engaging>(entity).is_none());
}
