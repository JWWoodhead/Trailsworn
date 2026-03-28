use bevy::prelude::*;
use rand::RngExt;

use crate::resources::abilities::{
    AbilityDef, AbilityEffect, AbilityRegistry, AbilitySlots, CastTarget, CastingState, Mana,
    Stamina, TargetType,
};
use crate::resources::body::{Body, BodyTemplates};
use crate::resources::casting::{calculate_ability_damage, calculate_ability_heal, AoeParams, resolve_aoe_targets};
use crate::resources::combat::{apply_damage, resolve_hit};
use crate::resources::damage::EquippedArmor;
use crate::resources::events::{AbilityCastEvent, AbilityLandedEvent, CastInterruptedEvent, DamageDealtEvent};
use crate::systems::spawning::EntityName;
use crate::resources::game_time::GameTime;
use crate::resources::map::GridPosition;
use crate::resources::stats::Attributes;
use crate::resources::status_effects::{ActiveStatusEffects, StatusEffectRegistry};
use crate::resources::threat::ThreatTable;

/// Mana regeneration rate: units per tick (0.5 per second at 60Hz).
const MANA_REGEN_PER_TICK: f32 = 0.5 / 60.0;
/// Stamina regeneration rate: units per tick (1.0 per second at 60Hz).
const STAMINA_REGEN_PER_TICK: f32 = 1.0 / 60.0;

/// Tick all ability cooldowns per simulation tick.
pub fn tick_ability_cooldowns(
    game_time: Res<GameTime>,
    mut query: Query<&mut AbilitySlots>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }
    for mut slots in &mut query {
        for _ in 0..game_time.ticks_this_frame {
            slots.tick_cooldowns();
        }
    }
}

/// Regenerate mana and stamina each tick.
pub fn regenerate_resources(
    game_time: Res<GameTime>,
    mut query: Query<(&mut Mana, &mut Stamina)>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }
    for (mut mana, mut stamina) in &mut query {
        for _ in 0..game_time.ticks_this_frame {
            mana.regenerate(MANA_REGEN_PER_TICK);
            stamina.regenerate(STAMINA_REGEN_PER_TICK);
        }
    }
}

/// Process newly-added CastingState components: spend resources and start cooldowns.
/// For instant casts (cast_time == 0), resolve effects immediately.
pub fn begin_cast(
    mut commands: Commands,
    ability_registry: Res<AbilityRegistry>,
    status_registry: Res<StatusEffectRegistry>,
    body_templates: Res<BodyTemplates>,
    mut damage_events: MessageWriter<DamageDealtEvent>,
    mut cast_events: MessageWriter<AbilityCastEvent>,
    mut landed_events: MessageWriter<AbilityLandedEvent>,
    mut casters: Query<
        (
            Entity,
            &CastingState,
            &mut AbilitySlots,
            &mut Mana,
            &mut Stamina,
            &Attributes,
            &GridPosition,
        ),
        Added<CastingState>,
    >,
    mut targets: Query<(
        Entity,
        &GridPosition,
        &mut Body,
        &EquippedArmor,
        &mut ActiveStatusEffects,
        &mut ThreatTable,
    )>,
    names: Query<&EntityName>,
) {
    for (caster_entity, casting, mut slots, mut mana, mut stamina, attributes, caster_pos) in &mut casters {
        let ability = match ability_registry.get(casting.ability_id) {
            Some(a) => a.clone(),
            None => {
                commands.entity(caster_entity).remove::<CastingState>();
                continue;
            }
        };

        // Spend resources
        if ability.mana_cost > 0 && !mana.spend(ability.mana_cost as f32) {
            commands.entity(caster_entity).remove::<CastingState>();
            continue;
        }
        if ability.stamina_cost > 0 && !stamina.spend(ability.stamina_cost as f32) {
            // Refund mana if stamina check fails
            mana.regenerate(ability.mana_cost as f32);
            commands.entity(caster_entity).remove::<CastingState>();
            continue;
        }

        // Start cooldown
        slots.start_cooldown(casting.slot_index, ability.cooldown_ticks);

        // Fire cast event
        let target_desc = match &casting.target {
            CastTarget::Entity(e) => names.get(*e).map(|n| n.0.clone()).unwrap_or_else(|_| "???".into()),
            CastTarget::SelfCast => "self".into(),
            CastTarget::Position { x, y } => format!("({:.0}, {:.0})", x, y),
            CastTarget::Direction { .. } => "area".into(),
        };
        cast_events.write(AbilityCastEvent {
            caster: caster_entity,
            ability_name: ability.name.clone(),
            target_description: target_desc,
            ability_id: Some(ability.id),
        });

        // Instant cast: resolve immediately
        if ability.cast_time_ticks == 0 {
            resolve_ability_effects(
                caster_entity,
                &ability,
                &casting.target,
                attributes,
                caster_pos,
                &body_templates,
                &status_registry,
                &mut damage_events,
                &mut targets,
            );
            let impact_pos = resolve_impact_position(&casting.target, caster_pos, &targets);
            landed_events.write(AbilityLandedEvent {
                caster: caster_entity,
                ability_id: ability.id,
                position: impact_pos,
                impact_vfx_scale: ability.impact_vfx_scale,
            });
            commands.entity(caster_entity).remove::<CastingState>();
        }
    }
}

/// Count down cast timers and resolve effects when casting completes.
pub fn tick_casting(
    mut commands: Commands,
    game_time: Res<GameTime>,
    ability_registry: Res<AbilityRegistry>,
    status_registry: Res<StatusEffectRegistry>,
    body_templates: Res<BodyTemplates>,
    mut damage_events: MessageWriter<DamageDealtEvent>,
    mut landed_events: MessageWriter<AbilityLandedEvent>,
    mut casters: Query<(
        Entity,
        &mut CastingState,
        &Attributes,
        &GridPosition,
    )>,
    mut targets: Query<(
        Entity,
        &GridPosition,
        &mut Body,
        &EquippedArmor,
        &mut ActiveStatusEffects,
        &mut ThreatTable,
    )>,
) {
    if game_time.ticks_this_frame == 0 {
        return;
    }

    for (caster_entity, mut casting, attributes, caster_pos) in &mut casters {
        // Skip newly-added (handled by begin_cast)
        if casting.remaining_ticks == 0 && casting.is_added() {
            continue;
        }

        for _ in 0..game_time.ticks_this_frame {
            if casting.remaining_ticks > 0 {
                casting.remaining_ticks -= 1;
            }
        }

        if casting.remaining_ticks == 0 {
            let ability = match ability_registry.get(casting.ability_id) {
                Some(a) => a.clone(),
                None => {
                    commands.entity(caster_entity).remove::<CastingState>();
                    continue;
                }
            };

            resolve_ability_effects(
                caster_entity,
                &ability,
                &casting.target,
                attributes,
                caster_pos,
                &body_templates,
                &status_registry,
                &mut damage_events,
                &mut targets,
            );
            let impact_pos = resolve_impact_position(&casting.target, caster_pos, &targets);
            landed_events.write(AbilityLandedEvent {
                caster: caster_entity,
                ability_id: ability.id,
                position: impact_pos,
                impact_vfx_scale: ability.impact_vfx_scale,
            });
            commands.entity(caster_entity).remove::<CastingState>();
        }
    }
}

/// Interrupt casts when the caster takes damage (if the ability is interruptible).
pub fn interrupt_casting(
    mut commands: Commands,
    ability_registry: Res<AbilityRegistry>,
    mut damage_events: MessageReader<DamageDealtEvent>,
    mut interrupt_events: MessageWriter<CastInterruptedEvent>,
    query: Query<&CastingState>,
) {
    for event in damage_events.read() {
        if let Ok(casting) = query.get(event.target) {
            if let Some(ability) = ability_registry.get(casting.ability_id) {
                if ability.interruptible {
                    interrupt_events.write(CastInterruptedEvent {
                        caster: event.target,
                        ability_id: casting.ability_id,
                    });
                    commands.entity(event.target).remove::<CastingState>();
                }
            }
        }
    }
}

/// Resolve all effects of an ability on its target(s).
fn resolve_ability_effects(
    caster_entity: Entity,
    ability: &AbilityDef,
    cast_target: &CastTarget,
    caster_attrs: &Attributes,
    caster_pos: &GridPosition,
    body_templates: &BodyTemplates,
    status_registry: &StatusEffectRegistry,
    damage_events: &mut MessageWriter<DamageDealtEvent>,
    targets: &mut Query<(
        Entity,
        &GridPosition,
        &mut Body,
        &EquippedArmor,
        &mut ActiveStatusEffects,
        &mut ThreatTable,
    )>,
) {
    let target_entities = resolve_cast_targets(caster_entity, ability, cast_target, caster_pos, targets);

    for target_entity in target_entities {
        // Skip self-damage for AoE
        if target_entity == caster_entity
            && matches!(
                ability.target_type,
                TargetType::CircleAoE | TargetType::ConeAoE | TargetType::LineAoE
            )
        {
            continue;
        }

        for effect in &ability.effects {
            match effect {
                AbilityEffect::Damage { damage_type, .. } => {
                    let raw_damage = calculate_ability_damage(effect, caster_attrs);
                    if raw_damage <= 0.0 {
                        continue;
                    }

                    if let Ok((_, _, mut body, armor, _, mut threat_table)) =
                        targets.get_mut(target_entity)
                    {
                        let template = match body_templates.get(&body.template_id) {
                            Some(t) => t,
                            None => continue,
                        };

                        let mut rng = rand::rng();
                        let coverage_roll: f32 = rng.random();

                        let hit = resolve_hit(raw_damage, *damage_type, template, armor, coverage_roll);
                        match hit {
                            crate::resources::combat::HitResult::Hit {
                                body_part_index,
                                damage_after_armor,
                                damage_type: dt,
                                ..
                            } => {
                                let result = apply_damage(&mut body, template, body_part_index, damage_after_armor);
                                let part_name = &template.parts[body_part_index].name;

                                damage_events.write(DamageDealtEvent {
                                    attacker: caster_entity,
                                    target: target_entity,
                                    amount: result.damage_dealt,
                                    damage_type: dt,
                                    body_part_name: part_name.clone(),
                                    part_destroyed: result.part_destroyed,
                                    target_killed: result.target_killed,
                                    ability_name: Some(ability.name.clone()),
                                    ability_id: Some(ability.id),
                                });

                                threat_table.add_threat(caster_entity, result.damage_dealt);
                            }
                            crate::resources::combat::HitResult::Miss => {}
                        }
                    }
                }
                AbilityEffect::Heal { .. } => {
                    let heal_amount = calculate_ability_heal(effect, caster_attrs);
                    if heal_amount <= 0.0 {
                        continue;
                    }

                    if let Ok((_, _, mut body, _, _, _)) = targets.get_mut(target_entity) {
                        let template = match body_templates.get(&body.template_id) {
                            Some(t) => t,
                            None => continue,
                        };
                        body.heal_distributed(heal_amount, template);
                    }
                }
                AbilityEffect::ApplyStatus {
                    status_id,
                    duration_ticks,
                    chance,
                } => {
                    let mut rng = rand::rng();
                    let roll: f32 = rng.random();
                    if roll < *chance {
                        if let Ok((_, _, _, _, mut effects, _)) = targets.get_mut(target_entity) {
                            effects.apply(*status_id, *duration_ticks, Some(caster_entity), status_registry);
                        }
                    }
                }
                AbilityEffect::GenerateThreat { amount } => {
                    if let Ok((_, _, _, _, _, mut threat_table)) = targets.get_mut(target_entity) {
                        threat_table.add_threat(caster_entity, *amount);
                    }
                }
                AbilityEffect::Knockback { .. } => {
                    // Deferred — requires pathfinding work
                }
            }
        }
    }
}

/// Determine which entities an ability should affect.
fn resolve_cast_targets(
    caster_entity: Entity,
    ability: &AbilityDef,
    cast_target: &CastTarget,
    caster_pos: &GridPosition,
    targets: &Query<(
        Entity,
        &GridPosition,
        &mut Body,
        &EquippedArmor,
        &mut ActiveStatusEffects,
        &mut ThreatTable,
    )>,
) -> Vec<Entity> {
    match &ability.target_type {
        TargetType::SelfOnly => {
            vec![caster_entity]
        }
        TargetType::SingleEnemy | TargetType::SingleAlly => match cast_target {
            CastTarget::Entity(e) => vec![*e],
            _ => Vec::new(),
        },
        TargetType::CircleAoE | TargetType::ConeAoE | TargetType::LineAoE => {
            let target_pos = match cast_target {
                CastTarget::Position { x, y } => (*x, *y),
                CastTarget::Entity(e) => {
                    if let Ok((_, pos, _, _, _, _)) = targets.get(*e) {
                        (pos.x as f32, pos.y as f32)
                    } else {
                        return Vec::new();
                    }
                }
                CastTarget::Direction { dx, dy } => {
                    (caster_pos.x as f32 + dx, caster_pos.y as f32 + dy)
                }
                CastTarget::SelfCast => (caster_pos.x as f32, caster_pos.y as f32),
            };

            let caster_f = (caster_pos.x as f32, caster_pos.y as f32);

            let mut candidate_entities: Vec<Entity> = Vec::new();
            let mut candidate_positions: Vec<(f32, f32)> = Vec::new();

            for (entity, pos, _, _, _, _) in targets.iter() {
                candidate_entities.push(entity);
                candidate_positions.push((pos.x as f32, pos.y as f32));
            }

            let params = AoeParams {
                aoe_radius: ability.aoe_radius,
                cone_half_angle: ability.cone_half_angle,
                aoe_length: ability.aoe_length,
                aoe_width: ability.aoe_width,
            };

            let hit_indices = resolve_aoe_targets(
                caster_f,
                &ability.target_type,
                target_pos,
                &params,
                &candidate_positions,
            );

            hit_indices
                .into_iter()
                .map(|i| candidate_entities[i])
                .collect()
        }
    }
}

/// Resolve the world-space impact position from a CastTarget.
/// Used to place the AbilityLandedEvent at the right spot.
fn resolve_impact_position(
    cast_target: &CastTarget,
    caster_pos: &GridPosition,
    targets: &Query<(
        Entity,
        &GridPosition,
        &mut Body,
        &EquippedArmor,
        &mut ActiveStatusEffects,
        &mut ThreatTable,
    )>,
) -> (f32, f32) {
    match cast_target {
        CastTarget::SelfCast => (caster_pos.x as f32, caster_pos.y as f32),
        CastTarget::Position { x, y } => (*x, *y),
        CastTarget::Direction { dx, dy } => (caster_pos.x as f32 + dx, caster_pos.y as f32 + dy),
        CastTarget::Entity(e) => {
            if let Ok((_, pos, _, _, _, _)) = targets.get(*e) {
                (pos.x as f32, pos.y as f32)
            } else {
                (caster_pos.x as f32, caster_pos.y as f32)
            }
        }
    }
}
