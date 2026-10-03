# Phase 1 Technical Spec — Combat & AI Vertical Slice

---

## Architectural Decision Record (ADR-002): Deterministic Simultaneous Resolution & Cooperative Movement Protocol

### Context
In Phase 0, we validated the basic FFI and rendering pipeline with two units and simple single-step teleportation. However, Hexabellum is designed as a **simultaneous turn-based tactical MOBA**. In such a game, both teams submit orders during a concurrent planning phase, and the simulation engine must resolve all actions deterministically in a single resolution phase.

Transitioning from a 1v1 movement prototype to a full 3v3 combat game introduces several complex challenges:
1. **Simultaneous Movement Collisions & Stacking**: If Unit A and Unit B both order a move to hex `(0, 0)`, or if Unit A moves into Unit B's hex while Unit B is moving into Unit A's hex (a swap), a naive sequential or uncoordinated check causes units to either collide, illegally stack on the same tile, or deadlock indefinitely.
2. **Pathfinding Blocking vs. Friendly Cooperation**: In tactical movement on narrow hex choke points, a friendly unit currently standing on a tile will often move away during the same round. If pathfinding treats every currently occupied tile as an absolute wall, units will refuse to step forward, causing gridlocks and preventing battle lines from engaging.
3. **Casualty Invalidation & Order Revocation**: When a faster hero (higher initiative) kills an enemy hero before that enemy acts, what happens to the deceased unit's planned moves and attacks? A naive engine might allow the dead unit's move to resolve or its attack to damage an ally from beyond the grave.
4. **Determinism Across Hardware & Compilers**: In Rust, `std::collections::HashMap` uses randomized SipHash keys by default. Iterating over units or orders directly from a `HashMap` produces non-deterministic order execution, breaking replayability and desynchronizing multiplayer clients.
5. **WASM / TypeScript FFI Type Hazards**: Rust's `u64` integers (used for `UnitId`) map to JavaScript `BigInt` under `wasm-bindgen`. Passing raw JavaScript `number` values causes immediate runtime `TypeError` exceptions in browser engines.

### Decisions

| Challenge | Architectural Decision | Implementation in Phase 1 |
|---|---|---|
| **Turn Order** | **Initiative-Based Sequential Resolution with Deterministic Tie-Breaking** | Orders are resolved in descending order of hero `initiative`. Ties are broken by unique `unit_id` ascending. |
| **Pathfinding vs. Movement** | **Decoupled Geometric BFS + Per-Step Dynamic Entry Checking** | Static map pathfinding uses a deterministic Breadth-First Search (BFS) over walkable geometry with lexicographical `(q, r)` tie-breaking. Dynamic occupancy is checked per step at resolution time via `TurnProcessor::enter_step`. |
| **Tile Contention & Swapping** | **Destination Claims & Vacated-Hex Cooperative Swapping Model** | All planned destination hexes are claimed at round start. Friendly units planning to relocate are considered "vacating" during planning. During resolution, `EntryCheck::WillLeave` enables cooperative position swapping. |
| **Casualty Handling** | **Immediate Casualty Cleanup & Order Revocation** | When a unit's HP reaches 0, it is immediately removed from the active board, its destination claims are released, and any remaining actions planned for that unit this round are cancelled. |
| **Determinism** | **Stable Vector Sorting Before Any Resolution Iteration** | Unit IDs and order lists are sorted into `Vec<UnitId>` and sorted deterministically before processing. AI target searches use composite tie-breaking keys `(distance, unit_id)`. |
| **FFI Safety** | **Explicit BigInt Conversions in the TS Bridge** | All `UnitId` values crossing the WASM bridge are explicitly coerced using `BigInt(unitId)` in `bridge.ts`. |
| **Scope Boundary** | **Turn Timer Deferred to Phase 2** | The turn timer with AI fallback is strictly scoped to Phase 2 (alongside minions and towers). Phase 1 focuses exclusively on the core tactical combat loop, AP economy, and local AI. |

---

## Phase 1 Goal

> **A fully playable 3v3 tactical battle where the player commands three heroes, plans movement and attacks using Action Points (AP), resolves simultaneous rounds deterministically against an AI opponent, and achieves victory or defeat.**

Phase 0 proved the rendering and FFI pipeline. Phase 1 delivers the core **tactical combat game**.

### What Phase 1 Adds

| System | Description |
|---|---|
| **Pathfinding** | Deterministic BFS on hex grid respecting obstacles and AP constraints |
| **AP System** | 3 AP per round: movement costs 1 AP/hex, basic attack costs 1 AP (`ATTACK_AP_COST = 1`) |
| **Tactical Combat** | Targeted attacks, range checks, damage calculation, HP bars, and unit death |
| **Simultaneous Resolution** | Both sides plan concurrently; engine resolves by initiative order with destination claims |
| **Tactical Enemy AI** | Autonomous AI commands Team 1: evaluates distance, maneuvers, and attacks |
| **3v3 Team Combat** | 3 heroes per team with varied initiative ratings (3, 2, 1) |
| **Win/Loss Condition** | Team elimination: Team 0 loses if all blue heroes die; wins if all red heroes die |
| **Battle Event Stream** | JSON-serializable event logs for animations, HUD logs, and deterministic replay |

### Definition of Done (DoD)

- [ ] `hexabellum-core` passes all unit and integration tests (`cargo test`).
- [ ] 3 player heroes (Team 0, Blue) and 3 AI heroes (Team 1, Red) spawn in balanced starting positions on a radius-5 hex arena (91 hexes).
- [ ] Selecting a living player hero highlights all reachable hexes based on current AP budget.
- [ ] Clicking a reachable hex plots a path, displays AP cost, and stages the move order.
- [ ] After planning movement (or while stationary), attackable enemies within attack range are highlighted with attack reticles.
- [ ] Clicking an enemy unit stages an attack action; clicking elsewhere stages a "Wait" action.
- [ ] The player can stage, review, and adjust orders for all 3 heroes independently before ending the turn.
- [ ] Clicking "End Turn" triggers simultaneous resolution: AI generates orders for Team 1, and the engine resolves all actions in initiative order.
- [ ] Units step incrementally along their paths; friendly units cooperatively yield tiles or swap positions without clipping.
- [ ] Attacks apply damage immediately; units whose HP drops to 0 are removed from the board, releasing claims and cancelling their pending actions.
- [ ] When all heroes of a team are eliminated, the match ends and displays a Victory or Defeat overlay.
- [ ] A "Restart" button allows immediate re-initialization of a fresh match.

---

## Architecture Delta from Phase 0

```
Phase 0 (Pipeline Slice)           Phase 1 (Tactical Combat Slice)
────────────────────────           ───────────────────────────────
1v1 heroes                    →    3v3 team battle (3 heroes per side)
Direct teleport (1 hex)       →    BFS shortest path + multi-hex AP movement
No Action Points (AP)         →    AP economy (3 AP/round, 1 AP/move, 1 AP/attack)
No combat or damage           →    Targeted basic attacks, HP pools, unit death
No AI (idle dummy unit)       →    Autonomous tactical AI with AP planning
Instant player resolution     →    Simultaneous planning + initiative-ordered resolution
No win/loss state             →    Team elimination win/loss conditions
Static single unit HUD        →    Selected unit inspector, order indicators, match overlays
```

---

## High-Level Architecture & Data Flow

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                   Browser Runtime                                      │
│                                                                                        │
│   ┌────────────────────────────────────────┐    ┌──────────────────────────────────┐   │
│   │           TypeScript Frontend          │    │         hexabellum-wasm          │   │
│   │                                        │    │                                  │   │
│   │  ┌───────────────┐  ┌───────────────┐  │    │  ┌────────────────────────────┐  │   │
│   │  │ PixiJS v8     │  │ Input State   │  │    │  │ WasmGame Facade            │  │   │
│   │  │ HexRenderer   │  │ Machine       │  │    │  │ - BigInt FFI boundary      │  │   │
│   │  │ - Grid/Hexes  │  │ - Select hero │  │    │  │ - JSON serialization       │  │   │
│   │  │ - Units & HP  │  │ - Pick move   │──┼────┼─►│ - Exposes engine methods   │  │   │
│   │  │ - AP pips     │  │ - Pick attack │  │    │  └─────────────┬──────────────┘  │   │
│   │  │ - Overlays    │  │ - Stage orders│  │    │                │                 │   │
│   │  └───────────────┘  └───────────────┘  │    └────────────────┼─────────────────┘   │
│   │          ▲                             │                     │                     │
│   │          │ Events & State JSON         │                     │ Rust Crate Call     │
│   │          └─────────────────────────────┼─────────────────────┤                     │
│   │                                        │                     ▼                     │
│   │  ┌──────────────────────────────────┐  │    ┌──────────────────────────────────┐   │
│   │  │ HUD & Inspector Panel            │  │    │         hexabellum-core          │   │
│   │  │ - Round counter & Status banner  │  │    │                                  │   │
│   │  │ - Unit stats (HP, AP, Init)      │  │    │  ┌────────────────────────────┐  │   │
│   │  │ - End Turn / Restart controls    │  │    │  │ GameEngine                 │  │   │
│   │  └──────────────────────────────────┘  │    │  │ - GameState (Map, Units)   │  │   │
│   └────────────────────────────────────────┘    │  │ - Pending TurnOrders       │  │   │
│                                                 │  └─────────────┬──────────────┘  │   │
│                                                 │                │                 │   │
│                                                 │       resolve(orders)            │   │
│                                                 │                ▼                 │   │
│                                                 │  ┌────────────────────────────┐  │   │
│                                                 │  │ TurnProcessor              │  │   │
│                                                 │  │ 1. Reset AP for all units  │  │   │
│                                                 │  │ 2. Snapshot planning state │  │   │
│                                                 │  │ 3. SimpleAI orders (T1)    │  │   │
│                                                 │  │ 4. Sort by initiative/id   │  │   │
│                                                 │  │ 5. Setup destination claims│  │   │
│                                                 │  │ 6. Process units in order: │  │   │
│                                                 │  │    - Step walk (A* / BFS)  │  │   │
│                                                 │  │    - Action (Attack/Wait)  │  │   │
│                                                 │  │    - Immediate casualty cln│  │   │
│                                                 │  │ 7. Check win condition     │  │   │
│                                                 │  │ 8. Emit GameEvent stream   │  │   │
│                                                 │  └────────────────────────────┘  │   │
│                                                 └──────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Simultaneous Resolution Pipeline

Resolving simultaneous turns deterministically requires an exact sequence of steps:

```
                  ┌──────────────────────────────────────────┐
                  │ 1. Planning Phase (Concurrent)           │
                  │ Player stages orders for Team 0 units    │
                  └────────────────────┬─────────────────────┘
                                       │ Click "End Turn"
                                       ▼
                  ┌──────────────────────────────────────────┐
                  │ 2. Engine Pre-Resolution Preparation     │
                  │ - Set phase = Phase::Resolution          │
                  │ - Emit GameEvent::RoundStarted           │
                  │ - Reset AP to max_ap for all living units│
                  │ - Clone state into read-only snapshot    │
                  └────────────────────┬─────────────────────┘
                                       │
                                       ▼
                  ┌──────────────────────────────────────────┐
                  │ 3. AI Order Generation                   │
                  │ SimpleAI generates orders for Team 1     │
                  │ evaluated against pre-resolution snapshot│
                  └────────────────────┬─────────────────────┘
                                       │
                                       ▼
                  ┌──────────────────────────────────────────┐
                  │ 4. Order Aggregation & Initiative Sort   │
                  │ Combine Player + AI orders.              │
                  │ Sort unit IDs by:                        │
                  │   initiative DESC, then unit_id ASC      │
                  └────────────────────┬─────────────────────┘
                                       │
                                       ▼
                  ┌──────────────────────────────────────────┐
                  │ 5. Destination Claims & Pending Movers   │
                  │ Register claims for all planned move     │
                  │ targets to prevent destination stacking  │
                  └────────────────────┬─────────────────────┘
                                       │
                                       ▼
             ┌──► ┌──────────────────────────────────────────┐
             │    │ 6. Process Unit in Initiative Order      │
             │    │ - Is unit still alive? (If dead: SKIP)   │
             │    │ - Release claim on own destination       │
             │    │ - Phase A: Execute step-by-step walk     │
             │    │   * EntryCheck: Free / WillLeave / Block │
             │    │   * Spend 1 AP per legal step            │
             │    │   * Emit GameEvent::UnitMoved            │
             │    │ - Phase B: Execute Action                │
             │    │   * Attack: verify post-move range & AP  │
             │    │   * Deduct 1 AP, apply damage to target  │
             │    │   * Emit GameEvent::UnitAttacked         │
             │    │   * If HP == 0: remove unit, revoke      │
             │    │     claims, emit GameEvent::UnitDied     │
             │    │   * Wait: emit GameEvent::UnitWaited     │
             │    └────────────────────┬─────────────────────┘
             │                         │
             └── More units in order? ─┴─► All units processed
                                                 │
                                                 ▼
                  ┌──────────────────────────────────────────┐
                  │ 7. Win Condition & Round Wrap-Up         │
                  │ - Check if Team 0 or Team 1 has 0 heroes │
                  │ - If winner: Phase::MatchEnd             │
                  │   Emit GameEvent::MatchEnded             │
                  │ - Else: round += 1, Phase::Planning      │
                  │   Emit GameEvent::RoundEnded             │
                  └──────────────────────────────────────────┘
```

---

## Core Data Structures & Game Logic (Rust)

### 1. Unit & Action Point System (`crates/core/src/unit.rs`)

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
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub team: TeamId,
    pub pos: HexCoord,
    pub hp: u32,
    pub max_hp: u32,

    // Phase 1 additions
    pub ap: u32,
    pub max_ap: u32,
    pub initiative: u32,
    pub attack_damage: u32,
    pub attack_range: u32,
}

/// Cost in Action Points (AP) of a basic attack action in Phase 1.
pub const ATTACK_AP_COST: u32 = 1;

impl Unit {
    /// Create a standard hero for Phase 1.
    /// Default stats: 100 HP, 3 AP, 20 attack damage, 1 attack range (adjacent hexes).
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
        }
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.hp > 0
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
}
```

---

### 2. Pathfinding & Reachable Space (`crates/core/src/hex.rs`)

On an unweighted hexagonal grid, movement between adjacent hexes costs exactly 1 AP. A deterministic Breadth-First Search (BFS) with lexicographical neighbor sorting computes the optimal shortest geometric path in $O(V + E)$ time without heap-ordering drift:

```rust
use crate::hex::HexCoord;
use std::collections::{HashMap, HashSet, VecDeque};

impl HexMap {
    /// Find the shortest walkable path from start to goal (BFS over static walkable hexes).
    ///
    /// Path length is measured on map geometry only — dynamic unit occupancy is checked
    /// step-by-step during turn resolution (see TurnProcessor::enter_step).
    /// Neighbor traversal is sorted by (q, r) for deterministic path tie-breaking.
    pub fn find_path(
        &self,
        start: HexCoord,
        goal: HexCoord,
    ) -> Option<Vec<HexCoord>> {
        if start == goal {
            return Some(vec![start]);
        }

        if !self.is_walkable(&start) || !self.is_walkable(&goal) {
            return None;
        }

        let mut came_from: HashMap<HexCoord, HexCoord> = HashMap::new();
        let mut visited: HashSet<HexCoord> = HashSet::new();
        let mut queue: VecDeque<HexCoord> = VecDeque::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let mut nbrs: Vec<HexCoord> = current
                .neighbors()
                .into_iter()
                .filter(|n| self.is_walkable(n) && !visited.contains(n))
                .collect();
            nbrs.sort_by_key(|h| (h.q, h.r)); // Deterministic tie-breaking

            for neighbor in nbrs {
                visited.insert(neighbor);
                came_from.insert(neighbor, current);

                if neighbor == goal {
                    // Reconstruct path from goal back to start
                    let mut path = vec![goal];
                    let mut node = goal;
                    while let Some(&prev) = came_from.get(&node) {
                        path.push(prev);
                        node = prev;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(neighbor);
            }
        }

        None
    }

    /// Compute all hexes reachable from start within the given AP budget.
    /// Returns a map of HexCoord -> AP cost.
    pub fn reachable_hexes(
        &self,
        start: HexCoord,
        ap_budget: u32,
        occupied: &HashSet<HexCoord>,
    ) -> HashMap<HexCoord, u32> {
        let mut result: HashMap<HexCoord, u32> = HashMap::new();
        let mut frontier: Vec<(HexCoord, u32)> = vec![(start, 0)];
        result.insert(start, 0);

        while let Some((current, cost)) = frontier.pop() {
            if cost >= ap_budget {
                continue;
            }

            for neighbor in current.neighbors() {
                if !self.is_walkable(&neighbor) || occupied.contains(&neighbor) {
                    continue;
                }

                let new_cost = cost + 1;
                if new_cost <= ap_budget {
                    let existing = result.get(&neighbor).copied().unwrap_or(u32::MAX);
                    if new_cost < existing {
                        result.insert(neighbor, new_cost);
                        frontier.push((neighbor, new_cost));
                    }
                }
            }
        }

        result.remove(&start); // Starting position is not a movement target
        result
    }

    /// Return all walkable hexes within attack range of a center position.
    pub fn hexes_in_range(&self, center: HexCoord, range: u32) -> Vec<HexCoord> {
        let mut results = Vec::new();
        for r in 1..=range {
            results.extend(center.ring(r));
        }
        results.retain(|h| self.is_walkable(h));
        results
    }
}
```

---

### 3. Game State & Victory Conditions (`crates/core/src/state.rs`)

```rust
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
            winner: None,
            next_unit_id: 1,
        }
    }

    pub fn add_unit(&mut self, unit: Unit) {
        self.next_unit_id = self.next_unit_id.max(unit.id + 1);
        self.units.insert(unit.id, unit);
    }

    pub fn get_unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(&id)
    }

    pub fn get_unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.get_mut(&id)
    }

    pub fn get_unit_at(&self, coord: &HexCoord) -> Option<&Unit> {
        self.units.values().find(|u| u.pos == *coord && u.is_alive())
    }

    pub fn is_occupied(&self, coord: &HexCoord) -> bool {
        self.get_unit_at(coord).is_some()
    }

    pub fn occupied_hexes(&self) -> HashSet<HexCoord> {
        self.units
            .values()
            .filter(|u| u.is_alive())
            .map(|u| u.pos)
            .collect()
    }

    pub fn alive_units(&self) -> Vec<&Unit> {
        self.units.values().filter(|u| u.is_alive()).collect()
    }

    pub fn team_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units
            .values()
            .filter(|u| u.team == team && u.is_alive())
            .collect()
    }

    pub fn enemy_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units
            .values()
            .filter(|u| u.team != team && u.is_alive())
            .collect()
    }

    pub fn team_has_heroes(&self, team: TeamId) -> bool {
        self.units.values().any(|u| {
            u.team == team && u.is_alive() && u.kind == UnitKind::Hero
        })
    }

    /// Check if either team has won by eliminating all enemy heroes.
    pub fn check_winner(&self) -> Option<TeamId> {
        let team0_alive = self.team_has_heroes(0);
        let team1_alive = self.team_has_heroes(1);

        if !team0_alive && !team1_alive {
            None // Simultaneous wipeout counts as a draw
        } else if !team0_alive {
            Some(1) // Team 1 (AI) wins
        } else if !team1_alive {
            Some(0) // Team 0 (Player) wins
        } else {
            None // Battle continues
        }
    }

    pub fn reset_all_ap(&mut self) {
        for unit in self.units.values_mut() {
            if unit.is_alive() {
                unit.reset_ap();
            }
        }
    }
}
```

---

### 4. Orders & Event Model (`crates/core/src/orders.rs` & `event.rs`)

#### `crates/core/src/orders.rs`
```rust
use crate::hex::HexCoord;
use crate::unit::UnitId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Wait,
    Attack { target_id: UnitId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitOrder {
    pub unit_id: UnitId,
    pub move_target: Option<HexCoord>,
    pub action: Action,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnOrders {
    pub orders: Vec<UnitOrder>,
}

impl TurnOrders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_order(&mut self, order: UnitOrder) {
        self.orders.retain(|o| o.unit_id != order.unit_id);
        self.orders.push(order);
    }

    pub fn get_order(&self, unit_id: UnitId) -> Option<&UnitOrder> {
        self.orders.iter().find(|o| o.unit_id == unit_id)
    }

    pub fn clear(&mut self) {
        self.orders.clear();
    }
}
```

#### `crates/core/src/event.rs`
```rust
use crate::hex::HexCoord;
use crate::unit::{TeamId, UnitId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameEvent {
    RoundStarted {
        round: u32,
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
    UnitDied {
        unit_id: UnitId,
        killed_by: UnitId,
    },
    UnitWaited {
        unit_id: UnitId,
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

### 5. Autonomous Tactical AI (`crates/core/src/ai.rs`)

The AI commands the enemy team (Team 1). To ensure absolute reproducibility:
1. Active units are iterated in sorted order by `unit_id`.
2. Target enemy selection tie-breaks by distance, then `enemy.id`.
3. Candidate destination hexes are filtered against planned landing zones (`final_occupancy`).
4. Requisite AP for attacking is reserved whenever moving within range.

```rust
use crate::hex::HexCoord;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::GameState;
use crate::unit::{TeamId, UnitId};
use std::collections::HashSet;

pub struct SimpleAI;

impl SimpleAI {
    pub fn generate_orders(state: &GameState, team: TeamId) -> TurnOrders {
        let mut orders = TurnOrders::new();
        let occupied = state.occupied_hexes();

        // Sort unit IDs deterministically
        let mut unit_ids: Vec<UnitId> = state
            .team_units(team)
            .iter()
            .filter(|u| u.is_alive())
            .map(|u| u.id)
            .collect();
        unit_ids.sort_unstable();

        for unit_id in unit_ids {
            let order = Self::generate_unit_order(state, unit_id, &occupied, &orders);
            orders.add_order(order);
        }

        orders
    }

    fn generate_unit_order(
        state: &GameState,
        unit_id: UnitId,
        occupied: &HashSet<HexCoord>,
        orders: &TurnOrders,
    ) -> UnitOrder {
        let unit = state.get_unit(unit_id).unwrap();
        let enemies = state.enemy_units(unit.team);

        if enemies.is_empty() {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            };
        }

        // Find nearest enemy with deterministic tie-breaking
        let nearest_enemy = enemies
            .iter()
            .min_by_key(|e| (unit.pos.distance(&e.pos), e.id))
            .unwrap();

        let distance_to_enemy = unit.pos.distance(&nearest_enemy.pos);

        // If enemy is already in range, attack immediately without moving
        if distance_to_enemy <= unit.attack_range {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Attack {
                    target_id: nearest_enemy.id,
                },
            };
        }

        // Reserve 1 AP for an attack if we can close into range this turn
        let ap_for_movement = if distance_to_enemy <= unit.attack_range + unit.max_ap {
            unit.max_ap.saturating_sub(1)
        } else {
            unit.max_ap
        };

        // Exclude own hex and friendly hexes that are vacating this round
        let mut blocked = occupied.clone();
        blocked.remove(&unit.pos);
        for other in state.team_units(unit.team) {
            if other.id == unit_id || !other.is_alive() {
                continue;
            }
            if let Some(next_order) = orders.get_order(other.id) {
                if let Some(dest) = next_order.move_target {
                    if dest != other.pos {
                        blocked.remove(&other.pos);
                    }
                }
            }
        }

        // Find reachable hexes within movement budget
        let reachable = state
            .map
            .reachable_hexes(unit.pos, ap_for_movement, &blocked);

        let mut candidates: Vec<(&HexCoord, u32)> = reachable.iter().map(|(h, c)| (h, *c)).collect();
        candidates.sort_by_key(|(hex, cost)| {
            (hex.distance(&nearest_enemy.pos), *cost, hex.q, hex.r)
        });

        // Ensure destination will not be occupied by friendly units arriving there
        let final_occupancy = |hex: &HexCoord| -> bool {
            state.units.values().any(|u| {
                u.id != unit_id
                    && u.is_alive()
                    && ((u.team != unit.team && u.pos == *hex)
                        || (u.team == unit.team
                            && u.pos == *hex
                            && orders
                                .get_order(u.id)
                                .map(|o| o.move_target != Some(*hex))
                                .unwrap_or(true)))
            }) || orders.orders.iter().any(|o| {
                o.unit_id != unit_id && o.move_target == Some(*hex)
            })
        };

        let (move_target, action) = if let Some((best_hex, _)) = candidates
            .iter()
            .find(|(hex, _)| !final_occupancy(hex))
        {
            let new_distance = best_hex.distance(&nearest_enemy.pos);

            if new_distance <= unit.attack_range && ap_for_movement > 0 {
                (
                    Some(**best_hex),
                    Action::Attack {
                        target_id: nearest_enemy.id,
                    },
                )
            } else {
                (Some(**best_hex), Action::Wait)
            }
        } else {
            (None, Action::Wait)
        };

        UnitOrder {
            unit_id,
            move_target,
            action,
        }
    }
}
```

---

### 6. Turn Resolution & Cooperative Movement (`crates/core/src/turn.rs`)

The `TurnProcessor` executes the complete resolution phase. It tracks destination claims to prevent tile stacking, resolves cooperative position swaps (`EntryCheck::WillLeave`), spends AP per step, executes basic attacks, and immediately handles unit deaths:

```rust
use crate::ai::SimpleAI;
use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::{GameState, Phase};
use crate::unit::{UnitId, ATTACK_AP_COST};
use std::collections::{HashMap, HashSet};

pub struct TurnProcessor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryCheck {
    Free,
    WillLeave,
    Blocked,
}

impl TurnProcessor {
    pub fn resolve(state: &mut GameState, player_orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        state.phase = Phase::Resolution;
        events.push(GameEvent::RoundStarted {
            round: state.round + 1,
        });

        // 1. Reset AP for all living units
        state.reset_all_ap();

        // 2. Snapshot pre-resolution state so AI decisions evaluate against planning baseline
        let snapshot = state.clone();

        // 3. Generate AI orders for Team 1
        let ai_orders = SimpleAI::generate_orders(&snapshot, 1);

        // 4. Combine all orders
        let mut all_orders = player_orders.clone();
        for order in ai_orders.orders {
            all_orders.add_order(order);
        }

        // 5. Deduplicate and sort unit IDs by initiative descending, then unit_id ascending
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

        // 6. Destination claims setup
        let mut claims: HashMap<HexCoord, UnitId> = HashMap::new();
        let mut pending_movers: HashSet<UnitId> = HashSet::new();
        for o in &all_orders.orders {
            if let Some(dest) = o.move_target {
                if let Some(u) = snapshot.get_unit(o.unit_id) {
                    if u.is_alive() && dest != u.pos {
                        claims.insert(dest, o.unit_id);
                        pending_movers.insert(o.unit_id);
                    }
                }
            }
        }

        // 7. Process each unit in initiative order
        for unit_id in unit_ids {
            let order = all_orders.get_order(unit_id).unwrap().clone();

            // Skip if the unit was killed by an earlier unit this round
            if !state.get_unit(unit_id).map(|u| u.is_alive()).unwrap_or(false) {
                if let Some(dest) = order.move_target {
                    if claims.get(&dest) == Some(&unit_id) {
                        claims.remove(&dest);
                    }
                }
                continue;
            }

            // Release own destination claim before attempting entry
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

        // 8. Check victory conditions
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
        mover_final_dest: Option<HexCoord>,
        hex: &HexCoord,
    ) -> EntryCheck {
        if let Some(u) = state.get_unit_at(hex) {
            if u.id == mover_id {
                return EntryCheck::Free;
            }
            if let Some(&claimer) = claims.get(hex) {
                if claimer != mover_id {
                    return EntryCheck::Blocked;
                }
            }
            if pending.contains(&u.id) {
                if mover_final_dest == Some(*hex) {
                    return EntryCheck::WillLeave;
                }
                return EntryCheck::Blocked;
            }
            return EntryCheck::Blocked;
        }
        if let Some(&claimer) = claims.get(hex) {
            if claimer != mover_id {
                return EntryCheck::Blocked;
            }
        }
        EntryCheck::Free
    }

    fn enter_step(
        state: &mut GameState,
        all_orders: &TurnOrders,
        claims: &mut HashMap<HexCoord, UnitId>,
        pending: &mut HashSet<UnitId>,
        mover_id: UnitId,
        mover_final_dest: Option<HexCoord>,
        step: HexCoord,
        ap_cost: u32,
    ) -> bool {
        match Self::check_entry(state, claims, pending, mover_id, mover_final_dest, &step) {
            EntryCheck::Free => {}
            EntryCheck::WillLeave => {
                // Cooperative swap: the occupant will relocate to the mover's starting hex
                let blocker_id = state.get_unit_at(&step).map(|u| u.id).unwrap();
                let my_pos = state.get_unit(mover_id).unwrap().pos;
                let blocker_dest = all_orders.get_order(blocker_id).and_then(|o| o.move_target);

                if blocker_dest != Some(my_pos) {
                    if !Self::enter_step(
                        state,
                        all_orders,
                        claims,
                        pending,
                        blocker_id,
                        blocker_dest,
                        my_pos,
                        ap_cost,
                    ) {
                        return false;
                    }
                }
                if Self::check_entry(state, claims, pending, mover_id, mover_final_dest, &step)
                    != EntryCheck::Free
                {
                    return false;
                }
            }
            EntryCheck::Blocked => return false,
        }

        let from = state.get_unit(mover_id).unwrap().pos;
        if from != step {
            if let Some(u) = state.get_unit_mut(mover_id) {
                u.pos = step;
                u.spend_ap(ap_cost);
            }
            if step == mover_final_dest.unwrap_or(step) {
                if claims.get(&step) == Some(&mover_id) {
                    claims.remove(&step);
                }
                pending.remove(&mover_id);
            } else {
                claims.remove(&from);
                claims.insert(step, mover_id);
            }
        }
        true
    }

    fn process_unit(
        state: &mut GameState,
        snapshot: &GameState,
        order: &UnitOrder,
        all_orders: &TurnOrders,
        claims: &mut HashMap<HexCoord, UnitId>,
        pending: &mut HashSet<UnitId>,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        let unit = match state.get_unit(order.unit_id) {
            Some(u) if u.is_alive() => u,
            _ => return events,
        };

        let unit_id = order.unit_id;
        let start_pos = unit.pos;
        let planned_unit = match snapshot.get_unit(unit_id) {
            Some(u) if u.is_alive() => u,
            _ => return events,
        };
        let ap_budget = planned_unit.ap.max(unit.ap);

        // Phase 1: Movement execution
        if let Some(move_target) = order.move_target {
            if move_target != start_pos {
                if let Some(path) = state.map.find_path(planned_unit.pos, move_target) {
                    let path_cost = (path.len() - 1) as u32;

                    if path_cost <= ap_budget {
                        let mut walked: Vec<HexCoord> = vec![planned_unit.pos];
                        for &step in &path[1..] {
                            if Self::enter_step(
                                state,
                                all_orders,
                                claims,
                                pending,
                                unit_id,
                                Some(move_target),
                                step,
                                1,
                            ) {
                                walked.push(step);
                            } else {
                                break;
                            }
                        }

                        let final_pos = state.get_unit(unit_id).unwrap().pos;
                        if final_pos != start_pos {
                            let ap_spent = (walked.len() - 1) as u32;
                            events.push(GameEvent::UnitMoved {
                                unit_id,
                                from: start_pos,
                                to: final_pos,
                                path: walked,
                                ap_spent,
                            });
                        }
                    }
                }
            }
        }

        // Phase 2: Action execution
        match &order.action {
            Action::Wait => {
                events.push(GameEvent::UnitWaited { unit_id });
            }
            Action::Attack { target_id } => {
                let attack_events = Self::process_attack(state, unit_id, *target_id);
                events.extend(attack_events);
            }
        }

        events
    }

    fn process_attack(
        state: &mut GameState,
        attacker_id: UnitId,
        target_id: UnitId,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        let (attacker, target) = match (state.get_unit(attacker_id), state.get_unit(target_id)) {
            (Some(a), Some(t)) => (a, t),
            _ => return events,
        };

        if !attacker.is_alive() || !target.is_alive() {
            return events;
        }

        if attacker.team == target.team || attacker_id == target_id {
            return events;
        }

        let distance = attacker.pos.distance(&target.pos);
        if distance > attacker.attack_range {
            return events;
        }

        if !attacker.can_afford(ATTACK_AP_COST) {
            return events;
        }

        let damage = attacker.attack_damage;

        if let Some(attacker) = state.get_unit_mut(attacker_id) {
            attacker.spend_ap(ATTACK_AP_COST);
        }

        let target_hp_remaining;
        if let Some(target) = state.get_unit_mut(target_id) {
            target.hp = target.hp.saturating_sub(damage);
            target_hp_remaining = target.hp;
        } else {
            return events;
        }

        events.push(GameEvent::UnitAttacked {
            attacker_id,
            target_id,
            damage,
            target_hp_remaining,
        });

        // Immediate death handling
        if target_hp_remaining == 0 {
            state.units.remove(&target_id);
            events.push(GameEvent::UnitDied {
                unit_id: target_id,
                killed_by: attacker_id,
            });
        }

        events
    }
}
```

---

### 7. Engine Facade (`crates/core/src/lib.rs`)

The `GameEngine` encapsulates state and order staging, exposing clean JSON and primitive APIs for the WASM bridge:

```rust
pub mod ai;
pub mod event;
pub mod hex;
pub mod orders;
pub mod state;
pub mod turn;
pub mod unit;

use crate::hex::{HexCoord, HexMap};
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::{GameState, Phase};
use crate::turn::TurnProcessor;
use crate::unit::{Unit, UnitId};
use std::collections::HashSet;

pub const PLAYER_TEAM: u8 = 0;
pub const ENEMY_TEAM: u8 = 1;

pub struct GameEngine {
    pub(crate) state: GameState,
    pub(crate) pending_orders: TurnOrders,
}

impl GameEngine {
    pub fn new() -> Self {
        let mut map = HexMap::new(5); // radius 5 = 91 hexes

        // Symmetrical center obstacles
        map.obstacles.insert(HexCoord::new(0, 0));
        map.obstacles.insert(HexCoord::new(1, -1));
        map.obstacles.insert(HexCoord::new(-1, 1));
        map.obstacles.insert(HexCoord::new(2, -2));
        map.obstacles.insert(HexCoord::new(-2, 2));

        let mut state = GameState::new(map);

        // Team 0 (Player) - West flank
        state.add_unit(Unit::new_hero(1, PLAYER_TEAM, HexCoord::new(-4, 0), 3));
        state.add_unit(Unit::new_hero(2, PLAYER_TEAM, HexCoord::new(-4, 1), 2));
        state.add_unit(Unit::new_hero(3, PLAYER_TEAM, HexCoord::new(-4, -1), 1));

        // Team 1 (AI) - East flank
        state.add_unit(Unit::new_hero(4, ENEMY_TEAM, HexCoord::new(4, 0), 3));
        state.add_unit(Unit::new_hero(5, ENEMY_TEAM, HexCoord::new(4, 1), 2));
        state.add_unit(Unit::new_hero(6, ENEMY_TEAM, HexCoord::new(4, -1), 1));

        Self {
            state,
            pending_orders: TurnOrders::new(),
        }
    }

    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap()
    }

    pub fn get_map_hexes(&self) -> String {
        let hexes: Vec<(i32, i32)> = self.state.map.all_hexes().iter().map(|h| (h.q, h.r)).collect();
        serde_json::to_string(&hexes).unwrap()
    }

    pub fn get_obstacles(&self) -> String {
        let obstacles: Vec<(i32, i32)> = self.state.map.obstacles.iter().map(|h| (h.q, h.r)).collect();
        serde_json::to_string(&obstacles).unwrap()
    }

    pub fn get_move_targets(&self, unit_id: UnitId) -> String {
        let mut targets: Vec<(i32, i32, u32)> = if let Some(unit) = self.state.get_unit(unit_id) {
            if !unit.is_alive() {
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
        targets.sort();
        serde_json::to_string(&targets).unwrap()
    }

    pub fn get_attack_targets(&self, unit_id: UnitId, from_q: i32, from_r: i32) -> String {
        let mut targets: Vec<UnitId> = if let Some(unit) = self.state.get_unit(unit_id) {
            if !unit.is_alive() {
                vec![]
            } else {
                let from_pos = HexCoord::new(from_q, from_r);
                let in_range = self.state.map.hexes_in_range(from_pos, unit.attack_range);
                let range_set: HashSet<HexCoord> = in_range.into_iter().collect();

                self.state
                    .enemy_units(unit.team)
                    .iter()
                    .filter(|e| range_set.contains(&e.pos))
                    .map(|e| e.id)
                    .collect()
            }
        } else {
            vec![]
        };
        targets.sort_unstable();
        serde_json::to_string(&targets).unwrap()
    }

    pub fn set_move_order_with(
        &mut self,
        unit_id: UnitId,
        q: i32,
        r: i32,
        reserve_attack_ap: bool,
    ) -> bool {
        let target = HexCoord::new(q, r);

        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == PLAYER_TEAM && self.state.phase == Phase::Planning {
                let has_planned_attack = self
                    .pending_orders
                    .get_order(unit_id)
                    .map(|o| matches!(o.action, Action::Attack { .. }))
                    .unwrap_or(false);

                let move_budget = if has_planned_attack || reserve_attack_ap {
                    unit.ap.saturating_sub(1)
                } else {
                    unit.ap
                };

                let mut occupied: HashSet<HexCoord> = self
                    .state
                    .units
                    .values()
                    .filter(|u| u.is_alive() && u.id != unit_id)
                    .map(|u| u.pos)
                    .collect();
                occupied.remove(&target);

                let reachable = self.state.map.reachable_hexes(unit.pos, move_budget, &occupied);

                if reachable.contains_key(&target) || target == unit.pos {
                    if let Some(order) = self.pending_orders.orders.iter_mut().find(|o| o.unit_id == unit_id) {
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

    pub fn set_move_order(&mut self, unit_id: UnitId, q: i32, r: i32) -> bool {
        self.set_move_order_with(unit_id, q, r, false)
    }

    pub fn set_attack_order(&mut self, unit_id: UnitId, target_id: UnitId) -> bool {
        let (from_pos, own_team) = match self.state.get_unit(unit_id) {
            Some(u) if u.is_alive() && u.team == PLAYER_TEAM => (u.pos, u.team),
            _ => return false,
        };

        let target_pos = match self.state.get_unit(target_id) {
            Some(t) if t.team != own_team && t.is_alive() => t.pos,
            _ => return false,
        };

        let unit = self.state.get_unit(unit_id).unwrap();
        let existing_move = self.pending_orders.get_order(unit_id).and_then(|o| o.move_target);
        let stand_and_attack = from_pos.distance(&target_pos) <= unit.attack_range && unit.can_afford(1);

        let attack_after_move = match existing_move {
            Some(hex) if hex != from_pos => {
                let mut occupied = self.state.occupied_hexes();
                occupied.remove(&unit.pos);
                let reachable = self.state.map.reachable_hexes(from_pos, unit.ap.saturating_sub(1), &occupied);
                reachable.contains_key(&hex) && hex.distance(&target_pos) <= unit.attack_range
            }
            _ => stand_and_attack,
        };

        if !(stand_and_attack || attack_after_move) {
            return false;
        }

        if let Some(order) = self.pending_orders.orders.iter_mut().find(|o| o.unit_id == unit_id) {
            order.action = Action::Attack { target_id };
        } else {
            self.pending_orders.add_order(UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Attack { target_id },
            });
        }
        true
    }

    pub fn set_wait_order(&mut self, unit_id: UnitId) -> bool {
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == PLAYER_TEAM {
                if let Some(order) = self.pending_orders.orders.iter_mut().find(|o| o.unit_id == unit_id) {
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
        let living_players = self.state.team_units(PLAYER_TEAM);
        !living_players.is_empty()
            && living_players
                .iter()
                .all(|u| self.pending_orders.get_order(u.id).is_some())
    }

    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::take(&mut self.pending_orders);
        let events = TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    pub fn restart(&mut self) {
        *self = GameEngine::new();
    }
}
```

---

## WebAssembly Interop Layer (`crates/wasm/src/lib.rs`)

`wasm-bindgen` maps Rust `u64` parameters to JavaScript `BigInt`. The WASM wrapper exposes all engine methods cleanly:

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

    pub fn restart(&mut self) {
        self.engine.restart();
    }
}

impl Default for WasmGame {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## Client Architecture (TypeScript + PixiJS v8)

### 1. FFI Bridge & Typed Decoders (`web/src/game/bridge.ts`)

> [!IMPORTANT]
> Because Rust `u64` crosses the WASM boundary as `BigInt`, all functions taking `unitId` or `targetId` must pass `BigInt(...)` to `WasmGame`. The TypeScript bridge handles this conversion so the rest of the frontend can work with standard numbers.

```typescript
import init, { WasmGame } from '../wasm/pkg/hexabellum_wasm';

let game: WasmGame | null = null;

export interface HexCoord {
  q: number;
  r: number;
}

export interface UnitData {
  id: number;
  kind: 'Hero' | 'Minion' | 'Tower' | 'Neutral';
  team: number;
  pos: HexCoord;
  hp: number;
  max_hp: number;
  ap: number;
  max_ap: number;
  initiative: number;
  attack_damage: number;
  attack_range: number;
}

export interface GameState {
  round: number;
  phase: 'Planning' | 'Resolution' | 'MatchEnd';
  units: Record<number, UnitData>;
  winner: number | null;
  next_unit_id: number;
}

export interface MoveTarget {
  q: number;
  r: number;
  cost: number;
}

export interface PendingOrder {
  unit_id: number;
  move_target: HexCoord | null;
  action: { Wait: true } | { Attack: { target_id: number } };
}

export interface TurnOrders {
  orders: PendingOrder[];
}

export interface GameEvent {
  type: string;
  [key: string]: unknown;
}

export async function initGame(): Promise<void> {
  await init();
  game = new WasmGame();
}

export function getState(): GameState {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_state());
}

export function getMapHexes(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_map_hexes()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getObstacles(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_obstacles()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getMoveTargets(unitId: number): MoveTarget[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_move_targets(BigInt(unitId))).map(
    ([q, r, cost]: [number, number, number]) => ({ q, r, cost })
  );
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

export function getPendingOrders(): TurnOrders {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_pending_orders());
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

Compatible with PixiJS v8, the renderer features layered containers, pointy-topped polygon calculations (`angle - Math.PI / 6`), AP dot indicators, health bars, attack target reticles, and movement path vectors:

```typescript
import * as PIXI from 'pixi.js';
import { HexCoord, UnitData, GameState, MoveTarget } from './bridge';

export const HEX_SIZE = 32;

export class HexRenderer {
  private app: PIXI.Application;
  private hexLayer = new PIXI.Container();
  private overlayLayer = new PIXI.Container();
  private pathLayer = new PIXI.Container();
  private unitLayer = new PIXI.Container();

  private unitSprites = new Map<number, PIXI.Container>();

  constructor(app: PIXI.Application) {
    this.app = app;
    this.app.stage.addChild(this.hexLayer);
    this.app.stage.addChild(this.overlayLayer);
    this.app.stage.addChild(this.pathLayer);
    this.app.stage.addChild(this.unitLayer);
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
        g.fill({ color: 0x121b2d });
        g.stroke({ color: 0x1f3453, width: 1 });
      }

      this.hexLayer.addChild(g);
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

      // Outer glow & unit circle
      g.circle(0, 0, HEX_SIZE * 0.52);
      g.fill({ color: baseColor });
      g.stroke({ color: 0xffffff, width: 2 });

      // HP Bar (Above unit)
      const hpWidth = HEX_SIZE * 0.9;
      const hpHeight = 5;
      const hpX = -hpWidth / 2;
      const hpY = -HEX_SIZE * 0.75;
      const hpRatio = Math.max(0, Math.min(1, unit.hp / unit.max_hp));

      g.rect(hpX, hpY, hpWidth, hpHeight);
      g.fill({ color: 0x111118 });
      g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
      g.fill({ color: hpRatio > 0.5 ? 0x00e676 : hpRatio > 0.25 ? 0xffea00 : 0xff1744 });

      // AP Pip Dots (Below unit)
      const apY = HEX_SIZE * 0.72;
      for (let i = 0; i < unit.max_ap; i++) {
        const apX = (i - (unit.max_ap - 1) / 2) * 10;
        g.circle(apX, apY, 3);
        g.fill({ color: i < unit.ap ? 0xffd600 : 0x424242 });
      }

      // Initiative badge
      const initText = new PIXI.Text({
        text: `⚡${unit.initiative}`,
        style: {
          fontSize: 10,
          fill: 0xffffff,
          fontFamily: 'Outfit, Inter, sans-serif',
          fontWeight: 'bold',
        },
      });
      initText.anchor.set(0.5);
      initText.y = 0;

      container.addChild(g);
      container.addChild(initText);
      this.unitLayer.addChild(container);
      this.unitSprites.set(id, container);
    }
  }

  drawMoveTargets(targets: MoveTarget[]): void {
    this.overlayLayer.removeChildren();

    for (const target of targets) {
      const { x, y } = this.hexToPixel(target.q, target.r);
      const g = new PIXI.Graphics();

      g.circle(x, y, HEX_SIZE * 0.45);
      g.fill({ color: 0x00e676, alpha: 0.35 });
      g.stroke({ color: 0x00e676, width: 2, alpha: 0.8 });

      const costText = new PIXI.Text({
        text: `${target.cost} AP`,
        style: {
          fontSize: 11,
          fill: 0xffffff,
          fontFamily: 'Outfit, Inter, sans-serif',
          fontWeight: 'bold',
        },
      });
      costText.anchor.set(0.5);
      costText.x = x;
      costText.y = y;

      this.overlayLayer.addChild(g);
      this.overlayLayer.addChild(costText);
    }
  }

  drawAttackTargets(targetIds: number[], state: GameState): void {
    for (const targetId of targetIds) {
      const unit = state.units[targetId];
      if (!unit) continue;

      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);
      const g = new PIXI.Graphics();

      // Pulsing crosshair ring
      g.circle(x, y, HEX_SIZE * 0.65);
      g.stroke({ color: 0xff1744, width: 3, alpha: 0.9 });

      this.overlayLayer.addChild(g);
    }
  }

  drawPath(path: HexCoord[]): void {
    this.pathLayer.removeChildren();
    if (path.length < 2) return;

    const g = new PIXI.Graphics();
    const start = this.hexToPixel(path[0].q, path[0].r);
    g.moveTo(start.x, start.y);

    for (let i = 1; i < path.length; i++) {
      const pt = this.hexToPixel(path[i].q, path[i].r);
      g.lineTo(pt.x, pt.y);
    }
    g.stroke({ color: 0xffd600, width: 3, alpha: 0.8 });
    this.pathLayer.addChild(g);
  }

  highlightUnit(unitId: number | null): void {
    for (const [id, container] of this.unitSprites) {
      const g = container.getChildAt(0) as PIXI.Graphics;
      if (id === unitId) {
        g.stroke({ color: 0xffd600, width: 3.5 });
      } else {
        g.stroke({ color: 0xffffff, width: 2 });
      }
    }
  }

  clearOverlays(): void {
    this.overlayLayer.removeChildren();
    this.pathLayer.removeChildren();
  }

  getApp(): PIXI.Application {
    return this.app;
  }
}
```

---

### 3. Interactive Input Controller (`web/src/game/input.ts`)

```typescript
import * as PIXI from 'pixi.js';
import { HexRenderer, HEX_SIZE } from './renderer';
import {
  getState,
  getMoveTargets,
  getAttackTargets,
  setMoveOrder,
  setAttackOrder,
  setWaitOrder,
  allUnitsOrdered,
  endTurn,
  restart,
  getMapHexes,
  getObstacles,
  HexCoord,
  MoveTarget,
  GameState,
} from './bridge';

type InputMode = 'idle' | 'choosingMove' | 'choosingAction';

export class InputHandler {
  private renderer: HexRenderer;
  private mode: InputMode = 'idle';
  private selectedUnitId: number | null = null;
  private moveTargets: MoveTarget[] = [];
  private plannedMove: HexCoord | null = null;
  private attackTargets: number[] = [];

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
    this.setupListeners();
  }

  private setupListeners(): void {
    const stage = this.renderer.getApp().stage;
    stage.eventMode = 'static';
    stage.hitArea = this.renderer.getApp().screen;

    stage.on('pointerdown', (event: PIXI.FederatedPointerEvent) => {
      this.handleClick(event.global.x, event.global.y);
    });
  }

  private handleClick(screenX: number, screenY: number): void {
    const state = getState();
    if (state.phase === 'MatchEnd') return;

    const hex = this.pixelToHex(screenX, screenY);
    if (!hex) return;

    switch (this.mode) {
      case 'idle':
        this.handleUnitSelection(hex, state);
        break;
      case 'choosingMove':
        this.handleMoveSelection(hex, state);
        break;
      case 'choosingAction':
        this.handleActionSelection(hex, state);
        break;
    }
  }

  private handleUnitSelection(hex: HexCoord, state: GameState): void {
    for (const [idStr, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team === 0) {
        const id = Number(idStr);
        this.selectedUnitId = id;
        this.mode = 'choosingMove';
        this.plannedMove = null;

        this.moveTargets = getMoveTargets(id);
        this.renderer.clearOverlays();
        this.renderer.highlightUnit(id);
        this.renderer.drawMoveTargets(this.moveTargets);

        this.updateInspector(unit);
        this.updateStatus(`Hero #${id} selected. Click a green hex to move or click self to stay.`);
        return;
      }
    }

    // Clicked empty ground
    this.selectedUnitId = null;
    this.mode = 'idle';
    this.renderer.clearOverlays();
    this.renderer.highlightUnit(null);
    this.updateInspector(null);
    this.updateStatus('Select a blue hero to issue orders.');
  }

  private handleMoveSelection(hex: HexCoord, state: GameState): void {
    if (this.selectedUnitId === null) return;
    const unit = state.units[this.selectedUnitId];
    if (!unit) return;

    const target = this.moveTargets.find(t => t.q === hex.q && t.r === hex.r);
    const clickedSelf = hex.q === unit.pos.q && hex.r === unit.pos.r;

    if (target || clickedSelf) {
      const dest = clickedSelf ? unit.pos : hex;
      setMoveOrder(this.selectedUnitId, dest.q, dest.r);
      this.plannedMove = dest;
      this.mode = 'choosingAction';

      // Draw planned move path
      if (!clickedSelf) {
        this.renderer.drawPath([unit.pos, dest]);
      }

      // Show attack targets from new destination
      this.attackTargets = getAttackTargets(this.selectedUnitId, dest.q, dest.r);
      this.renderer.drawAttackTargets(this.attackTargets, state);

      if (this.attackTargets.length > 0) {
        this.updateStatus(`Click a red reticle to Attack, or click elsewhere to Wait.`);
      } else {
        this.updateStatus(`No enemies in range. Click anywhere to confirm Wait action.`);
      }
    } else {
      // Re-select another unit or cancel
      this.handleUnitSelection(hex, state);
    }
  }

  private handleActionSelection(hex: HexCoord, state: GameState): void {
    if (this.selectedUnitId === null) return;

    // Check if clicked an enemy in range
    const targetUnit = Object.values(state.units).find(
      u => u.pos.q === hex.q && u.pos.r === hex.r && u.team === 1
    );

    if (targetUnit && this.attackTargets.includes(targetUnit.id)) {
      setAttackOrder(this.selectedUnitId, targetUnit.id);
    } else {
      setWaitOrder(this.selectedUnitId);
    }

    this.finishUnitOrders();
  }

  private finishUnitOrders(): void {
    this.selectedUnitId = null;
    this.mode = 'idle';
    this.plannedMove = null;
    this.moveTargets = [];
    this.attackTargets = [];

    this.renderer.clearOverlays();
    this.renderer.highlightUnit(null);
    this.updateInspector(null);

    if (allUnitsOrdered()) {
      this.updateStatus('All heroes have orders! Click "End Turn" to resolve.');
      const endBtn = document.getElementById('end-turn') as HTMLButtonElement;
      if (endBtn) endBtn.classList.add('ready');
    } else {
      this.updateStatus('Order confirmed. Select another hero.');
    }
  }

  public endTurn(): void {
    const events = endTurn();
    const state = getState();

    this.renderer.clearOverlays();
    this.renderer.drawUnits(state);

    const endBtn = document.getElementById('end-turn') as HTMLButtonElement;
    if (endBtn) endBtn.classList.remove('ready');

    if (state.phase === 'MatchEnd') {
      const winner = state.winner;
      const overlay = document.getElementById('game-over-modal') as HTMLElement;
      const title = document.getElementById('game-over-title') as HTMLElement;
      if (overlay && title) {
        overlay.style.display = 'flex';
        title.textContent = winner === 0 ? 'VICTORY' : 'DEFEAT';
        title.className = winner === 0 ? 'victory' : 'defeat';
      }
      this.updateStatus(`Match concluded! ${winner === 0 ? 'Player wins!' : 'AI wins!'}`);
    } else {
      this.updateStatus(`Round ${state.round} — Planning Phase. Select your heroes.`);
    }

    const roundEl = document.getElementById('round-counter');
    if (roundEl) roundEl.textContent = `Round ${state.round}`;
  }

  public restartGame(): void {
    restart();
    const state = getState();
    const hexes = getMapHexes();
    const obstacles = getObstacles();

    this.renderer.drawMap(hexes, obstacles);
    this.renderer.drawUnits(state);
    this.renderer.clearOverlays();

    const overlay = document.getElementById('game-over-modal') as HTMLElement;
    if (overlay) overlay.style.display = 'none';

    const roundEl = document.getElementById('round-counter');
    if (roundEl) roundEl.textContent = `Round 0`;

    this.updateStatus('New match started. Select a blue hero.');
  }

  private pixelToHex(screenX: number, screenY: number): HexCoord {
    const app = this.renderer.getApp();
    const cx = screenX - app.screen.width / 2;
    const cy = screenY - app.screen.height / 2;

    const q = ((Math.sqrt(3) / 3) * cx - (1 / 3) * cy) / HEX_SIZE;
    const r = ((2 / 3) * cy) / HEX_SIZE;
    return this.hexRound(q, r);
  }

  private hexRound(q: number, r: number): HexCoord {
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

  private updateStatus(text: string): void {
    const el = document.getElementById('status-message');
    if (el) el.textContent = text;
  }

  private updateInspector(unit: any): void {
    const panel = document.getElementById('unit-inspector');
    if (!panel) return;

    if (!unit) {
      panel.style.display = 'none';
      return;
    }

    panel.style.display = 'block';
    (document.getElementById('insp-name') as HTMLElement).textContent = `Hero #${unit.id}`;
    (document.getElementById('insp-team') as HTMLElement).textContent = unit.team === 0 ? 'Blue (Player)' : 'Red (Enemy)';
    (document.getElementById('insp-hp') as HTMLElement).textContent = `${unit.hp} / ${unit.max_hp}`;
    (document.getElementById('insp-ap') as HTMLElement).textContent = `${unit.ap} / ${unit.max_ap}`;
    (document.getElementById('insp-init') as HTMLElement).textContent = `${unit.initiative}`;
    (document.getElementById('insp-dmg') as HTMLElement).textContent = `${unit.attack_damage}`;
    (document.getElementById('insp-range') as HTMLElement).textContent = `${unit.attack_range}`;
  }
}
```

---

### 4. Client Entrypoint & Tactical HUD (`web/src/main.ts` & `web/index.html`)

#### `web/src/main.ts`
```typescript
import * as PIXI from 'pixi.js';
import { initGame, getState, getMapHexes, getObstacles } from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';

async function bootstrap() {
  await initGame();

  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  const app = new PIXI.Application();
  await app.init({
    canvas,
    resizeTo: window,
    backgroundColor: 0x0a0e17,
    antialias: true,
  });

  const renderer = new HexRenderer(app);
  const input = new InputHandler(renderer);

  // Initial draw
  const hexes = getMapHexes();
  const obstacles = getObstacles();
  const state = getState();

  renderer.drawMap(hexes, obstacles);
  renderer.drawUnits(state);

  // Wire UI buttons
  const endTurnBtn = document.getElementById('end-turn') as HTMLButtonElement;
  endTurnBtn.addEventListener('click', () => input.endTurn());

  const restartBtn = document.getElementById('btn-restart-modal') as HTMLButtonElement;
  restartBtn.addEventListener('click', () => input.restartGame());

  const resetBtn = document.getElementById('btn-quick-reset') as HTMLButtonElement;
  resetBtn.addEventListener('click', () => input.restartGame());

  console.log('Hexabellum Phase 1 Initialized');
}

bootstrap().catch(console.error);
```

#### `web/index.html`
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Hexabellum — Phase 1 Tactical Slice</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;600;700&family=JetBrains+Mono:wght@400;600&display=swap" rel="stylesheet">
  <style>
    :root {
      --bg: #0a0e17;
      --panel-bg: rgba(16, 24, 39, 0.85);
      --panel-border: rgba(0, 210, 255, 0.25);
      --primary: #00d2ff;
      --accent: #00e676;
      --danger: #ff3366;
      --text: #e2e8f0;
      --muted: #94a3b8;
    }
    * { margin: 0; padding: 0; box-sizing: border-box; user-select: none; }
    body {
      background: var(--bg);
      color: var(--text);
      font-family: 'Outfit', sans-serif;
      overflow: hidden;
      width: 100vw;
      height: 100vh;
    }
    #game-canvas {
      display: block;
      width: 100vw;
      height: 100vh;
    }
    /* Top Left Status HUD */
    #hud {
      position: fixed;
      top: 20px;
      left: 20px;
      z-index: 10;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .panel {
      background: var(--panel-bg);
      backdrop-filter: blur(12px);
      border: 1px solid var(--panel-border);
      border-radius: 12px;
      padding: 16px 20px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.5);
    }
    #round-counter {
      font-size: 22px;
      font-weight: 700;
      letter-spacing: 0.5px;
      color: var(--primary);
      text-transform: uppercase;
    }
    #status-message {
      font-size: 14px;
      color: var(--muted);
      max-width: 320px;
      line-height: 1.4;
      margin-top: 4px;
    }
    .hud-controls {
      display: flex;
      gap: 10px;
      margin-top: 8px;
    }
    .btn {
      padding: 10px 20px;
      font-family: 'Outfit', sans-serif;
      font-size: 14px;
      font-weight: 600;
      border-radius: 8px;
      cursor: pointer;
      transition: all 0.2s ease;
      outline: none;
    }
    .btn-primary {
      background: rgba(0, 210, 255, 0.15);
      border: 1.5px solid var(--primary);
      color: var(--primary);
    }
    .btn-primary:hover {
      background: var(--primary);
      color: var(--bg);
    }
    .btn-primary.ready {
      background: var(--accent);
      border-color: var(--accent);
      color: #000;
      animation: pulse 1.5s infinite;
    }
    .btn-secondary {
      background: rgba(148, 163, 184, 0.1);
      border: 1px solid var(--muted);
      color: var(--muted);
    }
    .btn-secondary:hover {
      background: var(--muted);
      color: var(--bg);
    }
    /* Right Unit Inspector */
    #unit-inspector {
      position: fixed;
      top: 20px;
      right: 20px;
      z-index: 10;
      min-width: 220px;
      display: none;
    }
    #unit-inspector h3 {
      font-size: 16px;
      color: var(--primary);
      margin-bottom: 12px;
      border-bottom: 1px solid var(--panel-border);
      padding-bottom: 6px;
    }
    .stat-row {
      display: flex;
      justify-content: space-between;
      font-size: 13px;
      margin: 6px 0;
      font-family: 'JetBrains Mono', monospace;
    }
    .stat-label { color: var(--muted); }
    .stat-val { font-weight: 600; }
    /* Bottom Guide */
    #guide {
      position: fixed;
      bottom: 20px;
      left: 20px;
      z-index: 10;
      font-size: 12px;
      color: var(--muted);
      background: rgba(10, 14, 23, 0.7);
      padding: 10px 16px;
      border-radius: 8px;
      border: 1px solid rgba(255,255,255,0.05);
    }
    /* Victory / Defeat Modal */
    #game-over-modal {
      position: fixed;
      inset: 0;
      background: rgba(0,0,0,0.8);
      backdrop-filter: blur(10px);
      z-index: 100;
      display: none;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 20px;
    }
    #game-over-title {
      font-size: 54px;
      font-weight: 700;
      letter-spacing: 2px;
    }
    #game-over-title.victory { color: var(--accent); text-shadow: 0 0 30px rgba(0, 230, 118, 0.4); }
    #game-over-title.defeat { color: var(--danger); text-shadow: 0 0 30px rgba(255, 51, 102, 0.4); }
    @keyframes pulse {
      0% { box-shadow: 0 0 0 0 rgba(0, 230, 118, 0.6); }
      70% { box-shadow: 0 0 0 10px rgba(0, 230, 118, 0); }
      100% { box-shadow: 0 0 0 0 rgba(0, 230, 118, 0); }
    }
  </style>
</head>
<body>
  <div id="hud" class="panel">
    <div id="round-counter">Round 0</div>
    <div id="status-message">Select a blue hero to issue orders.</div>
    <div class="hud-controls">
      <button id="end-turn" class="btn btn-primary">End Turn</button>
      <button id="btn-quick-reset" class="btn btn-secondary">Restart</button>
    </div>
  </div>

  <div id="unit-inspector" class="panel">
    <h3 id="insp-name">Hero Info</h3>
    <div class="stat-row"><span class="stat-label">Team:</span><span class="stat-val" id="insp-team">-</span></div>
    <div class="stat-row"><span class="stat-label">Health:</span><span class="stat-val" id="insp-hp">-</span></div>
    <div class="stat-row"><span class="stat-label">AP:</span><span class="stat-val" id="insp-ap">-</span></div>
    <div class="stat-row"><span class="stat-label">Initiative:</span><span class="stat-val" id="insp-init">-</span></div>
    <div class="stat-row"><span class="stat-label">Damage:</span><span class="stat-val" id="insp-dmg">-</span></div>
    <div class="stat-row"><span class="stat-label">Range:</span><span class="stat-val" id="insp-range">-</span></div>
  </div>

  <div id="guide">
    <strong>Controls:</strong> Click Blue Hero → Click Green Hex to Move (or self to stay) → Click Red Crosshair to Attack (or click ground to Wait) → Click End Turn.
  </div>

  <div id="game-over-modal">
    <h1 id="game-over-title">VICTORY</h1>
    <button id="btn-restart-modal" class="btn btn-primary" style="padding: 14px 32px; font-size: 18px;">Play Again</button>
  </div>

  <canvas id="game-canvas"></canvas>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

---

## Verification & Determinism Test Strategy

### Test Catalog (`cargo test`)

The engine test suite validates every core mechanic with unit and simulation tests:

| Test Name | Module | What it Validates |
|---|---|---|
| `test_engine_new_setup_3v3` | `lib.rs` | 3v3 heroes, positions, AP, and HP initialized correctly |
| `test_move_targets_respect_ap` | `lib.rs` | Reachable hexes never exceed AP budget and respect static obstacles |
| `test_attack_order_requires_range_or_planned_move` | `lib.rs` | Attack order validation checks reachability from start or planned move |
| `test_all_units_ordered_and_end_turn` | `lib.rs` | Turn ready check passes only when all living player units have orders |
| `test_movement_along_path_costs_ap` | `turn.rs` | Movement spends 1 AP per step; final position and events match path |
| `test_move_beyond_ap_budget_rejected` | `turn.rs` | Moves exceeding AP budget are rejected |
| `test_attack_in_range_deals_damage_and_kills` | `turn.rs` | Attack applies 20 damage, deducts 1 AP; fatal hit removes target and emits `UnitDied` |
| `test_attack_out_of_range_fails` | `turn.rs` | Attacks out of reach fail and do not spend AP |
| `test_initiative_order_determines_who_acts_first` | `turn.rs` | Higher initiative unit acts first; lower initiative unit reflects updated state |
| `test_dead_unit_cannot_complete_attack_after_move` | `turn.rs` | Unit killed by earlier initiative has move/attack orders cancelled immediately |
| `test_ai_moves_toward_and_attacks_player` | `turn.rs` | Autonomous AI navigates toward closest player hero and attacks when in range |
| `test_win_condition_and_round_increment` | `turn.rs` | Victory detected when enemy team wiped; round counter increments on draw/continue |
| `test_full_battle_runs_to_completion_deterministically` | `lib.rs` | Multi-round simulation to match end produces identical event hashes across runs |

---

## Acceptance Criteria

| # | Criterion | Verification Method | Status |
|---|---|---|:---:|
| 1 | 3 blue player heroes and 3 red AI heroes spawn on opposite flanks of the arena | Visual inspection + `test_engine_new_setup_3v3` | [x] |
| 2 | Clicking a player hero highlights reachable hexes annotated with AP costs | In-browser click + visual overlay | [x] |
| 3 | Clicking a reachable hex stages movement and displays the planned path | In-browser click + path line overlay | [x] |
| 4 | After staging movement, enemies within attack range display attack reticles | In-browser click + attack target query | [x] |
| 5 | Clicking an enemy reticle stages an attack; clicking elsewhere stages a wait | In-browser click + order inspection | [x] |
| 6 | Player can stage orders for all 3 heroes before ending the turn | UI inspection + `all_units_ordered()` | [x] |
| 7 | "End Turn" button transitions to "ready" when all units are ordered | CSS transition inspection | [x] |
| 8 | Resolution executes units in descending initiative order | Unit test `test_initiative_order_determines_who_acts_first` | [x] |
| 9 | Casualties from higher-initiative attacks have their orders revoked immediately | Unit test `test_dead_unit_cannot_complete_attack_after_move` | [x] |
| 10 | AI generates valid moves and attacks against player heroes each round | Integration test `test_ai_moves_toward_and_attacks_player` | [x] |
| 11 | Units on intersecting paths resolve cooperatively without tile stacking | Simulation test `test_full_battle_runs_to_completion_deterministically` | [x] |
| 12 | Eliminating all enemy heroes displays the Victory modal | In-game play + `test_win_condition_and_round_increment` | [x] |
| 13 | Losing all player heroes displays the Defeat modal | In-game play + `test_win_condition_and_round_increment` | [x] |
| 14 | Restart button resets state cleanly without browser reload | UI click + `restart()` FFI call | [x] |

---

## What is NOT in Phase 1

- ❌ Minion spawning and push lanes *(Phase 2)*
- ❌ Defensive and spawner towers *(Phase 2)*
- ❌ Fog of War vision system *(Phase 2)*
- ❌ Turn timer countdown with AI fallback *(Phase 2)*
- ❌ Tweened visual animations and floating damage numbers *(Phase 2)*
- ❌ Hero abilities, mana, or spells *(Phase 4)*
- ❌ Multiplayer WebSockets or dedicated server *(Phase 3)*

---

## Phase 2 Preview

Once Phase 1 tactical combat is established, Phase 2 turns the battle into a MOBA:
- **Minion Waves**: Spawner towers generate automated melee minions every 3 rounds that push down lanes.
- **Defensive Towers**: Stationary structures that guard lanes and auto-attack enemies in range.
- **Fog of War**: Radius-based vision hiding unrevealed enemy units.
- **Turn Timer**: 30-second planning timer with automated AI order backfill upon expiration.
- **Visual Polish**: PixiJS tweened movement paths, attack projectile flashes, and floating combat text.
