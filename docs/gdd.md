# HEXABELLUM

## Game Design Document — Identity & Content

---

## Document Version
`GDD v1.0 — Phase 7 Aligned`

---

## 1. Game Overview

### Title
**HEXABELLUM**

### Tagline
*"Six sides of war. One victor."*

### Elevator Pitch
Hexabellum is a **turn-based tactical MOBA** played on hexagonal arenas. Two teams of five heroes clash in simultaneous planning rounds, commanding movement, abilities, and positioning before the engine resolves each round deterministically. Destroy the enemy Core to win.

### Genre
Turn-Based Strategy / MOBA / Tactical PvP

### Platform
Web browser (WASM + TypeScript client, Rust authoritative server)

### Target Audience
- Players who love tactical depth (Into the Breach, Final Fantasy Tactics, Civilization)
- MOBA players who want strategic thinking without mechanical APM pressure
- Competitive players who enjoy simultaneous planning and mind games
- Age range: 16–35

### Core Fantasy
> "I am a battlefield commander who outthinks my opponent through positioning, timing, and resource management — not reflexes."

---

## 2. Design Pillars

Every design decision in Hexabellum must serve at least one of these pillars:

### Pillar 1: Tactical Clarity
The player must always understand **why** something happened. No hidden mechanics. No random nonsense. Information is power, and the UI must deliver it cleanly.

### Pillar 2: Simultaneous Mind Games
Both teams plan in secret. The tension comes from **predicting** what the enemy will do, not from reacting faster. Every round is a puzzle of "what will they do?"

### Pillar 3: Meaningful Progression
Gold, XP, levels, and items must create **visible power spikes** that change how the match plays. A level 5 hero should feel fundamentally different from a level 1 hero.

### Pillar 4: Team Interdependence
No hero wins alone. The Vanguard needs the Warden's healing. The Sniper needs the Ranger to scout. The Berserker needs space created by teammates.

### Pillar 5: Web-First Accessibility
The game must run in a browser tab. No downloads. No installs. A player can be in a match within 30 seconds of clicking a link.

---

## 3. World & Lore

### 3.1 The World: The Convergence

In a realm where reality is woven from **six primal forces**, the fabric of existence is maintained by ancient hexagonal ley-line formations called **Convergences**.

When two factions seek to claim a Convergence, they do not wage chaotic war. Instead, they engage in the **Bellum Sextum** — the Six-Sided War — a ritualized battle fought on the hexagonal grid of the Convergence itself.

The rules are ancient:
- Five champions per side.
- The Convergence spawns constructs (minions) to aid both factions.
- Guardian sentinels (towers) defend each faction's approach.
- The heart of each faction's power is crystallized in a **Core**.
- The war ends when one Core shatters.

### 3.2 The Factions

For the MVP, the two teams are not deeply differentiated factions with unique lore. Instead, they represent:

**Team Azure (Team 0)**
- Color: Blue / Cyan
- Identity: Precision, order, calculation
- Visual motif: Clean geometric shapes, crystalline edges

**Team Crimson (Team 1)**
- Color: Red / Orange
- Identity: Aggression, instinct, force
- Visual motif: Jagged edges, ember-like glow

In future seasons, these could evolve into named factions with distinct lore, but for now they serve as clear visual identifiers.

### 3.3 The Convergence Arena

The battlefield itself is alive. The hexagonal tiles pulse with faint energy. Obstacles are not merely rocks — they are **crystallized mana formations** that block passage and sight.

The Core is not a building. It is a **pulsing crystal heart** that anchors the faction's presence in the Convergence. When it shatters, the faction is expelled from reality.

The neutral guardians in the jungle are **ancient sentinels** left behind by the original architects of the Convergence. They do not take sides — they simply destroy anything that threatens their vigil.

---

## 4. Visual Identity

### 4.1 Art Direction

**Style:** Stylized 2D top-down with strong geometric clarity.

**Influences:**
- *Into the Breach* — clean grid readability
- *Hades* — bold character silhouettes and color
- *Slay the Spire* — UI clarity and card-like information design
- *Tron* — glowing edges and energy effects on dark backgrounds

**Key Principles:**
- Dark background (deep navy / charcoal) to make units and effects pop
- High contrast between interactive elements and background
- Hex grid lines are subtle but always visible
- Units have strong silhouettes readable at small zoom levels
- Team colors are the primary identifier (blue vs red)
- Abilities and effects use bright, saturated colors that contrast with the dark board

### 4.2 Color Palette

| Element | Color | Hex Code |
|---------|-------|----------|
| Background | Deep Space Navy | `#0a0a1a` |
| Hex Grid Lines | Muted Blue | `#0f3460` |
| Walkable Hex Fill | Dark Slate | `#16213e` |
| Obstacle / Vision Blocker | Charcoal Purple | `#2d2d44` |
| Team Azure | Bright Cyan | `#4fc3f7` |
| Team Crimson | Warm Red | `#ef5350` |
| Neutral / Objective | Gold Amber | `#ffc107` |
| HP Bar (healthy) | Green | `#4caf50` |
| HP Bar (low) | Orange | `#ff9800` |
| AP / Energy | Yellow | `#ffeb3b` |
| Fog of War | Black overlay | `#000000` at 70% alpha |
| Valid Move Target | Soft Green | `#4caf50` at 40% alpha |
| Attack Target | Red outline | `#ff5252` |
| Spell Target | Cyan outline | `#00bcd4` |

### 4.3 Unit Visual Language

| Unit Type | Shape | Size | Identifier |
|-----------|-------|------|------------|
| Hero | Circle with inner icon | Large | Team color ring + class symbol |
| Minion | Small triangle | Small | Team color fill |
| Tower | Square / Hexagon | Large | Team color + turret barrel |
| Spawner | Hexagon with pulse | Large | Team color + spawn animation |
| Core | Large crystal / diamond | Extra Large | Team color + glow effect |
| Neutral Guardian | Diamond | Medium | Gold / amber |
| Objective Vault | Hexagonal chest | Medium | Gold + lock icon |
| Obstacle (movement) | Dark filled hex | N/A | No outline |
| Obstacle (vision) | Dark filled hex + haze | N/A | Purple tint + fog wisps |

### 4.4 Visual Effects Style

- **Movement:** Smooth interpolation along hex path. Faint trail behind unit.
- **Attack:** Quick flash on target + damage number floating up.
- **Spell Cast:** Brief glow on caster → projectile/effect → impact.
- **Death:** Fade out + shrink. Small particle burst.
- **Level Up:** Golden ring pulse around hero.
- **Item Purchase:** Brief shimmer on hero.
- **Respawn:** Fade in from base crystal with upward particle effect.
- **Core Destruction:** Screen shake + shatter animation + victory banner.

---

## 5. Hero Roster

Each hero represents a distinct tactical role. They are not just stat sticks — they are characters with identity.

---

### 5.1 VANGUARD — The Unbreakable Wall

**Role:** Frontline / Tank / Area Control

**Visual Identity:**
- Large, broad-shouldered figure
- Heavy hexagonal shield on one arm
- Slow, deliberate movement animation
- Team-colored armor with reinforced plating

**Personality:**
- Stoic protector
- Speaks in short, decisive commands
- Believes the best offense is an impenetrable defense

**Ability: CLEAVE**
- Swings weapon in a wide arc around self
- Damages all adjacent enemies
- Visual: circular slash wave emanating from Vanguard

**Tactical Identity:**
Vanguard exists to **control space**. By positioning in the front line, Vanguard forces enemies to either go through him (and take Cleave damage) or go around (and lose tempo). Vanguard is the anchor of team fights.

**Stats (Base):**
```
HP: 140 | Damage: 18 | Range: 1 (melee)
AP: 3 | Initiative: 3 | Vision: 3
```

---

### 5.2 RANGER — The Watchful Eye

**Role:** Ranged Damage / Scout / Vision Control

**Visual Identity:**
- Lean, agile figure
- Longbow or energy crossbow
- Hood or visor that glows with team color
- Quick, precise movement animation

**Personality:**
- Quiet observer
- Speaks in observations, not opinions
- Sees the battlefield as a chess board

**Ability: BOLT**
- Fires a concentrated energy projectile at a single enemy
- Requires line of sight
- Visual: bright streak of light from Ranger to target

**Tactical Identity:**
Ranger is the team's **information advantage**. With the highest vision range, Ranger provides fog coverage and picks off isolated targets. Ranger punishes enemies who break line of sight discipline.

**Stats (Base):**
```
HP: 90 | Damage: 16 | Range: 2 (ranged)
AP: 3 | Initiative: 4 | Vision: 4
```

---

### 5.3 WARDEN — The Mending Light

**Role:** Support / Healer / Sustain

**Visual Identity:**
- Robed figure with glowing hands
- Soft aura of healing energy (team-colored)
- Gentle, floating movement animation
- Carries a staff or orb of light

**Personality:**
- Calm, reassuring presence
- Speaks in encouragement
- Believes every life saved is a victory

**Ability: MEND**
- Channels healing energy into an allied hero
- Restores HP
- Requires line of sight
- Visual: soft green/gold beam connecting Warden to target

**Tactical Identity:**
Warden is the team's **sustain engine**. Without Warden, every point of damage is permanent. With Warden, the team can endure prolonged fights and recover between engagements. Warden must be protected — losing Warden means losing the ability to recover.

**Stats (Base):**
```
HP: 100 | Damage: 12 | Range: 1 (melee)
AP: 3 | Initiative: 2 | Vision: 4
```

---

### 5.4 SNIPER — The Distant Thunder

**Role:** Long-Range Burst / Assassin / Pick

**Visual Identity:**
- Slim, focused figure
- Long rifle or extended energy weapon
- One eye covered by a targeting monocle
- Minimal movement animation (precise, economic)

**Personality:**
- Cold, calculating
- Speaks rarely, in single words
- Views each shot as a mathematical equation

**Ability: LONGSHOT**
- Fires a devastating long-range projectile
- Requires line of sight
- Cannot target adjacent enemies (minimum range 2)
- Visual: charging glow → high-speed tracer → impact explosion

**Tactical Identity:**
Sniper is the team's **executioner**. While Ranger provides consistent damage and vision, Sniper delivers devastating burst from safety. Sniper's minimum range means positioning is critical — get too close and you're vulnerable.

**Stats (Base):**
```
HP: 80 | Damage: 14 | Range: 3 (ranged)
AP: 3 | Initiative: 4 | Vision: 5
```

---

### 5.5 BERSERKER — The Unchained Fury

**Role:** Damage / Bruiser / Self-Buff

**Visual Identity:**
- Muscular, aggressive posture
- Dual weapons or massive fists
- Scars and battle damage visible
- Fast, jerky movement animation (eager to fight)

**Personality:**
- Wild, unrestrained
- Speaks in challenges and taunts
- Lives for the moment of impact

**Ability: FURY**
- Channels inner rage, boosting attack damage
- Self-buff for 2 rounds
- Visual: red/orange aura flares around Berserker, eyes glow

**Tactical Identity:**
Berserker is the team's **burst damage engine**. While other heroes rely on positioning and support, Berserker simply hits harder than everyone else. Fury creates windows of terrifying damage output, but Berserker must survive long enough to use them.

**Stats (Base):**
```
HP: 120 | Damage: 20 | Range: 1 (melee)
AP: 3 | Initiative: 3 | Vision: 3
```

---

## 6. Map & Environment Design

### 6.1 The Convergence Arena (Phase 7 Map)

The Phase 7 map is a **single-lane battlefield** with flanking routes and a central objective.

**Layout Philosophy:**
- Clear central lane for minion waves
- Side pockets for flanking and ambushes
- Vision blockers create tactical angles
- Base zones are safe but not impenetrable
- Objective sits at the contested center

**Visual Theme:**
- Dark stone floor with glowing hexagonal ley-lines
- Crystalline obstacles that refract light
- Energy barriers where vision blockers exist
- Core structures pulse with faction color
- Neutral jungle areas have organic, overgrown feel vs the structured lane

### 6.2 Key Locations

| Location | Visual Identity |
|----------|----------------|
| Team Base | Glowing crystal formations in team color, safe aura |
| Central Lane | Clean hex tiles, minion path markers |
| Side Pockets | Darker, shadowed hexes, vision blockers |
| Objective Vault | Golden hexagonal structure, ancient runes |
| Neutral Camps | Amber glow, guardian sentinel standing watch |

---

## 7. Audio Direction

### 7.1 Music

**Style:** Electronic-orchestral hybrid. Think "tactical tension meets epic fantasy."

**Influences:**
- *Darkest Dungeon* soundtrack — tension and dread
- *Tron: Legacy* soundtrack — electronic precision
- *Civilization VI* — strategic grandeur

**Tracks Needed (MVP):**
- Main Menu / Lobby — calm, anticipatory
- Hero Select — building energy
- Battle (early rounds) — measured, tactical
- Battle (late rounds) — urgent, intense
- Victory — triumphant, resolving
- Defeat — somber, reflective

### 7.2 Sound Effects

**Priority SFX:**
- Hex click / select
- Unit move (soft footstep or hover)
- Attack impact (melee vs ranged)
- Spell cast (unique per ability)
- Damage number popup
- Unit death
- Tower attack (heavy, mechanical)
- Core hit (crystalline crack)
- Core destruction (shattering explosion)
- Level up (ascending chime)
- Item purchase (coin + equip sound)
- Turn timer warning (last 5 seconds)
- Round resolution start (dramatic beat)

### 7.3 Audio Philosophy

- Sounds must communicate **game state** without visual attention
- A player listening with headphones should know when their hero takes damage, when a round resolves, and when the timer is low
- Avoid audio clutter — with 10+ units acting simultaneously, prioritize clarity over density

---

## 8. UI/UX Philosophy

### 8.1 Core UX Principle

> **"Information should be visible, not buried."**

Every piece of information the player needs to make a decision should be visible within **one click or one hover**. No nested menus during planning phase.

### 8.2 Screen Flow

```
Main Menu
  → Create Match / Join Match
    → Lobby (players, teams)
      → Hero Select (pick your champion)
        → Battle (planning → resolution loop)
          → Victory / Defeat Screen
            → Return to Lobby / Main Menu
```

### 8.3 Battle Screen Layout

```
┌─────────────────────────────────────────────────┐
│  [Round: 5]  [Timer: 0:23]  [Phase: Planning]  │
├─────────────────────────────────────────────────┤
│                                                 │
│                                                 │
│              HEX BATTLEFIELD                    │
│           (pan / zoom enabled)                  │
│                                                 │
│                                                 │
├─────────────────────────────────────────────────┤
│  [Team Roster]  │  [Selected Unit Panel]        │
│  Hero 1 ✓      │  HP: 85/100  AP: ●●●          │
│  Hero 2 ✓      │  Energy: ●●●○○  Level: 3      │
│  Hero 3 ☠ (2)  │  Gold: 145                    │
│  Hero 4 ✓      │  [Attack] [Spell] [Repair]    │
│  Hero 5 ✓      │  [Wait] [Shop]               │
└─────────────────────────────────────────────────┘
```

### 8.4 Information Hierarchy

**Always Visible:**
- Round number
- Turn timer
- Controlled hero HP / AP / Energy
- Team roster status (alive / dead / respawn timer)

**Visible on Select:**
- Full unit stats
- Ability cooldowns
- Item slots
- Movement range overlay

**Visible on Hover:**
- Enemy unit stats (if visible)
- Obstacle type
- Objective reward info
- Spell tooltip

---

## 9. Progression Fantasy

### 9.1 In-Match Progression

The player should feel their hero growing stronger through the match:

| Phase | Feeling |
|-------|---------|
| Rounds 1–3 | Fragile. Every hit matters. Positioning is life or death. |
| Rounds 4–6 | First item purchased. Slightly stronger. Starting to trade favorably. |
| Rounds 7–10 | Level 3+. Abilities come online. Team fights become decisive. |
| Rounds 11–15 | Multiple items. Level 4–5. Power spikes create win conditions. |
| Rounds 16+ | Full build. One mistake loses the game. Core is vulnerable. |

### 9.2 Out-of-Match Progression (Future)

Not in MVP, but the fantasy to build toward:
- Unlock new heroes
- Cosmetic skins for heroes
- Ranked ladder progression
- Season rewards
- Match history and replay viewing

---

## 10. Competitive Identity

### 10.1 What Makes Hexabellum Competitive?

Hexabellum is not about mechanical skill. It is about:

- **Prediction:** What will the enemy do this round?
- **Positioning:** Where should I be when the round resolves?
- **Resource Management:** Do I spend AP to move or save it for attack?
- **Team Coordination:** How do five players synchronize without real-time voice?
- **Adaptation:** The enemy bought armor. Do I switch to magic damage?

### 10.2 Skill Ceiling

The skill ceiling comes from:
- Reading enemy patterns over multiple rounds
- Predicting simultaneous actions
- Managing fog of war and vision control
- Timing ability usage around cooldowns
- Macro decisions (push lane vs contest objective vs defend base)

### 10.3 Spectator Potential

Because turns resolve simultaneously and dramatically, Hexabellum has natural spectator appeal:
- Viewers can see both teams' plans before resolution
- The "reveal" moment each round is inherently dramatic
- Casters can predict what will happen before it resolves

---

## 11. Naming Conventions

### 11.1 Game Title
**HEXABELLUM**
- "Hexa" = six (Greek)
- "Bellum" = war (Latin)
- Together: "Six-Sided War"

### 11.2 In-Game Terminology

| Term | Meaning |
|------|---------|
| Convergence | The hexagonal battlefield |
| Bellum Sextum | The ritual war (lore term) |
| Core | The faction's crystal heart (win condition) |
| Spawner | Construct generator |
| Guardian | Neutral camp sentinel |
| Vault | Central objective structure |
| Round | One full turn cycle (planning + resolution) |
| Initiative | Action order priority |
| AP (Action Points) | Resource for movement and actions |
| Energy | Resource for abilities |
| Fog of War | Hidden areas outside vision |

---

## 12. Tone & Voice

### 12.1 Game Voice

Hexabellum's tone is:
- **Confident** — the game knows what it is
- **Precise** — information is delivered clearly
- **Respectful** — never condescending to the player
- **Epic but grounded** — not overwrought, not silly

### 12.2 UI Copy Examples

| Context | Copy |
|---------|------|
| Turn start | "Round 7 — Plan your moves." |
| Timer warning | "5 seconds remaining." |
| Hero death | "Vanguard has fallen. Respawning in 3 rounds." |
| Level up | "Ranger reached Level 3." |
| Item purchased | "Longblade acquired. +6 Attack Damage." |
| Core destroyed | "The Crimson Core shatters. Victory is yours." |
| Defeat | "Your Core has been destroyed. Defeat." |
| Shop unavailable | "Return to base to access the shop." |
| No LOS | "No line of sight to target." |

---

## 13. Future Content Roadmap (Post-MVP)

### Season 1 Additions
- 5 new heroes (10 total)
- 3-lane map
- Recall / Town Portal ability
- Active items
- Ranked mode

### Season 2 Additions
- 5 more heroes (15 total)
- Jungle monster variety
- Objective respawn with escalating rewards
- Spectator mode
- Replay system

### Season 3 Additions
- Guild / Clan system
- Tournament mode
- Custom game creation
- Map editor (stretch goal)

---

## 14. Success Metrics

### MVP Success
- A full 5v5 match can be played from start to finish in a browser
- Average match duration: 15–25 minutes
- Players understand the turn loop within 2 rounds
- No player feels "out of the game" for more than 3 rounds (respawn timer)

### Engagement Targets
- 50% of players complete their first match
- 30% of players return for a second match
- Average session length: 20–30 minutes

### Competitive Targets
- Skill differentiation visible within 5 matches
- No single hero dominates pick rate (>40%)
- Win rate distribution: 45–55% for all heroes

---

## 15. Appendix: Quick Reference

### Hero Summary Table

| Hero | Role | HP | DMG | Range | Ability | Ability Type |
|------|------|----|-----|-------|---------|-------------|
| Vanguard | Tank | 140 | 18 | 1 | Cleave | AoE damage |
| Ranger | Scout/DPS | 90 | 16 | 2 | Bolt | Single target |
| Warden | Support | 100 | 12 | 1 | Mend | Heal ally |
| Sniper | Burst | 80 | 14 | 3 | Longshot | Long range |
| Berserker | Bruiser | 120 | 20 | 1 | Fury | Self buff |

### Economy Summary

| Source | Gold | XP |
|--------|------|-----|
| Passive (per round) | +6 | — |
| Minion kill | +10 | +10 |
| Hero kill | +30 | +30 |
| Neutral kill | +40 | +25 |
| Tower destroyed | +25/team | +20/team |
| Spawner destroyed | +30/team | +25/team |
| Vault destroyed | +50/team | +40/team |

### Item Summary

| Item | Cost | Effect |
|------|------|--------|
| Longblade | 100g | +6 Attack Damage |
| Plate Armor | 120g | +35 Max HP |
| Scout Lens | 80g | +1 Vision Range |
| Focus Charm | 100g | +1 Energy Regen |

---

*End of Game Design Document*

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | Current | Initial GDD aligned with Phase 7 technical spec |

---

*HEXABELLUM — Six sides of war. One victor.*