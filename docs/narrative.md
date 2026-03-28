# Narrative Functions

## Overview

39 narrative functions derived from Propp's folktale morphology, Shakespeare's tragic arcs, and Dostoevsky's moral tests. These are the building blocks of emergent stories — not scripted sequences, but patterns that should arise naturally from simulation when the right ingredients exist.

Not every story uses all functions. A typical arc uses 8-12 in sequence. The key property is they are **sequential and causal** — each one enables the next.

## The Three Shapes

- **Propp (Journey)**: outward and returning triumphant. Departure → trials → struggle → victory → return.
- **Shakespeare (Descent)**: spiralling downward to destruction. Greatness → flaw → escalation → isolation → catastrophe.
- **Dostoevsky (Test)**: turning inward toward painful self-awareness. Belief → transgression → guilt → crisis → ambiguous redemption.

## Unified Function List

| # | Function | Description | Origin |
|---|----------|-------------|--------|
| 1 | **Establishment** | A person holds a position, belief, or stable life | Shakespeare, Dostoevsky |
| 2 | **Absentation** | Someone important leaves or is lost | Propp |
| 3 | **Interdiction** | A rule, duty, or moral boundary exists | Propp, Dostoevsky |
| 4 | **Violation** | The boundary is crossed | Propp |
| 5 | **Temptation** | An opportunity aligned with a flaw or desire appears | Shakespeare |
| 6 | **Transgression** | Acting on temptation against one's principles | Shakespeare, Dostoevsky |
| 7 | **Justification** | Rationalising the transgression | Dostoevsky |
| 8 | **Initial Success** | The transgression appears to pay off | Shakespeare |
| 9 | **Trickery** | Deception by an antagonist | Propp |
| 10 | **Complicity** | The victim is deceived or complicit | Propp |
| 11 | **Villainy/Harm** | Someone causes deliberate harm | Propp |
| 12 | **Lack** | The protagonist needs something they don't have | Propp |
| 13 | **Mediation** | The problem becomes known, action demanded | Propp |
| 14 | **Counteraction** | The protagonist decides to act | Propp |
| 15 | **Departure** | Leaving home or safety | Propp |
| 16 | **Trial** | The protagonist is tested | Propp |
| 17 | **Aid** | Help from an unexpected source | Propp |
| 18 | **Guidance** | Led toward the goal by another | Propp |
| 19 | **Struggle** | Direct confrontation with the antagonist | Propp, Shakespeare |
| 20 | **Branding** | The hero is permanently marked or changed by the struggle | Propp |
| 21 | **Victory** | The antagonist is overcome | Propp |
| 22 | **Guilt** | Internal consequences of past actions | Shakespeare, Dostoevsky |
| 23 | **Escalation** | More wrongs needed to cover the first | Shakespeare |
| 24 | **Isolation** | Allies fall away, trust erodes | Shakespeare |
| 25 | **Counterforce** | Those wronged organise against the protagonist | Shakespeare |
| 26 | **Suffering of Innocents** | Consequences spread to the undeserving | Dostoevsky |
| 27 | **Unravelling** | The web of lies or crimes collapses | Shakespeare |
| 28 | **Pursuit** | The protagonist is hunted | Propp |
| 29 | **Rescue** | Saved from pursuit or destruction | Propp |
| 30 | **Crisis of Identity** | "Who am I after all this?" | Dostoevsky |
| 31 | **Confession** | The truth comes out, voluntarily or forced | Dostoevsky |
| 32 | **Recognition** | The protagonist is seen for who they truly are | Propp, Shakespeare |
| 33 | **Exposure** | A false claim or pretender is revealed | Propp |
| 34 | **Humiliation** | Public stripping of status | Dostoevsky |
| 35 | **Punishment** | The wrongdoer faces justice | Propp |
| 36 | **Transfiguration** | The protagonist is transformed | Propp |
| 37 | **Restoration** | Order returns, but changed | Shakespeare |
| 38 | **Redemption** | Forgiveness offered, but damage remains | Dostoevsky |
| 39 | **Ascension** | The protagonist is elevated — marriage, crown, title | Propp |

## What We Can Currently Detect

Functions that map to existing simulation events:

| Function | Current Event/System | Notes |
|----------|---------------------|-------|
| Establishment | Person seeded with occupation, settlement, faith | Implicit, not tracked as event |
| Absentation | LostParent, LostSpouse | Family member lost |
| Departure | DraftedToWar | Forced departure, not voluntary |
| Trial | SurvivedWar, SurvivedPlague | Survival as test |
| Struggle | War casualties, combat score | Abstract, not personal |
| Victory | SurvivedWar | Surviving = winning |
| Suffering of Innocents | Children dying (Illness, Plague, Famine) | Emergent from death causes |
| Branding | Notable promotion | Person marked as significant |

## What We Cannot Yet Detect

Functions that need new systems or events:

| Function | What's Missing |
|----------|---------------|
| Interdiction / Violation | No rules or moral codes exist for individuals |
| Temptation / Transgression | People don't make choices or have desires |
| Trickery / Complicity | No deception mechanics |
| Lack / Mediation / Counteraction | No personal goals or quests |
| Guidance / Aid | No mentor or helper relationships |
| Guilt / Justification | No internal state beyond faith |
| Escalation / Isolation | No social standing or reputation |
| Counterforce | No organised opposition to individuals |
| Confession / Exposure | No secrets or hidden truths |
| Humiliation / Punishment | No justice system |
| Transfiguration / Ascension | No status change beyond notable promotion |
| Crisis of Identity | No self-concept to challenge |
| Redemption | No forgiveness mechanics |

## Design Direction

The simulation should not *script* these arcs. It should create **conditions where they emerge**:

- **Faith conflict** drives Interdiction/Violation (your god says X, your settlement now worships Y)
- **War** drives Departure/Trial/Struggle/Victory/Branding
- **Resource scarcity** drives Lack/Mediation/Counteraction
- **Faction politics** drives Trickery/Complicity/Counterforce/Exposure
- **Personal relationships** drive Absentation/Guilt/Isolation/Confession

The goal is not to implement all 39 functions. It's to ensure the simulation produces enough *variety of situations* that these patterns appear naturally when you read a notable person's life events in sequence.
