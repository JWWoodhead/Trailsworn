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

**God selection:** Each run draws 5-8 gods from a hand-crafted pool of ~25. Each god is a bundle of:
- **Domain** — the magic school they embody (fire, bone, shadow, growth, storm, time, etc.)
- **Emotion/Aspect** — what they represent (rage, grief, joy, curiosity, hunger, etc.)
- **Terrain influence** — how their presence shaped geography (volcanic, fungal, crystalline, frozen, etc.)
- **Gift to mortals** — what they left behind (spells, crafting traditions, a mortal race trait)
- **Spell list** — hand-crafted, balanced abilities tied to this god's domain
- **Relationships** — hand-authored potential tensions/affinities with other gods (activated only if both are drawn)

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

## Propp's Morphology — How We Use It

Propp identified 31 narrative "functions" that appear in folk tales in a roughly fixed order. We don't use all 31 — we select subsets that map to our needs.

**For divine era god arcs**, a simplified Propp sequence might be:
1. **Initial Situation** — God rules domain, status quo
2. **Interdiction/Violation** — A rule is broken (by another god or mortals)
3. **Villainy** — Harm is caused (war, corruption, theft of power)
4. **Departure** — A god leaves or is driven from their domain
5. **Testing** — The god faces a trial
6. **Helper/Gift** — The god aids mortals or creates something
7. **Struggle** — Direct divine conflict
8. **Branding/Transformation** — The god is changed by events
9. **Resolution** — The arc reaches its conclusion (pre-Fall)

Each god gets a randomly ordered subset of these functions, filled with specifics from the simulation state. The result reads like a myth, not a spreadsheet.

**For mortal-era character arcs** (future — more simulation needed):
- Notable characters (heroes, leaders, villains) could follow Propp arcs
- NPCs the player meets might be mid-arc, creating dynamic quest hooks
- Requires deeper population simulation to support "everyday people" context

## Implementation Phases

### Phase 1: Foundation
- [x] Improve world map generation — 256x256 noise-driven with elevation/moisture/temperature, rivers, regions
- [x] Add terrain types — Sand, Snow, Swamp (DeadForest, Lava deferred to god system)
- [x] Improve zone generation — noise-based terrain with biome recipes, edge blending, river carving
- [x] In-game world map overlay (M key) with player position and zone info
- [ ] Design and implement initial god pool (6-8 gods to start, system supports 25)

### Phase 1b: World Gen Polish (before moving to gods)
- [ ] World map UI improvements — legend, settlement labels, zoom/pan, clickable zones
- [ ] Biome balance tuning — forest still dominant (~31%), needs testing across many seeds
- [ ] Zone-level visual verification — desert/tundra/swamp/coast zones need in-game visual QA
- [ ] Zone-level river appearance — current zone river carving is basic meandering stream, should match world-level river width/path
- [ ] Mountain range aesthetics — ridges could be more linear/dramatic, less blobby
- [ ] More ocean variety between seeds — some seeds still produce very land-heavy worlds
- [ ] Settlement generation improvements — current settlements are basic (dirt square + stone blocks), should reflect biome (sand buildings in desert, etc.)

### Phase 2: Divine History
- [ ] Propp narrative engine — template system for generating story arcs
- [ ] Divine era simulation — gods act on the world, shape geography
- [ ] The Fall generator — derives cause from divine era state
- [ ] Divine artifacts with real history
- [ ] God pool design — 6-8 hand-crafted gods with domains, emotions, terrain influence, spell lists

### Phase 3: Mortal History Upgrade
- [ ] Population model — demographics, migration, growth
- [ ] World map with borders, trade routes, resource distribution
- [ ] Existing faction sim upgraded to inherit from divine era
- [ ] Cultural memory of gods and the Fall

### Phase 4: Player-Facing Integration
- [ ] Ruins/sites generated from divine history
- [ ] Lore delivery — texts, NPC knowledge, environmental storytelling
- [ ] Quest generation using Propp templates
- [ ] The central mystery — discovering and acting on the Fall

## Starting God Pool (Initial 6-8)

To be designed. Each needs:
- Domain (magic school)
- Emotion/aspect
- Terrain influence
- 4-8 spells (hand-crafted, balanced)
- Relationship hooks to other gods in the pool
- Propp arc tendencies (e.g., a trickster god tends toward Violation/Deception functions)

The pool should cover enough variety that any 5-6 drawn from 6-8 feels meaningfully different. As the pool grows to 25, the combinatorial space explodes.

## Open Questions

- **Pool size per run:** 5-8 gods is the working range. Fewer = more focused, more = more complex. Needs playtesting.
- **God relationships:** Fully hand-authored pairs, or some system for emergent tension based on domain/emotion overlap?
- **Magic persistence after the Fall:** Does magic degrade over the mortal centuries? Are some spells lost? Or is it fully preserved just unguided?
- **The Fall — single cause or compound?** Is it always one event, or can it be a cascade?
- **Mortal races:** Still the existing 5 (Human, Dwarf, Elf, Orc, Goblin)? Do gods create/influence specific races?
- **Grammar/Markov chains:** Still TBD where these fit. Likely for procedural text generation (lore books, inscriptions, NPC dialogue about history) rather than structural generation.
