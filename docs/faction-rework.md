# Faction System Rework

## Status: In Progress

The faction system has been substantially reworked from its original territory-based model to an allegiance-based model. Most changes are implemented and compiling. The core remaining issue is **faction count tuning** — some seeds produce too many factions because formation criteria are too easily met.

## What Changed

### Data Model: Allegiance-Based Factions

The fundamental shift: **factions are groups of people, not owners of settlements.**

- `Person.faction_allegiance: u32` — every person has personal faction loyalty (0 = unaligned)
- `SettlementState.controlling_faction: u32` — **derived cache**, recomputed yearly from majority allegiance of residents. Renamed from `owner_faction`.
- `SettlementState.at_war: bool` — **removed**. Wars are between factions, not settlements. A settlement feels the effects of war through its residents being drafted.
- `SettlementState.conquered_by: Option<u32>` — **added**. Set during conquest, signals the allegiance shift system to convert residents.
- `FactionState.settlements: Vec<u32>` — now a **derived cache**, rebuilt yearly from which settlements the faction controls through majority allegiance.
- `FactionState.unhappy_years: u8` — **added**. Tracks consecutive years of average happiness < 20 for dissolution.

### Gauges Derived from Population

Military, wealth, and stability are no longer abstract 0-100 numbers that drain/regenerate through arbitrary formulas. They are **computed each year from real population data**:

- **Military** = sum of `combat_score` of all soldiers allegiant to the faction (via `faction_military_power()` in war.rs), normalized to 0-100
- **Wealth** = count of merchants allegiant to the faction, normalized to 0-100
- **Stability** = average happiness of all allegiant people, 0-100
- **Patron god** = most-worshipped god among allegiant people

Computed in `faction_stats.rs::compute_faction_stats()`, written to FactionState fields via `update_from_stats()`.

### Controlling Faction Derivation

`faction_stats.rs::recompute_controlling_factions()` runs after each year's population simulation:
- For each settlement, counts adult residents by `faction_allegiance` using BTreeMap (deterministic)
- Majority faction becomes `controlling_faction`
- Rebuilds each faction's `settlements` Vec

### Events Driven by Characters, Not Random Rolls

Almost all random probability rolls have been replaced with character-driven decisions:

| Event | Old | New |
|-------|-----|-----|
| War declaration | Random roll (2-60%) after sentiment check | Leader's ambition (ExpandTerritory/DestroyEnemy) + sentiment threshold. Peaceful leaders resist unless extreme provocation. |
| War ending | Duration-based random roll | Military ratio: ends when loser < 30% of winner, or mutual exhaustion (both < 10) |
| Betrayal | 5% hard random roll | Treacherous character with SeizePower ambition acts when victim is weaker |
| Alliance | 10-20% random gate | Diplomatic leader + friendly sentiment + shared threat/god |
| Alliance breaking | Random roll on low sentiment | Deterministic: breaks when sentiment < 0, or treacherous leader + sentiment < 10 |
| Trade agreement | 8% random gate | Automatic when sentiment >= 0 and not at war (limit 2 per year) |
| Coup | Random roll (1-20%) | Deterministic: usurper with higher renown exists AND stability < 30 |
| Settlement founding | Two random gates (5% + expansion weight) | Leader has ExpandTerritory ambition AND faction wealth >= 30 |
| Artifact discovery | 2-4% random gate | Scholar character in stable faction (stability > 40) |
| Faction formation | Was 3% random spawn | Leader-driven: specific people found factions from their traits + circumstances |
| Dissolution | 25-50% random chance | Zero allegiant people OR 5 consecutive years of unhappy_years |
| Absorption | 10% random roll | Immediate when weak faction meets strong same-identity neighbor |
| Monster attack | 8% pure random | **Removed** — no world-state connection |
| Hero rise | 6% pure random | **Removed** — heroes emerge from population notable system |

**Still uses probability:** Plague (condition-driven probability — this is correct, plague should be uncertain). Rebellion (happiness-scaled probability with race/faith mismatch bonuses).

### Leader-Driven Faction Formation

`faction_stats.rs::compute_formation_candidates()` scans every adult in every settlement for people whose traits + circumstances make them found a faction. The faction type follows from **why** they're acting:

- **Prophet** (zealot, `prophet_of.is_some()`) → Theocracy
- **Ambitious + different race from controlling faction** → TribalWarband
- **Treacherous/Cunning + unhappy** → ThievesGuild
- **Soldier + survived 2+ wars + ambitious** → MercenaryCompany
- **Merchant + charismatic** → MerchantGuild
- **Devout + different faith from controlling faction** → ReligiousOrder

A person only founds a new faction if no existing faction of that type matches their motivation (e.g., no theocracy for their god, no tribal clan for their race).

Kingdom upgrade: any faction controlling 3+ settlements becomes a Kingdom (type transformation, not new faction).

### Allegiance Shift System

`allegiance.rs::evaluate_allegiance_shifts()` runs yearly after happiness evaluation:

- **Conquest**: residents of conquered settlements with happiness < 40 shift to conqueror
- **Charismatic conversion**: Charismatic person converts one unaligned same-settlement resident per year
- **Faith-driven**: person whose god matches another faction's patron (devotion > 60, unhappy) shifts
- **Race-driven**: Purist trait + race matches another faction + unhappy → shift
- **Unhappiness conformity**: happiness < 20 → conform to settlement's controlling faction

### Prophet-Driven Religious Tension

`ProphetTensions` computed each year from population. Active prophets inflame sentiment (-5) between their faction and factions worshipping hostile gods.

### Faction Type Sentiment

In the upkeep loop:
- Theocracies: -2 sentiment toward factions with different patron gods
- Merchant guilds: +1 toward everyone (trade smooths relations)
- Bandits/ThievesGuilds: -1 toward Kingdoms in same region

### Succession Crisis

When a faction leader dies and 2+ Ambitious/PowerHungry characters compete for succession, `unhappy_years += 2` (accelerates potential dissolution).

### FactionType Additions

Added `Theocracy` variant to `FactionType` enum with:
- Name patterns: "The Holy See of {place}", "The {adj} Theocracy", etc.
- Initial gauges: (20, 40, 75) — low military, moderate wealth, high stability
- Relation friction: -10 vs MageCircle (same as ReligiousOrder)

## Files Changed

| File | What |
|------|------|
| `src/worldgen/population/types.rs` | `faction_allegiance` on Person, `AllegianceChanged` life event |
| `src/worldgen/history/state.rs` | `controlling_faction` (renamed), removed `at_war`, added `conquered_by`, `unhappy_years` |
| `src/worldgen/population/allegiance.rs` | **New** — allegiance shift system |
| `src/worldgen/population/faction_stats.rs` | Stats from allegiance, `recompute_controlling_factions()`, leader-driven formation |
| `src/worldgen/population/war.rs` | Military power from allegiance, not settlement ownership |
| `src/worldgen/population/mod.rs` | Wire allegiance shifts into yearly tick |
| `src/worldgen/population/happiness.rs` | War check uses person's faction, not settlement flag |
| `src/worldgen/population/migration.rs` | Uses person's allegiance for faction matching |
| `src/worldgen/population/seed.rs` | Initial allegiance from settlement's controlling faction |
| `src/worldgen/population/lifecycle.rs` | Newborns inherit mother's allegiance |
| `src/worldgen/history/world_events.rs` | All evaluate_* functions reworked, monster/hero removed |
| `src/worldgen/history/mod.rs` | Wire recompute_controlling_factions, compute formation candidates |
| `src/worldgen/names.rs` | Theocracy variant + name patterns |

## Known Issues

### Faction count too high on some seeds

Some seeds produce 30-300+ factions. The formation criteria find too many qualifying people. The dedup check ("don't found if a matching faction exists") helps but isn't sufficient — the check is inconsistent (some types check globally, some locally, some by race).

**Root cause not fully diagnosed.** The issue is likely:
1. The allegiance shift system doesn't aggressively enough recruit people into existing factions *before* formation runs
2. Formation runs in `simulate_year()` (world events) but allegiance shifts run in `advance_year()` (population sim) — so potential founders haven't been recruited yet
3. Wars create many unhappy people → many potential founders → many factions → more wars → feedback loop

**What needs investigation:**
- Run `dump_history_summary` on a high-faction seed and trace the lifecycle: what types form, how fast they die, whether they're getting absorbed
- Check whether the allegiance shift system is actually running and converting people
- Consider whether formation should move into the population sim (after allegiance shifts) rather than world events (before)
- The "join existing faction" logic should be in the allegiance shift system, not just in formation prevention

### Rebellion still uses probability

`evaluate_rebellion` still uses a happiness-scaled random roll. This is arguably correct (rebellion is inherently uncertain) but inconsistent with the "no random rolls" principle. Could be made deterministic: rebellion fires when a leader figure exists + conditions have persisted for N years.

## Original Design Document

The sections below are from the original planning document and may be outdated relative to what was implemented.

### What Factions Should Be

A faction is **a group of people with shared identity and goals**. The faction type determines what that identity is and how they pursue their goals. Factions should **emerge from population conditions**, not spawn randomly.

### Faction Types

| Type | Why They Form | What They Want | Power Source |
|------|-------------|----------------|-------------|
| **Kingdom/Monarchy** | Conquest, inheritance, territorial claim | Land, stability, legacy | Military (real soldiers), population size |
| **Theocracy** | Prophet with enough followers | Spread their god's doctrine | Devotion of population, prophets, temples |
| **Merchant Guild** | Charismatic merchant in trade hub | Wealth, trade access, economic influence | Merchants, trade routes |
| **Tribal/Clan** | Ambitious person of minority race | Survival, autonomy, homeland | Warriors of that race, territorial knowledge |
| **Religious Order/Cult** | Devout person with different faith | Convert others, purify the unfaithful | Fanatical followers, prophet doctrine |
| **Criminal Syndicate** | Treacherous/Cunning unhappy person | Wealth and power from the shadows | Treacherous/Cunning population |
| **Military Order** | War veterans with ambitious leader | Protect territory, serve as mercenaries | Soldiers with SurvivedWar events |

### Open Questions

- Should factions have explicit "influence" per settlement (a numeric score) rather than just headcount?
- How should the allegiance shift system interact with formation timing? (Currently formation runs before shifts)
- Should factions be able to recruit actively (sending charismatic agents to convert people in other settlements)?
- How do criminal syndicates interact with the faction that officially controls a settlement? (Currently they just coexist via allegiance)
- Should faction leaders come from the population (specific Person promoted to Character) rather than generated Characters?
