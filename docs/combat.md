# Combat System

## Health System

**Flat HP pool** via `Health { current, max }` component. Max HP = `50 + toughness * 10`. Every hit reduces current HP by post-armor damage. Death when `current <= 0`.
- `health.take_damage(amount)` — reduces current, returns actual damage dealt
- `health.heal(amount)` — restores current, capped at max
- `health.fraction()` — current/max for health bar display

Health bars, party portraits, hover tooltips, and AI (flee, heal decisions) all read `Health.fraction()`.

## Hit Resolution Chain
1. `accuracy_check(accuracy, dodge, roll)` — hit chance clamped 5%-95%
2. `select_body_part(template, coverage_roll)` — weighted by body part coverage
3. `armor.reduce_damage(part_index, damage_type, raw_damage)` — per-part armor resistances
4. `body.damage_part(index, damage, template)` — reduces part HP, cascades destruction to children
5. `health.take_damage(damage_after_armor)` — reduces flat HP pool

## Body Part System (secondary, for future injuries)
- Tree structure: Head -> (Brain, Eyes, Jaw), Torso -> (Heart, Lungs, Arms -> Hands, etc.)
- Each part has: max_hp, coverage weight, vital flag, capabilities (Sight, Movement, etc.)
- Destroying a part destroys all children
- Destroying a vital part (Brain, Heart) is **also** instant death (in addition to flat HP reaching 0)
- Body part damage runs in parallel with the flat HP pool — currently cosmetic but supports future limb injuries
- `BodyTemplate` loaded from data (currently `humanoid_template()`)
- `Body` component stores per-part runtime HP state

## Damage Types (10)
- Physical: Slashing, Piercing, Blunt
- Magical: Fire, Frost, Storm, Arcane, Holy, Shadow, Nature
- `Resistances` uses `HashMap<DamageType, f32>` — adding new types doesn't require struct changes

## Magic Schools (10)
- Elemental: Fire, Frost, Storm
- Divine: Holy, Shadow
- Arcane: Arcane, Enchantment
- Primal: Nature, Blood (forbidden)
- Death: Necromancy (forbidden)
- Schools define WHAT magic does, not HOW it's practiced

## Status Effects
- Duration (ticks), stacking with max stacks, tick effects (DoT/HoT)
- CC flags: Stunned, Rooted, Silenced, Feared, Sleeping
- Stat modifiers: MoveSpeedMul, AttackSpeedMul, AttributeFlat
- `ActiveStatusEffects` component tracks all active effects per entity

## Abilities
- `AbilityDef`: cast time, cooldown, mana/stamina cost, range, target type (Single/Circle/Cone/Line), effects chain
- `AbilitySlots`: per-entity known abilities with cooldown state
- `Mana` + `Stamina`: separate resource pools
- `CastingState`: tracks active casting (interruptible flag). Inserted by `execute_actions` when processing a `CastAbility` action.
- Casting pipeline: `begin_cast` (spend resources, resolve instants) -> `tick_casting` (countdown) -> `interrupt_casting` (on damage)
- On resolution, fires `AbilityLandedEvent` at the impact position for VFX

## Death
- Flat HP reaching 0, OR destroying a vital body part (Brain, Heart), kills the entity
- `cleanup_dead` inserts `Dead` marker, rotates sprite 90°, greys out to `Color::srgb(0.4, 0.4, 0.4)`, lowers z-layer to `FLOOR_ITEMS`
- Removes `InCombat`, `Engaging`, `CurrentTask`, `CastingState`, `MovePath`, `HitFlash`
- `Without<Dead>` guards on all targeting/combat queries prevent interacting with corpses
- Corpses persist until zone exit (no despawn timer)

## Targeting
- **bevy_picking** for sprite-based click targeting — all combat entities have `Pickable` component
- `HoveredTarget` resource updated each frame from Bevy's `HoverMap` (topmost entity under cursor)
- Right-click: if hovered entity is non-friendly, issue attack; otherwise move to ground
- Faction check: `!is_friendly()` (Neutral and Hostile entities are both attackable)
- Threat-based aggro ignores faction — if something hit you, you fight back regardless

## Threat
- `ThreatTable` per entity — tracks threat from each attacker
- Damage generates threat
- AI uses highest-threat target when available; skips faction check for threat sources (provoked enemies retaliate)

## UseCondition (AI ability gating)
- `Always` — always use
- `SelfHpBelow(f32)` — self HP fraction below threshold
- `TargetHpBelow(f32)` — engage target HP fraction below threshold
- `AllyHpBelow(f32)` — any same-faction ally has HP below threshold
- `EnemiesInRange(u32)` — at least N hostile entities within aggro range
- For `SingleAlly` abilities, AI targets the most wounded ally within range (not the engage target)

## Combat Feedback (VFX / Audio)

### Data-Driven Fields
- `WeaponDef.attack_sfx: Option<SfxKind>` — sound on auto-attack hit (per weapon class: SwordHit, MaceHit, etc.)
- `AbilityDef.cast_sfx / impact_sfx: Option<SfxKind>` — sounds on cast start and impact
- `AbilityDef.impact_vfx: Option<VfxKind>` — particle effect at impact point (overrides damage-type default)
- `AbilityDef.impact_vfx_scale: f32` — scale multiplier (1.0 = default, 6.0 = Fireball-sized AoE)
- `AbilityDef.cast_vfx: Option<VfxKind>` — particle effect on caster during cast (future)

### Healing Feedback
- `HealEvent` fired when `body.heal_distributed()` returns > 0 during ability resolution
- Green floating "+N" numbers at target position (same drift/fade as damage numbers)
- `VfxKind::ImpactHeal` particles at target (green→white upward rise)
- `SfxKind::HealLand` audio

### Projectile Visuals
- Cosmetic only — damage is applied instantly, projectiles are purely visual
- `Projectile` component: flies a small colored sprite from attacker to target position
- Auto-attacks: spawned when `!weapon.is_melee`
- Abilities: spawned when `ability.range > 2.0`
- Color from `ImpactKind::from_damage_type()`, speed from `weapon.projectile_speed` or 400 px/s default
- Z-layer: `render_layers::PROJECTILES` (4.0)

### Micro-Animations
- `AttackLunge`: attacker bumps toward target on hit (0.15s sine-eased, 8px magnitude)
- `HitFlash`: target sprite flashes white on damage (0.08s)
- `ScreenTrauma`: camera shake on hits (0.1 trauma), part destruction (0.25), kills (0.5)

### Particle System
- Uses `bevy_hanabi` 0.18 (GPU-accelerated particles, 2D mode)
- `VfxKind` enum maps to pre-built `EffectAsset` handles in `ParticleAssets` registry
- 11 generic per-damage-type impacts + 6 ability-specific overrides
- Effects are one-shot bursts (`SpawnerSettings::once`) in world space (`SimulationSpace::Global`)
- `AbilityLandedEvent` fires once at impact position for AoE effects (not per-target)

### Audio
- `SfxKind` enum maps to `Handle<AudioSource>` in `AudioAssets` registry
- Per-weapon-class attack sounds, per-ability cast/impact sounds
- Playback via `AudioPlayer` + `PlaybackSettings::DESPAWN` (fire-and-forget)
- Audio files not yet loaded (stub resource — add .ogg files to `assets/audio/`)
