# Population Simulation & Life Events

Design document for the person-level simulation layer. Describes the target architecture — not everything is built yet.

## Core Concept

Simulate **every person** in the world (~28,000 initial, ~68,000 total over 100 years) with a thin record, and track **life events** that happen to them. When a person's accumulated life events match a narrative pattern, they become **notable** — an active character whose story the player can discover.

Gods, factions, and wars are scaffolding. The actual stories are about **people** — a scholar who went mad searching for an artifact, two brothers driven to fight each other, a self-declared prophet killed for heresy. The population simulation produces the family fabric and life experiences that make those stories emerge organically.

Benchmarked at **1.2s release / 3MB** for the full 100-year lifecycle. Performance is not a concern. Prototype in `src/worldgen/population.rs`.

---

## Person Model

```
Person:
  id: u32
  birth_year: i32
  death_year: Option<i32>
  settlement_id: u32          — can change (migration, conquest, exile)
  sex: Sex
  mother: Option<u32>
  father: Option<u32>
  spouse: Option<u32>
  occupation: Occupation
  faith: Option<GodId>
  devotion: u8                — 0-100, personal (not settlement-level)
  life_events: Vec<LifeEvent>
  notable: bool               — promoted when life_events match a narrative pattern
```

**Occupation** (initial set): Farmer, Soldier, Smith, Merchant, Priest, Scholar
- Assigned at adulthood (16) based on settlement needs and parent occupation
- Can change from life events (drafted farmer becomes soldier, crisis-of-faith priest becomes scholar)

**Faith** tracks the individual's relationship to a god, separate from their settlement's patron. Most people share their settlement's patron, but individuals can diverge — that divergence is where stories come from.

---

## Life Events

A `LifeEvent` is a record of something that happened to a person. Append-only. Most people accumulate 0-5 over their lifetime. Notable characters accumulate more.

```
LifeEvent:
  year: i32
  kind: LifeEventKind
```

### Family Events

Fire from the population simulation (births, deaths, marriages).

| Event | Trigger | Data |
|-------|---------|------|
| `ChildBorn { child_id }` | Birth pass: both parents get this | child's person ID |
| `MarriedTo { spouse_id }` | Marriage pass: both partners get this | spouse's person ID |
| `LostParent { parent_id, cause }` | Death pass: living children of the deceased | dead parent's ID, DeathCause |
| `LostSpouse { spouse_id, cause }` | Death pass: living spouse of the deceased | dead spouse's ID, DeathCause |
| `LostChild { child_id, cause }` | Death pass: living parents of the deceased | dead child's ID, DeathCause |
| `LostSibling { sibling_id, cause }` | Death pass: living siblings (shared parent) of the deceased | dead sibling's ID, DeathCause |

**`DeathCause`**: `OldAge`, `War`, `Plague`, `DivineFlaw`, `Violence`, `Monster`

### War Events

Fire from `evaluate_war_declared` and `evaluate_war_ended` in the mortal simulation.

| Event | Trigger | Data |
|-------|---------|------|
| `DraftedToWar { enemy_faction_id }` | War declared: fraction of fighting-age people (16-50) in both factions' settlements. Farmers drafted become Soldiers. | enemy faction ID |
| `SurvivedWar { enemy_faction_id, won: bool }` | War ended: drafted people who survived. Won = greatness arc. Lost = bitterness/revenge arc. | enemy faction ID, outcome |
| `SettlementConquered { new_faction_id }` | War resolution conquers a settlement: all living residents. | conquering faction's ID |

**War casualties**: Each year a war is active, some fraction of drafted soldiers die (cause = `War`). This cascades `LostParent/Spouse/Child/Sibling` events to their families.

### Faith Events

Fire from `evaluate_worship`, `evaluate_flaw_triggers`, and the faith condition system.

| Event | Trigger | Data |
|-------|---------|------|
| `GainedFaith { god_id }` | Settlement establishes worship and person had no prior faith, OR personal conversion. | god ID |
| `LostFaith { god_id, reason }` | Condition-based (see Faith Loss Conditions). **NOT automatic.** | god ID, FaithLossReason |
| `WitnessedDivineEvent { god_id, event_kind }` | Divine site/artifact/terrain scar appears at or near person's settlement. Rare. | god ID, event kind |
| `FaithShaken { god_id }` | Intermediate state. Devotion drops significantly but faith not lost. Precondition for LostFaith. | god ID |

**`FaithLossReason`**: `FlawTriggered`, `GodFaded`, `SettlementConverted`, `PersonalTragedy`

#### Faith Loss Conditions

Faith loss is **deterministic, not random**. When a potential trigger occurs, evaluate the person's state:

**PersonalTragedy** (lost family member):
- Devotion < 30 + lost family → loses faith
- Devotion 30-60 + lost family → `FaithShaken` instead
- Devotion > 60 + lost family → devotion *increases* (doubling down)
- **Exception**: family member killed by the god's followers (holy war, cruelty flaw) → faith loss regardless, because the god is directly responsible

**FlawTriggered** (god's flaw damages the settlement):
- Devotion < 40 → loses faith
- Devotion 40-70 → `FaithShaken`, devotion drops 15-20
- Devotion > 70 → devotion drops 5-10 but faith holds
- Priests with devotion > 80 → faith holds, may become apologist or reformer

**GodFaded** (god has no worshippers for 20+ years):
- Most people lose faith over 1-5 years
- Devout (> 70) may hold on, becoming keepers of a dying faith
- Creates "last faithful" narrative — potential prophet arc if god returns

**SettlementConverted** (settlement changes patron):
- Devotion < 30 → adopts new faith passively
- Devotion 30-60 → `FaithShaken`, may convert over 2-5 years
- Devotion > 60 → resists, potential exile or martyrdom arc
- Priests → never convert passively, become heretics or martyrs

### Status Events

Fire from faction events and the role/occupation system.

| Event | Trigger | Data |
|-------|---------|------|
| `GainedOccupation { occupation }` | Adulthood (16) or occupation change from life event | new occupation |
| `RoseToProminence { role }` | Becomes leader, general, head priest, etc. | role gained |
| `LostPosition { role, reason }` | Coup, conquest, faction dissolution, personal failure | role lost, why |
| `Exiled` | Religious persecution, political (coup victim), punishment | |
| `Migrated { from, to }` | Marriage, exile, opportunity, fleeing war/plague | settlement IDs |

---

## Notable Promotion

A person becomes notable when their accumulated `life_events` match a **narrative precondition**. The check runs when a life event is appended — not every person every year.

### Phase 1: Simple Threshold

- **3+ war events** (drafted, survived, lost family to war) → military notable
- **Lost family + faith change** → crisis notable
- **Rose to prominence + divine event nearby** → religious notable
- **Lost position + lost family** → villain/exile notable
- **2+ FaithShaken in 10 years** → crisis-of-faith notable

### Phase 2: Narrative Grammar Entry (future)

**Shakespeare entry**: Risen to prominence + flaw-adjacent trait (Ambitious, PowerHungry) + 2+ success events → watching for catalyst

**Dostoevsky entry**: Strong thesis (high devotion, devout trait) + transgression (god's flaw triggers, contradictory evidence) → "test" begins

**Propp entry**: Departure event (LostParent, Exiled, SettlementConquered) + active/brave trait → "journey" begins

---

## Simulation Hook Points

### Mortal Events → Life Events

| Existing Function (`history/mod.rs`) | Life Events Generated |
|---|---|
| Death pass (~line 475) | `LostParent/Spouse/Child/Sibling` for kin. Cause from context. |
| `evaluate_war_declared` (~line 672) | `DraftedToWar` for fighting-age people in both factions. |
| `evaluate_war_ended` (~line 715) | `SurvivedWar` for surviving soldiers. War deaths cascade family events. |
| `evaluate_plague` (~line 1063) | Deaths (cause `Plague`), fraction of population weighted young/old. |
| `evaluate_monster_attack` (~line 1093) | Deaths (cause `Monster`), small number. |
| `evaluate_leader_changed` (~line 968) | `RoseToProminence` / `LostPosition` for leaders. |
| `evaluate_betrayal` (~line 802) | Betrayer notoriety. `LostPosition` for stability damage. |
| `evaluate_settlement_founded` (~line 1190) | `Migrated` for settlers. |
| `evaluate_faction_dissolved` (~line 1270) | `Exiled`/`Migrated` for members. `LostPosition` for leaders. |

### Divine Events → Life Events

| Existing Function (`divine/ (territory.rs, worship.rs, drives.rs, conflict.rs, flaws.rs)`) | Life Events Generated |
|---|---|
| `evaluate_worship` — establishment (~line 355) | `GainedFaith` for settlement residents. |
| `evaluate_worship` — conversion (~line 383) | `FaithShaken`/`LostFaith` (condition-based). `GainedFaith` for converts. |
| `evaluate_worship` — erosion (~line 326, 402) | `FaithShaken` when personal devotion drops. |
| `evaluate_flaw_triggers` — Obsession/Cruelty/Isolation | `FaithShaken`/`LostFaith` for worshippers (condition-based). |
| `evaluate_drive_actions` — temple/sacred site | `WitnessedDivineEvent` for nearby residents. |
| `evaluate_drive_actions` — champion chosen (~line 549) | `RoseToProminence` (once champions are real people). |
| `evaluate_divine_war_resolution` (~line 723) | Deaths (cause `DivineFlaw`). `SettlementConquered` if territory flips. |

### Population Sim → Life Events

| Phase (`population.rs`) | Life Events Generated |
|---|---|
| Birth pass | `ChildBorn` for both parents. |
| Marriage pass | `MarriedTo` for both partners. |
| Death pass | `LostParent/Spouse/Child/Sibling` for kin. Cause = `OldAge` unless overridden. |

---

## Cross-Domain Integration

Life events are written directly to the person's `life_events` vec within their source domain. No cross-domain event system needed for life events — they target *people*, not settlements or factions.

Divine events that affect settlement devotion (handled by `CrossDomainEvent` in `history/mod.rs`) trigger a person-level faith evaluation after application:

```
1. Divine phase emits CrossDomainEvents (WorshipEstablished, DevotionChanged)
2. apply_cross_domain_events() updates settlement state
3. evaluate_faith_impact() scans affected settlements' residents
   → Check personal faith/devotion conditions
   → Append LifeEvents (GainedFaith, FaithShaken, LostFaith)
   → Check notable promotion conditions
```

---

## Implementation Phases

### Phase 1: Population Fabric
- Expand Person struct with occupation, faith, devotion, life_events
- Wire population sim into `generate_history`
- Family events generate LifeEvents for kin
- Simple notable threshold (3+ significant events)
- **Replaces** current `generate_character` for background population
- **Keeps** Character struct for promoted notables

### Phase 2: War & Plague Hooks
- War drafts people, active wars kill soldiers each year
- War resolution creates SurvivedWar events
- Plague/monster kills fraction of settlement population
- DeathCause cascades to family events
- Settlement conquest events for all residents

### Phase 3: Faith System
- Personal faith/devotion on every person
- Faith gain/loss conditions (deterministic, not random)
- FaithShaken as intermediate state
- `evaluate_faith_impact()` after cross-domain event application
- Divine flaws connect to individual stories

### Phase 4: Status & Occupation
- Occupation assignment and change
- RoseToProminence / LostPosition tracking
- Exile and migration mechanics
- Settlement-level roles (head priest, garrison commander)

### Phase 5: Narrative Grammar Integration
- Grammar state machines (Propp, Shakespeare, Dostoevsky)
- Notable promotion via grammar entry detection
- Characters progress through narrative beats
- The payoff: memorable stories emerge from infrastructure

---

## Open Questions

- **Sibling detection**: Scan all people for shared parents is O(n). Cache `children: Vec<u32>` on Person or build family index?
- **Notable cap**: 1-3 per settlement per generation as soft cap?
- **Dead person compaction**: Compact dead non-notable people whose children are also dead?
- **Person → Character bridge**: How does promotion work? Copy? Upgrade in place? Linked structs?
- **Occupation distribution**: Hamlet = almost all farmers. City = specialization. What ratios?
- **Migration frequency**: Rare (marriage, exile, post-war) to preserve settlement identity?
