use bevy::prelude::*;
use rand::RngExt;

use crate::resources::ai::AiState;
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::combat::{
    accuracy_check, apply_damage, calculate_accuracy, calculate_damage, calculate_dodge,
    resolve_hit,
};
use crate::resources::damage::{EquippedArmor, EquippedWeapon};
use crate::resources::events::{AttackMissedEvent, DamageDealtEvent};
use crate::resources::game_time::GameTime;
use crate::resources::map::GridPosition;
use crate::resources::stats::Attributes;
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::threat::ThreatTable;
use crate::systems::spawning::EntityName;

/// Tick weapon cooldowns each simulation tick.
pub fn tick_weapon_cooldowns(
    game_time: Res<GameTime>,
    mut query: Query<&mut EquippedWeapon>,
) {
    for _ in 0..game_time.ticks_this_frame {
        for mut weapon in &mut query {
            weapon.tick();
        }
    }
}

/// Auto-attack system: entities with an Engaging AI state attack their target.
pub fn auto_attack(
    game_time: Res<GameTime>,
    body_templates: Res<BodyTemplates>,
    status_registry: Res<StatusEffectRegistry>,
    mut damage_events: MessageWriter<DamageDealtEvent>,
    mut miss_events: MessageWriter<AttackMissedEvent>,
    mut attackers: Query<(
        Entity,
        &GridPosition,
        &AiState,
        &Attributes,
        &mut EquippedWeapon,
        &ActiveStatusEffects,
        &EntityName,
    )>,
    mut defenders: Query<(
        &GridPosition,
        &mut Body,
        &EquippedArmor,
        &Attributes,
        &mut ThreatTable,
        &EntityName,
    )>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    // Collect attacks to process (avoid borrow conflicts)
    let mut attacks: Vec<(Entity, Entity)> = Vec::new();

    for (attacker_entity, attacker_pos, ai_state, _, weapon, status_effects, _) in &attackers {
        let cc_flags = status_effects.combined_cc_flags(&status_registry);
        if !cc_flags.can_attack() {
            continue;
        }

        let target_entity = match ai_state {
            AiState::Engaging { target } => *target,
            _ => continue,
        };

        if !weapon.is_ready() {
            continue;
        }

        // Range check
        if let Ok((target_pos, _, _, _, _, _)) = defenders.get(target_entity) {
            let dx = attacker_pos.x as f32 - target_pos.x as f32;
            let dy = attacker_pos.y as f32 - target_pos.y as f32;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= weapon.weapon.range {
                attacks.push((attacker_entity, target_entity));
            }
        }
    }

    // Process attacks
    for (attacker_entity, target_entity) in attacks {
        let (_, _, _, attacker_attrs, mut weapon, _, _) =
            attackers.get_mut(attacker_entity).unwrap();

        weapon.start_cooldown();

        let raw_damage = calculate_damage(attacker_attrs, weapon.weapon.base_damage, weapon.weapon.is_melee);
        let accuracy = calculate_accuracy(attacker_attrs, 0.7, 0.0);

        let (_, mut target_body, target_armor, target_attrs, mut threat_table, _) =
            defenders.get_mut(target_entity).unwrap();

        let dodge = calculate_dodge(target_attrs);

        let mut rng = rand::rng();
        let hit_roll: f32 = rng.random();
        let coverage_roll: f32 = rng.random();

        if !accuracy_check(accuracy, dodge, hit_roll) {
            miss_events.write(AttackMissedEvent {
                attacker: attacker_entity,
                target: target_entity,
            });
            continue;
        }

        let template = match body_templates.get(&target_body.template_id) {
            Some(t) => t,
            None => continue,
        };

        let hit = resolve_hit(
            raw_damage,
            weapon.weapon.damage_type,
            template,
            target_armor,
            coverage_roll,
        );

        match hit {
            crate::resources::combat::HitResult::Hit {
                body_part_index,
                damage_after_armor,
                damage_type,
                ..
            } => {
                let result = apply_damage(&mut target_body, template, body_part_index, damage_after_armor);
                let part_name = &template.parts[body_part_index].name;

                damage_events.write(DamageDealtEvent {
                    target: target_entity,
                    amount: result.damage_dealt,
                    damage_type,
                    body_part_name: part_name.clone(),
                    part_destroyed: result.part_destroyed,
                    target_killed: result.target_killed,
                });

                threat_table.add_threat(attacker_entity, result.damage_dealt);
            }
            crate::resources::combat::HitResult::Miss => {
                miss_events.write(AttackMissedEvent {
                    attacker: attacker_entity,
                    target: target_entity,
                });
            }
        }
    }
}

/// Remove dead entities from the game.
pub fn cleanup_dead(
    mut commands: Commands,
    body_templates: Res<BodyTemplates>,
    query: Query<(Entity, &Body, &EntityName)>,
) {
    for (entity, body, name) in &query {
        let template = match body_templates.get(&body.template_id) {
            Some(t) => t,
            None => continue,
        };
        if body.is_dead(template) {
            info!("{} has died!", name.0);
            commands.entity(entity).despawn();
        }
    }
}

/// Tick status effect durations and remove expired ones.
pub fn tick_status_effects(
    game_time: Res<GameTime>,
    mut query: Query<&mut ActiveStatusEffects>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    for mut effects in &mut query {
        for _ in 0..game_time.ticks_this_frame {
            for effect in &mut effects.effects {
                if effect.remaining_ticks > 0 {
                    effect.remaining_ticks -= 1;
                }
            }
        }
        effects.remove_expired();
    }
}
