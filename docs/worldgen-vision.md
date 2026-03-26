# Worldgen Vision: The Divine Epochs

Design document for the procedural generation overhaul. This describes the target architecture — not everything will be built at once.

## Core Concept

The world is shaped by **dead gods**. Before mortals ruled, a pantheon of divine beings shaped geography, created magic, and warred with each other. Then they all disappeared — **the Fall**. Centuries of mortal history followed, building on the ruins of the divine era. The player enters a world layered with this history, and the central quest is discovering what caused the Fall and whether a god can be restored.

Every playthrough draws a **different subset of gods** from a hand-crafted pool, which determines:
- What magic schools exist in this world
- What terrain/biome features appear
- What factions emerged and what powers they wield
- What the Fall looked like and what caused it
- What the player's endgame looks like

## The Three Epochs

### Epoch 1: The Divine Era

Gods are alive and rule domains. This epoch is **simulated during worldgen** using Vladimir Propp's morphology of the folktale to give each god a narrative arc rather than just an event log.

**God selection:** Each run draws 6 gods from a pool (currently 8 archetypes, growing to ~25). Each god archetype has:
- **Domain** — fixed magic school (Fire, Frost, Storm, Holy, Shadow, Nature, Necromancy, Arcane)
- **Terrain influence** — fixed geography shaping (volcanic, frozen, windswept, blighted, crystalline, etc.)
- **Gift to mortals** — fixed civilizational gift (forge-craft, agriculture, navigation, magic itself, etc.)
- **Spell list** — 5 spells per god as pure data (not yet gameplay-integrated)
- **Propp tendencies** — which narrative functions their story gravitates toward

**Randomized per run:**
- **Name** — syllable-based generation (prefix + optional mid + suffix), producing names like "Voraphel", "Tharnaes", "Xariel"
- **Personality traits** — 2-4 `CharacterTrait` values rolled from weighted tables with domain-specific modifiers and thematic blocklists. Uses the same trait pool as mortal characters (Warlike, Wise, Treacherous, Devout, etc.)
- **Relationships** — emergent, computed from domain category overlap, trait axis alignment, and forbidden school dynamics. No hand-authored pairs.

**Propp-driven narrative arcs:** Each god's history follows a subset of Propp's 31 narrative functions, giving their story a recognizable folktale shape:
- Departure (a god leaves their domain)
- Testing (challenged by another god or mortal event)
- Villainy (a god causes harm — intentional or not)
- Struggle (divine conflict)
- Transformation (a god changes, gains/loses power)
- etc.

The divine era simulation is simpler than the mortal era — fewer actors, bigger moves, mythic scale. Events are things like: "The God of Bone raised a mountain range to wall off the God of Storm's domain" or "The God of Dream gifted mortals prophetic sight, angering the God of Time."

**Output:** A set of god histories, divine artifacts, terrain features baked into geography, origin of magic schools, and the seeds of mortal civilizations.

### Epoch 2: The Fall

All gods disappear. The cause is **procedurally determined** each run from the state of the divine era — maybe one god betrayed the others, maybe mortals did something, maybe it was an inevitable consequence of a specific divine conflict. The Fall is the central mystery of each playthrough.

**Key design constraint:** The Fall must be:
- Discoverable through gameplay (ruins, texts, NPC knowledge, artifacts)
- Consistent with the divine era history (not random — it follows from what happened)
- Different each run (because different gods = different divine era = different Fall)

**Effects of the Fall:**
- Magic persists but is no longer replenished/guided — fragmented, degrading, misunderstood
- Divine domains become wild/corrupted zones
- Artifacts of the gods become the most powerful items in the world
- Mortal civilizations that depended on divine patronage collapse or adapt

### Epoch 3: The Mortal Era

Centuries of mortal history after the Fall. This is an evolution of the **existing history simulation**, but now:
- Factions inherit territory from divine domains
- Available magic comes from whichever gods existed (their spells persist)
- Cultural values are shaped by which god their ancestors served
- Artifacts from the divine era are treasured/feared/contested
- The Fall is remembered differently by different cultures (some blame specific gods, some blame mortals, some deny it happened)

**Population model needed:** Settlements with actual population counts, growth rates, migration. Factions need demographic weight, not just abstract gauges. Trade routes, resource distribution, demographic pressure driving expansion.

**World map upgrade needed:** Real geography with contiguous regions, coastlines, rivers, mountain ranges. Divine domains map to geographic regions. Factions have actual borders. Adjacency matters for trade, war, migration.

## What Is Procedural vs Hand-Crafted

| Element | Approach | Why |
|---------|----------|-----|
| Gods (pool of ~25) | **Hand-crafted** | Need to be balanced, thematically rich, have designed relationships |
| God selection per run (5-8) | **Procedural** | Core replayability mechanic |
| Spells/abilities | **Hand-crafted** | Must be balanced for gameplay |
| Divine era narrative arcs | **Procedural** (Propp templates) | Unique story each run |
| The Fall (cause & effects) | **Procedural** (derived from divine era state) | Central mystery, must vary |
| World geography | **Procedural** (noise + divine influence) | Must support god domains as contiguous regions |
| Mortal era history | **Procedural** (existing sim, upgraded) | Already built, needs enhancement |
| Faction names/characters | **Procedural** (existing generators) | Already built, works well |
| Quest structure | **Procedural** (Propp templates) | Player-facing narrative arcs |
| Individual quest content | **Mix** | Hand-crafted encounters, procedural context |

## Narrative Grammars — Beyond Propp

The core insight: different literary traditions produce different kinds of stories, and a procedural narrative engine should be able to generate stories that feel like *stories*, not event logs. We identify four narrative grammars, each inspired by a different storytelling tradition. **Any grammar can apply at any scale** — gods, mortal leaders, heroes, factions. Assignment depends on traits and situation, not power level.

### Grammar 1: Propp (The Journey)

Departure → trials → magical aid → struggle → victory → return. The classic hero's journey.

**Fits characters with:** Brave, Honorable, Loyal — the virtue/peace axis.

**Simplified function sequence:**
1. **Initial Situation** — rules domain, status quo
2. **Interdiction/Violation** — a rule is broken
3. **Villainy** — harm is caused
4. **Departure** — leaves home/domain
5. **Testing** — faces a trial
6. **Helper/Gift** — receives or gives aid
7. **Struggle** — direct conflict
8. **Transformation** — changed by events
9. **Resolution** — the arc concludes

**Examples:** A god who departs their domain to aid mortals. A mortal hero on a quest. A faction that rises from exile to reclaim its homeland.

### Grammar 2: Shakespeare (The Descent)

Greatness → flaw → catalyst → spiral → too-late knowledge → destruction. The tragic ruler.

**Fits characters with:** Ambitious, PowerHungry, Ruthless — the aggression/ambition axis.

**Function sequence:**
1. **Eminence** — the character holds power and respect
2. **Flaw Revealed** — a moral weakness is exposed (to the reader, not the character)
3. **Catalyst** — an external event activates the flaw (a temptation, a rival, a prophecy)
4. **Transgression** — the character makes a decisive wrong choice
5. **Spiral** — increasingly desperate actions, loyal allies destroyed or alienated
6. **Self-Knowledge** — the character finally understands what they've done (too late)
7. **Destruction** — catastrophic fall restores a kind of order

**Examples:** A god who overreaches and triggers the Fall. A mortal king who destroys his own kingdom through paranoia. A faction leader whose ambition leads to a war that consumes their people.

### Grammar 3: Dostoevsky (The Test)

Idea held → tested through action → fever/crisis → encounter with humility → confession/redemption (ambiguous). The ideologue.

**Fits characters with:** Scholarly, Fanatical, Wise — the intellect/zeal axis.

**Function sequence:**
1. **Thesis** — the character holds a philosophical conviction
2. **Transgressive Act** — they test the idea through action (often extreme)
3. **Fever** — psychological disintegration, the consequences unfold
4. **The Humble Mirror** — they encounter someone who embodies the opposite of their thesis
5. **Confession** — a breakdown or admission
6. **Ambiguous Redemption** — the outcome is uncertain, the idea is neither fully vindicated nor fully refuted

**Examples:** A god who believes mortals must suffer to grow, and tests this by withdrawing protection. A mortal scholar who pursues forbidden magic and loses themselves. A religious leader who enforces doctrine so rigidly that it breaks their own faith.

### Grammar 4: Dickens (The Web)

Hidden connections → identity mysteries → coincidental bonds revealed → secret benefactors/antagonists exposed. The structural pattern.

**Not assigned to individual characters.** This grammar operates across *multiple* arcs simultaneously. It's about how the player (or a historian in-world) discovers that seemingly unrelated events — a war in the north, a plague in the south, an artifact discovered underground — all trace back to the same god's actions during the divine era.

**Function:** When generating history, certain events should be secretly linked. Two factions at war might both unknowingly worship the same god's splinter doctrines. An artifact discovered by one faction was created during a divine conflict that also shaped the mountain range another faction calls home. These connections aren't visible during simulation — they're visible when the player pieces together lore.

**Implementation:** Tag events/artifacts/locations with causal chains back to divine-era actions. The player-facing lore delivery system (Phase 4) surfaces these connections through ruins, texts, and NPC knowledge.

### Grammar Assignment

When a notable character is generated (god or mortal), their dominant trait axis determines their grammar:

| Dominant Axis | Grammar | Why |
|---------------|---------|-----|
| Virtue/Peace (Brave, Honorable, Loyal, Peaceful) | Propp | Classic hero/helper arc |
| Aggression/Ambition (Ambitious, PowerHungry, Warlike, Ruthless) | Shakespeare | Tragic overreach |
| Intellect/Zeal (Scholarly, Fanatical, Wise, Cunning) | Dostoevsky | Ideas tested to breaking |
| Darkness/Fear (Cruel, Treacherous, Paranoid, Corrupt) | Shakespeare or Dostoevsky | Depends on whether they act from power (Shakespeare) or ideology (Dostoevsky) |

The Dickens web is always layered on top as a structural concern, not a per-character assignment.

### Design Principle: Cause and Effect, Not Random Chance

**Events should happen because conditions make them inevitable, not because a dice roll succeeded.** This is the Caves of Qud philosophy: the simulation creates situations where outcomes are *determined by state*, not by probability.

The current history simulation relies heavily on probability checks ("20% chance of war if sentiment < -20"). The narrative grammar system should replace this with **condition-driven state machines:**

- A character doesn't betray with a 5% chance. A character betrays because they have `Treacherous`, they're in an alliance with a weakening faction, their ambition is `SeizePower`, and the allied leader just lost a war. All conditions true → betrayal happens.
- A god doesn't trigger the Fall with a random roll. The Fall happens because a Shakespeare-grammar god reached their "destruction" beat, they had negative relationships with 3+ other gods, and their domain was the most militarily powerful. The cause is traceable.
- A faction doesn't found a settlement with an 8% probability. A faction founds a settlement because they have wealth > threshold, stability > threshold, and an unclaimed region adjacent to their territory. The conditions are met → it happens.

**Randomness is for initial conditions** (which gods are drawn, what traits they get, where settlements start). **Determinism is for consequences** (given this state, what must happen next).

### How Grammars Work in Simulation

Grammars are **state machines**, not probability modifiers. Each grammar defines:
1. A sequence of **beats** (narrative phases)
2. **Entry conditions** for each beat (what world state triggers the transition)
3. **Effects** of each beat (what the character does/causes while in this phase)

The grammar tracks which beat the character is currently in. Beats advance when their entry conditions are met — not on a timer or a roll.

**Example — Shakespeare grammar for a mortal king with `Ambitious, Ruthless`:**
1. **Eminence** (entry: character becomes Leader) → effect: military +10, wealth +5
2. **Catalyst** (entry: a rival faction exists with sentiment < -10) → effect: character fixates on the rival
3. **Transgression** (entry: character's military > rival's AND character has Ruthless) → effect: declares war regardless of alliance obligations
4. **Spiral** (entry: war lasts > 5 years OR ally betrays) → effect: stability -5/year, paranoia increases, purges advisors
5. **Self-Knowledge** (entry: stability < 20 OR military < 15) → effect: generates a "confession" cultural memory
6. **Destruction** (entry: stability < 0 OR defeated in war) → effect: faction dissolves or leader is overthrown

Each transition is **deterministic given the state**. Different world states produce different stories, but none of them require a coin flip.

When two characters' grammars collide — a Shakespeare villain versus a Propp hero — the interaction creates emergent narrative tension. The hero's "struggle" beat requires an antagonist; the villain's "spiral" beat requires someone to destroy. They find each other through the state machine, not through random encounter.

## The Doctrine Layer (Design Thinking)

The bridge between "gods exist" (Phase 1) and "gods shaped mortal civilization" (Phases 2-3).

### Teachings vs Personality

A god has two layers:
- **What they teach** (fixed per archetype): their domain + gift to mortals. The Fire god teaches forge-craft. The Nature goddess teaches agriculture. This is their *function* in the world.
- **How they teach it** (randomized per run): their personality traits. A Cruel nature goddess teaches agriculture through survival-of-the-fittest ("the weak harvest feeds the strong"). A Wise nature goddess teaches agriculture through careful stewardship ("tend the soil and it tends you").

**Doctrine = teachings filtered through personality.** The same domain produces different doctrine each run because the god's personality colors the interpretation. This is the core replayability mechanism for the divine layer.

### Doctrine → Culture Pipeline

When mortal factions form in a god's domain during the mortal era:
1. **Inherit doctrine** — the faction starts with cultural values and taboos derived from their god's domain + traits
2. **Trait bias** — children born in the faction have their character trait rolling biased toward the god's trait modifiers (a faction under a Warlike fire god produces more Warlike characters)
3. **Doctrine drift** — over mortal centuries, doctrine drifts. Random events cause factions to reinterpret, emphasize different aspects, or abandon their god's teachings
4. **Schisms** — when a god has contradictory traits (Cruel + Wise), their followers might split into factions emphasizing cruelty vs wisdom. Both claim to follow the true teaching.

### Doctrine → Values/Taboos Mapping (Draft)

| God Domain | Gift | Base Cultural Value | Trait-Dependent Additions |
|-----------|------|-------------------|--------------------------|
| Fire | Forge-craft | Craftsmanship | +MilitaryProwess if Warlike, +Commerce if Ambitious |
| Frost | Preservation | Resilience | +Scholarship if Wise, +Independence if Reclusive |
| Storm | Navigation | Commerce | +Expansion if Ambitious, +Scholarship if Scholarly |
| Holy | Law & healing | Piety | +Unity if Just, +MilitaryProwess if Fanatical |
| Shadow | Stealth & writing | Scholarship | +Commerce if Cunning, +Independence if Paranoid |
| Nature | Agriculture | Resilience | +Piety if Devout, +Diplomacy if Peaceful |
| Necromancy | Knowledge of death | Scholarship | taboo: Religion. +MilitaryProwess if Cruel |
| Arcane | Magic itself | Scholarship | +Expansion if Ambitious, +Craftsmanship if Wise |

### Connection to Narrative Grammars

A god's personality determines their narrative grammar. A god on a Shakespeare arc (Ambitious, PowerHungry) will have their "spiral" and "destruction" phases generate events that become doctrine for their followers — "our god fell because of hubris" becomes a cultural memory and taboo against ambition. A god on a Propp arc (Brave, Honorable) will have their "helper/gift" and "victory" phases generate events that become cultural values — "our god triumphed through courage" becomes a cultural value of MilitaryProwess or Bravery.

The Fall itself is the ultimate doctrinal event. What caused the Fall — and which god's arc was responsible — shapes the entire mortal era's relationship with divinity.

## Implementation Phases

### Phase 1: Foundation
- [x] Improve world map generation — 256x256 noise-driven with elevation/moisture/temperature, rivers, regions
- [x] Add terrain types — Sand, Snow, Swamp (DeadForest, Lava deferred to god system)
- [x] Improve zone generation — noise-based terrain with biome recipes, edge blending, river carving
- [x] In-game world map overlay (M key) with player position and zone info
- [x] Design and implement initial god pool (8 gods: Vorthak, Seraphel, Kaelthos, Luminael, Neth, Yrathis, Morvrith, Aethon. 6 drawn per run. Emergent relationships. Spells as data only.)

### Phase 1b: World Gen Polish (before moving to gods)
- [x] World map UI improvements — legend, settlement icons + name labels, zoom/pan (scroll + arrows), clickable zones
- [x] Biome balance tuning — forest capped <30%, moisture threshold raised to 0.55, ocean boost reduced to 0.20
- [x] Zone-level visual verification — terrain composition tests per biome, coast/swamp water tests added
- [x] Zone-level river appearance — world-level entry/exit edges + width passed to zone carving, noise-driven curved paths with riverbanks
- [x] Mountain range aesthetics — domain-warped ridge noise with seed-driven directional stretch, gap-filling smoothing pass
- [x] More ocean variety between seeds — continent-scale noise layer + adaptive ocean threshold (15-60% ocean guaranteed)
- [x] Settlement generation improvements — biome-aware theming (sand/snow/swamp/coast), settlement names generated and displayed on map

### Phase 2: Divine History & Narrative Engine
- [ ] Doctrine layer — gods' teachings (domain + gift) filtered through personality (traits) to produce doctrine per run
- [ ] Narrative grammar system — assign Propp/Shakespeare/Dostoevsky arc templates to characters based on traits
- [ ] Divine era simulation — gods act on the world using narrative beats, shape geography, create artifacts
- [ ] The Fall generator — derives cause from divine era state (which god's arc ended in what way)
- [ ] Dickens web — track hidden connections between gods/artifacts/events for player discovery

### Phase 3: Mortal History Upgrade
- [ ] Doctrine → culture pipeline — factions inherit god doctrines, which set initial cultural values/taboos and bias character trait rolling
- [ ] Doctrine drift — factions reinterpret teachings over centuries, creating schisms and heresies
- [ ] Mortal character arcs — notable characters get narrative grammars that shape their event probabilities
- [ ] Population model — demographics, migration, growth
- [ ] World map with borders, trade routes, resource distribution
- [ ] Religious events — schisms, heresies, miracles, conversions (evaluate_religious_schism implementation)

### Phase 4: Player-Facing Integration
- [ ] Ruins/sites generated from divine history
- [ ] Lore delivery — texts, NPC knowledge, environmental storytelling
- [ ] Quest generation using Propp templates
- [ ] The central mystery — discovering and acting on the Fall

## God Pool (Implemented)

8 archetypes covering 8 of 10 magic schools (Enchantment and Blood reserved for expansion):

| # | Title | Domain | Terrain | Gift | Propp Tendencies |
|---|-------|--------|---------|------|-----------------|
| 1 | God of Fire | Fire | Volcanic (Stone/Sand, future: Lava) | Forge-craft | Villainy, Struggle, Transformation |
| 2 | Goddess of Frost | Frost | Frozen (Snow/Water, future: Ice) | Preservation | Departure, Testing, HelperGift |
| 3 | God of Storm | Storm | Windswept (Grass/Stone) | Navigation | Violation, Testing, Transformation |
| 4 | God of Holy Light | Holy | Radiant (Grass/Stone, future: HallowedGround) | Law & healing | Interdiction, HelperGift, Struggle |
| 5 | God of Shadow | Shadow | Darkened (Forest/Swamp, future: Shadowlands) | Stealth & writing | Departure, Villainy, Testing |
| 6 | Goddess of Nature | Nature | Overgrown (Forest/Swamp, future: DeepWild) | Agriculture | InitialSituation, HelperGift, Transformation |
| 7 | God of Death | Necromancy | Blighted (Dirt/Stone, future: Blight) | Knowledge of death | Villainy, Departure, Struggle |
| 8 | God of Arcane Knowledge | Arcane | Crystalline (Stone/Mountain, future: Crystal) | Magic itself | InitialSituation, Violation, Transformation |

Each archetype has trait weight modifiers (e.g., Fire boosts Warlike+15, Ruthless+12) and a thematic blocklist (e.g., Holy blocks Treacherous, Corrupt, Cowardly). Names are generated from ~27,000 possible combinations via syllable tables.

## Open Questions

**Resolved:**
- ~~Pool size per run~~ → 6 gods drawn per run (fixed for now, pool grows to 25)
- ~~God relationships~~ → Fully emergent from domain category + trait axes + forbidden school dynamics
- ~~God names~~ → Syllable-based procedural generation, unique per run

**Still open:**
- **Magic persistence after the Fall:** Does magic degrade over the mortal centuries? Are some spells lost? Or is it fully preserved just unguided?
- **The Fall — single cause or compound?** Is it always one event, or can it be a cascade?
- **Mortal races:** Still the existing 5 (Human, Dwarf, Elf, Orc, Goblin)? Do gods create/influence specific races?
- **Doctrine drift:** How fast do mortal factions reinterpret their god's teachings? Can a faction's doctrine become unrecognizable over centuries?
- **Narrative grammar assignment:** Should a character's grammar be fixed at birth, or can it shift as their traits evolve through events? (A Propp hero who gains Treacherous might shift to Shakespeare.)
- **Markov chains / procedural text:** Still TBD. Likely for lore books, inscriptions, NPC dialogue about history. God names currently use simpler syllable tables.
