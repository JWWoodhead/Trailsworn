# Combat Feedback: VFX, Audio, and Animations

## How to Add a New Particle Effect

1. Add a variant to `VfxKind` in `resources/particles.rs`
2. Add a builder function in `resources/particle_defs.rs` (use `radial_burst()` for simple effects, `custom_burst()` for effects with acceleration or drag)
3. Register it in `setup_particle_effects()`: `assets.handles.insert(VfxKind::MyEffect, effects.add(my_effect()));`
4. Reference it from `AbilityDef.impact_vfx` in `ability_defs.rs`
5. Set `impact_vfx_scale` on the ability (1.0 = default, higher = bigger)

## How to Add a New Sound Effect

1. Add a variant to `SfxKind` in `resources/audio.rs`
2. Load the .ogg file in `systems/audio.rs` `setup_audio()` using `asset_server.load()`
3. Reference it from `AbilityDef.cast_sfx` / `AbilityDef.impact_sfx` or `WeaponDef.attack_sfx`

## Event Flow

```
Auto-attack lands
  → DamageDealtEvent
    → spawn_combat_effects:
        - AttackLunge on attacker
        - HitFlash on target
        - Particle impact (WeaponDef damage type → ImpactKind → VfxKind fallback)
        - ScreenTrauma
        - Audio (WeaponDef.attack_sfx → SfxKind fallback)

Ability cast begins
  → AbilityCastEvent
    → spawn_cast_effects:
        - Audio (AbilityDef.cast_sfx)

Ability resolves (instant or after cast time)
  → AbilityLandedEvent (once, at impact position)
    → spawn_ability_landed_effects:
        - Big particle burst (AbilityDef.impact_vfx, scaled by impact_vfx_scale)
  → DamageDealtEvent (per target hit)
    → spawn_combat_effects:
        - Per-target hit feedback (lunge, flash, particles, trauma, audio)

Cast interrupted
  → CastInterruptedEvent
    → spawn_interrupt_effects:
        - Audio (SfxKind::CastInterrupt)

Healing applied (ability resolution)
  → HealEvent
    → spawn_heal_numbers:
        - Green "+N" floating text at target
    → spawn_heal_effects:
        - Particle burst (AbilityDef.impact_vfx → VfxKind::ImpactHeal fallback)
        - Audio (AbilityDef.impact_sfx → SfxKind::HealLand fallback)
```

## Particle Effect Design

All impact effects use `SpawnerSettings::once(N)` (burst N particles then stop) in `SimulationSpace::Global` (particles stay at world position). The `z_layer_2d` is set to 4.5 (between PROJECTILES and UI_OVERLAY layers).

Two helper functions in `particle_defs.rs`:

- **`radial_burst()`** — simple radial particle explosion with color gradient. Parameters: name, count, speed range, lifetime, size range, color keyframes.
- **`custom_burst()`** — radial burst with optional acceleration (upward drift for fire/heal) or drag (deceleration for frost). Additional parameters: accel vector, drag coefficient, volume vs surface spawning.

### Effect Guidelines by Damage Type

| Type | Colors | Behavior |
|------|--------|----------|
| Slashing | White → gray | Fast outward arc |
| Piercing | Blue-gray | Tight, small burst |
| Blunt | Brown/tan | Wide, chunky |
| Fire | Yellow → orange → red | Upward drift (accel) |
| Frost | White → light blue | Slow spread (drag) |
| Storm | White → purple-blue | Fast, erratic |
| Arcane | Magenta → purple | Spiral outward |
| Holy | White → gold | Upward rise (accel) |
| Shadow | Dark purple → black | Inward collapse |
| Nature | Green → dark green | Outward scatter |
| Heal | Green → white | Upward rise (accel) |

Ability-specific overrides use 2-3x more particles and wider spread than generic defaults.

## Key Files

| File | Purpose |
|------|---------|
| `resources/particles.rs` | `VfxKind` enum, `ParticleAssets` registry |
| `resources/particle_defs.rs` | `EffectAsset` builder functions, `setup_particle_effects` |
| `resources/audio.rs` | `SfxKind` enum, `AudioAssets` registry |
| `resources/vfx.rs` | `AttackLunge`, `HitFlash`, `Projectile`, `ScreenTrauma`, `DespawnTimer`, `ImpactKind` |
| `systems/vfx.rs` | All spawn + tick systems for combat feedback (damage, heal, projectiles) |
| `systems/audio.rs` | `setup_audio` startup system |
| `resources/events.rs` | `DamageDealtEvent`, `AttackMissedEvent`, `AbilityCastEvent`, `AbilityLandedEvent`, `CastInterruptedEvent`, `HealEvent` |

## bevy_hanabi Notes

- Version 0.18 maps to Bevy 0.18. Cargo: `bevy_hanabi = { version = "0.18", default-features = false, features = ["2d"] }`
- `HanabiPlugin` registered in `main.rs`
- `ExprWriter` is consumed by `.finish()` — capture all expressions before calling it
- Use `bevy_hanabi::Gradient` (aliased as `HanabiGradient` in particle_defs.rs) to avoid ambiguity with `bevy::prelude::Gradient`
- `SpawnerSettings::once()` takes `CpuValue<f32>` — use `(count as f32).into()`, not `count.into()`
- `ColorOverLifetimeModifier::new(gradient)` — don't use struct literal (needs `blend`/`mask` fields)
- No `with_z_layer_2d()` builder — set `effect.z_layer_2d = 4.5` directly after construction
