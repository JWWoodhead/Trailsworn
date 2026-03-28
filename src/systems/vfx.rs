use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::resources::abilities::AbilityRegistry;
use crate::resources::audio::{AudioAssets, SfxKind};
use crate::resources::damage::EquippedWeapon;
use crate::resources::events::{AbilityLandedEvent, AttackMissedEvent, CastInterruptedEvent, DamageDealtEvent, HealEvent};
use crate::resources::map::MapSettings;
use crate::resources::particles::ParticleAssets;
use crate::resources::map::render_layers;
use crate::resources::vfx::{
    AttackLunge, DespawnTimer, HitFlash, ImpactKind, Projectile, ScreenTrauma,
    PROJECTILE_DEFAULT_SPEED, PROJECTILE_SIZE,
};

// ---------------------------------------------------------------------------
// Spawn systems — read combat Messages, create effect components/entities
// ---------------------------------------------------------------------------

/// React to damage and miss events: lunge, flash, impact sprite, screen shake, audio.
pub fn spawn_combat_effects(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageDealtEvent>,
    mut miss_events: MessageReader<AttackMissedEvent>,
    ability_registry: Res<AbilityRegistry>,
    audio_assets: Res<AudioAssets>,
    particle_assets: Res<ParticleAssets>,
    map_settings: Res<MapSettings>,
    mut screen_trauma: ResMut<ScreenTrauma>,
    transforms: Query<&Transform>,
    sprites: Query<&Sprite>,
    weapons: Query<&EquippedWeapon>,
) {
    for event in damage_events.read() {
        let target_pos = transforms.get(event.target).map(|t| t.translation);
        let attacker_pos = transforms.get(event.attacker).map(|t| t.translation);

        // Attack lunge on the attacker
        if let (Ok(a_pos), Ok(t_pos)) = (attacker_pos, target_pos) {
            let direction = Vec2::new(t_pos.x - a_pos.x, t_pos.y - a_pos.y);
            if direction.length_squared() > 0.1 {
                commands.entity(event.attacker).insert(AttackLunge::new(direction));
            }
        }

        // Hit flash on the target
        if let Ok(target_entity) = sprites.get(event.target) {
            commands.entity(event.target).insert(HitFlash::new(target_entity.color));
        }

        // Particle impact effect at target position
        if let Ok(t_pos) = target_pos {
            // Two-tier VFX lookup: ability override → damage-type default
            let vfx_kind = if let Some(ability_id) = event.ability_id {
                ability_registry
                    .get(ability_id)
                    .and_then(|a| a.impact_vfx)
                    .unwrap_or_else(|| {
                        ImpactKind::from_damage_type(event.damage_type).default_vfx()
                    })
            } else {
                ImpactKind::from_damage_type(event.damage_type).default_vfx()
            };

            if let Some(effect_handle) = particle_assets.get(vfx_kind) {
                commands.spawn((
                    ParticleEffect::new(effect_handle.clone()),
                    Transform::from_translation(Vec3::new(t_pos.x, t_pos.y, 4.5)),
                    DespawnTimer::new(2.0),
                ));
            }
        }

        // Screen shake
        if event.target_killed {
            screen_trauma.add(crate::resources::vfx::SHAKE_TRAUMA_KILL);
        } else if event.part_destroyed {
            screen_trauma.add(crate::resources::vfx::SHAKE_TRAUMA_DESTROY);
        } else {
            screen_trauma.add(crate::resources::vfx::SHAKE_TRAUMA_HIT);
        }

        // Audio — use data-driven SFX from ability or weapon, with generic fallbacks
        let sfx = if let Some(ability_id) = event.ability_id {
            ability_registry
                .get(ability_id)
                .and_then(|a| a.impact_sfx)
                .unwrap_or(SfxKind::SpellImpact)
        } else {
            // Auto-attack: read weapon's attack_sfx
            weapons
                .get(event.attacker)
                .ok()
                .and_then(|w| w.weapon.attack_sfx)
                .unwrap_or(SfxKind::MeleeHit)
        };
        if let Some(handle) = audio_assets.get(sfx) {
            commands.spawn((
                AudioPlayer(handle.clone()),
                PlaybackSettings::DESPAWN,
            ));
        }

        // Death audio
        if event.target_killed {
            if let Some(handle) = audio_assets.get(SfxKind::Death) {
                commands.spawn((
                    AudioPlayer(handle.clone()),
                    PlaybackSettings::DESPAWN,
                ));
            }
        }

        // Cosmetic projectile for ranged attacks
        let is_ranged = if let Some(ability_id) = event.ability_id {
            ability_registry
                .get(ability_id)
                .map(|a| a.range > 2.0)
                .unwrap_or(false)
        } else {
            weapons
                .get(event.attacker)
                .map(|w| !w.weapon.is_melee)
                .unwrap_or(false)
        };
        if is_ranged {
            if let (Ok(a_pos), Ok(t_pos)) = (attacker_pos, target_pos) {
                let start = Vec2::new(a_pos.x, a_pos.y);
                let end = Vec2::new(t_pos.x, t_pos.y);
                let color = ImpactKind::from_damage_type(event.damage_type).color();
                let speed = weapons
                    .get(event.attacker)
                    .ok()
                    .filter(|w| w.weapon.projectile_speed > 0.0)
                    .map(|w| w.weapon.projectile_speed * map_settings.tile_size)
                    .unwrap_or(PROJECTILE_DEFAULT_SPEED);

                commands.spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::splat(PROJECTILE_SIZE)),
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(
                        start.x,
                        start.y,
                        render_layers::PROJECTILES,
                    )),
                    Projectile {
                        start,
                        end,
                        speed,
                        progress: 0.0,
                    },
                ));
            }
        }
    }

    for event in miss_events.read() {
        // Lunge on miss too (the attacker still swings)
        let target_pos = transforms.get(event.target).map(|t| t.translation);
        let attacker_pos = transforms.get(event.attacker).map(|t| t.translation);
        if let (Ok(a_pos), Ok(t_pos)) = (attacker_pos, target_pos) {
            let direction = Vec2::new(t_pos.x - a_pos.x, t_pos.y - a_pos.y);
            if direction.length_squared() > 0.1 {
                commands.entity(event.attacker).insert(AttackLunge::new(direction));
            }
        }

        // Miss audio
        if let Some(handle) = audio_assets.get(SfxKind::MeleeMiss) {
            commands.spawn((
                AudioPlayer(handle.clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
    }
}

/// React to ability cast events: audio.
pub fn spawn_cast_effects(
    mut commands: Commands,
    mut cast_events: MessageReader<crate::resources::events::AbilityCastEvent>,
    ability_registry: Res<AbilityRegistry>,
    audio_assets: Res<AudioAssets>,
) {
    for event in cast_events.read() {
        let sfx = event
            .ability_id
            .and_then(|id| ability_registry.get(id))
            .and_then(|a| a.cast_sfx)
            .unwrap_or(SfxKind::SpellCast);
        if let Some(handle) = audio_assets.get(sfx) {
            commands.spawn((
                AudioPlayer(handle.clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
    }
}

/// React to cast interrupt events: audio.
pub fn spawn_interrupt_effects(
    mut commands: Commands,
    mut interrupt_events: MessageReader<CastInterruptedEvent>,
    audio_assets: Res<AudioAssets>,
) {
    for _event in interrupt_events.read() {
        if let Some(handle) = audio_assets.get(SfxKind::CastInterrupt) {
            commands.spawn((
                AudioPlayer(handle.clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
    }
}

/// React to ability landed events: spawn impact VFX at the landing position.
/// This fires once per ability resolution, not per target — so AoE spells get
/// one big particle burst at the center rather than per-target hits.
pub fn spawn_ability_landed_effects(
    mut commands: Commands,
    mut landed_events: MessageReader<AbilityLandedEvent>,
    ability_registry: Res<AbilityRegistry>,
    particle_assets: Res<ParticleAssets>,
    map_settings: Res<MapSettings>,
) {
    let ts = map_settings.tile_size;

    for event in landed_events.read() {
        let vfx_kind = ability_registry
            .get(event.ability_id)
            .and_then(|a| a.impact_vfx);

        let vfx_kind = match vfx_kind {
            Some(v) => v,
            None => continue, // No VFX for this ability
        };

        if let Some(effect_handle) = particle_assets.get(vfx_kind) {
            // Convert tile position to world position
            let world_x = event.position.0 * ts;
            let world_y = event.position.1 * ts;
            let scale = event.impact_vfx_scale.max(0.1);

            commands.spawn((
                ParticleEffect::new(effect_handle.clone()),
                Transform::from_translation(Vec3::new(world_x, world_y, 4.5))
                    .with_scale(Vec3::splat(scale)),
                DespawnTimer::new(2.0),
            ));
        }
    }
}

/// Spawn heal VFX and audio when healing is applied.
pub fn spawn_heal_effects(
    mut commands: Commands,
    mut heal_events: MessageReader<HealEvent>,
    ability_registry: Res<AbilityRegistry>,
    audio_assets: Res<AudioAssets>,
    particle_assets: Res<ParticleAssets>,
    transforms: Query<&Transform>,
) {
    for event in heal_events.read() {
        let Ok(t_pos) = transforms.get(event.target).map(|t| t.translation) else {
            continue;
        };

        // Particles: use ability impact_vfx or fall back to generic heal
        let vfx_kind = event
            .ability_id
            .and_then(|id| ability_registry.get(id))
            .and_then(|a| a.impact_vfx)
            .unwrap_or(crate::resources::particles::VfxKind::ImpactHeal);

        if let Some(effect_handle) = particle_assets.get(vfx_kind) {
            commands.spawn((
                ParticleEffect::new(effect_handle.clone()),
                Transform::from_translation(Vec3::new(t_pos.x, t_pos.y, 4.5)),
                DespawnTimer::new(2.0),
            ));
        }

        // Audio: use ability impact_sfx or fall back to HealLand
        let sfx = event
            .ability_id
            .and_then(|id| ability_registry.get(id))
            .and_then(|a| a.impact_sfx)
            .unwrap_or(SfxKind::HealLand);
        if let Some(handle) = audio_assets.get(sfx) {
            commands.spawn((AudioPlayer(handle.clone()), PlaybackSettings::DESPAWN));
        }
    }
}

// ---------------------------------------------------------------------------
// Tick systems — advance animations, clean up expired effects
// ---------------------------------------------------------------------------

/// Advance attack lunge progress. Remove when done.
pub fn tick_attack_lunge(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AttackLunge)>,
) {
    let dt = time.delta_secs();
    for (entity, mut lunge) in &mut query {
        lunge.progress += dt / lunge.duration;
        if lunge.is_done() {
            commands.entity(entity).remove::<AttackLunge>();
        }
    }
}

/// Tick hit flash timer. Override sprite to white while active, restore on expiry.
pub fn tick_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut HitFlash, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut flash, mut sprite) in &mut query {
        flash.timer -= dt;
        if flash.is_done() {
            // Restore original color
            sprite.color = flash.original_color;
            commands.entity(entity).remove::<HitFlash>();
        } else {
            // Flash white
            sprite.color = Color::WHITE;
        }
    }
}

/// Advance cosmetic projectiles and despawn on arrival.
pub fn tick_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Projectile, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut proj, mut transform) in &mut query {
        let total_dist = (proj.end - proj.start).length();
        if total_dist < 0.1 {
            commands.entity(entity).despawn();
            continue;
        }
        proj.progress += (proj.speed * dt) / total_dist;
        if proj.progress >= 1.0 {
            commands.entity(entity).despawn();
        } else {
            let pos = proj.start.lerp(proj.end, proj.progress);
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }
}

/// Decay screen trauma and apply camera shake offset.
pub fn tick_screen_trauma(
    time: Res<Time>,
    mut screen_trauma: ResMut<ScreenTrauma>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    let dt = time.delta_secs();
    let offset = screen_trauma.tick(dt);

    for mut transform in &mut camera_query {
        // We apply the offset additively. Since camera_pan sets the translation
        // each frame based on its own state, we just add our shake on top.
        // This works because camera_pan runs in Input, and we run in Render.
        transform.translation.x += offset.x;
        transform.translation.y += offset.y;
    }
}

/// Tick all DespawnTimer components and despawn expired entities.
pub fn cleanup_despawn_timers(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DespawnTimer)>,
) {
    let dt = time.delta_secs();
    for (entity, mut timer) in &mut query {
        timer.remaining -= dt;
        if timer.is_done() {
            commands.entity(entity).despawn();
        }
    }
}
