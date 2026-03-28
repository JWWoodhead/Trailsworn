use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use super::particles::{ParticleAssets, VfxKind};

// Alias to avoid ambiguity with bevy::prelude::Gradient
type HanabiGradient<T> = bevy_hanabi::Gradient<T>;

/// Build all particle EffectAssets and store handles in ParticleAssets.
/// Called once at startup.
pub fn setup_particle_effects(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let mut assets = ParticleAssets::default();

    // Generic damage-type impacts
    assets.handles.insert(VfxKind::ImpactSlash, effects.add(slash_impact()));
    assets.handles.insert(VfxKind::ImpactPierce, effects.add(pierce_impact()));
    assets.handles.insert(VfxKind::ImpactBlunt, effects.add(blunt_impact()));
    assets.handles.insert(VfxKind::ImpactFire, effects.add(fire_impact(16, 5.0)));
    assets.handles.insert(VfxKind::ImpactFrost, effects.add(frost_impact(14, 4.0)));
    assets.handles.insert(VfxKind::ImpactStorm, effects.add(storm_impact()));
    assets.handles.insert(VfxKind::ImpactArcane, effects.add(arcane_impact()));
    assets.handles.insert(VfxKind::ImpactHoly, effects.add(holy_impact()));
    assets.handles.insert(VfxKind::ImpactShadow, effects.add(shadow_impact()));
    assets.handles.insert(VfxKind::ImpactNature, effects.add(nature_impact()));
    assets.handles.insert(VfxKind::ImpactHeal, effects.add(heal_impact(16, 5.0)));

    // Ability-specific overrides (larger/more dramatic)
    assets.handles.insert(VfxKind::CleaveImpact, effects.add(cleave_impact()));
    assets.handles.insert(VfxKind::ShieldBashImpact, effects.add(shield_bash_impact()));
    assets.handles.insert(VfxKind::FireballImpact, effects.add(fire_impact(120, 20.0)));
    assets.handles.insert(VfxKind::FrostBoltImpact, effects.add(frost_impact(28, 8.0)));
    assets.handles.insert(VfxKind::HealLand, effects.add(heal_impact(28, 8.0)));
    assets.handles.insert(VfxKind::AimedShotImpact, effects.add(aimed_shot_impact()));

    commands.insert_resource(assets);
}

// ---------------------------------------------------------------------------
// Helper: radial burst with color gradient
// ---------------------------------------------------------------------------

fn radial_burst(
    name: &str,
    count: u32,
    speed_min: f32,
    speed_max: f32,
    lifetime: f32,
    size_start: f32,
    size_end: f32,
    colors: &[(f32, Vec4)],
) -> EffectAsset {
    let writer = ExprWriter::new();

    let age = writer.lit(0.0).expr();
    let lt = writer.lit(lifetime).expr();
    let center = writer.lit(Vec3::ZERO).expr();
    let axis = writer.lit(Vec3::Z).expr();
    let radius = writer.lit(2.0).expr();
    let speed = (writer.lit(speed_min) + writer.lit(speed_max - speed_min) * writer.rand(ScalarType::Float)).expr();
    let vel_center = writer.lit(Vec3::ZERO).expr();
    let vel_axis = writer.lit(Vec3::Z).expr();

    let module = writer.finish();

    let mut gradient = HanabiGradient::new();
    for &(key, color) in colors {
        gradient.add_key(key, color);
    }
    let size_gradient = HanabiGradient::linear(Vec3::splat(size_start), Vec3::splat(size_end));

    let capacity = (count as f32 * 1.5) as u32;

    let mut effect = EffectAsset::new(capacity, SpawnerSettings::once((count as f32).into()), module)
        .with_name(name)
        .with_simulation_space(SimulationSpace::Global)
        .init(SetAttributeModifier::new(Attribute::AGE, age))
        .init(SetAttributeModifier::new(Attribute::LIFETIME, lt))
        .init(SetPositionCircleModifier { center, axis, radius, dimension: ShapeDimension::Surface })
        .init(SetVelocityCircleModifier { center: vel_center, axis: vel_axis, speed })
        .render(ColorOverLifetimeModifier::new(gradient))
        .render(SizeOverLifetimeModifier { gradient: size_gradient, screen_space_size: false });
    effect.z_layer_2d = 4.5;
    effect
}

/// Build a custom burst effect with acceleration (fire, frost, heal, holy).
fn custom_burst(
    name: &str,
    count: u32,
    spawn_radius: f32,
    speed_min: f32,
    speed_max: f32,
    lifetime_min: f32,
    lifetime_range: f32,
    size_start: f32,
    size_end: f32,
    colors: &[(f32, Vec4)],
    accel: Option<Vec3>,
    drag: Option<f32>,
    volume: bool,
) -> EffectAsset {
    let writer = ExprWriter::new();

    let age = writer.lit(0.0).expr();
    let lt = (writer.lit(lifetime_min) + writer.lit(lifetime_range) * writer.rand(ScalarType::Float)).expr();
    let center = writer.lit(Vec3::ZERO).expr();
    let axis = writer.lit(Vec3::Z).expr();
    let r = writer.lit(spawn_radius).expr();
    let speed = (writer.lit(speed_min) + writer.lit(speed_max - speed_min) * writer.rand(ScalarType::Float)).expr();
    let vel_center = writer.lit(Vec3::ZERO).expr();
    let vel_axis = writer.lit(Vec3::Z).expr();
    let accel_expr = accel.map(|a| writer.lit(a).expr());
    let drag_expr = drag.map(|d| writer.lit(d).expr());

    let module = writer.finish();

    let mut gradient = HanabiGradient::new();
    for &(key, color) in colors {
        gradient.add_key(key, color);
    }
    let size_gradient = HanabiGradient::linear(Vec3::splat(size_start), Vec3::splat(size_end));

    let capacity = (count as f32 * 1.5) as u32;
    let dimension = if volume { ShapeDimension::Volume } else { ShapeDimension::Surface };

    let mut effect = EffectAsset::new(capacity, SpawnerSettings::once((count as f32).into()), module)
        .with_name(name)
        .with_simulation_space(SimulationSpace::Global)
        .init(SetAttributeModifier::new(Attribute::AGE, age))
        .init(SetAttributeModifier::new(Attribute::LIFETIME, lt))
        .init(SetPositionCircleModifier { center, axis, radius: r, dimension })
        .init(SetVelocityCircleModifier { center: vel_center, axis: vel_axis, speed });

    if let Some(a) = accel_expr {
        effect = effect.update(AccelModifier::new(a));
    }
    if let Some(d) = drag_expr {
        effect = effect.update(LinearDragModifier::new(d));
    }

    effect = effect
        .render(ColorOverLifetimeModifier::new(gradient))
        .render(SizeOverLifetimeModifier { gradient: size_gradient, screen_space_size: false });
    effect.z_layer_2d = 4.5;
    effect
}

// ---------------------------------------------------------------------------
// Physical impacts
// ---------------------------------------------------------------------------

fn slash_impact() -> EffectAsset {
    radial_burst("impact_slash", 10, 30.0, 60.0, 0.2, 3.0, 0.0, &[
        (0.0, Vec4::new(1.0, 1.0, 1.0, 1.0)),
        (0.5, Vec4::new(0.8, 0.8, 0.8, 0.7)),
        (1.0, Vec4::new(0.6, 0.6, 0.6, 0.0)),
    ])
}

fn pierce_impact() -> EffectAsset {
    radial_burst("impact_pierce", 7, 25.0, 50.0, 0.18, 2.0, 0.0, &[
        (0.0, Vec4::new(0.7, 0.7, 0.9, 1.0)),
        (1.0, Vec4::new(0.5, 0.5, 0.7, 0.0)),
    ])
}

fn blunt_impact() -> EffectAsset {
    radial_burst("impact_blunt", 12, 20.0, 45.0, 0.25, 4.0, 0.5, &[
        (0.0, Vec4::new(0.7, 0.6, 0.4, 1.0)),
        (0.5, Vec4::new(0.5, 0.4, 0.3, 0.8)),
        (1.0, Vec4::new(0.3, 0.2, 0.1, 0.0)),
    ])
}

fn cleave_impact() -> EffectAsset {
    radial_burst("cleave_impact", 20, 35.0, 70.0, 0.25, 4.0, 0.0, &[
        (0.0, Vec4::new(1.0, 1.0, 1.0, 1.0)),
        (0.3, Vec4::new(0.9, 0.9, 0.9, 0.9)),
        (1.0, Vec4::new(0.6, 0.6, 0.6, 0.0)),
    ])
}

fn shield_bash_impact() -> EffectAsset {
    radial_burst("shield_bash_impact", 14, 25.0, 55.0, 0.2, 5.0, 1.0, &[
        (0.0, Vec4::new(1.0, 1.0, 0.8, 1.0)),
        (0.2, Vec4::new(0.8, 0.7, 0.4, 0.9)),
        (1.0, Vec4::new(0.4, 0.3, 0.2, 0.0)),
    ])
}

fn aimed_shot_impact() -> EffectAsset {
    radial_burst("aimed_shot_impact", 10, 30.0, 55.0, 0.2, 2.5, 0.0, &[
        (0.0, Vec4::new(0.8, 0.7, 0.5, 1.0)),
        (1.0, Vec4::new(0.4, 0.3, 0.2, 0.0)),
    ])
}

// ---------------------------------------------------------------------------
// Magical impacts
// ---------------------------------------------------------------------------

fn fire_impact(count: u32, spawn_radius: f32) -> EffectAsset {
    custom_burst("impact_fire", count, spawn_radius, 20.0, 80.0, 0.3, 0.3, 8.0, 2.0, &[
        (0.0, Vec4::new(1.0, 0.95, 0.5, 1.0)),
        (0.2, Vec4::new(1.0, 0.7, 0.1, 1.0)),
        (0.5, Vec4::new(1.0, 0.3, 0.0, 0.8)),
        (0.8, Vec4::new(0.6, 0.1, 0.0, 0.4)),
        (1.0, Vec4::new(0.2, 0.02, 0.0, 0.0)),
    ], Some(Vec3::new(0.0, 40.0, 0.0)), None, true)
}

fn frost_impact(count: u32, spawn_radius: f32) -> EffectAsset {
    custom_burst("impact_frost", count, spawn_radius, 10.0, 35.0, 0.35, 0.2, 3.5, 0.5, &[
        (0.0, Vec4::new(1.0, 1.0, 1.0, 1.0)),
        (0.4, Vec4::new(0.6, 0.85, 1.0, 0.9)),
        (1.0, Vec4::new(0.3, 0.6, 0.9, 0.0)),
    ], None, Some(3.0), true)
}

fn storm_impact() -> EffectAsset {
    radial_burst("impact_storm", 12, 50.0, 90.0, 0.15, 3.0, 0.0, &[
        (0.0, Vec4::new(1.0, 1.0, 1.0, 1.0)),
        (0.3, Vec4::new(0.6, 0.6, 1.0, 0.9)),
        (1.0, Vec4::new(0.3, 0.2, 0.8, 0.0)),
    ])
}

fn arcane_impact() -> EffectAsset {
    radial_burst("impact_arcane", 14, 20.0, 45.0, 0.3, 3.0, 0.5, &[
        (0.0, Vec4::new(0.9, 0.4, 1.0, 1.0)),
        (0.5, Vec4::new(0.6, 0.2, 0.9, 0.8)),
        (1.0, Vec4::new(0.3, 0.1, 0.5, 0.0)),
    ])
}

fn holy_impact() -> EffectAsset {
    custom_burst("impact_holy", 16, 4.0, 10.0, 40.0, 0.3, 0.2, 3.0, 0.5, &[
        (0.0, Vec4::new(1.0, 1.0, 1.0, 1.0)),
        (0.3, Vec4::new(1.0, 0.95, 0.6, 0.9)),
        (0.7, Vec4::new(0.95, 0.8, 0.3, 0.5)),
        (1.0, Vec4::new(0.8, 0.7, 0.2, 0.0)),
    ], Some(Vec3::new(0.0, 20.0, 0.0)), None, true)
}

fn shadow_impact() -> EffectAsset {
    radial_burst("impact_shadow", 12, 15.0, 35.0, 0.35, 4.0, 1.0, &[
        (0.0, Vec4::new(0.4, 0.1, 0.5, 1.0)),
        (0.5, Vec4::new(0.2, 0.05, 0.3, 0.8)),
        (1.0, Vec4::new(0.05, 0.0, 0.1, 0.0)),
    ])
}

fn nature_impact() -> EffectAsset {
    radial_burst("impact_nature", 14, 15.0, 40.0, 0.3, 3.0, 0.5, &[
        (0.0, Vec4::new(0.3, 0.9, 0.2, 1.0)),
        (0.5, Vec4::new(0.2, 0.7, 0.1, 0.7)),
        (1.0, Vec4::new(0.1, 0.4, 0.05, 0.0)),
    ])
}

fn heal_impact(count: u32, spawn_radius: f32) -> EffectAsset {
    custom_burst("impact_heal", count, spawn_radius, 10.0, 35.0, 0.35, 0.2, 3.0, 0.5, &[
        (0.0, Vec4::new(0.4, 1.0, 0.5, 1.0)),
        (0.4, Vec4::new(0.6, 1.0, 0.7, 0.9)),
        (1.0, Vec4::new(1.0, 1.0, 1.0, 0.0)),
    ], Some(Vec3::new(0.0, 25.0, 0.0)), None, true)
}
