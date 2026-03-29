# Worldgen Vision: Gods Among Mortals

Design document for the procedural generation system. Describes the target architecture — not everything is built yet.

## Core Concept

Gods are **real, present, and competing**. They don't die — they fade when forgotten and return when worshipped. The world is shaped by the ongoing tension between divine ambition and mortal lives. Every playthrough draws a **different subset of gods** from a hand-crafted pool, which determines available magic, terrain features, faction cultures, and the stories the world tells.

The player enters a world layered with 100+ years of intertwined god-and-mortal history. The gods are still out there — some powerful with vast followings, some faded to whispers, some scheming to return.

### What Varies Per Run
- Which 6 gods are drawn from the pool of 8+ archetypes
- Each god's randomized name, personality traits, drive, and flaw
- Which gods gained worshippers and which faded
- What factions formed under which divine patrons
- What artifacts, sacred sites, and races the gods created
- What wars, betrayals, and alliances shaped the world

## God Personality — The Three Axes

Every drawn god has three layers that create their character:

### 1. Domain (fixed per archetype)
What the god *is*. Fire, Frost, Storm, Holy, Shadow, Nature, Death, Arcane. Determines terrain influence, magic school, gift to mortals, and associated creatures. Shapes the god's emotional register and worship style:
- **Fire**: creates through destruction and transformation. Intense, consuming, demanding worship.
- **Frost**: creates through preservation and stillness. Austere, remote, patient.
- **Storm**: creates through force and unpredictability. Volatile, brilliant, mercurial.
- **Holy**: creates through law and revelation. Certain, structured, radiant.
- **Shadow**: creates through secrecy and subtlety. Guarded, watchful, hidden worship.
- **Nature**: creates through growth. Patient, nurturing, but terrifying when provoked.
- **Death**: creates through ending and transformation. Patient, inevitable, lonely.
- **Arcane**: creates through pure understanding. Detached, curious, cerebral.

### 2. Drive (rolled from domain + traits)
What the god *wants most*. The engine of their story:
- **Knowledge** — understand the secret of creation (Odin, Thoth)
- **Dominion** — everything under their rule (Zeus, Ra)
- **Worship** — be the most revered by mortals (Apollo, Amun)
- **Perfection** — create the ultimate work (Hephaestus)
- **Justice** — make the world fair and ordered (Athena, Ma'at)
- **Love** — protect what they've become attached to (Isis, Freya)
- **Freedom** — be unbound, break every rule (Loki, Hermes)
- **Legacy** — their creations must outlast everything (Prometheus)
- **Vindication** — prove they were right (Hera, Set)
- **Supremacy** — be the strongest, defeat all rivals (Ares, Thor)

### 3. Flaw (rolled from domain + traits)
How pursuing the drive *destroys them*. The tragic dimension:
- **Hubris** — believes they're above consequences
- **Jealousy** — can't stand others having what they want
- **Obsession** — pursues the drive past all reason
- **Cruelty** — punishes the wrong people
- **Blindness** — can't see the cost to others
- **Isolation** — pushes everyone away
- **Betrayal** — breaks trust to get what they want
- **Sacrifice** — gives up too much, loses themselves
- **Rigidity** — so committed they can't adapt
- **Hollowness** — gets what they want and finds it meaningless

Domain sets base weights for drives/flaws. Rolled traits shift those weights. The same Fire god archetype might get Supremacy/Hubris one run and Knowledge/Obsession the next — different stories from the same building blocks.

## Worship as Core Resource

Gods need worshippers. Without them, they fade.

- **Settlement patronage**: each of the ~70 world map settlements can worship one god
- **Conversion**: gods in whose territory a settlement falls can claim or convert it
- **Drive affects worship**: a Worship-driven god courts mortals aggressively; a Knowledge-driven god attracts seekers passively
- **Power = worshippers**: a god's ability to act (create artifacts, shape terrain, fight wars) scales directly with how many settlements worship them
- **Fading**: a god with zero worshippers for 20+ years fades — still exists but can't act
- **Revival**: if worship returns, faded gods reawaken

This creates natural stories: a god who neglects worshippers (Obsession flaw) weakens. A god who alienates everyone (Isolation flaw) fades. A god who steals worshippers (Dominion drive) triggers conflict.

## Divine Creatures

Each domain has 4 associated mythical creature types:
- **Guardian** — guards sacred sites (Salamander, Ice Wyrm, Treant, etc.)
- **Warrior** — fights in divine wars (Forge Golem, Frost Giant, Griffin, etc.)
- **Emissary** — appears to mortals as omens (Phoenix, Thunderbird, Spectral Hound, etc.)
- **Companion** — accompanies the god (Fire Drake, Crystal Spirit, Familiar, etc.)

These persist after a god fades — abandoned servants haunting the ruins of divine sites. This explains *why* specific enemy types exist in specific zones.

## Flaw Pressure System

God flaws don't fire constantly — they trigger when pressure builds:
- **Hubris** pressure builds from victories and successful creations
- **Jealousy** pressure builds when other gods gain worshippers or create things
- **Obsession** pressure builds steadily (the drive itself is the pressure)
- **Cruelty** pressure builds from frustration (contested territory, broken pacts)
- **Isolation** pressure builds from deteriorating relationships

When pressure exceeds threshold (~80), the flaw triggers a narrative event:
- An obsessed god neglects their worshippers (devotion drops)
- A jealous god turns on the most successful rival (relationship plummets)
- A cruel god lashes out and their own followers suffer
- An isolated god withdraws and becomes unreachable
- A betraying god shatters a pact for personal gain

These events create the drama of the world's history — they're not random, they emerge from who the gods are.

## God Influence on Mortal Events

Patron gods color faction behavior:
- **Supremacy/Dominion** patron: +15% war probability for their factions
- **Worship/Love** patron: -10% war probability
- **Same patron**: -20% war between co-religionists, +25% alliance chance
- **Hostile patrons**: +15% war probability between their factions
- **High devotion** (>60): +2 faction stability/year

Planned but not yet implemented:
- **Holy War**: war between factions of different patron gods
- **Religious Schism**: faction changes patron or goes patron-less
- **Divine Intervention**: god spends power to influence a mortal war

## Narrative Grammars (Design Thinking)

Four literary-inspired story templates that can apply to any character:

1. **Propp (Journey)**: departure → trials → aid → struggle → victory → return. Fits Brave, Honorable, Loyal.
2. **Shakespeare (Descent)**: greatness → flaw → catalyst → spiral → destruction. Fits Ambitious, PowerHungry, Ruthless.
3. **Dostoevsky (Test)**: thesis → transgression → crisis → humility → ambiguous redemption. Fits Scholarly, Fanatical, Wise.
4. **Dickens (Web)**: hidden connections across multiple arcs. Not per-character — structural layer.

Not yet implemented as state machines. Currently, drives and flaws create narrative-shaped behavior organically. Full grammar system is a future enhancement.

## Doctrine Layer (Design Thinking)

Gods have two layers:
- **What they teach** (fixed): domain + gift to mortals
- **How they teach it** (randomized): personality traits color the interpretation

A Cruel nature god teaches agriculture through survival-of-the-fittest. A Wise nature god teaches through careful stewardship. Same domain, different doctrine, different culture.

**Doctrine → Culture Pipeline** (planned):
1. Factions inherit cultural values/taboos from patron god's domain + traits
2. Character trait rolling biased toward patron god's modifiers
3. Doctrine drift over centuries — reinterpretation, schisms
4. Contradictory traits in a god → followers split into competing doctrines

## What Is Procedural vs Hand-Crafted

| Element | Approach | Why |
|---------|----------|-----|
| God archetypes (pool of 8+) | **Hand-crafted** | Balanced, thematically rich |
| God selection per run (6) | **Procedural** | Core replayability |
| God personality (drive/flaw) | **Procedural** (weighted from domain + traits) | Unique character each run |
| Spells/abilities | **Hand-crafted** | Must be balanced for gameplay |
| World geography | **Procedural** (noise + divine influence) | God domains as contiguous regions |
| History (god + mortal) | **Procedural** (unified simulation) | 100+ years of intertwined events |
| Faction names/characters | **Procedural** (existing generators) | Works well |

## Implementation Status

### Complete
- [x] World map generation — 256x256 with elevation/moisture/temperature, rivers, regions, ~70 settlements
- [x] Zone generation — noise-based terrain, biome recipes, edge blending, rivers, features, settlements
- [x] God pool — 8 archetypes with domain/terrain/gifts/spells/traits
- [x] God personality — drives and flaws rolled from domain + traits
- [x] Divine creatures — 4 per domain with roles
- [x] God behavior system — territory expansion, worship competition, drive-based actions, flaw triggers
- [x] Unified history simulation — gods and mortals in same timeline, 10-phase year loop
- [x] Settlement worship — patron gods, devotion, conversion
- [x] Divine artifacts, sites, races, terrain scars
- [x] World map UI — legend, settlement icons, zoom/pan, clickable zones

- [x] Population simulation — person-level lifecycle (birth/death/marriage), 10 occupations, family life events
- [x] Settlement resources — 5 resources (Food/Timber/Ore/Leather/Stone), terrain-aware production, stockpiles with spoilage
- [x] War hooks — real soldier drafting, combat scoring, yearly casualties, SurvivedWar events
- [x] Plague hooks — condition-driven (overcrowding, famine, war), one-time population kill pulse
- [x] Famine system — food deficit kills infants/elderly, occupation rebalancing prevents spirals
- [x] Notable promotion — eventful people become Characters (capped per settlement per generation)
- [x] Polytheistic faith — people have relationships with multiple gods, settlement patron derived from population
- [x] Causal events — every event carries `EventCause`, full narrative chain from life_events chronology
- [x] Person traits — 28 traits seeded at birth (2), earned deterministically from life events, opposing pairs
- [x] Contextual death causes — Illness/Accident/Childbirth/Violence by age and occupation, OldAge only for 70+
- [x] Trade — merchant-driven resource sharing, distance-scaled, intra-faction + allied/treaty
- [x] Happiness & migration — happiness from conditions/traits/faith, unhappy families migrate to better settlements
- [x] God seats of power on cities/towns — fixes god fade before worship could be established
- [x] Narrative function reference — 39 functions from Propp/Shakespeare/Dostoevsky (see [docs/narrative.md](narrative.md))

### Planned
- [ ] God influence modifiers on mortal event evaluation (holy war, divine intervention, schisms)
- [ ] Patron god → faction culture pipeline (doctrine → values/taboos)
- [ ] Narrative function detection from life event sequences
- [ ] Interpersonal conflict — rivalries, betrayals, power struggles between individuals
- [ ] Positive trait triggers — acts of courage, leadership, discovery (currently most triggers are negative)
- [ ] Exile as faction decision (distinct from voluntary migration)
- [ ] World map borders, trade routes (visual)
- [ ] Ruins/sites from divine history visible in gameplay zones
- [ ] Lore delivery (texts, NPC knowledge, environmental storytelling)
- [ ] Quest generation tied to divine history

## Open Questions

- **Magic persistence**: how does a faded god's magic school work for mortals? Degraded? Preserved? Requires active worship?
- **Doctrine drift speed**: how fast do mortal factions reinterpret their god's teachings?
- **Creature behavior**: do divine creatures serve faded gods? Become wild? Hostile?
- **God revival gameplay**: can the player deliberately restore a faded god by rebuilding worship?
- **Multiple patron gods**: can a faction worship more than one god? Syncretism?
