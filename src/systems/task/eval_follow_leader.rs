use bevy::prelude::*;

use crate::resources::task::{Action, AiBrain, Task, TaskEvaluator, TaskSource};

/// Propose following the party leader.
pub fn follow_leader(
    mut query: Query<&mut AiBrain>,
) {
    for mut brain in &mut query {
        if brain.routine_eval_cooldown != 0 {
            continue;
        }
        let evaluators = brain.evaluators.clone();
        for evaluator in &evaluators {
            if let TaskEvaluator::FollowLeader { leader } = evaluator {
                brain.proposals.push(Task::new(
                    "follow", 20, TaskSource::Evaluator,
                    vec![Action::FollowEntity { leader: *leader, distance: 5.0 }],
                ));
            }
        }
    }
}
