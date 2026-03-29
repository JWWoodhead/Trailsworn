use bevy::prelude::*;
use rand::RngExt;

use std::f32::consts::FRAC_PI_2;

use crate::resources::abilities::CastingState;
use crate::resources::body::{Body, BodyTemplates, Health};
use crate::resources::combat::{
    Dead, InCombat, accuracy_check, apply_damage, calculate_accuracy, calculate_damage,
    calculate_dodge, resolve_hit,
};
use crate::resources::map::render_layers;
use crate::resources::movement::MovePath;
use crate::resources::task::{CurrentTask, Engaging};
use crate::resources::vfx::HitFlash;
use crate::pathfinding::has_line_of_sight;
use crate::resources::damage::{EquippedArmor, EquippedWeapon};
use crate::resources::events::{AttackMissedEvent, DamageDealtEvent};
use crate::resources::game_time::GameTime;
use crate::resources::map::{GridPosition, TileWorld};
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

/// Auto-attack system: entities with an EngageTarget action attack their target.
pub fn auto_attack(
    game_time: Res<GameTime>,
    tile_world: Res<TileWorld>,
    body_templates: Res<BodyTemplates>,
    status_registry: Res<StatusEffectRegistry>,
    mut damage_events: MessageWriter<DamageDealtEvent>,
    mut miss_events: MessageWriter<AttackMissedEvent>,
    mut attackers: Query<(
        Entity,
        &GridPosition,
        &Engaging,
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
        &mut Health,
    ), Without<Dead>>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    // Collect attacks to process (avoid borrow conflicts)
    let mut attacks: Vec<(Entity, Entity)> = Vec::new();

    for (attacker_entity, attacker_pos, engaging, _, weapon, status_effects, _) in &attackers {
        let cc_flags = status_effects.combined_cc_flags(&status_registry);
        if !cc_flags.can_attack() {
            continue;
        }

        let target_entity = engaging.target;

        if !weapon.is_ready() {
            continue;
        }

        // Range check
        if let Ok((target_pos, _, _, _, _, _, _)) = defenders.get(target_entity) {
            let dx = attacker_pos.x as f32 - target_pos.x as f32;
            let dy = attacker_pos.y as f32 - target_pos.y as f32;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= weapon.weapon.range {
                // LOS check for ranged weapons
                if !weapon.weapon.is_melee && !has_line_of_sight(
                    (attacker_pos.x, attacker_pos.y),
                    (target_pos.x, target_pos.y),
                    tile_world.width,
                    &tile_world.blocks_los,
                ) {
                    continue;
                }
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

        let (_, mut target_body, target_armor, target_attrs, mut threat_table, _, mut health) =
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
                    attacker: attacker_entity,
                    target: target_entity,
                    amount: result.damage_dealt,
                    damage_type,
                    body_part_name: part_name.clone(),
                    part_destroyed: result.part_destroyed,
                    target_killed: result.target_killed,
                    ability_name: None,
                    ability_id: None,
                });

                threat_table.add_threat(attacker_entity, result.damage_dealt);
                health.take_damage(result.damage_dealt);
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

/// Turn dead entities into corpses. Records zone kills, cleans up equipment,
/// then leaves the entity as a rotated, greyed-out sprite on the ground.
pub fn cleanup_dead(
    mut commands: Commands,
    body_templates: Res<BodyTemplates>,
    current_zone: Res<crate::resources::world::CurrentZone>,
    mut zone_cache: ResMut<crate::resources::zone_persistence::ZoneStateCache>,
    mut instance_registry: ResMut<crate::resources::items::ItemInstanceRegistry>,
    mut query: Query<(
        Entity,
        &Body,
        &EntityName,
        &Health,
        &mut Sprite,
        &mut Transform,
        Option<&crate::resources::zone_persistence::ZoneSpawnIndex>,
        Option<&crate::resources::items::Equipment>,
    ), Without<Dead>>,
) {
    for (entity, body, name, health, mut sprite, mut transform, spawn_idx, equipment) in &mut query {
        let template = match body_templates.get(&body.template_id) {
            Some(t) => t,
            None => continue,
        };
        if body.is_dead(template) || health.is_dead() {
            info!("{} has died!", name.0);

            // Track zone kill for persistence
            if let Some(idx) = spawn_idx {
                let snapshot = zone_cache.get_or_create_mut(current_zone.world_pos);
                snapshot.dead_indices.insert(idx.0);
                snapshot.alive_overrides.remove(&idx.0);
            }

            // Clean up ItemInstances to prevent orphan leak
            if let Some(eq) = equipment {
                for (_, instance_id) in &eq.slots {
                    instance_registry.remove(*instance_id);
                }
            }

            // Turn into a corpse: rotate, grey out, lower to ground layer
            sprite.color = Color::srgb(0.4, 0.4, 0.4);
            transform.rotation = Quat::from_rotation_z(FRAC_PI_2);
            transform.translation.z = render_layers::FLOOR_ITEMS;

            // Disable all combat/movement, mark as dead
            let mut ec = commands.entity(entity);
            ec.insert(Dead);
            ec.remove::<InCombat>();
            ec.remove::<Engaging>();
            ec.remove::<CurrentTask>();
            ec.remove::<CastingState>();
            ec.remove::<MovePath>();
            ec.remove::<HitFlash>();
        }
    }
}

/// Tick status effect durations, apply DoT/HoT, and remove expired ones.
pub fn tick_status_effects(
    game_time: Res<GameTime>,
    status_registry: Res<StatusEffectRegistry>,
    body_templates: Res<BodyTemplates>,
    mut query: Query<(&mut ActiveStatusEffects, &mut Body, &mut Health), Without<Dead>>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    for (mut effects, mut body, mut health) in &mut query {
        for _ in 0..game_time.ticks_this_frame {
            for effect in &mut effects.effects {
                if effect.remaining_ticks > 0 {
                    effect.remaining_ticks -= 1;
                }

                // Process tick effects (DoT/HoT)
                if effect.tick_timer > 0 {
                    effect.tick_timer -= 1;
                }
                if effect.tick_timer == 0 {
                    if let Some(def) = status_registry.get(effect.status_id) {
                        if def.tick_interval_ticks > 0 {
                            effect.tick_timer = def.tick_interval_ticks;

                            if let Some(tick) = &def.tick_effect {
                                let amount = tick.amount * effect.stacks as f32;
                                if tick.is_heal {
                                    if let Some(template) = body_templates.get(&body.template_id) {
                                        body.heal_distributed(amount, template);
                                        health.heal(amount);
                                    }
                                } else {
                                    // DoT: apply as direct HP damage (bypasses armor)
                                    health.take_damage(amount);
                                }
                            }
                        }
                    }
                }
            }
        }
        effects.remove_expired();
    }
}
