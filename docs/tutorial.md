# HEXABELLUM

## Tutorial Level & Onboarding Wizard — Planning Specification

---

## Document Version

\`Tutorial v1.0 — Planning Spec, Aligned with GDD v2.0, Architecture v2.0, and UI/UX v2.0 (FTUE §10)\`

---

## 1. Purpose & Document Scope

This document plans **a guided, in-game tutorial level** — an interactive "wizard" flow starring a lore wizard (the **Archmage of the Convergence**) — that teaches brand-new players the complete rules of Hexabellum by doing, not reading.

It defines:

1. The tutorial concept, learning objectives, and lesson breakdown (Sections 3–4).
2. The "wizard" presentation: step-by-step guided flow, dialogue, objectives, and input gating (Section 5).
3. The scenario scripting format used to author lessons data-driven (Section 6).
4. The technical architecture across \`hexabellum-core\`, \`hexabellum-wasm\`, \`hexabellum-protocol\`, and the web client (Section 7).
5. The delivery plan in milestones with acceptance criteria (Section 8).
6. UX rules, content/copy direction, metrics, testing, risks, and open questions (Sections 9–12).

Rules of record remain in [\`docs/gdd.md\`](gdd.md). This document makes **no rule changes** — it teaches existing rules.

### 1.1 Relationship to Existing Onboarding (UI/UX §10)

| Surface | Audience | What it does |
|---|---|---|
| **Tutorial Level (this doc)** | First-time players | A self-contained, guided, single-player level: 13 short beats, ~10–15 min, ending in a "graduation" PvAI match. Skippable at any time, replayable. |
| **Contextual Coaching Assistant (ui-ux.md §10)** | All players, in-match | Dismissible round-by-round tips during real matches (Round 1: move, Round 2: engage, Round 3: economy). |
| **Codex (\`[?]\` help button)** | All players | Full static reference: rules manual, hero sheets, item database. |

The tutorial level is the **first entry point** on the FTUE funnel: first launch → "Learn to Play" card → guided level → graduation match → contextual hints keep supporting real matches.

---

## 2. Design Principles

1. **Learn by doing.** Every lesson ends with the player *performing* the action being taught. No passive video walls of text; maximum 3 short dialogue lines per concept, always interruptible.
2. **Same engine, same rules.** The tutorial runs the real deterministic simulation from \`hexabellum-core\` — no "fake" tutorial rules. If a lesson teaches a rule, the engine enforces it identically to a real match.
3. **Zero pressure, zero ambiguity.** Relaxed timer (no 30s countdown), generous hint escalation (try → hint → auto-demo), input constrained to the current lesson's valid actions, and explicit success criteria shown at all times.
4. **Progressive disclosure.** 13 lessons, each teaching exactly one new rule or system. Earlier systems are never re-introduced; only *referenced*.
5. **Deterministic & offline.** Runs 100% client-side in WASM with a fixed scenario seed. No server, no account, no network — it must work from a single opened URL.
6. **Skip & replay.** Every lesson is skippable; the whole level is skippable; progress persists locally; a "Restart tutorial" option lives in settings so veterans can re-learn after patches.
7. **Lore-consistent voice.** The Archmage speaks in the GDD's existing terminology (Section 13.1 of gdd.md) — "Sovereign Core", "Bellum Sextum", "AP" — so tutorial vocabulary == in-game vocabulary.


---

## 3. The Concept: "The Archmage's Trial"

> *"Every general was once a stranger to the grid. Six sides of war. Let me show you how the first side is conquered."*

The tutorial is framed as **the Archmage of the Convergence** — a lore-native wizard character of the Convergence realm (gdd.md §3) — walking the player through a scaled-down training arena called **The Proving Grounds**. The Archmage:

- Appears as a sprite standing on a fixed tile, with a **dialogue portrait** (bottom-left DOM card, glassmorphic per the UI/UX component library).
- Narrates each rule *as the player discovers it* (moment-of-need teaching), not as a preamble.
- Reacts to failure: if the player makes an invalid order, the Archmage calmly explains *why* it failed (the engine's validation error is the single source of truth for the message).
- Never punishes: mistakes are free retries. There is no tutorial "game over" — only "try again" and "watch me show you".

### 3.1 Why a wizard character (and not a silent checklist)?

- **Narrative glue** across 13 otherwise disconnected rules. The Archmage is a named, quotable NPC consistent with the Convergence lore.
- **Perceived help, not UI clutter**: a character *asking* ("Where would you march?") reads as guidance, not as a menu.
- **Localization-ready**: one authored voice per lesson maps cleanly to text/speech assets later.
- **Brand asset**: the Archmage can recur in post-launch content (Season 1+ challenge scenarios, per gdd.md §12), making the tutorial an investment, not a cost.

### 3.2 Alternative considered: silent objective checklist only

Rejected as the primary mode — checklists teach *what* but not *why*, and the GDD's Pillar 1 (Tactical Clarity) demands players understand *why* outcomes occur. The Archmage's narration carries the "why" (e.g., "His Initiative is higher, so his attack resolved before his counter-strike could start"). A **quiet mode** (checklist only, no dialogue) ships as a settings toggle for veterans who want to re-run the drills (Section 9.5).

---

## 4. Learning Objectives & Lesson Breakdown

Each lesson teaches one rule or system. "Rules covered" links to the GDD section that is the source of truth.

| # | Lesson | Rules / Systems Covered (GDD ref.) | Player Must Do (success condition) |
|---|---|---|---|
| 0 | **The Convergence Calls** | Game identity, what a hero is, the hex grid, AP pips on the HUD (§1, §13.1) | Dismiss intro; select own hero; read the HUD tour (3 pinned callouts: AP pips, round counter, order lock button). |
| 1 | **The Three Steps** | Hex movement, 1 AP per hex, 3 AP budget, tile occupancy (§6.2) | Move 1 hex, then 2 hexes, then a 3-hex path; attempt to walk through an occupied hex and see it rejected. |
| 2 | **A Strike Is a Promise** | Basic attack, HP bars, deterministic combat preview (§7.3, §11.2) | Attack the training dummy at range 1; open the hover preview and confirm the exact damage number matches the result. |
| 3 | **The Round Resolves** | Planning → lock → resolution; initiative order; **death-before-acting** (§7.1, §7.2 stage 6, §7.3) | Lock orders with a dummy that dies from a higher-initiative ally's attack; observe the dummy's pending action cancel. Archmage narrates the initiative numbers. |
| 4 | **Spells of the Convergence** | Abilities: AP cost, energy cost, cooldown, targeting, LOS (§5.1–5.5, §6.3) | Cast Ranger's \`Bolt\` (single target, range 3); then attempt a second cast and observe the cooldown block; attempt a cast without enough energy and read the hint. |
| 5 | **Clockwork Legions** | Minion spawning, spawners, lane structure (§2, §8.1–8.2) | Watch a wave spawn from the spawner; last-hit a minion; damage the spawner to see waves stop. |
| 6 | **Towers Watch** | Tower auto-attack, target priority Minion → Hero, tower repair (§8.2) | Let a minion draw tower aggro; observe 25 DMG retaliation; repair the tower (+25 HP / 1 AP). |
| 7 | **The Shadow Knows** | Fog of war, 3 visibility states, LOS raycasting, vision radius (§6.3) | Scout behind a mana-crystal wall with the Ranger (vision 4); observe Lit / Explored-Memory / Unexplored states; fire \`Bolt\` at a target the fog just revealed and at an obscured one (blocked). |
| 8 | **Gold, Growth, Glory** | Gold economy (+6G passive, last-hit bounties), XP thresholds, levels 1–5 (§9.1, §9.2) | Earn gold via last-hits; watch the XP bar cross L2; observe the +12 HP / +3 DMG / +1 Energy bonus apply. |
| 9 | **The Sanctuary** | Base Zone: +15 HP regen, Base-Only Shop, 3 item slots, items (§8.3, §9.3) | Return to base zone, confirm the shop unlocks, buy \`Plate Armor\`, confirm +35 Max HP + instant heal on a damaged hero. |
| 10 | **Death Is Not the End** | Death, 3-round respawn countdown, progression preserved (§8.4) | Let the dummy kill the hero (scripted: Archmage orders a killing blow); watch the 3-round countdown; observe respawn at full HP with gold/XP/items intact. |
| 11 | **The Ancient Vault** | Neutral objectives, contested last-hit, team buff (+50G/+40XP, +5 AD 5 rounds) (§8.2) | Damage the Vault; observe the team-wide buff on last-hit. |
| 12 | **Shatter the Core** | Win condition, Core HP, non-repairable, match end screen (§8.2, §10.3) | Lead a small scripted push: the Archmage's team attacks the enemy Core; the player's job is to keep the push alive (heal/block one scripted counter-strike) until the Core shatters → graduation screen. |
| — | **Graduation** | — | Play a full standard 3v3 PvAI match with no hints; a "Trial Complete" card offers: "Enter the Bellum Sextum" (PvAI) / "Play online" (matchmaking). |

**Total:** 13 lessons (0–12) + graduation. Target completion time **10–15 minutes** for a new player; skippable to the graduation match in ~90 seconds.

> **Sequencing note:** Lessons 5, 6, 8, 9, 10, 11, 12 depend on systems delivered in later phases (minions/towers/fog — Phase 2; abilities — Phase 4; economy — Phase 6; Vault/Core win — Phase 7). The tutorial is delivered in **milestones aligned to those phases** (Section 8): each milestone ships the lessons whose systems exist, so a new player at launch always sees the complete 13-lesson flow.

---

## 5. The Wizard Flow: Presentation Specification

### 5.1 Flow Architecture

\`\`\`mermaid
stateDiagram-v2
    [*] --> IntroCard
    IntroCard --> Lesson0: "Begin the Trial"
    state LessonFlow {
        Lesson0 --> Briefing: Archmage dialogue (3 lines max)
        Briefing --> Objective: Objective pinned to HUD checklist
        Objective --> PlayerAct: Input gated to lesson
        PlayerAct --> Valid: Engine accepts order
        PlayerAct --> Invalid: Engine rejects order
        Invalid --> ArchmageExplain: "Here is why..." (validation msg)
        ArchmageExplain --> PlayerAct
        Valid --> Confirmed: Player locks round / action completes
        Confirmed --> ResolveWatch: Resolution playback (can be skimmed)
        ResolveWatch --> Debrief: Archmage narration of *why*
        Debrief --> NextLesson: "Continue"
    }
    Lesson12 --> Graduation: Core shatters
    Graduation --> [*]: "Trial Complete" card
\`\`\`

### 5.2 The Step Anatomy

Every moment of the tutorial is a **step** (Section 6 schema). Steps come in five types:

| Type | Behavior |
|---|---|
| \`dialogue\` | Archmage line (portrait + text box); timer paused; "Continue" button or click-to-advance. |
| \`callout\` | Pinned highlight on an existing HUD/board element (AP pips, timer, round counter) with a short caption. |
| \`objective\` | Active checklist item + spotlight/ring on the relevant board region. |
| \`input-gate\` | Input restricted: only the actions the lesson requires are enabled (e.g., Lesson 1: only own-hero movement; enemy and ability buttons dimmed). Invalid actions still fire and are explained by the Archmage instead of a hard error buzz (soft-fail, Section 9). |
| \`milestone\` | Non-interactive beat: scripted AI actions resolve (e.g., a minion wave spawns); Archmage narrates. |

### 5.3 Hint Escalation (Anti-Stuck Ladder)

Per active objective, three escalating stages on a local timer:

1. **Stage 1 (0–45s):** silent spotlight on the target region; objective text visible.
2. **Stage 2 (45s+):** Archmage line with a concrete instruction ("Select the glowing hex **north-east** of your hero").
3. **Stage 3 (90s+):** **"Watch me show you"** button appears; accepting it plays a 5s auto-demo (a drag animation over the board executing the step), then the objective re-opens for the player.

No hint ever silently auto-completes the objective — the Stage 3 auto-demo is the *deliberate* skip path and the player chose it.

### 5.4 Board & HUD Modifications During Tutorial

- **Timer:** no countdown. The round "locks" only when the active objective completes (or when the player presses the order-lock button, which is itself the objective in most lessons).
- **Objective checklist (top-left card):** current lesson name + 1–3 checkbox items; completed lessons collapse into a "Trial progress 7/13" bar.
- **Spotlight:** a radial dim overlay (reuses the fog-of-war rendering pass — one extra uniform) with a cut-out ring around the relevant hexes/entity; \`#00e676\` emerald ring for movement targets, \`#ff1744\` for attack targets (existing design tokens).
- **Archmage portrait:** bottom-left glassmorphic card, 96px portrait, text box, \`[Skip]\` and \`[Auto-demo]\` buttons; collapses to a 48px badge when inactive.
- **Fog:** fully active — it is itself a lesson, and information asymmetry is core to the game identity.
- **Scripted units:** "Training Dummies" and the Archmage's "Acolyte" squad carry a **sand-colored team ring + bell glyph**, never confused with Azure/Crimson.


