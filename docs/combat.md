# Combat System

## Hit Resolution Chain
1. `accuracy_check(accuracy, dodge, roll)` — hit chance clamped 5%-95%
2. `select_body_part(template, coverage_roll)` — weighted by body part coverage
3. `armor.reduce_damage(part_index, damage_type, raw_damage)` — per-part armor resistances
4. `body.damage_part(index, damage, template)` — reduces HP, cascades destruction to children

## Body Part System
- Tree structure: Head -> (Brain, Eyes, Jaw), Torso -> (Heart, Lungs, Arms -> Hands, etc.)
- Each part has: max_hp, coverage weight, vital flag, capabilities (Sight, Movement, etc.)
- Destroying a part destroys all children
- Destroying a vital part (Brain, Heart) kills the entity
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

## Threat
- `ThreatTable` per entity — tracks threat from each attacker
- Damage generates threat
- AI evaluators use highest-threat target when available
