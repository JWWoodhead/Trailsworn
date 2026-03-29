# Faction System Rework

## Why

The current faction system was built before the population simulation existed. Factions are abstract political entities with gauge numbers (military 0-100, wealth 0-100, stability 0-100) that fight wars based on sentiment scores and "own" settlements. None of this connects to the actual simulated population of ~10k people with races, traits, faith, happiness, occupations, and life histories.

### Current Problems

1. **Most settlements are unowned** — map-generated settlements have `owner_faction: 0`. Only faction-generated settlements (1 per initial faction) have owners. The ~50 map settlements are political orphans.

2. **Factions don't connect to gods** — `FactionState.patron_god` exists but is never used. Factions don't worship, don't care about faith, and wars are never religious.

3. **Faction strength is abstract** — `military_strength: u32` is a number that drains in war and regenerates at +1/year. It doesn't reflect actual soldiers. We have real soldiers with combat scores in the population sim, but factions don't use them.

4. **Wars have no real cause** — wars start when sentiment between two factions drops below -15. Sentiment comes from random friction (-2 to -7 at 40% chance per year) and proximity (-1 to -2/year). No resource competition, no religious conflict, no territorial disputes.

5. **New factions spawn from nothing** — 3% random chance per year, no settlements, no population. They usually dissolve immediately (50% chance if 0 settlements).

6. **Faction dissolution is meaningless** — when a faction dissolves, its people just continue living in settlements with `owner_faction` pointing at a dead faction. No rebellion, no succession, no migration.

7. **Faction type is cosmetic** — Kingdom, MercenaryCompany, ReligiousOrder, ThievesGuild, MerchantGuild, MageCircle, BanditClan, TribalWarband all behave identically. The type only affects initial gauge values.

## What Factions Should Be

A faction is **a group of people with shared identity and goals**. The faction type determines what that identity is and how they pursue their goals. Factions should **emerge from population conditions**, not spawn randomly.

### Faction Types

| Type | Why They Form | What They Want | Power Source |
|------|-------------|----------------|-------------|
| **Kingdom/Monarchy** | Conquest, inheritance, territorial claim | Land, stability, legacy | Military (real soldiers), population size |
| **Theocracy** | Prophet with enough followers in a settlement | Spread their god's doctrine | Devotion of population, prophets, temples |
| **Merchant Guild** | Settlement with strong trade connections | Wealth, trade access, economic influence | Merchants in population, trade routes |
| **Tribal/Clan** | Shared race in a region, especially minorities | Survival, autonomy, homeland | Warriors of that race, territorial knowledge |
| **Religious Order/Cult** | Fanatical devotion to a specific doctrine | Convert others, purify the unfaithful | Fanatical followers, prophet doctrine |
| **Criminal Syndicate** | Concentration of Treacherous/Cunning unhappy people | Wealth and power from the shadows | Treacherous/Cunning population |
| **Military Order** | War veterans concentrated in a settlement | Protect territory, serve as mercenaries | Soldiers with SurvivedWar events |

### Core Principles

1. **Every settlement belongs to someone** — either a faction or it's independent (self-governing). No `owner_faction: 0`.

2. **Faction strength = population** — military comes from actual soldiers, wealth from actual merchants/trade, stability from happiness of population.

3. **Factions form from conditions** — a prophet creates a theocracy, a dominant race creates a tribal clan, a merchant-heavy city creates a guild, a conqueror creates a kingdom.

4. **Faction identity = shared traits/faith/race** — members share something. When that shared thing breaks down (faith conflict, racial mixing, trait divergence), the faction fractures.

5. **Wars have reasons** — resource scarcity, religious conflict (prophet of god X in faction worshipping god Y), racial expansion, revenge for past conquests.

6. **Factions can merge, split, and transform** — a religious order could become a theocracy. A kingdom could split into tribal clans after a defeat. A merchant guild could be absorbed by a kingdom.

## Implementation Approach

### Phase 1: Connect Factions to Population

**Don't rework faction types yet.** First, make existing factions use real population data:

- **Military strength = sum of combat_score of all soldiers in faction settlements**
- **Wealth = number of merchants × trade routes**
- **Stability = average happiness of population in faction settlements**
- **Remove abstract gauge regeneration** — these numbers are computed, not stored
- **Assign all unowned settlements to factions** — use proximity/region to determine which faction claims each map settlement at the start

### Phase 2: Faction-God Connection

- **Faction patron god = the most-worshipped god across faction settlements** (derived from population, like settlement patron)
- **Religious wars**: when a faction's population worships god X but a neighboring faction worships god Y, and a prophet is preaching, sentiment drops
- **Faction sentiment modified by god relationships** — if gods are hostile, their factions are too

### Phase 3: Emergent Faction Formation

Replace random faction spawning with condition-based formation:

- **Theocracy**: a prophet with 50+ followers in a settlement that has no faction → theocracy forms
- **Tribal clan**: a settlement where 70%+ of population is a single non-dominant race (racial minority large enough to self-govern) → clan forms
- **Criminal syndicate**: 10+ people with Treacherous/Cunning traits and happiness < 30 in a single settlement → syndicate forms
- **Military order**: 10+ war veterans (SurvivedWar events) in a settlement → military order forms
- **Merchant guild**: settlement with 20+ merchants and active trade routes → guild forms
- **Kingdom**: any faction that controls 3+ settlements → becomes a kingdom

### Phase 4: Faction Behavior by Type

Each faction type has different priorities:

- **Kingdom**: expand territory, defend borders, maintain stability
- **Theocracy**: spread faith, build temples, persecute heretics
- **Merchant guild**: establish trade routes, avoid wars, accumulate wealth
- **Tribal clan**: defend homeland, maintain racial purity, resist outsiders
- **Religious order**: convert settlements, send missionaries, reject material wealth
- **Criminal syndicate**: infiltrate other factions, assassinate, steal
- **Military order**: fight wars on behalf of others, train soldiers, defend settlements

### Phase 5: Dissolution and Succession

Replace random dissolution with meaningful collapse:

- **Faction dissolves when population support drops** — if average happiness of faction members < 20 for 5+ years
- **Succession crisis**: when a leader dies, competing claimants based on traits (Ambitious, PowerHungry characters)
- **Rebellion**: conquered settlement with different race/faith than ruling faction → rebellion chance based on unhappiness
- **Absorption**: weak faction near strong faction → may voluntarily join (if same race/faith)

## Data Model Changes

### FactionState Rework

```rust
pub struct FactionState {
    pub id: u32,
    pub name: String,
    pub faction_type: FactionType,
    pub founded_year: i32,
    pub dissolved_year: Option<i32>,
    pub home_settlement: u32,  // primary settlement ID
    pub settlements: Vec<u32>, // all owned settlement IDs

    // Identity — what holds this faction together
    pub founding_race: Race,     // original race (may diverge from current population)
    pub founding_god: Option<GodId>, // original patron (may diverge)
    pub founding_prophet: Option<u32>, // person ID if formed by a prophet

    // Leader
    pub leader_id: Option<u32>,  // Character ID

    // Derived each year from population (NOT stored gauges)
    // military_strength → computed from soldiers
    // wealth → computed from merchants + trade
    // stability → computed from happiness
}
```

### FactionType Expansion

```rust
pub enum FactionType {
    Kingdom,
    Theocracy,
    MerchantGuild,
    TribalClan,
    ReligiousOrder,
    CriminalSyndicate,
    MilitaryOrder,
}
```

### Settlement Ownership

Every settlement must have an owner. At worldgen start:
1. Faction-generated settlements keep their owner
2. Map settlements are assigned to the nearest faction by distance
3. Settlements too far from any faction become independent (self-governing, no faction — but still have a de facto leader)

## Existing Code to Modify

### `src/worldgen/history/state.rs`
- Rework `FactionState` struct
- Change `FactionType` enum
- Remove `military_strength`, `wealth`, `stability` as stored fields (compute them)

### `src/worldgen/history/world_events.rs`
- `evaluate_war_declared` — use real sentiment sources (religion, race, resources)
- `evaluate_war_ended` — use real military power from population
- `evaluate_faction_dissolved` — based on population happiness, not arbitrary checks
- `evaluate_new_faction` → `evaluate_faction_formation` — condition-based, not random
- Settlement upkeep — faction stability from actual population

### `src/worldgen/history/mod.rs`
- Initial faction setup — assign map settlements to factions
- Faction gauges computed each year from population

### `src/worldgen/population/war.rs`
- `faction_military_power` already exists and computes from real soldiers — wire this in

### `src/worldgen/population/faith.rs`
- Faction patron god derived from population faith

## Dependencies

The population module is ready. The following systems exist and can be used:
- Per-person race, traits, faith, happiness, occupation
- Combat score per soldier
- Merchant count per settlement (for trade capacity)
- Prophet system (for theocracy formation)
- Migration system (for population movement after conquest)
- Settlement patron god and dominant race (derived yearly)
- Trade routes between settlements

## Testing

Use the `multi_seed_comparison` test to verify across 8 seeds:
- Factions alive should be 3-6 (not 1-3)
- War count should remain 10-30
- Faction types should vary by seed
- Religious wars should appear when gods/prophets conflict
- New factions should emerge from population conditions, not random spawning

## Open Questions

- Should independent settlements (no faction) be a permanent state or always get absorbed?
- Can a person belong to multiple factions? (e.g., merchant guild member in a kingdom)
- How do factions tax their settlements? Does wealth flow from settlements to faction?
- Should faction leaders come from the population (notable people) rather than generated characters?
- How do criminal syndicates interact with the faction that officially controls a settlement?
