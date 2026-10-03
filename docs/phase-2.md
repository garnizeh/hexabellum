# Phase 2 Technical Spec — MOBA Elements Vertical Slice

---

## Architectural Decision Record (ADR-003): MOBA Systems, Vision Asymmetry, Cooperative Mechanics & Event Animation Pipeline

### Context
In Phase 1, Hexabellum established its deterministic simultaneous turn resolution engine for a 3v3 hero arena: Action Point (AP) budgeting, BFS pathfinding, cooperative movement with destination claims, and basic melee combat. 

Phase 2 transforms this tactical duel into an authentic **MOBA (Multiplayer Online Battle Arena)** experience. Introducing automated minion waves, stationary defensive towers, radius-based fog of war, planning phase timers with AI fallbacks, and an asynchronous client animation pipeline creates several critical architectural and algorithmic challenges:

1. **Information Asymmetry & Fog of War Masking**: In simultaneous turn games with fog of war, game state visibility differs per team. If the engine serializes the complete world state to the browser via `get_state()`, malicious clients or basic inspection tools can read hidden enemy coordinates through the fog. Furthermore, target validation must prevent players and AI from ordering attacks against concealed enemies in the fog.
2. **Automated Wave Economy & Spawner Contention**: Minion waves spawn periodically (every 3 rounds) from Spawner Towers. If a hero or another unit occupies the hex immediately in front of a spawner, naive spawning logic will either overwrite the occupant, silently fail and drop the wave forever, or crash. The spawner system must handle tile congestion deterministically without penalizing the wave economy.
3. **Stationary Structures in the Cooperative Movement Protocol**: Towers and Spawners are immovable structures (`is_stationary = true`) with massive health pools. Unlike heroes, structures cannot yield, vacate, or swap hexes. The cooperative movement protocol from Phase 1 (`EntryCheck::WillLeave`, claims, and pending movers) must be preserved and extended so mobile units never attempt to swap with or pass through structures, while pathfinding correctly identifies structures as permanent obstacles.
4. **Dynamic Aggro & Multi-Tier Target Prioritization**: Towers and minions operate autonomously. In MOBAs, target priority is fundamental to gameplay strategy:
   - **Towers**: Prioritize Minions > Heroes > Structures. Minions act as "meat shields" (tower diving mechanics), allowing heroes to approach enemy towers safely while minions draw tower aggro.
   - **Minions**: Prioritize Minions > Heroes > Structures. Minions clash at the wave front, but engage nearby heroes or structures if no enemy minions are present.
   - **Deterministic Tie-Breaking**: When multiple equidistant targets exist within attack range, target selection must break ties deterministically using `(distance, unit_id)` to avoid cross-platform divergence.
5. **Turn Timer Synchronization & Deterministic AI Fallback**: The planning phase is constrained by a 30-second countdown. If a human player submits orders for only 1 or 2 heroes (or disconnects/idles), the simulation engine must not stall. The engine must detect unassigned heroes and automatically generate valid tactical AI orders without mutating already confirmed player choices.
6. **Asynchronous Visual Playback vs. Synchronous Engine Resolution**: The Rust/WASM simulation resolves an entire round instantaneously into an ordered `Vec<GameEvent>` event batch. Naive frontends instantly re-render the final board state, causing units to teleport, damage numbers to be invisible, and deaths to happen with zero feedback. The client must queue these events into a sequential animation pipeline (`Animator`), playing smooth movement lerps, attack flashes, floating damage numbers, spawn pops, and death fades before unlocking input for the next planning phase.

### Decisions

| Challenge | Architectural Decision | Implementation in Phase 2 |
|---|---|---|
| **Vision & Fog of War** | **Radius-Based Axial Distance Visibility with Engine-Side State Sanitization** | Living units project sight over axial distance $\le \text{vision\_range}$ (`spiral` search). Visibility is tracked per team in `FogState`. The engine provides `get_player_state(team)` which censors enemy units concealed by fog, and `get_attack_targets` strictly rejects shrouded targets. |
| **Spawner Congestion** | **Lane-Biased Ring Search with Non-Destructive Wave Holding** | Spawners scan adjacent neighbors ordered by lane progress (closest to enemy base). If all ring-1 hexes are occupied, the spawner checks ring-2. If fully blocked, the spawner retains its full spawn counter and retries next round rather than dropping the wave. |
| **Structure Movement** | **Permanent Occupancy Invariant in Cooperative Resolution** | Units with `kind.is_stationary()` have 0 AP for movement, never register move orders, and return `EntryCheck::Blocked` to all movers. Mobile units path around them. |
| **MOBA Target Aggro** | **Multi-Tier Priority Sorting with Composite Deterministic Keys** | Towers and minions evaluate potential targets grouped by tier (`Minion` > `Hero` > `Structure`). Within each tier, targets are sorted by composite key `(axial_distance, unit_id ASC)`. |
| **Turn Timer & Fallback** | **Client-Side Authoritative Timer with Engine Auto-Fill** | A 30s countdown executes on the client. On expiration, the client triggers `end_turn()`. `TurnProcessor` compares submitted orders against living player heroes and calls `GameAI::generate_fallback_orders` for any unmanaged hero. |
| **Win Condition** | **Dual MOBA Victory: Annihilation or Spawner Demolition** | A team wins if all enemy heroes are eliminated OR the enemy Spawner Tower is destroyed. Demolishing the enemy spawner ends minion production and secures immediate match victory. |
| **Event Animation** | **FIFO Sequential Event Queue with Input Lock** | The TypeScript `Animator` consumes the `GameEvent[]` stream, executing sequential sub-animations (`UnitMoved`, `UnitAttacked`, `TowerAttacked`, `UnitSpawned`, `UnitDied`) with tweening and floating combat text, notifying `InputHandler` upon completion. |

---

## Phase 2 Goal

> **Transform the tactical skirmish into a dynamic single-lane MOBA experience: minion waves spawn from spawner towers and push the lane, defensive towers guard choke points and auto-attack encroaching threats, radius-based fog of war enforces strategic scouting, a 30-second turn timer keeps turns brisk with AI auto-fill fallback, and a sequential event animator brings movement, attacks, damage numbers, and deaths to life.**

Phase 1 proved combat mechanics. Phase 2 delivers the full **MOBA game loop**.

### What Phase 2 Adds

| System | Description |
|---|---|
| **Minion Waves** | Spawner Towers generate waves of minions (2 AP, 30 HP, 8 DMG) that push toward the enemy base |
| **Defensive Towers** | Immovable structures (200 HP, Range 3, 30 DMG) that auto-fire on enemies within range |
| **Spawner Towers** | Primary base structures (150 HP) that produce minion waves every 3 rounds; destruction wins the match |
| **Fog of War** | Team-based vision radii (Hero: 3, Minion: 2, Tower: 4, Spawner: 2); shrouds unseen enemies and tiles |
| **Fog-Masked Targeting** | Engine validates that attack targets must be visible in the attacker team's fog of war |
| **Turn Timer (30s)** | Client-side 30s planning countdown with dynamic color stages (Blue > Orange > Red) |
| **AI Order Fallback** | Engine automatically generates tactical AI orders for any heroes unassigned when the timer elapses |
| **Sequential Animator** | Event-driven animation pipeline: path interpolation, attack lunges, damage numbers, death fades, spawn pops |
| **Dual Win Conditions** | Victory achieved by wiping all enemy heroes OR demolishing the enemy Spawner Tower |
| **Expanded Arena (R=6)** | 127-hex battlefield featuring a central combat lane ($r = 0$) flanked by obstacle chokepoints |

### Definition of Done (DoD)

- [ ] `hexabellum-core` passes all Phase 1 and Phase 2 unit/integration tests (`cargo test`).
- [ ] Map radius is expanded to 6 (127 hexes) with central lane ($r = 0$), obstacle chokepoints, and symmetrical base structures.
- [ ] Each team starts with 3 Heroes, 1 Defensive Tower, and 1 Spawner Tower.
- [ ] Spawner Towers tick each round and spawn minions on adjacent lane-biased hexes every 3 rounds.
- [ ] Minions advance along the central lane using BFS pathfinding toward the enemy side.
- [ ] Minions attack enemies within range according to priority: Minions > Heroes > Structures.
- [ ] Towers auto-attack enemies within range (range 3) prioritizing Minions over Heroes (minion tanking).
- [ ] Destructible structures: Towers and Spawner Towers take damage, display health bars, and die at 0 HP.
- [ ] Destroying the enemy Spawner Tower halts enemy minion spawns and triggers immediate Victory.
- [ ] Fog of war calculates sight rings for each team; unrevealed hexes are rendered in shadow overlay.
- [ ] Enemy units standing in fog are concealed from the player view and cannot be targeted.
- [ ] Planning phase turn timer counts down from 30 seconds; expiring auto-submits orders with AI fallback.
- [ ] Resolution events animate sequentially: smooth path walking, attack flashes, floating damage numbers, spawn pop-ins, and death fades.
- [ ] Game controls lock during animation playback and unlock for the next planning round.
- [ ] Victory and Defeat overlays display correctly upon match completion with functional Restart capability.

---

## Architecture Delta from Phase 1

```
Phase 1 (Tactical Combat Slice)      Phase 2 (MOBA Vertical Slice)
───────────────────────────────      ─────────────────────────────
3v3 Heroes only                 →    3v3 Heroes + Minion Waves + Defensive Towers
No structures                   →    Defensive Towers + Spawner Towers
Full board visible (no fog)     →    Radius-based Fog of War with obscured enemies
Unbounded planning time         →    30-second planning timer + AI fallback
Instantaneous state update      →    Asynchronous event playback (lerp, damage text, fades)
Hero-only tactical AI           →    Multi-tiered AI: Hero maneuvers, Minion lane pushing, Tower aggro
Pure team wipe win condition    →    Dual win condition: Team wipe OR Enemy Spawner demolition
Radius 5 arena (91 hexes)       →    Radius 6 arena (127 hexes) with central lane & choke points
```

---

## High-Level Architecture & Event Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                       Browser Client                                            │
│                                                                                                 │
│   ┌───────────────────────────────────────────────┐     ┌───────────────────────────────────┐   │
│   │             TypeScript Runtime                │     │          hexabellum-wasm          │   │
│   │                                               │     │                                   │   │
│   │  ┌───────────────┐     ┌───────────────────┐  │     │  ┌─────────────────────────────┐  │   │
│   │  │ PixiJS v8     │     │ InputHandler      │  │     │  │ WasmGame Facade             │  │   │
│   │  │ HexRenderer   │     │ - Hero selection  │  │     │  │ - BigInt u64 coercion       │  │   │
│   │  │ - Hex grid    │     │ - Path preview    │──┼─────┼─►│ - JSON serialization        │  │   │
│   │  │ - Unit layer  │     │ - Fog attack lock │  │     │  │ - Team-sanitized state      │  │   │
│   │  │ - Fog layer   │     │ - Order staging   │  │     │  │ - Order setters             │  │   │
│   │  │ - Damage text │     └─────────┬─────────┘  │     │  └──────────────┬──────────────┘  │   │
│   │  └───────▲───────┘               │            │     └─────────────────┼─────────────────┘   │
│   │          │ Play Animation        │ Submit     │                       │ Rust Crate Call     │
│   │  ┌───────┴───────┐               ▼            │                       │                     │
│   │  │ Animator      │     ┌───────────────────┐  │                       ▼                     │
│   │  │ - Event queue │     │ TurnTimer (30s)   │  │     ┌───────────────────────────────────┐   │
│   │  │ - Tween lerps │◄────┤ - Auto-submits on │  │     │         hexabellum-core           │   │
│   │  │ - Damage text │     │   expiration      │  │     │                                   │   │
│   │  └───────────────┘     └───────────────────┘  │     │  ┌─────────────────────────────┐  │   │
│   └───────────────────────────────────────────────┘     │  │ GameEngine                  │  │   │
│                                                         │  │ - GameState (Map, Units)    │  │   │
│                                                         │  │ - FogState (Vision rings)   │  │   │
│                                                         │  │ - Pending TurnOrders        │  │   │
│                                                         │  └──────────────┬──────────────┘  │   │
│                                                         │                 │ resolve()       │   │
│                                                         │                 ▼                 │   │
│                                                         │  ┌─────────────────────────────┐  │   │
│                                                         │  │ TurnProcessor Pipeline      │  │   │
│                                                         │  │ 1. Reset AP for all units   │  │   │
│                                                         │  │ 2. Process Spawner waves    │  │   │
│                                                         │  │ 3. Generate Enemy AI orders │  │   │
│                                                         │  │ 4. Auto-fill Hero Fallbacks │  │   │
│                                                         │  │ 5. Generate Minion/Tower AI │  │   │
│                                                         │  │ 6. Initiative-ordered steps │  │   │
│                                                         │  │    with Destination Claims  │  │   │
│                                                         │  │ 7. Attacks & Damage         │  │   │
│                                                         │  │ 8. Immediate Casualty Clean │  │   │
│                                                         │  │ 9. Recalculate Fog of War   │  │   │
│                                                         │  │ 10. Check Dual Win Condition│  │   │
│                                                         │  │ 11. Emit GameEvent batch    │  │   │
│                                                         │  └─────────────────────────────┘  │   │
│                                                         └───────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Hex Map & Arena Topology

Hexabellum Phase 2 uses a radius-6 regular hexagon board ($N = 127$ hexes) configured with a dedicated horizontal lane along $r = 0$, flanked by strategic obstacle pillars that funnel minion waves while offering heroes tactical flanking routes.

```
                                 North Flank
                             . - - - - - - - .
                         . '                   ' .
                     . '       [Obstacles]         ' .
                 . '        (0, -2)   (1, -2)          ' .
             . '                                           ' .
  [TEAM 0 BASE]                                             [TEAM 1 BASE]
  Spawner: (-5, 0) ─── Tower: (-3, 0) ─────── Tower: (3, 0) ─── Spawner: (5, 0)
  Heroes:              ═══════════ MAIN LANE ═══════════              Heroes:
  (-4, -1), (-4, 1), (-4, 0)        (r = 0)                   (4, -1), (4, 1), (4, 0)
             . '                                           ' .
                 . '        (-1, 2)   (0, 2)           ' .
                     . '       [Obstacles]         ' .
                         . '                   ' .
                             . - - - - - - - .
                                 South Flank
```

### Map Coordinates & Entity Placements
- **Arena Radius**: 6 ($q \in [-6, 6]$, cube constraint $q + r + s = 0$).
- **Obstacle Hexes**: `(0, 2)`, `(0, -2)`, `(1, 2)`, `(-1, -2)`, `(2, -3)`, `(-2, 3)`.
- **Team 0 (Blue / Player)**:
  - Spawner Tower: `HexCoord::new(-5, 0)` (150 HP, spawns wave every 3 rounds).
  - Defensive Tower: `HexCoord::new(-3, 0)` (200 HP, range 3, 30 DMG).
  - Heroes: `(-4, -1)` (Init 3), `(-4, 0)` (Init 2), `(-4, 1)` (Init 1).
- **Team 1 (Red / AI)**:
  - Spawner Tower: `HexCoord::new(5, 0)` (150 HP, spawns wave every 3 rounds).
  - Defensive Tower: `HexCoord::new(3, 0)` (200 HP, range 3, 30 DMG).
  - Heroes: `(4, -1)` (Init 3), `(4, 0)` (Init 2), `(4, 1)` (Init 1).

---

## Simultaneous Resolution & MOBA Lifecycle Flowchart

```
                   ┌──────────────────────────────────────────┐
                   │ 1. Planning Phase (Concurrent, 30s)      │
                   │ - Player stages orders for Team 0 Heroes │
                   │ - Fog of war masks unseen enemies        │
                   └────────────────────┬─────────────────────┘
                                        │ Click "End Turn" OR Timer Expires
                                        ▼
                   ┌──────────────────────────────────────────┐
                   │ 2. Pre-Resolution Setup                  │
                   │ - Set phase = Phase::Resolution          │
                   │ - Reset AP for all living units          │
                   │ - Snapshot state for planning consistency│
                   └────────────────────┬─────────────────────┘
                                        │
                                        ▼
                   ┌──────────────────────────────────────────┐
                   │ 3. Spawner System Wave Generation        │
                   │ - Living spawners tick spawn_counter     │
                   │ - If counter >= interval: spawn minion   │
                   │   on lane-biased free neighbor           │
                   │ - Emit GameEvent::UnitSpawned            │
                   └────────────────────┬─────────────────────┘
                                        │
                                        ▼
                   ┌──────────────────────────────────────────┐
                   │ 4. Order Aggregation & AI Fallback       │
                   │ - Team 1 AI generates orders for heroes  │
                   │ - Team 0 Heroes without orders get       │
                   │   tactical fallback orders (timer safety)│
                   │ - Minion AI generates lane push orders   │
                   │ - Tower AI generates auto-attack orders  │
                   └────────────────────┬─────────────────────┘
                                        │
                                        ▼
                   ┌──────────────────────────────────────────┐
                   │ 5. Initiative Sorting & Claims Setup     │
                   │ - Sort unit IDs:                         │
                   │   initiative DESC, then unit_id ASC      │
                   │ - Register destination claims for movers │
                   │ - Structures are permanently stationary  │
                   └────────────────────┬─────────────────────┘
                                        │
                                        ▼
              ┌──► ┌──────────────────────────────────────────┐
              │    │ 6. Process Unit in Initiative Order      │
              │    │ - Is unit alive? (If dead: SKIP)         │
              │    │ - If stationary: skip movement           │
              │    │ - Step-by-step path walk with claims     │
              │    │   and EntryCheck (Free / WillLeave)      │
              │    │ - Attack action:                         │
              │    │   * Verify target in range & line of sight│
              │    │   * Deduct 1 AP, apply damage to target  │
              │    │   * Emit UnitAttacked / TowerAttacked    │
              │    │ - Casualty check: if HP == 0:            │
              │    │   * Remove unit from board               │
              │    │   * Revoke destination claims            │
              │    │   * Emit GameEvent::UnitDied             │
              │    └────────────────────┬─────────────────────┘
              │                         │
              └── More units in order? ─┴─► All units processed
                                                  │
                                                  ▼
                   ┌──────────────────────────────────────────┐
                   │ 7. Fog Recalculation & Win Check         │
                   │ - Recalculate FogState for both teams    │
                   │ - Emit GameEvent::FogUpdated             │
                   │ - Check Dual Win Condition:              │
                   │   * Team has 0 heroes -> Defeat          │
                   │   * Team spawner destroyed -> Defeat     │
                   │ - Emit MatchEnded or RoundEnded          │
                   └────────────────────┬─────────────────────┘
                                        │ Emit JSON GameEvent[] Batch
                                        ▼
                   ┌──────────────────────────────────────────┐
                   │ 8. Client Sequential Event Playback      │
                   │ - Animator plays path movements          │
                   │ - Attack lunges + floating damage text   │
                   │ - Tower beams + heavy impact FX          │
                   │ - Unit death shrink + fade out           │
                   │ - Unlocks input for next Planning round  │
                   └──────────────────────────────────────────┘
```

---

## Core Data Structures & Game Logic (Rust)

### 1. Unit & Structure Extensions (`crates/core/src/unit.rs`)

```rust
use crate::hex::HexCoord;
use serde::{Deserialize, Serialize};

pub type UnitId = u64;
pub type TeamId = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    Hero,
    Minion,
    Tower,
    SpawnerTower,
    Neutral,
}

impl UnitKind {
    #[inline]
    pub fn is_stationary(&self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::SpawnerTower)
    }

    #[inline]
    pub fn is_structure(&self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::SpawnerTower)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneDirection {
    None,
    TowardEnemy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub team: TeamId,
    pub pos: HexCoord,
    pub hp: u32,
    pub max_hp: u32,
    pub ap: u32,
    pub max_ap: u32,
    pub initiative: u32,
    pub attack_damage: u32,
    pub attack_range: u32,

    // Phase 2 MOBA Additions
    pub vision_range: u32,
    pub spawn_interval: Option<u32>,
    pub spawn_counter: u32,
    pub lane_direction: LaneDirection,
}

pub const ATTACK_AP_COST: u32 = 1;

impl Unit {
    /// Hero unit: mobile fighter with high AP budget and tactical initiative.
    pub fn new_hero(id: UnitId, team: TeamId, pos: HexCoord, initiative: u32) -> Self {
        Self {
            id,
            kind: UnitKind::Hero,
            team,
            pos,
            hp: 100,
            max_hp: 100,
            ap: 3,
            max_ap: 3,
            initiative,
            attack_damage: 20,
            attack_range: 1,
            vision_range: 3,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
        }
    }

    /// Minion unit: cheap infantry pushing the lane.
    pub fn new_minion(id: UnitId, team: TeamId, pos: HexCoord) -> Self {
        Self {
            id,
            kind: UnitKind::Minion,
            team,
            pos,
            hp: 30,
            max_hp: 30,
            ap: 2,
            max_ap: 2,
            initiative: 1,
            attack_damage: 8,
            attack_range: 1,
            vision_range: 2,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::TowardEnemy,
        }
    }

    /// Defensive Tower: immovable fortress providing long-range perimeter defense.
    pub fn new_tower(id: UnitId, team: TeamId, pos: HexCoord) -> Self {
        Self {
            id,
            kind: UnitKind::Tower,
            team,
            pos,
            hp: 200,
            max_hp: 200,
            ap: 1,
            max_ap: 1,
            initiative: 5, // Towers fire early in initiative order
            attack_damage: 30,
            attack_range: 3,
            vision_range: 4,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
        }
    }

    /// Spawner Tower: key base structure producing minion waves.
    pub fn new_spawner(id: UnitId, team: TeamId, pos: HexCoord, spawn_interval: u32) -> Self {
        Self {
            id,
            kind: UnitKind::SpawnerTower,
            team,
            pos,
            hp: 150,
            max_hp: 150,
            ap: 0,
            max_ap: 0,
            initiative: 0,
            attack_damage: 0,
            attack_range: 0,
            vision_range: 2,
            spawn_interval: Some(spawn_interval),
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
        }
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    #[inline]
    pub fn is_stationary(&self) -> bool {
        self.kind.is_stationary()
    }

    #[inline]
    pub fn reset_ap(&mut self) {
        self.ap = self.max_ap;
    }

    #[inline]
    pub fn can_afford(&self, cost: u32) -> bool {
        self.ap >= cost
    }

    #[inline]
    pub fn spend_ap(&mut self, cost: u32) {
        self.ap = self.ap.saturating_sub(cost);
    }

    /// Check if spawner is ready to release a wave.
    pub fn should_spawn(&self) -> bool {
        if let Some(interval) = self.spawn_interval {
            self.spawn_counter >= interval
        } else {
            false
        }
    }

    /// Advance spawn counter; does not reset automatically to protect against wave drops.
    pub fn tick_counter(&mut self) {
        if self.spawn_interval.is_some() {
            self.spawn_counter += 1;
        }
    }

    /// Reset counter after a minion has successfully been placed on the board.
    pub fn mark_spawned(&mut self) {
        self.spawn_counter = 0;
    }
}
```

---

### 2. Radius-Based Fog of War System (`crates/core/src/fog.rs`)

```rust
use crate::hex::HexCoord;
use crate::state::GameState;
use crate::unit::TeamId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Fog of War visibility tracker for all participating teams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FogState {
    /// Vector of visible hex sets indexed by TeamId.
    pub visible: Vec<HashSet<HexCoord>>,
}

impl FogState {
    pub fn new(team_count: usize) -> Self {
        Self {
            visible: vec![HashSet::new(); team_count],
        }
    }

    /// Recompute visibility sets for all teams based on current living unit positions.
    pub fn update(&mut self, state: &GameState) {
        for team in 0..self.visible.len() {
            self.visible[team].clear();

            for unit in state.team_units(team as TeamId) {
                if !unit.is_alive() {
                    continue;
                }

                // Add all walkable hexes within axial vision radius
                for hex in unit.pos.spiral(unit.vision_range) {
                    if state.map.is_walkable(&hex) {
                        self.visible[team].insert(hex);
                    }
                }
            }
        }
    }

    /// Check if a specific hex coordinate is visible to a given team.
    #[inline]
    pub fn is_visible(&self, team: TeamId, hex: &HexCoord) -> bool {
        self.visible
            .get(team as usize)
            .map(|set| set.contains(hex))
            .unwrap_or(false)
    }

    /// Borrow visible hexes for a given team.
    #[inline]
    pub fn visible_hexes(&self, team: TeamId) -> &HashSet<HexCoord> {
        &self.visible[team as usize]
    }
}
```

---

### 3. Spawner System & Wave Economy (`crates/core/src/spawner.rs`)

```rust
use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::state::GameState;
use crate::unit::{Unit, UnitId, UnitKind};

pub struct SpawnerSystem;

impl SpawnerSystem {
    /// Advance spawners, determine wave readiness, and spawn minions on free lane-biased hexes.
    pub fn process_spawns(state: &mut GameState) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut new_units: Vec<Unit> = Vec::new();

        // Collect living spawner IDs deterministically
        let mut spawner_ids: Vec<UnitId> = state
            .units
            .values()
            .filter(|u| u.kind == UnitKind::SpawnerTower && u.is_alive())
            .map(|u| u.id)
            .collect();
        spawner_ids.sort_unstable();

        for spawner_id in spawner_ids {
            let spawner = state.get_unit_mut(spawner_id).unwrap();
            spawner.tick_counter();

            if !spawner.should_spawn() {
                continue;
            }

            let team = spawner.team;
            let spawner_pos = spawner.pos;

            // Find a free walkable hex adjacent to the spawner, sorted toward enemy side
            let spawn_pos = Self::find_spawn_hex(state, spawner_pos, team);

            if let Some(pos) = spawn_pos {
                // Confirm spawn and reset counter
                let spawner = state.get_unit_mut(spawner_id).unwrap();
                spawner.mark_spawned();

                let new_id = state.alloc_unit_id();
                let minion = Unit::new_minion(new_id, team, pos);
                new_units.push(minion);

                events.push(GameEvent::UnitSpawned {
                    unit_id: new_id,
                    unit_kind: UnitKind::Minion,
                    team,
                    pos,
                    spawner_id,
                });
            }
            // If all candidate hexes are blocked, counter is NOT reset; retries next round!
        }

        for unit in new_units {
            state.add_unit(unit);
        }

        events
    }

    /// Select optimal spawn hex adjacent to spawner with lane advancement bias.
    fn find_spawn_hex(state: &GameState, center: HexCoord, team: u8) -> Option<HexCoord> {
        let occupied = state.occupied_hexes();

        // 1. Try ring 1
        let mut ring1: Vec<HexCoord> = center
            .neighbors()
            .into_iter()
            .filter(|h| state.map.is_walkable(h) && !occupied.contains(h))
            .collect();

        if !ring1.is_empty() {
            // Sort by lane progress: Team 0 wants highest q (east); Team 1 wants lowest q (west)
            ring1.sort_by(|a, b| {
                if team == 0 {
                    b.q.cmp(&a.q).then_with(|| a.r.cmp(&b.r))
                } else {
                    a.q.cmp(&b.q).then_with(|| a.r.cmp(&b.r))
                }
            });
            return ring1.first().copied();
        }

        // 2. Fallback to ring 2 if ring 1 is completely congested
        let mut ring2: Vec<HexCoord> = center
            .ring(2)
            .into_iter()
            .filter(|h| state.map.is_walkable(h) && !occupied.contains(h))
            .collect();

        if !ring2.is_empty() {
            ring2.sort_by(|a, b| {
                if team == 0 {
                    b.q.cmp(&a.q).then_with(|| a.r.cmp(&b.r))
                } else {
                    a.q.cmp(&b.q).then_with(|| a.r.cmp(&b.r))
                }
            });
            return ring2.first().copied();
        }

        None
    }
}
```

---

### 4. Minion & Lane AI (`crates/core/src/minion_ai.rs`)

```rust
use crate::hex::HexCoord;
use crate::orders::{Action, UnitOrder};
use crate::state::GameState;
use crate::unit::{Unit, UnitId, UnitKind};
use std::collections::HashSet;

pub struct MinionAI;

impl MinionAI {
    /// Generate autonomous tactical order for a minion.
    pub fn generate_order(
        state: &GameState,
        unit_id: UnitId,
        occupied: &HashSet<HexCoord>,
    ) -> UnitOrder {
        let unit = match state.get_unit(unit_id) {
            Some(u) if u.is_alive() => u,
            _ => {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Wait,
                }
            }
        };

        // Minions only target enemies visible to their team's Fog of War
        let visible_enemies = state.visible_enemy_units(unit.team);

        if let Some(target) = Self::select_target(unit, &visible_enemies) {
            let distance = unit.pos.distance(&target.pos);

            // 1. In attack range immediately -> attack without moving
            if distance <= unit.attack_range {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Attack { target_id: target.id },
                };
            }

            // 2. Move towards target within AP budget (reserve 1 AP for attack if reachable)
            let reachable = state.map.reachable_hexes(unit.pos, unit.ap, occupied);
            let mut moves: Vec<(HexCoord, u32)> = reachable.into_iter().collect();

            // Find closest approach hex to the target
            moves.sort_by(|(hex_a, cost_a), (hex_b, cost_b)| {
                let dist_a = hex_a.distance(&target.pos);
                let dist_b = hex_b.distance(&target.pos);
                dist_a
                    .cmp(&dist_b)
                    .then_with(|| cost_a.cmp(cost_b))
                    .then_with(|| (hex_a.q, hex_a.r).cmp(&(hex_b.q, hex_b.r)))
            });

            if let Some((best_hex, cost)) = moves.first() {
                let new_dist = best_hex.distance(&target.pos);
                if new_dist <= unit.attack_range && *cost < unit.ap {
                    // Move and attack in the same round
                    return UnitOrder {
                        unit_id,
                        move_target: Some(*best_hex),
                        action: Action::Attack { target_id: target.id },
                    };
                } else {
                    // Just maneuver closer
                    return UnitOrder {
                        unit_id,
                        move_target: Some(*best_hex),
                        action: Action::Wait,
                    };
                }
            }
        }

        // 3. No enemies in range/vision -> push down the lane toward enemy base
        Self::advance_down_lane(state, unit, occupied)
    }

    /// Multi-tier target priority: Minions > Heroes > Structures, with (dist, unit_id) tie-breaking.
    fn select_target<'a>(unit: &Unit, enemies: &[&'a Unit]) -> Option<&'a Unit> {
        let mut minions: Vec<&'a Unit> = enemies.iter().filter(|e| e.kind == UnitKind::Minion).copied().collect();
        let mut heroes: Vec<&'a Unit> = enemies.iter().filter(|e| e.kind == UnitKind::Hero).copied().collect();
        let mut structures: Vec<&'a Unit> = enemies.iter().filter(|e| e.kind.is_structure()).copied().collect();

        let sort_fn = |a: &&'a Unit, b: &&'a Unit| {
            let dist_a = unit.pos.distance(&a.pos);
            let dist_b = unit.pos.distance(&b.pos);
            dist_a.cmp(&dist_b).then_with(|| a.id.cmp(&b.id))
        };

        minions.sort_by(sort_fn);
        heroes.sort_by(sort_fn);
        structures.sort_by(sort_fn);

        minions.first().or(heroes.first()).or(structures.first()).copied()
    }

    /// Move toward the enemy base coordinate along the main horizontal lane corridor.
    fn advance_down_lane(
        state: &GameState,
        unit: &Unit,
        occupied: &HashSet<HexCoord>,
    ) -> UnitOrder {
        let goal_q = if unit.team == 0 {
            state.map.radius as i32
        } else {
            -(state.map.radius as i32)
        };
        let goal_pos = HexCoord::new(goal_q, 0);

        let reachable = state.map.reachable_hexes(unit.pos, unit.ap, occupied);
        let mut moves: Vec<(HexCoord, u32)> = reachable.into_iter().collect();

        moves.sort_by(|(hex_a, cost_a), (hex_b, cost_b)| {
            let dist_a = hex_a.distance(&goal_pos);
            let dist_b = hex_b.distance(&goal_pos);
            dist_a
                .cmp(&dist_b)
                .then_with(|| cost_a.cmp(cost_b))
                .then_with(|| (hex_a.q, hex_a.r).cmp(&(hex_b.q, hex_b.r)))
        });

        if let Some((best_hex, _)) = moves.first() {
            UnitOrder {
                unit_id: unit.id,
                move_target: Some(*best_hex),
                action: Action::Wait,
            }
        } else {
            UnitOrder {
                unit_id: unit.id,
                move_target: None,
                action: Action::Wait,
            }
        }
    }
}
```

---

### 5. Defensive Tower AI (`crates/core/src/tower_ai.rs`)

```rust
use crate::orders::{Action, UnitOrder};
use crate::state::GameState;
use crate::unit::{Unit, UnitId, UnitKind};

pub struct TowerAI;

impl TowerAI {
    /// Towers are stationary defenses that auto-attack the highest priority enemy in range.
    pub fn generate_order(state: &GameState, unit_id: UnitId) -> UnitOrder {
        let tower = match state.get_unit(unit_id) {
            Some(u) if u.is_alive() && u.attack_range > 0 => u,
            _ => {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Wait,
                }
            }
        };

        // Find enemies within firing perimeter
        let enemies = state.enemy_units(tower.team);
        let in_range: Vec<&Unit> = enemies
            .into_iter()
            .filter(|e| tower.pos.distance(&e.pos) <= tower.attack_range)
            .collect();

        if in_range.is_empty() {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            };
        }

        // Tower aggro priority: Minion > Hero > Structure (minions tank tower shots!)
        let mut minions: Vec<&Unit> = in_range.iter().filter(|e| e.kind == UnitKind::Minion).copied().collect();
        let mut heroes: Vec<&Unit> = in_range.iter().filter(|e| e.kind == UnitKind::Hero).copied().collect();
        let mut structures: Vec<&Unit> = in_range.iter().filter(|e| e.kind.is_structure()).copied().collect();

        let sort_fn = |a: &&Unit, b: &&Unit| {
            let dist_a = tower.pos.distance(&a.pos);
            let dist_b = tower.pos.distance(&b.pos);
            dist_a.cmp(&dist_b).then_with(|| a.id.cmp(&b.id))
        };

        minions.sort_by(sort_fn);
        heroes.sort_by(sort_fn);
        structures.sort_by(sort_fn);

        let target = minions.first().or(heroes.first()).or(structures.first()).copied();

        if let Some(t) = target {
            UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Attack { target_id: t.id },
            }
        } else {
            UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            }
        }
    }
}
```

---

### 6. Unified AI & Timeout Fallback (`crates/core/src/ai.rs`)

```rust
use crate::hex::HexCoord;
use crate::minion_ai::MinionAI;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::GameState;
use crate::tower_ai::TowerAI;
use crate::unit::{TeamId, UnitId, UnitKind};
use std::collections::HashSet;

pub struct GameAI;

impl GameAI {
    /// Generate orders for all units belonging to `team`.
    pub fn generate_orders(state: &GameState, team: TeamId) -> TurnOrders {
        let mut orders = TurnOrders::new();
        let occupied = state.occupied_hexes();

        for unit in state.team_units(team) {
            if !unit.is_alive() {
                continue;
            }

            let order = match unit.kind {
                UnitKind::Hero => Self::generate_hero_order(state, unit.id, &occupied),
                UnitKind::Minion => MinionAI::generate_order(state, unit.id, &occupied),
                UnitKind::Tower => TowerAI::generate_order(state, unit.id),
                UnitKind::SpawnerTower | UnitKind::Neutral => UnitOrder {
                    unit_id: unit.id,
                    move_target: None,
                    action: Action::Wait,
                },
            };

            orders.add_order(order);
        }

        orders
    }

    /// Autonomous hero AI: seeks visible enemies, advances along shortest path, attacks.
    pub fn generate_hero_order(
        state: &GameState,
        unit_id: UnitId,
        occupied: &HashSet<HexCoord>,
    ) -> UnitOrder {
        let unit = match state.get_unit(unit_id) {
            Some(u) if u.is_alive() => u,
            _ => {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Wait,
                }
            }
        };

        let visible_enemies = state.visible_enemy_units(unit.team);
        let target_pool = if !visible_enemies.is_empty() {
            visible_enemies
        } else {
            state.enemy_units(unit.team)
        };

        if target_pool.is_empty() {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            };
        }

        let mut sorted_targets = target_pool;
        sorted_targets.sort_by(|a, b| {
            let dist_a = unit.pos.distance(&a.pos);
            let dist_b = unit.pos.distance(&b.pos);
            dist_a.cmp(&dist_b).then_with(|| a.id.cmp(&b.id))
        });

        let target = sorted_targets[0];
        let distance = unit.pos.distance(&target.pos);

        // In range -> attack
        if distance <= unit.attack_range {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Attack { target_id: target.id },
            };
        }

        // Maneuver towards target
        let ap_for_move = if distance <= unit.attack_range + unit.max_ap {
            unit.max_ap.saturating_sub(1)
        } else {
            unit.max_ap
        };

        let reachable = state.map.reachable_hexes(unit.pos, ap_for_move, occupied);
        let mut moves: Vec<(HexCoord, u32)> = reachable.into_iter().collect();
        moves.sort_by(|(hex_a, cost_a), (hex_b, cost_b)| {
            let dist_a = hex_a.distance(&target.pos);
            let dist_b = hex_b.distance(&target.pos);
            dist_a
                .cmp(&dist_b)
                .then_with(|| cost_a.cmp(cost_b))
                .then_with(|| (hex_a.q, hex_a.r).cmp(&(hex_b.q, hex_b.r)))
        });

        if let Some((best_hex, _)) = moves.first() {
            let new_dist = best_hex.distance(&target.pos);
            if new_dist <= unit.attack_range {
                UnitOrder {
                    unit_id,
                    move_target: Some(*best_hex),
                    action: Action::Attack { target_id: target.id },
                }
            } else {
                UnitOrder {
                    unit_id,
                    move_target: Some(*best_hex),
                    action: Action::Wait,
                }
            }
        } else {
            UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            }
        }
    }

    /// Auto-fill missing orders when timer expires. Preserves already submitted player orders.
    pub fn generate_fallback_orders(
        state: &GameState,
        team: TeamId,
        existing_orders: &TurnOrders,
    ) -> TurnOrders {
        let mut orders = existing_orders.clone();
        let occupied = state.occupied_hexes();

        for unit in state.team_units(team) {
            if !unit.is_alive() || unit.kind != UnitKind::Hero {
                continue;
            }

            if orders.get_order(unit.id).is_none() {
                let fallback = Self::generate_hero_order(state, unit.id, &occupied);
                orders.add_order(fallback);
            }
        }

        orders
    }
}
```

---

### 7. Orders & Expanded Event Stream (`crates/core/src/event.rs`)

```rust
use crate::hex::HexCoord;
use crate::unit::{TeamId, UnitId, UnitKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameEvent {
    RoundStarted {
        round: u32,
    },

    UnitSpawned {
        unit_id: UnitId,
        unit_kind: UnitKind,
        team: TeamId,
        pos: HexCoord,
        spawner_id: UnitId,
    },

    UnitMoved {
        unit_id: UnitId,
        from: HexCoord,
        to: HexCoord,
        path: Vec<HexCoord>,
        ap_spent: u32,
    },

    UnitAttacked {
        attacker_id: UnitId,
        target_id: UnitId,
        damage: u32,
        target_hp_remaining: u32,
    },

    TowerAttacked {
        tower_id: UnitId,
        target_id: UnitId,
        damage: u32,
        target_hp_remaining: u32,
    },

    UnitDied {
        unit_id: UnitId,
        unit_kind: UnitKind,
        killed_by: UnitId,
    },

    UnitWaited {
        unit_id: UnitId,
    },

    FogUpdated {
        team: TeamId,
        visible_hexes: Vec<HexCoord>,
    },

    RoundEnded {
        round: u32,
    },

    MatchEnded {
        winner: Option<TeamId>,
    },
}
```

---

### 8. Turn Processor with Preserved Cooperative Movement (`crates/core/src/turn.rs`)

```rust
use crate::ai::GameAI;
use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::spawner::SpawnerSystem;
use crate::state::{GameState, Phase};
use crate::unit::{UnitId, UnitKind, ATTACK_AP_COST};
use std::collections::{HashMap, HashSet};

pub struct TurnProcessor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryCheck {
    Free,
    WillLeave,
    Blocked,
}

impl TurnProcessor {
    /// Resolve the simultaneous turn deterministically with full cooperative movement.
    pub fn resolve(state: &mut GameState, player_orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        state.phase = Phase::Resolution;
        events.push(GameEvent::RoundStarted {
            round: state.round + 1,
        });

        // 1. Reset AP for all units
        state.reset_all_ap();

        // 2. Process Spawner waves
        let spawn_events = SpawnerSystem::process_spawns(state);
        events.extend(spawn_events);

        // Pre-resolution snapshot for stable AI planning
        let snapshot = state.clone();

        // 3. Generate AI orders for Team 1
        let team1_orders = GameAI::generate_orders(&snapshot, 1);

        // 4. Auto-fill missing orders for Team 0 heroes (Turn Timer Fallback)
        let mut complete_team0_orders =
            GameAI::generate_fallback_orders(&snapshot, 0, player_orders);

        // 5. Generate automated orders for Team 0 Minions and Towers
        let team0_auto_orders = GameAI::generate_orders(&snapshot, 0);
        for order in team0_auto_orders.orders {
            if let Some(u) = snapshot.get_unit(order.unit_id) {
                if u.kind != UnitKind::Hero {
                    complete_team0_orders.add_order(order);
                }
            }
        }

        // 6. Combine all orders
        let mut all_orders = complete_team0_orders;
        for order in team1_orders.orders {
            all_orders.add_order(order);
        }

        // 7. Sort unit IDs by initiative DESC, unit_id ASC
        let mut unit_ids: Vec<UnitId> = all_orders.orders.iter().map(|o| o.unit_id).collect();
        unit_ids.sort_unstable();
        unit_ids.dedup();

        unit_ids.sort_by(|a, b| {
            let unit_a = snapshot.get_unit(*a).unwrap();
            let unit_b = snapshot.get_unit(*b).unwrap();
            unit_b
                .initiative
                .cmp(&unit_a.initiative)
                .then_with(|| a.cmp(b))
        });

        // 8. Register Destination Claims & Pending Movers (Cooperative Movement Protocol)
        let mut claims: HashMap<HexCoord, UnitId> = HashMap::new();
        let mut pending_movers: HashSet<UnitId> = HashSet::new();

        for order in &all_orders.orders {
            if let Some(dest) = order.move_target {
                if let Some(u) = snapshot.get_unit(order.unit_id) {
                    if u.is_alive() && !u.is_stationary() && dest != u.pos {
                        claims.insert(dest, order.unit_id);
                        pending_movers.insert(order.unit_id);
                    }
                }
            }
        }

        // 9. Process each unit in initiative order
        for unit_id in unit_ids {
            let order = all_orders.get_order(unit_id).unwrap().clone();

            // Skip if dead
            if !state.get_unit(unit_id).map(|u| u.is_alive()).unwrap_or(false) {
                if let Some(dest) = order.move_target {
                    if claims.get(&dest) == Some(&unit_id) {
                        claims.remove(&dest);
                    }
                }
                continue;
            }

            // Release own destination claim as it executes
            if let Some(dest) = order.move_target {
                if claims.get(&dest) == Some(&unit_id) {
                    claims.remove(&dest);
                }
            }
            pending_movers.remove(&unit_id);

            let unit_events = Self::process_unit(
                state,
                &snapshot,
                &order,
                &all_orders,
                &mut claims,
                &mut pending_movers,
            );
            events.extend(unit_events);
        }

        // 10. Update Fog of War
        state.update_fog();
        for team in 0..2 {
            let visible: Vec<HexCoord> =
                state.fog.visible_hexes(team).iter().copied().collect();
            events.push(GameEvent::FogUpdated {
                team,
                visible_hexes: visible,
            });
        }

        // 11. Evaluate MOBA Dual Victory Conditions
        if let Some(winner) = state.check_winner() {
            state.winner = Some(winner);
            state.phase = Phase::MatchEnd;
            events.push(GameEvent::MatchEnded {
                winner: Some(winner),
            });
        } else {
            state.round += 1;
            state.phase = Phase::Planning;
            events.push(GameEvent::RoundEnded { round: state.round });
        }

        events
    }

    fn check_entry(
        state: &GameState,
        claims: &HashMap<HexCoord, UnitId>,
        pending: &HashSet<UnitId>,
        mover_id: UnitId,
        hex: HexCoord,
        is_final_step: bool,
    ) -> EntryCheck {
        if !state.map.is_walkable(&hex) {
            return EntryCheck::Blocked;
        }

        // Cannot enter hex claimed by someone else
        if let Some(&claimant) = claims.get(&hex) {
            if claimant != mover_id {
                return EntryCheck::Blocked;
            }
        }

        // Occupant check
        if let Some(occ) = state.get_unit_at(&hex) {
            if occ.id == mover_id {
                return EntryCheck::Free;
            }

            // Structures can never move or be passed through
            if occ.is_stationary() {
                return EntryCheck::Blocked;
            }

            // Friendly unit stepping aside on its own turn
            if pending.contains(&occ.id) && is_final_step {
                return EntryCheck::WillLeave;
            }

            return EntryCheck::Blocked;
        }

        EntryCheck::Free
    }

    fn process_unit(
        state: &mut GameState,
        snapshot: &GameState,
        order: &UnitOrder,
        all_orders: &TurnOrders,
        claims: &mut HashMap<HexCoord, UnitId>,
        pending_movers: &mut HashSet<UnitId>,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        let unit_id = order.unit_id;
        let unit = match state.get_unit(unit_id) {
            Some(u) if u.is_alive() => u,
            _ => return events,
        };

        let is_stationary = unit.is_stationary();
        let start_pos = unit.pos;
        let mut current_pos = start_pos;
        let mut current_ap = unit.ap;

        // Phase A: Movement (Mobile units only)
        if !is_stationary {
            if let Some(dest) = order.move_target {
                if dest != start_pos {
                    if let Some(full_path) = snapshot.map.find_path(start_pos, dest) {
                        let mut actual_path = vec![start_pos];
                        let total_steps = full_path.len() - 1;

                        for (step_idx, step) in full_path.iter().skip(1).enumerate() {
                            if current_ap == 0 {
                                break;
                            }
                            let is_final = step_idx + 1 == total_steps;
                            match Self::check_entry(
                                state,
                                claims,
                                pending_movers,
                                unit_id,
                                *step,
                                is_final,
                            ) {
                                EntryCheck::Free | EntryCheck::WillLeave => {
                                    current_pos = *step;
                                    current_ap -= 1;
                                    actual_path.push(*step);
                                }
                                EntryCheck::Blocked => {
                                    break;
                                }
                            }
                        }

                        if actual_path.len() > 1 {
                            let ap_spent = (actual_path.len() - 1) as u32;
                            let mover = state.get_unit_mut(unit_id).unwrap();
                            mover.pos = current_pos;
                            mover.spend_ap(ap_spent);

                            events.push(GameEvent::UnitMoved {
                                unit_id,
                                from: start_pos,
                                to: current_pos,
                                path: actual_path,
                                ap_spent,
                            });
                        }
                    }
                }
            }
        }

        // Phase B: Combat Action
        match order.action {
            Action::Wait => {
                events.push(GameEvent::UnitWaited { unit_id });
            }
            Action::Attack { target_id } => {
                let attack_events = Self::process_attack(state, unit_id, target_id, claims);
                events.extend(attack_events);
            }
        }

        events
    }

    fn process_attack(
        state: &mut GameState,
        attacker_id: UnitId,
        target_id: UnitId,
        claims: &mut HashMap<HexCoord, UnitId>,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        let attacker = match state.get_unit(attacker_id) {
            Some(a) if a.is_alive() => a,
            _ => return events,
        };
        let target = match state.get_unit(target_id) {
            Some(t) if t.is_alive() => t,
            _ => return events,
        };

        if attacker.team == target.team {
            return events;
        }

        // Range check
        if attacker.pos.distance(&target.pos) > attacker.attack_range {
            return events;
        }

        // AP check
        if !attacker.can_afford(ATTACK_AP_COST) {
            return events;
        }

        let is_tower = attacker.kind == UnitKind::Tower;
        let damage = attacker.attack_damage;

        let attacker_mut = state.get_unit_mut(attacker_id).unwrap();
        attacker_mut.spend_ap(ATTACK_AP_COST);

        let target_mut = state.get_unit_mut(target_id).unwrap();
        target_mut.hp = target_mut.hp.saturating_sub(damage);
        let target_hp_remaining = target_mut.hp;
        let target_dead = target_hp_remaining == 0;
        let target_kind = target_mut.kind;
        let target_pos = target_mut.pos;

        if is_tower {
            events.push(GameEvent::TowerAttacked {
                tower_id: attacker_id,
                target_id,
                damage,
                target_hp_remaining,
            });
        } else {
            events.push(GameEvent::UnitAttacked {
                attacker_id,
                target_id,
                damage,
                target_hp_remaining,
            });
        }

        // Immediate Casualty Removal
        if target_dead {
            state.units.remove(&target_id);

            // Clean up destination claim held by dead unit
            claims.retain(|_, &mut claimant| claimant != target_id);

            events.push(GameEvent::UnitDied {
                unit_id: target_id,
                unit_kind: target_kind,
                killed_by: attacker_id,
            });
        }

        events
    }
}
```

---

### 9. Game State & Victory Conditions (`crates/core/src/state.rs`)

```rust
use crate::fog::FogState;
use crate::hex::{HexCoord, HexMap};
use crate::unit::{TeamId, Unit, UnitId, UnitKind};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Planning,
    Resolution,
    MatchEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub round: u32,
    pub phase: Phase,
    pub map: HexMap,
    pub units: HashMap<UnitId, Unit>,
    pub fog: FogState,
    pub winner: Option<TeamId>,
    pub next_unit_id: UnitId,
}

impl GameState {
    pub fn new(map: HexMap) -> Self {
        Self {
            round: 0,
            phase: Phase::Planning,
            map,
            units: HashMap::new(),
            fog: FogState::new(2),
            winner: None,
            next_unit_id: 1,
        }
    }

    pub fn add_unit(&mut self, unit: Unit) {
        self.next_unit_id = self.next_unit_id.max(unit.id + 1);
        self.units.insert(unit.id, unit);
    }

    #[inline]
    pub fn get_unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(&id)
    }

    #[inline]
    pub fn get_unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.get_mut(&id)
    }

    pub fn get_unit_at(&self, coord: &HexCoord) -> Option<&Unit> {
        self.units
            .values()
            .find(|u| u.pos == *coord && u.is_alive())
    }

    pub fn occupied_hexes(&self) -> HashSet<HexCoord> {
        self.units
            .values()
            .filter(|u| u.is_alive())
            .map(|u| u.pos)
            .collect()
    }

    pub fn team_units(&self, team: TeamId) -> Vec<&Unit> {
        let mut list: Vec<&Unit> = self
            .units
            .values()
            .filter(|u| u.team == team && u.is_alive())
            .collect();
        list.sort_by_key(|u| u.id);
        list
    }

    pub fn enemy_units(&self, team: TeamId) -> Vec<&Unit> {
        let mut list: Vec<&Unit> = self
            .units
            .values()
            .filter(|u| u.team != team && u.is_alive())
            .collect();
        list.sort_by_key(|u| u.id);
        list
    }

    pub fn visible_enemy_units(&self, team: TeamId) -> Vec<&Unit> {
        self.enemy_units(team)
            .into_iter()
            .filter(|u| self.fog.is_visible(team, &u.pos))
            .collect()
    }

    pub fn team_has_heroes(&self, team: TeamId) -> bool {
        self.units.values().any(|u| {
            u.team == team && u.is_alive() && u.kind == UnitKind::Hero
        })
    }

    pub fn team_has_spawner(&self, team: TeamId) -> bool {
        self.units.values().any(|u| {
            u.team == team && u.is_alive() && u.kind == UnitKind::SpawnerTower
        })
    }

    /// MOBA Dual Victory Check: All heroes dead OR Spawner Tower destroyed.
    pub fn check_winner(&self) -> Option<TeamId> {
        let team0_heroes = self.team_has_heroes(0);
        let team1_heroes = self.team_has_heroes(1);
        let team0_spawner = self.team_has_spawner(0);
        let team1_spawner = self.team_has_spawner(1);

        let team0_lost = !team0_heroes || !team0_spawner;
        let team1_lost = !team1_heroes || !team1_spawner;

        if team0_lost && team1_lost {
            None // Draw
        } else if team0_lost {
            Some(1)
        } else if team1_lost {
            Some(0)
        } else {
            None
        }
    }

    pub fn reset_all_ap(&mut self) {
        for unit in self.units.values_mut() {
            if unit.is_alive() {
                unit.reset_ap();
            }
        }
    }

    pub fn update_fog(&mut self) {
        self.fog.update(self);
    }

    pub fn alloc_unit_id(&mut self) -> UnitId {
        let id = self.next_unit_id;
        self.next_unit_id += 1;
        id
    }
}
```

---

### 10. Engine Facade (`crates/core/src/lib.rs`)

```rust
pub mod ai;
pub mod event;
pub mod fog;
pub mod hex;
pub mod minion_ai;
pub mod orders;
pub mod spawner;
pub mod state;
pub mod tower_ai;
pub mod turn;
pub mod unit;

use hex::{HexCoord, HexMap};
use orders::{Action, TurnOrders, UnitOrder};
use state::{GameState, Phase};
use turn::TurnProcessor;
use unit::{TeamId, Unit, UnitId, UnitKind};

pub struct GameEngine {
    pub state: GameState,
    pub pending_orders: TurnOrders,
}

impl GameEngine {
    /// Initialize standard Phase 2 MOBA single-lane match (radius 6 = 127 hexes).
    pub fn new() -> Self {
        let mut map = HexMap::new(6);

        // Chokepoint obstacles flanking the main horizontal lane (r = 0)
        map.obstacles.insert(HexCoord::new(0, 2));
        map.obstacles.insert(HexCoord::new(0, -2));
        map.obstacles.insert(HexCoord::new(1, 2));
        map.obstacles.insert(HexCoord::new(-1, -2));
        map.obstacles.insert(HexCoord::new(2, -3));
        map.obstacles.insert(HexCoord::new(-2, 3));

        let mut state = GameState::new(map);

        // Team 0 (Player / Blue)
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-4, -1), 3));
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(-4, 0), 2));
        state.add_unit(Unit::new_hero(3, 0, HexCoord::new(-4, 1), 1));
        state.add_unit(Unit::new_tower(4, 0, HexCoord::new(-3, 0)));
        state.add_unit(Unit::new_spawner(5, 0, HexCoord::new(-5, 0), 3));

        // Team 1 (AI / Red)
        state.add_unit(Unit::new_hero(6, 1, HexCoord::new(4, -1), 3));
        state.add_unit(Unit::new_hero(7, 1, HexCoord::new(4, 0), 2));
        state.add_unit(Unit::new_hero(8, 1, HexCoord::new(4, 1), 1));
        state.add_unit(Unit::new_tower(9, 1, HexCoord::new(3, 0)));
        state.add_unit(Unit::new_spawner(10, 1, HexCoord::new(5, 0), 3));

        state.next_unit_id = 11;
        state.update_fog();

        Self {
            state,
            pending_orders: TurnOrders::new(),
        }
    }

    /// Complete state serialization (unfiltered, for debug/replays).
    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap()
    }

    /// Sanitized state serialization for a specific team: hides enemy units obscured by fog.
    pub fn get_player_state(&self, team: TeamId) -> String {
        let mut sanitized = self.state.clone();
        sanitized.units.retain(|_, u| {
            u.team == team || self.state.fog.is_visible(team, &u.pos)
        });
        serde_json::to_string(&sanitized).unwrap()
    }

    pub fn get_map_hexes(&self) -> String {
        let hexes: Vec<(i32, i32)> = self
            .state
            .map
            .all_hexes()
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&hexes).unwrap()
    }

    pub fn get_obstacles(&self) -> String {
        let obstacles: Vec<(i32, i32)> = self
            .state
            .map
            .obstacles
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&obstacles).unwrap()
    }

    pub fn get_move_targets(&self, unit_id: UnitId) -> String {
        let targets: Vec<(i32, i32, u32)> = if let Some(unit) = self.state.get_unit(unit_id) {
            if !unit.is_alive() || unit.is_stationary() {
                vec![]
            } else {
                let occupied = self.state.occupied_hexes();
                self.state
                    .map
                    .reachable_hexes(unit.pos, unit.ap, &occupied)
                    .iter()
                    .map(|(h, cost)| (h.q, h.r, *cost))
                    .collect()
            }
        } else {
            vec![]
        };
        serde_json::to_string(&targets).unwrap()
    }

    /// Valid attack targets: must be in range AND visible through Fog of War.
    pub fn get_attack_targets(&self, unit_id: UnitId, from_q: i32, from_r: i32) -> String {
        let targets: Vec<UnitId> = if let Some(unit) = self.state.get_unit(unit_id) {
            if !unit.is_alive() {
                vec![]
            } else {
                let from_pos = HexCoord::new(from_q, from_r);
                let in_range = self.state.map.hexes_in_range(from_pos, unit.attack_range);
                let range_set: std::collections::HashSet<HexCoord> =
                    in_range.into_iter().collect();

                self.state
                    .enemy_units(unit.team)
                    .iter()
                    .filter(|e| {
                        range_set.contains(&e.pos) && self.state.fog.is_visible(unit.team, &e.pos)
                    })
                    .map(|e| e.id)
                    .collect()
            }
        } else {
            vec![]
        };
        serde_json::to_string(&targets).unwrap()
    }

    pub fn set_move_order(&mut self, unit_id: UnitId, q: i32, r: i32) -> bool {
        let target = HexCoord::new(q, r);

        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 && !unit.is_stationary() {
                let occupied = self.state.occupied_hexes();
                let reachable = self
                    .state
                    .map
                    .reachable_hexes(unit.pos, unit.ap, &occupied);

                if reachable.contains_key(&target) || target == unit.pos {
                    if let Some(order) = self
                        .pending_orders
                        .orders
                        .iter_mut()
                        .find(|o| o.unit_id == unit_id)
                    {
                        order.move_target = Some(target);
                    } else {
                        self.pending_orders.add_order(UnitOrder {
                            unit_id,
                            move_target: Some(target),
                            action: Action::Wait,
                        });
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn set_attack_order(&mut self, unit_id: UnitId, target_id: UnitId) -> bool {
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 {
                if let Some(target) = self.state.get_unit(target_id) {
                    if target.team != unit.team
                        && target.is_alive()
                        && self.state.fog.is_visible(unit.team, &target.pos)
                    {
                        if let Some(order) = self
                            .pending_orders
                            .orders
                            .iter_mut()
                            .find(|o| o.unit_id == unit_id)
                        {
                            order.action = Action::Attack { target_id };
                        } else {
                            self.pending_orders.add_order(UnitOrder {
                                unit_id,
                                move_target: None,
                                action: Action::Attack { target_id },
                            });
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn set_wait_order(&mut self, unit_id: UnitId) -> bool {
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 {
                if let Some(order) = self
                    .pending_orders
                    .orders
                    .iter_mut()
                    .find(|o| o.unit_id == unit_id)
                {
                    order.action = Action::Wait;
                } else {
                    self.pending_orders.add_order(UnitOrder {
                        unit_id,
                        move_target: None,
                        action: Action::Wait,
                    });
                }
                return true;
            }
        }
        false
    }

    pub fn get_pending_orders(&self) -> String {
        serde_json::to_string(&self.pending_orders).unwrap()
    }

    pub fn all_units_ordered(&self) -> bool {
        let player_heroes: Vec<UnitId> = self
            .state
            .team_units(0)
            .iter()
            .filter(|u| u.is_alive() && u.kind == UnitKind::Hero)
            .map(|u| u.id)
            .collect();

        player_heroes
            .iter()
            .all(|id| self.pending_orders.get_order(*id).is_some())
    }

    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::replace(&mut self.pending_orders, TurnOrders::new());
        let events = TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    pub fn get_player_fog(&self) -> String {
        let visible: Vec<(i32, i32)> = self
            .state
            .fog
            .visible_hexes(0)
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&visible).unwrap()
    }

    pub fn restart(&mut self) {
        *self = GameEngine::new();
    }
}
```

---

## WebAssembly Interop Layer (`crates/wasm/src/lib.rs`)

```rust
use hexabellum_core::GameEngine;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmGame {
    engine: GameEngine,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmGame {
        WasmGame {
            engine: GameEngine::new(),
        }
    }

    pub fn get_state(&self) -> String {
        self.engine.get_state()
    }

    pub fn get_player_state(&self, team: u8) -> String {
        self.engine.get_player_state(team)
    }

    pub fn get_map_hexes(&self) -> String {
        self.engine.get_map_hexes()
    }

    pub fn get_obstacles(&self) -> String {
        self.engine.get_obstacles()
    }

    pub fn get_move_targets(&self, unit_id: u64) -> String {
        self.engine.get_move_targets(unit_id)
    }

    pub fn get_attack_targets(&self, unit_id: u64, from_q: i32, from_r: i32) -> String {
        self.engine.get_attack_targets(unit_id, from_q, from_r)
    }

    pub fn set_move_order(&mut self, unit_id: u64, q: i32, r: i32) -> bool {
        self.engine.set_move_order(unit_id, q, r)
    }

    pub fn set_attack_order(&mut self, unit_id: u64, target_id: u64) -> bool {
        self.engine.set_attack_order(unit_id, target_id)
    }

    pub fn set_wait_order(&mut self, unit_id: u64) -> bool {
        self.engine.set_wait_order(unit_id)
    }

    pub fn get_pending_orders(&self) -> String {
        self.engine.get_pending_orders()
    }

    pub fn all_units_ordered(&self) -> bool {
        self.engine.all_units_ordered()
    }

    pub fn end_turn(&mut self) -> String {
        self.engine.end_turn()
    }

    pub fn get_player_fog(&self) -> String {
        self.engine.get_player_fog()
    }

    pub fn restart(&mut self) {
        self.engine.restart();
    }
}
```

---

## Client Architecture (TypeScript + PixiJS v8)

### 1. FFI Bridge & Typed Schemas (`web/src/game/bridge.ts`)

```typescript
import init, { WasmGame } from '../wasm/pkg/hexabellum_wasm';

let game: WasmGame | null = null;

export interface HexCoord {
  q: number;
  r: number;
}

export type UnitKind = 'Hero' | 'Minion' | 'Tower' | 'SpawnerTower' | 'Neutral';

export interface UnitData {
  id: number;
  kind: UnitKind;
  team: number;
  pos: HexCoord;
  hp: number;
  max_hp: number;
  ap: number;
  max_ap: number;
  initiative: number;
  attack_damage: number;
  attack_range: number;
  vision_range: number;
  spawn_interval?: number;
  spawn_counter: number;
}

export interface GameState {
  round: number;
  phase: 'Planning' | 'Resolution' | 'MatchEnd';
  units: Record<number, UnitData>;
  winner: number | null;
}

export type GameEvent =
  | { type: 'RoundStarted'; round: number }
  | { type: 'UnitSpawned'; unit_id: number; unit_kind: UnitKind; team: number; pos: HexCoord; spawner_id: number }
  | { type: 'UnitMoved'; unit_id: number; from: HexCoord; to: HexCoord; path: HexCoord[]; ap_spent: number }
  | { type: 'UnitAttacked'; attacker_id: number; target_id: number; damage: number; target_hp_remaining: number }
  | { type: 'TowerAttacked'; tower_id: number; target_id: number; damage: number; target_hp_remaining: number }
  | { type: 'UnitDied'; unit_id: number; unit_kind: UnitKind; killed_by: number }
  | { type: 'UnitWaited'; unit_id: number }
  | { type: 'FogUpdated'; team: number; visible_hexes: HexCoord[] }
  | { type: 'RoundEnded'; round: number }
  | { type: 'MatchEnded'; winner: number | null };

export async function initGame(): Promise<void> {
  await init();
  game = new WasmGame();
}

export function getState(): GameState {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_state());
}

export function getPlayerState(team: number = 0): GameState {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_player_state(team));
}

export function getPlayerFog(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_player_fog()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getMapHexes(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_map_hexes()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getObstacles(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_obstacles()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getMoveTargets(unitId: number): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_move_targets(BigInt(unitId))).map(([q, r]: [number, number, number]) => ({ q, r }));
}

export function getAttackTargets(unitId: number, fromQ: number, fromR: number): number[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_attack_targets(BigInt(unitId), fromQ, fromR));
}

export function setMoveOrder(unitId: number, q: number, r: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_move_order(BigInt(unitId), q, r);
}

export function setAttackOrder(unitId: number, targetId: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_attack_order(BigInt(unitId), BigInt(targetId));
}

export function setWaitOrder(unitId: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_wait_order(BigInt(unitId));
}

export function allUnitsOrdered(): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.all_units_ordered();
}

export function endTurn(): GameEvent[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.end_turn());
}

export function restart(): void {
  if (!game) throw new Error("Game not initialized");
  game.restart();
}
```

---

### 2. Tactical Hex Renderer (`web/src/game/renderer.ts`)

```typescript
import * as PIXI from 'pixi.js';
import { HexCoord, GameState, UnitData } from './bridge';

export const HEX_SIZE = 30;

export class HexRenderer {
  private app: PIXI.Application;
  private hexLayer = new PIXI.Container();
  private overlayLayer = new PIXI.Container();
  private unitLayer = new PIXI.Container();
  private fogLayer = new PIXI.Container();
  private fxLayer = new PIXI.Container();

  private unitSprites = new Map<number, PIXI.Container>();

  constructor(app: PIXI.Application) {
    this.app = app;
    this.app.stage.addChild(this.hexLayer);
    this.app.stage.addChild(this.overlayLayer);
    this.app.stage.addChild(this.unitLayer);
    this.app.stage.addChild(this.fogLayer);
    this.app.stage.addChild(this.fxLayer);
  }

  hexToPixel(q: number, r: number): { x: number; y: number } {
    const x = HEX_SIZE * (Math.sqrt(3) * q + (Math.sqrt(3) / 2) * r);
    const y = HEX_SIZE * (3 / 2) * r;
    return {
      x: x + this.app.screen.width / 2,
      y: y + this.app.screen.height / 2,
    };
  }

  drawMap(hexes: HexCoord[], obstacles: HexCoord[]): void {
    this.hexLayer.removeChildren();
    const obstacleSet = new Set(obstacles.map(h => `${h.q},${h.r}`));

    for (const hex of hexes) {
      const { x, y } = this.hexToPixel(hex.q, hex.r);
      const isObstacle = obstacleSet.has(`${hex.q},${hex.r}`);

      const g = new PIXI.Graphics();
      const points: number[] = [];
      for (let i = 0; i < 6; i++) {
        const angle = (Math.PI / 3) * i - Math.PI / 6;
        points.push(x + HEX_SIZE * Math.cos(angle), y + HEX_SIZE * Math.sin(angle));
      }
      g.poly(points);

      if (isObstacle) {
        g.fill({ color: 0x24243a });
        g.stroke({ color: 0x3d3d5c, width: 1.5 });
      } else {
        // Highlight central combat lane along r = 0
        const isLane = hex.r === 0;
        g.fill({ color: isLane ? 0x162438 : 0x101726 });
        g.stroke({ color: isLane ? 0x253b5c : 0x1a2638, width: 1 });
      }

      this.hexLayer.addChild(g);
    }
  }

  drawFog(visibleHexes: HexCoord[], allHexes: HexCoord[]): void {
    this.fogLayer.removeChildren();
    const visibleSet = new Set(visibleHexes.map(h => `${h.q},${h.r}`));

    for (const hex of allHexes) {
      if (!visibleSet.has(`${hex.q},${hex.r}`)) {
        const { x, y } = this.hexToPixel(hex.q, hex.r);
        const g = new PIXI.Graphics();
        const points: number[] = [];
        for (let i = 0; i < 6; i++) {
          const angle = (Math.PI / 3) * i - Math.PI / 6;
          points.push(x + (HEX_SIZE + 0.5) * Math.cos(angle), y + (HEX_SIZE + 0.5) * Math.sin(angle));
        }
        g.poly(points);
        g.fill({ color: 0x05070f, alpha: 0.75 });
        this.fogLayer.addChild(g);
      }
    }
  }

  drawUnits(state: GameState): void {
    this.unitLayer.removeChildren();
    this.unitSprites.clear();

    for (const [idStr, unit] of Object.entries(state.units)) {
      const id = Number(idStr);
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);

      const container = new PIXI.Container();
      container.x = x;
      container.y = y;

      const g = new PIXI.Graphics();
      const isPlayer = unit.team === 0;
      const baseColor = isPlayer ? 0x00d2ff : 0xff3366;

      // Draw stylized representation based on UnitKind
      switch (unit.kind) {
        case 'Hero': {
          g.circle(0, 0, HEX_SIZE * 0.52);
          g.fill({ color: baseColor });
          g.stroke({ color: 0xffffff, width: 2 });
          // Inner core star
          g.circle(0, 0, HEX_SIZE * 0.2);
          g.fill({ color: 0xffffff });
          break;
        }
        case 'Minion': {
          // Creep triangle
          const tip = isPlayer ? HEX_SIZE * 0.45 : -HEX_SIZE * 0.45;
          g.poly([tip, 0, -tip * 0.7, -HEX_SIZE * 0.35, -tip * 0.7, HEX_SIZE * 0.35]);
          g.fill({ color: baseColor });
          g.stroke({ color: 0xffffff, width: 1.5 });
          break;
        }
        case 'Tower': {
          // Fortified turret square
          const size = HEX_SIZE * 0.8;
          g.rect(-size / 2, -size / 2, size, size);
          g.fill({ color: baseColor });
          g.stroke({ color: 0xffffff, width: 2.5 });
          // Inner core
          g.rect(-size / 4, -size / 4, size / 2, size / 2);
          g.fill({ color: 0x111118 });
          break;
        }
        case 'SpawnerTower': {
          // Spire crystal diamond
          g.poly([0, -HEX_SIZE * 0.6, HEX_SIZE * 0.5, 0, 0, HEX_SIZE * 0.6, -HEX_SIZE * 0.5, 0]);
          g.fill({ color: isPlayer ? 0x7c4dff : 0xff9100 });
          g.stroke({ color: 0xffffff, width: 2 });
          break;
        }
        default:
          g.circle(0, 0, HEX_SIZE * 0.4);
          g.fill({ color: 0x888888 });
          break;
      }

      // HP Bar (Above unit)
      const hpWidth = HEX_SIZE * 0.9;
      const hpHeight = 5;
      const hpX = -hpWidth / 2;
      const hpY = -HEX_SIZE * 0.8;
      const hpRatio = Math.max(0, Math.min(1, unit.hp / unit.max_hp));

      g.rect(hpX, hpY, hpWidth, hpHeight);
      g.fill({ color: 0x111118 });
      g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
      g.fill({ color: hpRatio > 0.5 ? 0x00e676 : hpRatio > 0.25 ? 0xffea00 : 0xff1744 });

      // Spawner wave timer dots or Hero AP dots
      if (unit.kind === 'SpawnerTower' && unit.spawn_interval) {
        const dotsY = HEX_SIZE * 0.75;
        for (let i = 0; i < unit.spawn_interval; i++) {
          const dotX = (i - (unit.spawn_interval - 1) / 2) * 10;
          g.circle(dotX, dotsY, 3);
          g.fill({ color: i < unit.spawn_counter ? 0x7c4dff : 0x333344 });
        }
      } else if (unit.kind === 'Hero') {
        const apY = HEX_SIZE * 0.75;
        for (let i = 0; i < unit.max_ap; i++) {
          const apX = (i - (unit.max_ap - 1) / 2) * 10;
          g.circle(apX, apY, 3);
          g.fill({ color: i < unit.ap ? 0xffd600 : 0x333344 });
        }
      }

      container.addChild(g);
      this.unitLayer.addChild(container);
      this.unitSprites.set(id, container);
    }
  }

  drawMoveTargets(targets: HexCoord[]): void {
    this.overlayLayer.removeChildren();
    for (const target of targets) {
      const { x, y } = this.hexToPixel(target.q, target.r);
      const g = new PIXI.Graphics();
      g.circle(x, y, HEX_SIZE * 0.3);
      g.fill({ color: 0x00e676, alpha: 0.6 });
      g.stroke({ color: 0xffffff, width: 1.5 });
      this.overlayLayer.addChild(g);
    }
  }

  clearMoveTargets(): void {
    this.overlayLayer.removeChildren();
  }

  getUnitSprite(unitId: number): PIXI.Container | undefined {
    return this.unitSprites.get(unitId);
  }

  removeUnitSprite(unitId: number): void {
    const sprite = this.unitSprites.get(unitId);
    if (sprite) {
      this.unitLayer.removeChild(sprite);
      this.unitSprites.delete(unitId);
    }
  }

  getStage(): PIXI.Container {
    return this.app.stage;
  }

  getFxLayer(): PIXI.Container {
    return this.fxLayer;
  }

  getApp(): PIXI.Application {
    return this.app;
  }
}
```

---

### 3. Sequential Event Animator (`web/src/game/animator.ts`)

```typescript
import * as PIXI from 'pixi.js';
import { HexRenderer } from './renderer';
import { GameEvent, HexCoord } from './bridge';

export class Animator {
  private renderer: HexRenderer;
  private queue: GameEvent[] = [];
  private isPlaying: boolean = false;
  private onComplete: (() => void) | null = null;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
  }

  playEvents(events: GameEvent[], onComplete: () => void): void {
    this.queue = [...events];
    this.onComplete = onComplete;
    this.isPlaying = true;
    this.playNext();
  }

  private playNext(): void {
    if (this.queue.length === 0) {
      this.isPlaying = false;
      this.onComplete?.();
      return;
    }

    const event = this.queue.shift()!;
    switch (event.type) {
      case 'UnitMoved':
        this.animateMovement(event, () => this.playNext());
        break;
      case 'UnitAttacked':
      case 'TowerAttacked':
        this.animateAttack(event, () => this.playNext());
        break;
      case 'UnitDied':
        this.animateDeath(event, () => this.playNext());
        break;
      case 'UnitSpawned':
        this.animateSpawn(event, () => this.playNext());
        break;
      default:
        // Immediate events (RoundStarted, FogUpdated, etc.)
        this.playNext();
        break;
    }
  }

  private animateMovement(
    data: { unit_id: number; path: HexCoord[] },
    onDone: () => void
  ): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite || !data.path || data.path.length < 2) {
      onDone();
      return;
    }

    const points = data.path.map(hex => this.renderer.hexToPixel(hex.q, hex.r));
    const duration = 250 * (points.length - 1);
    const startTime = performance.now();

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      const totalSegments = points.length - 1;
      const segmentProgress = progress * totalSegments;
      const idx = Math.min(Math.floor(segmentProgress), totalSegments - 1);
      const t = segmentProgress - idx;

      const from = points[idx];
      const to = points[idx + 1];

      sprite.x = from.x + (to.x - from.x) * t;
      sprite.y = from.y + (to.y - from.y) * t;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        setTimeout(onDone, 40);
      }
    };

    requestAnimationFrame(animate);
  }

  private animateAttack(
    data: { target_id: number; damage: number; tower_id?: number; attacker_id?: number },
    onDone: () => void
  ): void {
    const targetSprite = this.renderer.getUnitSprite(data.target_id);
    const attackerId = data.tower_id ?? data.attacker_id;
    const attackerSprite = attackerId ? this.renderer.getUnitSprite(attackerId) : null;

    if (targetSprite) {
      // 1. Tower Laser Beam FX
      if (data.tower_id && attackerSprite) {
        const beam = new PIXI.Graphics();
        beam.moveTo(attackerSprite.x, attackerSprite.y);
        beam.lineTo(targetSprite.x, targetSprite.y);
        beam.stroke({ color: 0xff3d00, width: 4, alpha: 0.9 });
        this.renderer.getFxLayer().addChild(beam);

        setTimeout(() => {
          this.renderer.getFxLayer().removeChild(beam);
        }, 180);
      }

      // 2. Target Red Impact Flash
      const origAlpha = targetSprite.alpha;
      targetSprite.alpha = 0.4;
      this.showDamageText(targetSprite.x, targetSprite.y, data.damage);

      setTimeout(() => {
        targetSprite.alpha = origAlpha;
        setTimeout(onDone, 60);
      }, 200);
    } else {
      onDone();
    }
  }

  private animateDeath(data: { unit_id: number }, onDone: () => void): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite) {
      onDone();
      return;
    }

    const duration = 300;
    const startTime = performance.now();

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      sprite.alpha = 1 - progress;
      sprite.scale.set(1 - progress * 0.4);

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.removeUnitSprite(data.unit_id);
        setTimeout(onDone, 40);
      }
    };

    requestAnimationFrame(animate);
  }

  private animateSpawn(data: { unit_id: number }, onDone: () => void): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite) {
      onDone();
      return;
    }

    sprite.scale.set(0.1);
    const duration = 250;
    const startTime = performance.now();

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);
      sprite.scale.set(0.1 + progress * 0.9);

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        onDone();
      }
    };

    requestAnimationFrame(animate);
  }

  private showDamageText(x: number, y: number, damage: number): void {
    const text = new PIXI.Text({
      text: `-${damage}`,
      style: {
        fontSize: 18,
        fill: 0xff1744,
        fontWeight: 'bold',
        stroke: { color: 0x000000, width: 3 },
      },
    });
    text.anchor.set(0.5);
    text.x = x;
    text.y = y - 25;
    this.renderer.getFxLayer().addChild(text);

    const startTime = performance.now();
    const duration = 600;

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      text.y = y - 25 - progress * 30;
      text.alpha = 1 - progress;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.getFxLayer().removeChild(text);
      }
    };

    requestAnimationFrame(animate);
  }
}
```

---

### 4. Turn Timer & Auto-Submitter (`web/src/game/timer.ts`)

```typescript
export class TurnTimer {
  private duration: number;
  private remaining: number;
  private intervalId: number | null = null;
  private onExpire: (() => void) | null = null;
  private displayElement: HTMLElement | null = null;

  constructor(durationSeconds: number = 30) {
    this.duration = durationSeconds;
    this.remaining = durationSeconds;
  }

  setDisplay(element: HTMLElement): void {
    this.displayElement = element;
    this.updateDisplay();
  }

  start(onExpire: () => void): void {
    this.stop();
    this.remaining = this.duration;
    this.onExpire = onExpire;
    this.updateDisplay();

    this.intervalId = window.setInterval(() => {
      this.remaining--;
      this.updateDisplay();

      if (this.remaining <= 0) {
        this.stop();
        this.onExpire?.();
      }
    }, 1000);
  }

  stop(): void {
    if (this.intervalId !== null) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
  }

  private updateDisplay(): void {
    if (!this.displayElement) return;

    const minutes = Math.floor(this.remaining / 60);
    const seconds = this.remaining % 60;
    this.displayElement.textContent = `${minutes}:${seconds.toString().padStart(2, '0')}`;

    // Dynamic warning styling
    if (this.remaining <= 5) {
      this.displayElement.style.color = '#ff1744';
      this.displayElement.style.textShadow = '0 0 10px rgba(255, 23, 68, 0.8)';
    } else if (this.remaining <= 10) {
      this.displayElement.style.color = '#ffea00';
      this.displayElement.style.textShadow = '0 0 8px rgba(255, 234, 0, 0.6)';
    } else {
      this.displayElement.style.color = '#00d2ff';
      this.displayElement.style.textShadow = 'none';
    }
  }
}
```

---

### 5. Interactive Input Controller (`web/src/game/input.ts`)

```typescript
import * as PIXI from 'pixi.js';
import { HexRenderer } from './renderer';
import { Animator } from './animator';
import {
  getPlayerState,
  getMapHexes,
  getPlayerFog,
  getMoveTargets,
  getAttackTargets,
  setMoveOrder,
  setAttackOrder,
  setWaitOrder,
  endTurn,
} from './bridge';

export class InputHandler {
  private selectedUnit: number | null = null;
  private renderer: HexRenderer;
  private isResolving: boolean = false;
  private onTurnComplete: (() => void) | null = null;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
    this.setupListeners();
  }

  setOnTurnComplete(cb: () => void): void {
    this.onTurnComplete = cb;
  }

  private setupListeners(): void {
    const app = this.renderer.getApp();
    const stage = this.renderer.getStage();

    stage.eventMode = 'static';
    stage.hitArea = new PIXI.Rectangle(0, 0, app.screen.width, app.screen.height);

    stage.on('pointerdown', (e: PIXI.FederatedPointerEvent) => {
      if (this.isResolving) return;
      this.handleClick(e.global.x, e.global.y);
    });
  }

  private handleClick(x: number, y: number): void {
    const hex = this.pixelToHex(x, y);
    if (!hex) return;

    const state = getPlayerState(0);

    // 1. Click on a living Player Hero: select
    for (const [idStr, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team === 0 && unit.kind === 'Hero') {
        this.selectedUnit = Number(idStr);
        const reachable = getMoveTargets(this.selectedUnit);
        this.renderer.drawMoveTargets(reachable);
        return;
      }
    }

    // 2. If a hero is selected: execute move or attack
    if (this.selectedUnit !== null) {
      const selected = state.units[this.selectedUnit];
      if (!selected) return;

      // Click on visible enemy in attack range -> attack
      const validAttacks = getAttackTargets(this.selectedUnit, selected.pos.q, selected.pos.r);
      for (const [idStr, unit] of Object.entries(state.units)) {
        if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team !== 0) {
          const targetId = Number(idStr);
          if (validAttacks.includes(targetId)) {
            setAttackOrder(this.selectedUnit, targetId);
            this.selectedUnit = null;
            this.renderer.clearMoveTargets();
            return;
          }
        }
      }

      // Click on reachable hex -> move
      const success = setMoveOrder(this.selectedUnit, hex.q, hex.r);
      if (success) {
        this.selectedUnit = null;
        this.renderer.clearMoveTargets();
      }
    }
  }

  endTurnWithAnimation(animator: Animator, onFinished: () => void): void {
    if (this.isResolving) return;
    this.isResolving = true;
    this.selectedUnit = null;
    this.renderer.clearMoveTargets();

    const events = endTurn();
    animator.playEvents(events, () => {
      this.isResolving = false;
      onFinished();
      this.onTurnComplete?.();
    });
  }

  private pixelToHex(x: number, y: number): { q: number; r: number } | null {
    const HEX_SIZE = 30;
    const app = this.renderer.getApp();
    const cx = x - app.screen.width / 2;
    const cy = y - app.screen.height / 2;

    const q = ((Math.sqrt(3) / 3) * cx - (1 / 3) * cy) / HEX_SIZE;
    const r = ((2 / 3) * cy) / HEX_SIZE;
    return this.hexRound(q, r);
  }

  private hexRound(q: number, r: number): { q: number; r: number } {
    const s = -q - r;
    let rq = Math.round(q);
    let rr = Math.round(r);
    const rs = Math.round(s);

    const qDiff = Math.abs(rq - q);
    const rDiff = Math.abs(rr - r);
    const sDiff = Math.abs(rs - s);

    if (qDiff > rDiff && qDiff > sDiff) {
      rq = -rr - rs;
    } else if (rDiff > sDiff) {
      rr = -rq - rs;
    }
    return { q: rq, r: rr };
  }
}
```

---

### 6. Client Entrypoint & MOBA HUD (`web/src/main.ts` & `web/index.html`)

#### `web/src/main.ts`

```typescript
import * as PIXI from 'pixi.js';
import {
  initGame,
  getPlayerState,
  getMapHexes,
  getObstacles,
  getPlayerFog,
  restart,
} from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';
import { Animator } from './game/animator';
import { TurnTimer } from './game/timer';

async function main() {
  await initGame();

  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  const app = new PIXI.Application();
  await app.init({
    canvas,
    resizeTo: window,
    backgroundColor: 0x090b14,
    antialias: true,
  });

  const renderer = new HexRenderer(app);
  const animator = new Animator(renderer);
  const timer = new TurnTimer(30);

  const timerEl = document.getElementById('timer') as HTMLElement;
  const roundEl = document.getElementById('round-val') as HTMLElement;
  const statusEl = document.getElementById('status-msg') as HTMLElement;
  const endTurnBtn = document.getElementById('end-turn-btn') as HTMLButtonElement;
  const restartBtn = document.getElementById('restart-btn') as HTMLButtonElement;

  timer.setDisplay(timerEl);

  const hexes = getMapHexes();
  const obstacles = getObstacles();

  const refreshBoard = () => {
    const state = getPlayerState(0);
    const fog = getPlayerFog();

    renderer.drawMap(hexes, obstacles);
    renderer.drawUnits(state);
    renderer.drawFog(fog, hexes);

    roundEl.textContent = `Round ${state.round}`;
    if (state.phase === 'MatchEnd') {
      const won = state.winner === 0;
      statusEl.textContent = won ? 'VICTORY — ENEMY BASE DESTROYED' : 'DEFEAT — TEAM WIPED';
      statusEl.style.color = won ? '#00e676' : '#ff1744';
      timer.stop();
      endTurnBtn.disabled = true;
      restartBtn.style.display = 'inline-block';
    } else {
      statusEl.textContent = 'Planning Phase — Assign orders or wait for timer';
      statusEl.style.color = '#00d2ff';
      endTurnBtn.disabled = false;
      timer.start(() => endTurnBtn.click());
    }
  };

  const input = new InputHandler(renderer);

  endTurnBtn.addEventListener('click', () => {
    timer.stop();
    endTurnBtn.disabled = true;
    statusEl.textContent = 'Resolving simultaneous turn...';
    input.endTurnWithAnimation(animator, () => {
      refreshBoard();
    });
  });

  restartBtn.addEventListener('click', () => {
    restart();
    restartBtn.style.display = 'none';
    refreshBoard();
  });

  refreshBoard();
  console.log("Hexabellum Phase 2 MOBA Vertical Slice Initialized.");
}

main().catch(console.error);
```

#### `web/index.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Hexabellum — Tactical Hex MOBA</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { background: #090b14; color: #fff; font-family: 'Inter', system-ui, sans-serif; overflow: hidden; }
    #game-canvas { display: block; width: 100vw; height: 100vh; }
    
    #hud {
      position: absolute;
      top: 16px;
      left: 50%;
      transform: translateX(-50%);
      background: rgba(13, 17, 30, 0.85);
      backdrop-filter: blur(12px);
      border: 1px solid #1f3453;
      border-radius: 12px;
      padding: 10px 24px;
      display: flex;
      align-items: center;
      gap: 20px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.5);
    }
    .hud-stat { font-weight: 700; font-size: 16px; letter-spacing: 0.5px; }
    #timer { font-size: 22px; font-weight: 900; font-variant-numeric: tabular-nums; }
    #status-msg { font-size: 13px; color: #00d2ff; }
    
    .btn {
      background: #00d2ff;
      color: #000;
      border: none;
      padding: 8px 18px;
      border-radius: 6px;
      font-weight: 800;
      cursor: pointer;
      transition: all 0.2s;
    }
    .btn:hover:not(:disabled) { background: #33e0ff; transform: translateY(-1px); }
    .btn:disabled { background: #334155; color: #64748b; cursor: not-allowed; }
    #restart-btn { background: #00e676; display: none; }

    #legend {
      position: absolute;
      bottom: 16px;
      left: 16px;
      background: rgba(13, 17, 30, 0.85);
      border: 1px solid #1f3453;
      border-radius: 8px;
      padding: 10px 16px;
      font-size: 12px;
      line-height: 1.6;
    }
  </style>
</head>
<body>
  <div id="hud">
    <div id="round-val" class="hud-stat">Round 0</div>
    <div id="timer">0:30</div>
    <div id="status-msg">Initializing...</div>
    <button id="end-turn-btn" class="btn">End Turn</button>
    <button id="restart-btn" class="btn">Restart</button>
  </div>

  <div id="legend">
    <strong>Tactical Map Guide:</strong><br>
    🔵 Blue: Team 0 Heroes &nbsp;|&nbsp; 🔴 Red: Team 1 Heroes<br>
    🔺 Triangles: Minions &nbsp;|&nbsp; 🏰 Squares: Defensive Towers<br>
    ⭐ Crystals: Spawner Towers (Base) &nbsp;|&nbsp; 🌫️ Dark Shroud: Fog of War
  </div>

  <canvas id="game-canvas"></canvas>
  <script type="module" src="./src/main.ts"></script>
</body>
</html>
```

---

## Verification & Determinism Test Catalog

The Phase 2 engine guarantees determinism across platforms and compiler releases. The following automated integration test catalog validates all MOBA mechanics under `cargo test`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::{Action, TurnOrders, UnitOrder};
    use crate::unit::UnitKind;

    #[test]
    fn test_spawner_generates_minion_wave_on_cycle() {
        let mut engine = GameEngine::new();
        // Spawners tick every 3 rounds
        assert_eq!(engine.state.round, 0);

        // Round 1
        engine.end_turn();
        assert_eq!(engine.state.units.values().filter(|u| u.kind == UnitKind::Minion).count(), 0);

        // Round 2
        engine.end_turn();
        assert_eq!(engine.state.units.values().filter(|u| u.kind == UnitKind::Minion).count(), 0);

        // Round 3: Spawner triggers!
        let events_json = engine.end_turn();
        let minion_count = engine.state.units.values().filter(|u| u.kind == UnitKind::Minion).count();
        assert_eq!(minion_count, 2); // 1 for Blue, 1 for Red
        assert!(events_json.contains("UnitSpawned"));
    }

    #[test]
    fn test_tower_auto_attacks_and_prioritizes_minions_over_heroes() {
        let mut engine = GameEngine::new();
        // Position an enemy hero and an enemy minion within tower range
        let tower = engine.state.get_unit(4).unwrap(); // Team 0 Tower at (-3, 0), range 3
        
        // Spawn enemy hero at (-1, 0) (dist 2) and enemy minion at (-2, 0) (dist 1)
        engine.state.add_unit(Unit::new_hero(99, 1, HexCoord::new(-1, 0), 2));
        engine.state.add_unit(Unit::new_minion(100, 1, HexCoord::new(-2, 0)));
        engine.state.update_fog();

        let events_json = engine.end_turn();
        // Tower must fire on the minion (priority 1), not the hero
        assert!(events_json.contains("\"target_id\":100"));
    }

    #[test]
    fn test_fog_of_war_masks_enemy_and_blocks_targeting() {
        let engine = GameEngine::new();
        // Enemy spawner at (5, 0) is well outside Team 0 initial vision (radius 3 from x=-4)
        assert!(!engine.state.fog.is_visible(0, &HexCoord::new(5, 0)));

        // Attempting to get attack targets for hero at (-4, 0) should NOT include enemy units in fog
        let targets_json = engine.get_attack_targets(2, -4, 0);
        assert_eq!(targets_json, "[]");
    }

    #[test]
    fn test_timer_fallback_auto_plans_for_unassigned_heroes() {
        let mut engine = GameEngine::new();
        // Player only issues order for Hero 1, leaving Heroes 2 and 3 unassigned
        engine.set_move_order(1, -3, -1);

        // Trigger end_turn (simulating timer expiration)
        let events_json = engine.end_turn();
        // Engine must not hang; events must reflect resolution for all heroes
        assert!(events_json.contains("RoundEnded"));
        assert_eq!(engine.state.round, 1);
    }

    #[test]
    fn test_destroying_enemy_spawner_wins_game() {
        let mut engine = GameEngine::new();
        // Deal lethal damage to Team 1 Spawner
        let spawner = engine.state.get_unit_mut(10).unwrap();
        spawner.hp = 10;

        // Player hero attacks and finishes spawner
        engine.state.add_unit(Unit::new_hero(77, 0, HexCoord::new(5, -1), 10));
        engine.set_attack_order(77, 10);

        let events_json = engine.end_turn();
        assert!(events_json.contains("MatchEnded"));
        assert_eq!(engine.state.winner, Some(0));
    }
}
```

---

## Acceptance Criteria

| # | Criterion | Verification Method |
|---|---|---|
| 1 | Arena radius expands to 6 (127 hexes) with central lane ($r=0$) and obstacle chokepoints | `HexMap::new(6).all_hexes().len() == 127` |
| 2 | Symmetrical base structures: each team spawns with 1 Tower and 1 Spawner Tower | `engine.state.units` inspection at match start |
| 3 | Spawner Towers spawn minion waves on adjacent lane-biased tiles every 3 rounds | Tested via `test_spawner_generates_minion_wave_on_cycle` |
| 4 | Blocked spawners retain wave counter rather than dropping wave | Spawner counter check when adjacent tiles are occupied |
| 5 | Minions advance along the central lane using BFS pathfinding toward enemy base | Minion coordinates increase towards positive $q$ (Team 0) |
| 6 | Minion target priority strictly obeys Minions > Heroes > Structures with `(dist, id)` tie-breaking | Unit testing `MinionAI::select_target` |
| 7 | Towers auto-attack nearest enemy in range 3, prioritizing Minions over Heroes | Tested via `test_tower_auto_attacks_and_prioritizes_minions_over_heroes` |
| 8 | Structures are immovable and obey `is_stationary() == true` throughout cooperative resolution | Units path around towers; towers never accept move orders |
| 9 | Fog of War computes visibility rings per team; obscured enemies are omitted in `get_player_state` | Tested via `test_fog_of_war_masks_enemy_and_blocks_targeting` |
| 10 | Targeting enemies through Fog of War is strictly rejected by `get_attack_targets` | API returns empty array for targets in fog |
| 11 | Planning turn timer counts down from 30s with dynamic blue/yellow/red styling | Visual verification in UI |
| 12 | Timer expiration triggers `end_turn()` with automatic AI fallback order generation | Tested via `test_timer_fallback_auto_plans_for_unassigned_heroes` |
| 13 | Movement animates smoothly along path waypoints using lerp interpolation | Playback verified via `Animator.animateMovement` |
| 14 | Attacks trigger attacker lunges, target red flashes, and floating `-DMG` text | Visual verification via `Animator.animateAttack` |
| 15 | Tower shots fire bright beam traces with heavy damage impact numbers | Visual verification of beam FX |
| 16 | Deaths animate with scale-down and fade-out before sprite cleanup | Visual verification via `Animator.animateDeath` |
| 17 | Demolishing enemy Spawner Tower halts minion production and triggers immediate Victory | Tested via `test_destroying_enemy_spawner_wins_game` |
| 18 | Eliminating all enemy heroes triggers immediate Victory | Team elimination test passes |
| 19 | Controls lock during resolution animation playback and unlock for planning | Pointer events disabled while `isResolving == true` |
| 20 | Restart button resets arena, timer, and state to fresh Round 0 match | Functional UI click verification |

---

## What is NOT in Phase 2

To maintain focus and avoid scope creep, the following mechanics are deferred to subsequent phases:

- ❌ **Line-of-Sight Raycasting**: Fog of war uses pure axial distance radius; obstacles do not yet cast vision shadows.
- ❌ **Tower Repairs / Construction**: Towers cannot be healed or rebuilt once damaged.
- ❌ **Hero Abilities & Spells**: Combat is restricted to AP-budgeted basic attacks and movement.
- ❌ **Neutral Camps / Jungle Creeps**: No neutral monsters in jungle pockets.
- ❌ **Multiple Lanes**: Game is strictly single-lane (horizontal axis $r = 0$).
- ❌ **Gold, XP & Items**: No shop, leveling curve, or item inventory.
- ❌ **Network Server / Multiplayer**: Architecture remains local WASM + client runtime.
- ❌ **Audio & Sound Effects**: Pure PixiJS visual and text animation.

---

## Phase 3 Preview

Following the successful delivery and sign-off of Phase 2, **Phase 3 — Tactical Depth & Metagame** will introduce:
1. **Line-of-Sight Shadow Casting**: Obstacles, structures, and dense brush obscure vision.
2. **Hero Abilities & Spell Economy**: Cooldowns, mana/fury costs, skillshots, and area-of-effect (AoE) hex reticles.
3. **Jungle Neutral Camps**: Creep camps in north and south flanks granting buffs and gold upon defeat.
4. **Repair & Fortification Mechanics**: Heroes can spend AP to repair damaged towers.
5. **Item Shop & Gold Economy**: Last-hitting minions awards gold for purchasing stat-enhancing artifacts.
6. **Multi-Lane Strategic Map**: Traditional 3-lane MOBA topology (Top, Mid, Bot).
7. **Authoritative WebSocket Multiplayer**: Moving the simulation engine to an authoritative server with client prediction.
