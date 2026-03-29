use bevy::prelude::*;

use crate::resources::abilities::{
    AbilityRegistry, AbilitySlots, CastTarget, Mana, Stamina, TargetType,
};
use crate::resources::combat_behavior::{CombatBehavior, UseCondition};
use crate::resources::body::{Body, BodyTemplates, Health};
use crate::resources::casting::validate_cast;
use crate::resources::combat::{Dead, InCombat};
use crate::resources::faction::{Faction, FactionRelations};
use crate::resources::map::{GridPosition, TileWorld};
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::task::{Action, AiBrain, Engaging, Task, TaskEvaluator, TaskSource};

/// Propose casting an ability when conditions are met.
pub fn use_ability(
    ability_registry: Res<AbilityRegistry>,
    body_templates: Res<BodyTemplates>,
    status_registry: Res<StatusEffectRegistry>,
    tile_world: Res<TileWorld>,
    faction_relations: Res<FactionRelations>,
    mut query: Query<(
        Entity,
        &GridPosition,
        &Faction,
        &CombatBehavior,
        &Body,
        &Health,
        &ActiveStatusEffects,
        &mut AiBrain,
        Option<&Engaging>,
        Option<&AbilitySlots>,
        Option<&Mana>,
        Option<&Stamina>,
    )>,
    potential_targets: Query<(Entity, &GridPosition, &Faction, &Body, &Health), (With<InCombat>, Without<Dead>)>,
    allies: Query<(Entity, &GridPosition, &Faction, &Body, &Health), Without<Dead>>,
) {
    for (self_entity, grid_pos, self_faction, behavior, body, health, status_effects, mut brain, engaging, slots, mana, stamina) in &mut query {
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
            Ok((_, pos, _, _, _)) => pos,
            Err(_) => continue,
        };

        let _template = match body_templates.get(&body.template_id) {
            Some(t) => t,
            None => continue,
        };
        let self_hp_fraction = health.fraction();

        let mut priorities = behavior.ability_priorities.clone();
        priorities.sort_by(|a, b| b.priority.cmp(&a.priority));

        for prio in &priorities {
            let condition_met = match &prio.condition {
                UseCondition::Always => true,
                UseCondition::SelfHpBelow(threshold) => self_hp_fraction < *threshold,
                UseCondition::TargetHpBelow(threshold) => {
                    potential_targets
                        .get(engage_target)
                        .ok()
                        .map(|(_, _, _, _, target_hp)| target_hp.fraction() < *threshold)
                        .unwrap_or(false)
                }
                UseCondition::AllyHpBelow(threshold) => {
                    allies.iter().any(|(e, _, f, _, ally_hp)| {
                        if e == self_entity || f.0 != self_faction.0 {
                            return false;
                        }
                        ally_hp.fraction() < *threshold
                    })
                }
                UseCondition::EnemiesInRange(min_count) => {
                    let count = potential_targets
                        .iter()
                        .filter(|(e, pos, f, _, _)| {
                            *e != self_entity
                                && faction_relations.is_hostile(self_faction.0, f.0)
                                && {
                                    let dx = grid_pos.x.abs_diff(pos.x);
                                    let dy = grid_pos.y.abs_diff(pos.y);
                                    (dx * dx + dy * dy) <= (behavior.aggro_range as u32).pow(2)
                                }
                        })
                        .count() as u32;
                    count >= *min_count
                }
            };
            if !condition_met {
                continue;
            }

            let ability = match ability_registry.get(prio.ability_id) {
                Some(a) => a,
                None => continue,
            };

            let target_pos_tuple = Some((target_pos.x, target_pos.y));
            let los_data = Some((tile_world.width, tile_world.blocks_los.as_slice()));
            if validate_cast(
                ability, slots, prio.slot_index, mana_ref, stamina_ref, &cc_flags,
                (grid_pos.x, grid_pos.y), target_pos_tuple, los_data,
            )
            .is_err()
            {
                continue;
            }

            let cast_target = match ability.target_type {
                TargetType::SelfOnly => CastTarget::SelfCast,
                TargetType::SingleEnemy => CastTarget::Entity(engage_target),
                TargetType::SingleAlly => {
                    // Find the most wounded ally (including self) within ability range
                    let best_ally = allies
                        .iter()
                        .filter(|(_, _, f, _, _)| f.0 == self_faction.0)
                        .filter_map(|(e, ally_pos, _, _, ally_hp)| {
                            let damage = ally_hp.max - ally_hp.current;
                            if damage <= 0.0 {
                                return None;
                            }
                            let dx = grid_pos.x as f32 - ally_pos.x as f32;
                            let dy = grid_pos.y as f32 - ally_pos.y as f32;
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist > ability.range {
                                return None;
                            }
                            Some((e, damage))
                        })
                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    match best_ally {
                        Some((ally_entity, _)) => CastTarget::Entity(ally_entity),
                        None => continue,
                    }
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
