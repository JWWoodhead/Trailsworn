use bevy::prelude::*;

use crate::resources::abilities::{
    AbilityRegistry, AbilitySlots, CastTarget, Mana, Stamina, TargetType,
};
use crate::resources::combat_behavior::{CombatBehavior, UseCondition};
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::casting::validate_cast;
use crate::resources::combat::InCombat;
use crate::resources::faction::{Faction};
use crate::resources::map::GridPosition;
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::task::{Action, AiBrain, Engaging, Task, TaskEvaluator, TaskSource};

/// Propose casting an ability when conditions are met.
pub fn use_ability(
    ability_registry: Res<AbilityRegistry>,
    body_templates: Res<BodyTemplates>,
    status_registry: Res<StatusEffectRegistry>,
    mut query: Query<(
        &GridPosition,
        &CombatBehavior,
        &Body,
        &ActiveStatusEffects,
        &mut AiBrain,
        Option<&Engaging>,
        Option<&AbilitySlots>,
        Option<&Mana>,
        Option<&Stamina>,
    )>,
    potential_targets: Query<(Entity, &GridPosition, &Faction, &Body), With<InCombat>>,
) {
    for (grid_pos, behavior, body, status_effects, mut brain, engaging, slots, mana, stamina) in &mut query {
        if brain.combat_eval_cooldown != 0 {
            continue;
        }
        if !brain.evaluators.iter().any(|e| matches!(e, TaskEvaluator::UseAbility)) {
            continue;
        }
        if !behavior.auto_use_abilities {
            continue;
        }

        let cc_flags = status_effects.combined_cc_flags(&status_registry);
        let engage_target = match engaging {
            Some(e) => e.target,
            None => continue,
        };
        let slots = match slots {
            Some(s) => s,
            None => continue,
        };
        let mana_ref = match mana {
            Some(m) => m,
            None => continue,
        };
        let stamina_ref = match stamina {
            Some(s) => s,
            None => continue,
        };

        let target_pos = match potential_targets.get(engage_target) {
            Ok((_, pos, _, _)) => pos,
            Err(_) => continue,
        };

        let template = match body_templates.get(&body.template_id) {
            Some(t) => t,
            None => continue,
        };
        let self_hp_fraction = 1.0 - body.pain_level(template);

        let mut priorities = behavior.ability_priorities.clone();
        priorities.sort_by(|a, b| b.priority.cmp(&a.priority));

        for prio in &priorities {
            let condition_met = match &prio.condition {
                UseCondition::Always => true,
                UseCondition::SelfHpBelow(threshold) => self_hp_fraction < *threshold,
                UseCondition::TargetHpBelow(_) => true,
                UseCondition::AllyHpBelow(_) => false,
                UseCondition::EnemiesInRange(_) => false,
            };
            if !condition_met {
                continue;
            }

            let ability = match ability_registry.get(prio.ability_id) {
                Some(a) => a,
                None => continue,
            };

            let target_pos_tuple = Some((target_pos.x, target_pos.y));
            if validate_cast(
                ability, slots, prio.slot_index, mana_ref, stamina_ref, &cc_flags,
                (grid_pos.x, grid_pos.y), target_pos_tuple,
            )
            .is_err()
            {
                continue;
            }

            let cast_target = match ability.target_type {
                TargetType::SelfOnly => CastTarget::SelfCast,
                TargetType::SingleEnemy | TargetType::SingleAlly => {
                    CastTarget::Entity(engage_target)
                }
                TargetType::CircleAoE => CastTarget::Position {
                    x: target_pos.x as f32,
                    y: target_pos.y as f32,
                },
                TargetType::ConeAoE | TargetType::LineAoE => {
                    let dx = target_pos.x as f32 - grid_pos.x as f32;
                    let dy = target_pos.y as f32 - grid_pos.y as f32;
                    CastTarget::Direction { dx, dy }
                }
            };

            brain.proposals.push(Task::new(
                "use_ability", 70, TaskSource::Evaluator,
                vec![Action::CastAbility {
                    ability_id: prio.ability_id,
                    slot_index: prio.slot_index,
                    target: cast_target,
                    initiated: false,
                }],
            ));
            break; // First valid ability wins
        }
    }
}
