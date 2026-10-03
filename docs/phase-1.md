# Phase 1 Technical Spec — Combat & AI Vertical Slice

## Phase 1 Goal

> **A playable 3v3 tactical battle where you command three heroes, move them using AP, attack enemies, fight an AI opponent, and win or lose.**

Phase 0 proved the pipeline. Phase 1 makes it a **game**.

### What Phase 1 Adds

| System | Description |
|--------|-------------|
| Pathfinding | A* on hex grid, respecting obstacles and units |
| AP System | Movement costs AP, attacks cost AP |
| Combat | Attack action, damage, HP, death |
| Initiative | Units act in initiative order during resolution |
| AI | Enemy heroes move toward you and attack |
| Multiple Units | 3v3 instead of 1v1 |
| Win/Loss | All enemy heroes dead = win; all your heroes dead = lose |
| Turn Timer | Optional configurable countdown |

### Definition of Done

- [ ] 3 heroes per team rendered on the board
- [ ] Selecting a hero shows reachable hexes (based on AP)
- [ ] Clicking a reachable hex plans movement (path shown)
- [ ] After moving, attackable enemies highlighted
- [ ] Clicking an enemy plans an attack
- [ ] "Wait" action available
- [ ] All 3 heroes can receive orders before ending turn
- [ ] AI generates orders for enemy team
- [ ] Resolution processes all units in initiative order
- [ ] Damage applied, units die, removed from board
- [ ] Win/loss detected and displayed
- [ ] Round counter increments
- [ ] Can restart after match ends

---

## Architecture Delta from Phase 0

```
Phase 0                          Phase 1
─────────                        ─────────
1v1 heroes                  →    3v3 heroes
Direct move (teleport)      →    A* pathfinding
No AP                       →    AP-based movement/actions
No combat                   →    Attack + damage + death
No AI                       →    Simple enemy AI
No initiative               →    Initiative-based resolution
No win condition             →    Win/loss detection
No timer                    →    Optional turn timer
```

---

## Updated Data Structures

### `crates/core/src/unit.rs` — Updated

```rust
use crate::hex::HexCoord;
use serde::{Serialize, Deserialize};

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

impl Unit {
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
            attack_range: 1, // adjacent hexes
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    pub fn reset_ap(&mut self) {
        self.ap = self.max_ap;
    }

    pub fn can_afford(&self, cost: u32) -> bool {
        self.ap >= cost
    }

    pub fn spend_ap(&mut self, cost: u32) {
        self.ap = self.ap.saturating_sub(cost);
    }
}
```

### `crates/core/src/hex.rs` — Add Pathfinding

Add these functions to the existing `hex.rs`:

```rust
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;

/// Node for A* pathfinding.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AStarNode {
    coord: HexCoord,
    g: u32, // cost from start
    f: u32, // g + heuristic
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap behavior
        other.f.cmp(&self.f)
            .then_with(|| other.g.cmp(&self.g))
    }
}

impl HexMap {
    /// Find path from start to goal using A*.
    /// Returns None if no path exists.
    /// `occupied` is a set of hexes blocked by units.
    pub fn find_path(
        &self,
        start: HexCoord,
        goal: HexCoord,
        occupied: &HashSet<HexCoord>,
    ) -> Option<Vec<HexCoord>> {
        if start == goal {
            return Some(vec![start]);
        }

        if !self.is_walkable(&goal) || occupied.contains(&goal) {
            return None;
        }

        let mut open = BinaryHeap::new();
        let mut came_from: HashMap<HexCoord, HexCoord> = HashMap::new();
        let mut g_score: HashMap<HexCoord, u32> = HashMap::new();

        g_score.insert(start, 0);
        open.push(AStarNode {
            coord: start,
            g: 0,
            f: start.distance(&goal),
        });

        while let Some(current) = open.pop() {
            if current.coord == goal {
                // Reconstruct path
                let mut path = vec![goal];
                let mut node = goal;
                while let Some(&prev) = came_from.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                path.reverse();
                return Some(path);
            }

            let current_g = *g_score.get(&current.coord).unwrap_or(&u32::MAX);

            for neighbor in current.coord.neighbors() {
                if !self.is_walkable(&neighbor) {
                    continue;
                }
                if occupied.contains(&neighbor) && neighbor != goal {
                    continue;
                }

                let tentative_g = current_g + 1;

                if tentative_g < *g_score.get(&neighbor).unwrap_or(&u32::MAX) {
                    came_from.insert(neighbor, current.coord);
                    g_score.insert(neighbor, tentative_g);
                    open.push(AStarNode {
                        coord: neighbor,
                        g: tentative_g,
                        f: tentative_g + neighbor.distance(&goal),
                    });
                }
            }
        }

        None // No path found
    }

    /// Get all hexes reachable from start within given AP budget.
    /// Returns a map of hex -> AP cost to reach it.
    /// Uses Dijkstra's algorithm (all edges cost 1).
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
                if !self.is_walkable(&neighbor) {
                    continue;
                }
                if occupied.contains(&neighbor) {
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

        result.remove(&start); // Don't include starting position
        result
    }

    /// Get hexes within attack range of a position.
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

### `crates/core/src/state.rs` — Updated

```rust
use crate::hex::{HexCoord, HexMap};
use crate::unit::{Unit, UnitId, TeamId};
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

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

    /// Get all occupied hexes (for pathfinding).
    pub fn occupied_hexes(&self) -> HashSet<HexCoord> {
        self.units.values()
            .filter(|u| u.is_alive())
            .map(|u| u.pos)
            .collect()
    }

    /// Get all alive units.
    pub fn alive_units(&self) -> Vec<&Unit> {
        self.units.values().filter(|u| u.is_alive()).collect()
    }

    /// Get all alive units for a team.
    pub fn team_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units.values()
            .filter(|u| u.team == team && u.is_alive())
            .collect()
    }

    /// Get all alive enemy units for a given team.
    pub fn enemy_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units.values()
            .filter(|u| u.team != team && u.is_alive())
            .collect()
    }

    /// Check if a team has any alive heroes.
    pub fn team_has_heroes(&self, team: TeamId) -> bool {
        self.units.values().any(|u| {
            u.team == team && u.is_alive() && u.kind == crate::unit::UnitKind::Hero
        })
    }

    /// Check win condition.
    pub fn check_winner(&self) -> Option<TeamId> {
        let team0_alive = self.team_has_heroes(0);
        let team1_alive = self.team_has_heroes(1);

        if !team0_alive && !team1_alive {
            None // Draw - shouldn't happen in normal play
        } else if !team0_alive {
            Some(1) // Team 1 wins
        } else if !team1_alive {
            Some(0) // Team 0 wins
        } else {
            None
        }
    }

    /// Reset AP for all units at start of round.
    pub fn reset_all_ap(&mut self) {
        for unit in self.units.values_mut() {
            if unit.is_alive() {
                unit.reset_ap();
            }
        }
    }
}
```

### `crates/core/src/event.rs` — Updated

```rust
use crate::hex::HexCoord;
use crate::unit::{UnitId, TeamId};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameEvent {
    RoundStarted { round: u32 },

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

    RoundEnded { round: u32 },

    MatchEnded {
        winner: Option<TeamId>,
    },
}
```

---

## New Module: Orders

### `crates/core/src/orders.rs`

```rust
use crate::hex::HexCoord;
use crate::unit::UnitId;
use serde::{Serialize, Deserialize};

/// Action a unit can take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Wait,
    Attack { target_id: UnitId },
}

/// A single unit's orders for the round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitOrder {
    pub unit_id: UnitId,
    pub move_target: Option<HexCoord>,
    pub action: Action,
}

/// All orders collected during planning phase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnOrders {
    pub orders: Vec<UnitOrder>,
}

impl TurnOrders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_order(&mut self, order: UnitOrder) {
        // Replace existing order for same unit
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

---

## New Module: AI

### `crates/core/src/ai.rs`

```rust
use crate::state::GameState;
use crate::orders::{TurnOrders, UnitOrder, Action};
use crate::hex::HexCoord;
use crate::unit::{UnitId, TeamId};
use std::collections::HashSet;

/// Simple AI for Phase 1.
/// Strategy: move toward nearest enemy, attack if in range.
pub struct SimpleAI;

impl SimpleAI {
    /// Generate orders for all units of a team.
    pub fn generate_orders(state: &GameState, team: TeamId) -> TurnOrders {
        let mut orders = TurnOrders::new();
        let occupied = state.occupied_hexes();

        for unit in state.team_units(team) {
            if !unit.is_alive() {
                continue;
            }

            let order = Self::generate_unit_order(state, unit.id, &occupied);
            orders.add_order(order);
        }

        orders
    }

    fn generate_unit_order(
        state: &GameState,
        unit_id: UnitId,
        occupied: &HashSet<HexCoord>,
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

        // Find nearest enemy
        let nearest_enemy = enemies.iter()
            .min_by_key(|e| unit.pos.distance(&e.pos))
            .unwrap();

        let distance_to_enemy = unit.pos.distance(&nearest_enemy.pos);

        // If enemy is in attack range, attack without moving
        if distance_to_enemy <= unit.attack_range {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Attack { target_id: nearest_enemy.id },
            };
        }

        // Try to move toward enemy
        // Reserve 1 AP for attack if we'll be in range after moving
        let ap_for_movement = if distance_to_enemy <= unit.attack_range + unit.max_ap {
            unit.max_ap - 1 // Reserve AP for attack
        } else {
            unit.max_ap // Just move, can't attack this round
        };

        // Find best hex to move to (closest to enemy within AP budget)
        let reachable = state.map.reachable_hexes(
            unit.pos,
            ap_for_movement,
            &occupied,
        );

        let best_move = reachable.iter()
            .min_by_key(|(hex, _)| hex.distance(&nearest_enemy.pos));

        let (move_target, action) = if let Some((best_hex, _)) = best_move {
            let new_distance = best_hex.distance(&nearest_enemy.pos);

            if new_distance <= unit.attack_range {
                // Can attack after moving
                (
                    Some(*best_hex),
                    Action::Attack { target_id: nearest_enemy.id },
                )
            } else {
                // Just move closer
                (Some(*best_hex), Action::Wait)
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

## Updated Turn Processor

### `crates/core/src/turn.rs`

```rust
use crate::state::{GameState, Phase};
use crate::event::GameEvent;
use crate::orders::{TurnOrders, UnitOrder, Action};
use crate::hex::HexCoord;
use crate::unit::UnitId;
use crate::ai::SimpleAI;
use std::collections::HashSet;

pub struct TurnProcessor;

impl TurnProcessor {
    /// Resolve the current round.
    pub fn resolve(state: &mut GameState, player_orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        events.push(GameEvent::RoundStarted { round: state.round + 1 });

        // Reset AP for all units
        state.reset_all_ap();

        // Generate AI orders for team 1
        let ai_orders = SimpleAI::generate_orders(state, 1);

        // Combine all orders
        let mut all_orders = player_orders.clone();
        for order in ai_orders.orders {
            all_orders.add_order(order);
        }

        // Collect all unit IDs that have orders, sorted by initiative
        let mut unit_ids: Vec<UnitId> = all_orders.orders.iter()
            .map(|o| o.unit_id)
            .collect();

        // Sort by initiative (descending), then by unit_id for determinism
        unit_ids.sort_by(|a, b| {
            let unit_a = state.get_unit(*a).unwrap();
            let unit_b = state.get_unit(*b).unwrap();
            unit_b.initiative.cmp(&unit_a.initiative)
                .then_with(|| a.cmp(b))
        });

        // Process each unit in initiative order
        for unit_id in unit_ids {
            let order = all_orders.get_order(unit_id).unwrap().clone();
            let unit_events = Self::process_unit(state, &order);
            events.extend(unit_events);

            // Check for deaths and remove dead units
            Self::cleanup_dead(state, &mut events);
        }

        // Check win condition
        if let Some(winner) = state.check_winner() {
            state.winner = Some(winner);
            state.phase = Phase::MatchEnd;
            events.push(GameEvent::MatchEnded { winner: Some(winner) });
        } else {
            state.round += 1;
            state.phase = Phase::Planning;
            events.push(GameEvent::RoundEnded { round: state.round });
        }

        events
    }

    /// Process a single unit's orders.
    fn process_unit(state: &mut GameState, order: &UnitOrder) -> Vec<GameEvent> {
        let mut events = Vec::new();

        // Check if unit is alive
        let unit = match state.get_unit(order.unit_id) {
            Some(u) if u.is_alive() => u,
            _ => return events, // Dead units skip
        };

        let unit_id = order.unit_id;
        let start_pos = unit.pos;
        let ap_budget = unit.ap;

        // Phase 1: Movement
        if let Some(move_target) = order.move_target {
            if let Some(unit) = state.get_unit(unit_id) {
                if unit.is_alive() && move_target != unit.pos {
                    let occupied = state.occupied_hexes();

                    if let Some(path) = state.map.find_path(unit.pos, move_target, &occupied) {
                        let path_cost = (path.len() - 1) as u32; // -1 because path includes start

                        if path_cost <= ap_budget {
                            // Move the unit
                            if let Some(unit) = state.get_unit_mut(unit_id) {
                                unit.pos = move_target;
                                unit.spend_ap(path_cost);
                            }

                            events.push(GameEvent::UnitMoved {
                                unit_id,
                                from: start_pos,
                                to: move_target,
                                path: path.clone(),
                                ap_spent: path_cost,
                            });
                        }
                    }
                }
            }
        }

        // Phase 2: Action
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

    /// Process an attack action.
    fn process_attack(
        state: &mut GameState,
        attacker_id: UnitId,
        target_id: UnitId,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        // Get attacker and target
        let (attacker, target) = match (state.get_unit(attacker_id), state.get_unit(target_id)) {
            (Some(a), Some(t)) => (a, t),
            _ => return events,
        };

        if !attacker.is_alive() || !target.is_alive() {
            return events;
        }

        // Check range
        let distance = attacker.pos.distance(&target.pos);
        if distance > attacker.attack_range {
            return events; // Out of range, attack fails
        }

        // Check AP
        let attack_cost = 1;
        if !attacker.can_afford(attack_cost) {
            return events; // Not enough AP
        }

        // Spend AP
        if let Some(attacker) = state.get_unit_mut(attacker_id) {
            attacker.spend_ap(attack_cost);
        }

        // Apply damage
        let damage = attacker.attack_damage;
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

        events
    }

    /// Remove dead units and emit death events.
    fn cleanup_dead(state: &mut GameState, events: &mut Vec<GameEvent>) {
        let dead_units: Vec<(UnitId, Option<UnitId>)> = state.units.values()
            .filter(|u| !u.is_alive())
            .map(|u| (u.id, None)) // TODO: track killer
            .collect();

        for (unit_id, killed_by) in dead_units {
            // Remove from units map
            state.units.remove(&unit_id);

            events.push(GameEvent::UnitDied {
                unit_id,
                killed_by: killed_by.unwrap_or(0),
            });
        }
    }
}
```

---

## Updated Game Engine

### `crates/core/src/lib.rs`

```rust
pub mod hex;
pub mod unit;
pub mod state;
pub mod turn;
pub mod event;
pub mod orders;
pub mod ai;

use hex::{HexCoord, HexMap};
use unit::{Unit, UnitId, TeamId};
use state::{GameState, Phase};
use orders::{TurnOrders, UnitOrder, Action};
use turn::TurnProcessor;
use std::collections::HashSet;

pub struct GameEngine {
    state: GameState,
    pending_orders: TurnOrders,
}

impl GameEngine {
    /// Create a new 3v3 game.
    pub fn new() -> Self {
        let mut map = HexMap::new(5); // radius 5 = 91 hexes

        // Add obstacles
        map.obstacles.insert(HexCoord::new(0, 0));
        map.obstacles.insert(HexCoord::new(1, -1));
        map.obstacles.insert(HexCoord::new(-1, 1));
        map.obstacles.insert(HexCoord::new(2, -2));
        map.obstacles.insert(HexCoord::new(-2, 2));

        let mut state = GameState::new(map);

        // Team 0 (player) - left side
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-4, 0), 3));
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(-4, 1), 2));
        state.add_unit(Unit::new_hero(3, 0, HexCoord::new(-4, -1), 1));

        // Team 1 (enemy) - right side
        state.add_unit(Unit::new_hero(4, 1, HexCoord::new(4, 0), 3));
        state.add_unit(Unit::new_hero(5, 1, HexCoord::new(4, 1), 2));
        state.add_unit(Unit::new_hero(6, 1, HexCoord::new(4, -1), 1));

        Self {
            state,
            pending_orders: TurnOrders::new(),
        }
    }

    /// Get current game state as JSON.
    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap()
    }

    /// Get all walkable hexes as JSON.
    pub fn get_map_hexes(&self) -> String {
        let hexes: Vec<(i32, i32)> = self.state.map.all_hexes()
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&hexes).unwrap()
    }

    /// Get obstacles as JSON.
    pub fn get_obstacles(&self) -> String {
        let obstacles: Vec<(i32, i32)> = self.state.map.obstacles
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&obstacles).unwrap()
    }

    /// Get valid move targets for a unit (based on AP).
    pub fn get_move_targets(&self, unit_id: UnitId) -> String {
        let targets: Vec<(i32, i32, u32)> = if let Some(unit) = self.state.get_unit(unit_id) {
            if !unit.is_alive() {
                vec![]
            } else {
                let occupied = self.state.occupied_hexes();
                self.state.map.reachable_hexes(unit.pos, unit.ap, &occupied)
                    .iter()
                    .map(|(h, cost)| (h.q, h.r, *cost))
                    .collect()
            }
        } else {
            vec![]
        };
        serde_json::to_string(&targets).unwrap()
    }

    /// Get valid attack targets for a unit from a given position.
    pub fn get_attack_targets(&self, unit_id: UnitId, from_q: i32, from_r: i32) -> String {
        let targets: Vec<UnitId> = if let Some(unit) = self.state.get_unit(unit_id) {
            if !unit.is_alive() {
                vec![]
            } else {
                let from_pos = HexCoord::new(from_q, from_r);
                let in_range = self.state.map.hexes_in_range(from_pos, unit.attack_range);
                let range_set: HashSet<HexCoord> = in_range.into_iter().collect();

                self.state.enemy_units(unit.team)
                    .iter()
                    .filter(|e| range_set.contains(&e.pos))
                    .map(|e| e.id)
                    .collect()
            }
        } else {
            vec![]
        };
        serde_json::to_string(&targets).unwrap()
    }

    /// Set move order for a unit.
    pub fn set_move_order(&mut self, unit_id: UnitId, q: i32, r: i32) -> bool {
        let target = HexCoord::new(q, r);

        // Validate unit exists and is player's
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 {
                // Check if target is reachable
                let occupied = self.state.occupied_hexes();
                let reachable = self.state.map.reachable_hexes(unit.pos, unit.ap, &occupied);

                if reachable.contains_key(&target) || target == unit.pos {
                    // Update or create order
                    if let Some(order) = self.pending_orders.orders.iter_mut()
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

    /// Set attack order for a unit.
    pub fn set_attack_order(&mut self, unit_id: UnitId, target_id: UnitId) -> bool {
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 {
                // Check if target is enemy
                if let Some(target) = self.state.get_unit(target_id) {
                    if target.team != unit.team && target.is_alive() {
                        // Update or create order
                        if let Some(order) = self.pending_orders.orders.iter_mut()
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

    /// Set wait order for a unit.
    pub fn set_wait_order(&mut self, unit_id: UnitId) -> bool {
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 {
                if let Some(order) = self.pending_orders.orders.iter_mut()
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

    /// Get current orders as JSON (for UI display).
    pub fn get_pending_orders(&self) -> String {
        serde_json::to_string(&self.pending_orders).unwrap()
    }

    /// Check if all player units have orders.
    pub fn all_units_ordered(&self) -> bool {
        let player_units: Vec<UnitId> = self.state.team_units(0)
            .iter()
            .filter(|u| u.is_alive())
            .map(|u| u.id)
            .collect();

        player_units.iter().all(|id| {
            self.pending_orders.get_order(*id).is_some()
        })
    }

    /// End turn and resolve.
    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::replace(&mut self.pending_orders, TurnOrders::new());
        let events = TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    /// Restart the game.
    pub fn restart(&mut self) {
        *self = GameEngine::new();
    }
}
```

---

## Updated WASM Bindings

### `crates/wasm/src/lib.rs`

```rust
use wasm_bindgen::prelude::*;
use hexabellum_core::GameEngine;

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
```

---

## Updated Client

### `web/src/game/bridge.ts` — Updated

```typescript
import init, { WasmGame } from '../wasm/pkg/hexabellum_wasm';

let game: WasmGame | null = null;

export interface HexCoord {
  q: number;
  r: number;
}

export interface UnitData {
  id: number;
  kind: string;
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
  phase: string;
  units: Record<number, UnitData>;
  winner: number | null;
}

export interface MoveTarget {
  q: number;
  r: number;
  cost: number;
}

export interface GameEvent {
  type: string;
  [key: string]: any;
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
  return JSON.parse(game.get_move_targets(unitId)).map(
    ([q, r, cost]: [number, number, number]) => ({ q, r, cost })
  );
}

export function getAttackTargets(unitId: number, fromQ: number, fromR: number): number[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_attack_targets(unitId, fromQ, fromR));
}

export function setMoveOrder(unitId: number, q: number, r: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_move_order(unitId, q, r);
}

export function setAttackOrder(unitId: number, targetId: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_attack_order(unitId, targetId);
}

export function setWaitOrder(unitId: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_wait_order(unitId);
}

export function getPendingOrders(): any {
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

### `web/src/game/renderer.ts` — Updated

```typescript
import * as PIXI from 'pixi.js';
import { HexCoord, UnitData, GameState, MoveTarget } from './bridge';

const HEX_SIZE = 30;

export class HexRenderer {
  private app: PIXI.Application;
  private hexLayer: PIXI.Container;
  private unitLayer: PIXI.Container;
  private overlayLayer: PIXI.Container;
  private pathLayer: PIXI.Container;

  private hexGraphics: Map<string, PIXI.Graphics> = new Map();
  private unitSprites: Map<number, PIXI.Container> = new Map();

  constructor(canvas: HTMLCanvasElement) {
    this.app = new PIXI.Application({
      view: canvas,
      width: window.innerWidth,
      height: window.innerHeight,
      backgroundColor: 0x1a1a2e,
      antialias: true,
    });

    this.hexLayer = new PIXI.Container();
    this.overlayLayer = new PIXI.Container();
    this.pathLayer = new PIXI.Container();
    this.unitLayer = new PIXI.Container();

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
    this.hexGraphics.clear();

    const obstacleSet = new Set(obstacles.map(h => `${h.q},${h.r}`));

    for (const hex of hexes) {
      const { x, y } = this.hexToPixel(hex.q, hex.r);
      const isObstacle = obstacleSet.has(`${hex.q},${hex.r}`);

      const g = new PIXI.Graphics();

      g.moveTo(x + HEX_SIZE, y);
      for (let i = 1; i <= 6; i++) {
        const angle = (Math.PI / 3) * i;
        g.lineTo(
          x + HEX_SIZE * Math.cos(angle),
          y + HEX_SIZE * Math.sin(angle)
        );
      }
      g.closePath();

      if (isObstacle) {
        g.fill({ color: 0x2d2d44 });
        g.stroke({ color: 0x444466, width: 1 });
      } else {
        g.fill({ color: 0x16213e });
        g.stroke({ color: 0x0f3460, width: 1 });
      }

      this.hexLayer.addChild(g);
      this.hexGraphics.set(`${hex.q},${hex.r}`, g);
    }
  }

  drawUnits(state: GameState): void {
    this.unitLayer.removeChildren();
    this.unitSprites.clear();

    for (const [id, unit] of Object.entries(state.units)) {
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);

      const container = new PIXI.Container();
      container.x = x;
      container.y = y;

      const g = new PIXI.Graphics();

      // Team colors
      const color = unit.team === 0 ? 0x4fc3f7 : 0xef5350;

      // Unit body
      g.circle(0, 0, HEX_SIZE * 0.5);
      g.fill({ color });
      g.stroke({ color: 0xffffff, width: 2 });

      // HP bar
      const hpWidth = HEX_SIZE * 0.8;
      const hpHeight = 4;
      const hpX = -hpWidth / 2;
      const hpY = -HEX_SIZE * 0.7;
      const hpRatio = unit.hp / unit.max_hp;

      g.rect(hpX, hpY, hpWidth, hpHeight);
      g.fill({ color: 0x333333 });
      g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
      g.fill({ color: hpRatio > 0.5 ? 0x4caf50 : 0xff9800 });

      // AP indicator (small dots)
      const apY = HEX_SIZE * 0.7;
      for (let i = 0; i < unit.max_ap; i++) {
        const apX = (i - (unit.max_ap - 1) / 2) * 8;
        g.circle(apX, apY, 3);
        g.fill({ color: i < unit.ap ? 0xffeb3b : 0x555555 });
      }

      // Initiative number
      const initText = new PIXI.Text({
        text: `${unit.initiative}`,
        style: {
          fontSize: 10,
          fill: 0xffffff,
          fontFamily: 'Arial',
        },
      });
      initText.anchor.set(0.5);
      initText.y = -HEX_SIZE * 0.9;

      container.addChild(g);
      container.addChild(initText);

      this.unitLayer.addChild(container);
      this.unitSprites.set(Number(id), container);
    }
  }

  drawMoveTargets(targets: MoveTarget[]): void {
    this.overlayLayer.removeChildren();

    for (const target of targets) {
      const { x, y } = this.hexToPixel(target.q, target.r);

      const g = new PIXI.Graphics();

      // Color based on AP cost
      const alpha = 0.3 + (0.4 * (1 - target.cost / 3));
      g.circle(x, y, HEX_SIZE * 0.4);
      g.fill({ color: 0x4caf50, alpha });

      // Show cost
      const costText = new PIXI.Text({
        text: `${target.cost}`,
        style: {
          fontSize: 12,
          fill: 0xffffff,
          fontFamily: 'Arial',
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

  drawAttackTargets(targets: number[], state: GameState): void {
    this.overlayLayer.removeChildren();

    for (const targetId of targets) {
      const unit = state.units[targetId];
      if (!unit) continue;

      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);

      const g = new PIXI.Graphics();
      g.circle(x, y, HEX_SIZE * 0.6);
      g.stroke({ color: 0xff5252, width: 3, alpha: 0.8 });

      this.overlayLayer.addChild(g);
    }
  }

  drawPath(path: HexCoord[]): void {
    this.pathLayer.removeChildren();

    if (path.length < 2) return;

    const g = new PIXI.Graphics();
    g.moveTo(
      this.hexToPixel(path[0].q, path[0].r).x,
      this.hexToPixel(path[0].q, path[0].r).y
    );

    for (let i = 1; i < path.length; i++) {
      const { x, y } = this.hexToPixel(path[i].q, path[i].r);
      g.lineTo(x, y);
    }

    g.stroke({ color: 0xffeb3b, width: 2, alpha: 0.6 });
    this.pathLayer.addChild(g);
  }

  highlightUnit(unitId: number | null): void {
    for (const [id, container] of this.unitSprites) {
      const g = container.getChildAt(0) as PIXI.Graphics;
      if (id === unitId) {
        g.stroke({ color: 0xffeb3b, width: 3 });
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

  getStage(): PIXI.Container {
    return this.app.stage;
  }
}
```

### `web/src/game/input.ts` — Updated

```typescript
import * as PIXI from 'pixi.js';
import { HexRenderer } from './renderer';
import {
  getState, getMoveTargets, getAttackTargets,
  setMoveOrder, setAttackOrder, setWaitOrder,
  allUnitsOrdered, endTurn, restart,
  UnitData, HexCoord, MoveTarget,
} from './bridge';

type InputMode = 'idle' | 'unitSelected' | 'choosingMove' | 'choosingAction';

export class InputHandler {
  private renderer: HexRenderer;
  private mode: InputMode = 'idle';
  private selectedUnit: number | null = null;
  private moveTargets: MoveTarget[] = [];
  private plannedMove: HexCoord | null = null;
  private attackTargets: number[] = [];

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
    this.setupListeners();
  }

  private setupListeners(): void {
    const stage = this.renderer.getStage();
    stage.eventMode = 'static';

    stage.on('pointerdown', (event: PIXI.FederatedPointerEvent) => {
      const pos = event.global;
      this.handleClick(pos.x, pos.y);
    });
  }

  private handleClick(x: number, y: number): void {
    const state = getState();

    if (state.phase === 'MatchEnd') return;

    const hex = this.pixelToHex(x, y);
    if (!hex) return;

    switch (this.mode) {
      case 'idle':
      case 'unitSelected':
        this.handleUnitSelection(hex);
        break;

      case 'choosingMove':
        this.handleMoveSelection(hex);
        break;

      case 'choosingAction':
        this.handleActionSelection(hex);
        break;
    }
  }

  private handleUnitSelection(hex: HexCoord): void {
    const state = getState();

    // Check if clicked on a unit
    for (const [id, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team === 0) {
        this.selectedUnit = Number(id);
        this.mode = 'choosingMove';
        this.plannedMove = null;

        // Show move targets
        this.moveTargets = getMoveTargets(Number(id));
        this.renderer.drawMoveTargets(this.moveTargets);
        this.renderer.highlightUnit(Number(id));
        this.renderer.clearOverlays();
        this.renderer.drawMoveTargets(this.moveTargets);

        this.updateStatus(`Select move destination for unit ${id}`);
        return;
      }
    }

    // Clicked empty hex while in idle - deselect
    this.selectedUnit = null;
    this.mode = 'idle';
    this.renderer.clearOverlays();
    this.renderer.highlightUnit(null);
  }

  private handleMoveSelection(hex: HexCoord): void {
    if (this.selectedUnit === null) return;

    // Check if clicked hex is a valid move target
    const target = this.moveTargets.find(t => t.q === hex.q && t.r === hex.r);

    if (target) {
      // Set move order
      const success = setMoveOrder(this.selectedUnit, hex.q, hex.r);
      if (success) {
        this.plannedMove = { q: hex.q, r: hex.r };
        this.mode = 'choosingAction';

        // Show attack targets from new position
        this.attackTargets = getAttackTargets(this.selectedUnit, hex.q, hex.r);
        const state = getState();
        this.renderer.clearOverlays();
        this.renderer.drawAttackTargets(this.attackTargets, state);

        this.updateStatus(`Choose action: attack or wait`);
      }
    } else if (hex.q === this.getUnitPos(this.selectedUnit).q &&
               hex.r === this.getUnitPos(this.selectedUnit).r) {
      // Clicked on own unit - stay in place, go to action selection
      this.plannedMove = null;
      this.mode = 'choosingAction';

      const unitPos = this.getUnitPos(this.selectedUnit);
      this.attackTargets = getAttackTargets(this.selectedUnit, unitPos.q, unitPos.r);
      const state = getState();
      this.renderer.clearOverlays();
      this.renderer.drawAttackTargets(this.attackTargets, state);

      this.updateStatus(`Choose action: attack or wait`);
    }
  }

  private handleActionSelection(hex: HexCoord): void {
    if (this.selectedUnit === null) return;

    const state = getState();

    // Check if clicked on an attack target
    for (const [id, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team === 1) {
        const targetId = Number(id);
        if (this.attackTargets.includes(targetId)) {
          const success = setAttackOrder(this.selectedUnit, targetId);
          if (success) {
            this.finishUnitOrders();
            return;
          }
        }
      }
    }

    // Clicked elsewhere - wait
    setWaitOrder(this.selectedUnit);
    this.finishUnitOrders();
  }

  private finishUnitOrders(): void {
    this.selectedUnit = null;
    this.mode = 'idle';
    this.plannedMove = null;
    this.moveTargets = [];
    this.attackTargets = [];

    this.renderer.clearOverlays();
    this.renderer.highlightUnit(null);

    // Check if all units have orders
    if (allUnitsOrdered()) {
      this.updateStatus('All units ordered! Click End Turn');
    } else {
      this.updateStatus('Select next unit');
    }
  }

  private getUnitPos(unitId: number): HexCoord {
    const state = getState();
    return state.units[unitId]?.pos || { q: 0, r: 0 };
  }

  endTurn(): void {
    const events = endTurn();

    // Process events for animation
    this.processEvents(events);

    // Redraw
    const state = getState();
    this.renderer.drawUnits(state);

    if (state.phase === 'MatchEnd') {
      const winner = state.winner;
      const message = winner === 0 ? 'Victory!' : winner === 1 ? 'Defeat!' : 'Draw!';
      this.updateStatus(`Match ended: ${message}`);
      this.showMatchEndOverlay(winner);
    } else {
      this.updateStatus(`Round ${state.round} - Select your units`);
    }
  }

  private processEvents(events: any[]): void {
    // For Phase 1, we just log events
    // In Phase 2, we'll animate them
    console.log('Round events:', events);
  }

  private showMatchEndOverlay(winner: number | null): void {
    // Show restart button
    const restartBtn = document.getElementById('restart') as HTMLButtonElement;
    if (restartBtn) {
      restartBtn.style.display = 'block';
    }
  }

  restartGame(): void {
    restart();
    const state = getState();
    const hexes = this.getMapHexes();
    const obstacles = this.getObstacles();

    this.renderer.drawMap(hexes, obstacles);
    this.renderer.drawUnits(state);

    const restartBtn = document.getElementById('restart') as HTMLButtonElement;
    if (restartBtn) {
      restartBtn.style.display = 'none';
    }

    this.updateStatus('Round 1 - Select your units');
  }

  private getMapHexes(): HexCoord[] {
    // Import from bridge
    const { getMapHexes } = require('./bridge');
    return getMapHexes();
  }

  private getObstacles(): HexCoord[] {
    const { getObstacles } = require('./bridge');
    return getObstacles();
  }

  private pixelToHex(x: number, y: number): HexCoord | null {
    const HEX_SIZE = 30;
    const app = this.renderer.getApp();

    const cx = x - app.screen.width / 2;
    const cy = y - app.screen.height / 2;

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

  private updateStatus(message: string): void {
    const status = document.getElementById('status') as HTMLElement;
    if (status) {
      status.textContent = message;
    }
  }
}
```

### `web/src/main.ts` — Updated

```typescript
import { initGame, getState, getMapHexes, getObstacles } from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';

async function main() {
  await initGame();

  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  const renderer = new HexRenderer(canvas);

  // Draw initial state
  const hexes = getMapHexes();
  const obstacles = getObstacles();
  const state = getState();

  renderer.drawMap(hexes, obstacles);
  renderer.drawUnits(state);

  // Create input handler
  const input = new InputHandler(renderer);

  // End Turn button
  const endTurnBtn = document.getElementById('end-turn') as HTMLButtonElement;
  endTurnBtn.addEventListener('click', () => {
    input.endTurn();
  });

  // Restart button
  const restartBtn = document.getElementById('restart') as HTMLButtonElement;
  restartBtn.addEventListener('click', () => {
    input.restartGame();
  });

  // Round counter
  const roundDisplay = document.getElementById('round') as HTMLElement;
  const updateRound = () => {
    const currentState = getState();
    roundDisplay.textContent = `Round ${currentState.round}`;
  };
  updateRound();

  endTurnBtn.addEventListener('click', updateRound);

  console.log("Hexabellum Phase 1 initialized!");
}

main().catch(console.error);
```

### `web/index.html` — Updated

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Hexabellum - Phase 1</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      background: #0a0a1a;
      color: #eee;
      font-family: 'Segoe UI', sans-serif;
      overflow: hidden;
    }
    #game-canvas {
      display: block;
      width: 100vw;
      height: 100vh;
    }
    #hud {
      position: fixed;
      top: 10px;
      left: 10px;
      z-index: 10;
      display: flex;
      flex-direction: column;
      gap: 10px;
    }
    #round {
      font-size: 18px;
      font-weight: bold;
      color: #4fc3f7;
    }
    #status {
      font-size: 14px;
      color: #aaa;
      max-width: 300px;
    }
    .btn {
      padding: 12px 24px;
      background: #0f3460;
      color: #4fc3f7;
      border: 2px solid #4fc3f7;
      border-radius: 8px;
      font-size: 16px;
      cursor: pointer;
      transition: all 0.2s;
    }
    .btn:hover {
      background: #4fc3f7;
      color: #0a0a1a;
    }
    .btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
    #restart {
      display: none;
      background: #4a148c;
      border-color: #ce93d8;
      color: #ce93d8;
    }
    #restart:hover {
      background: #ce93d8;
      color: #4a148c;
    }
    #instructions {
      position: fixed;
      bottom: 10px;
      left: 10px;
      font-size: 14px;
      color: #888;
      max-width: 400px;
      line-height: 1.4;
    }
    #unit-panel {
      position: fixed;
      top: 10px;
      right: 10px;
      background: rgba(15, 52, 96, 0.9);
      padding: 15px;
      border-radius: 8px;
      border: 1px solid #4fc3f7;
      min-width: 200px;
      display: none;
    }
    #unit-panel.visible {
      display: block;
    }
    #unit-panel h3 {
      color: #4fc3f7;
      margin-bottom: 10px;
    }
    #unit-panel .stat {
      display: flex;
      justify-content: space-between;
      margin: 5px 0;
    }
  </style>
</head>
<body>
  <div id="hud">
    <div id="round">Round 0</div>
    <div id="status">Select your units</div>
    <button id="end-turn" class="btn">End Turn</button>
    <button id="restart" class="btn">Restart</button>
  </div>

  <div id="unit-panel">
    <h3>Unit Info</h3>
    <div class="stat"><span>HP:</span> <span id="unit-hp">-</span></div>
    <div class="stat"><span>AP:</span> <span id="unit-ap">-</span></div>
    <div class="stat"><span>Initiative:</span> <span id="unit-init">-</span></div>
  </div>

  <canvas id="game-canvas"></canvas>

  <div id="instructions">
    1. Click a blue hero to select<br>
    2. Click a highlighted hex to move (shows AP cost)<br>
    3. Click a red-outlined enemy to attack, or click elsewhere to wait<br>
    4. Repeat for all 3 heroes<br>
    5. Click "End Turn" to resolve
  </div>

  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

---

## Acceptance Criteria

| # | Criterion | Status |
|---|-----------|--------|
| 1 | 3 blue heroes and 3 red heroes render on the board | ☐ |
| 2 | Selecting a blue hero shows reachable hexes with AP cost | ☐ |
| 3 | Clicking a reachable hex plans movement | ☐ |
| 4 | After planning move, attackable enemies are highlighted | ☐ |
| 5 | Clicking a highlighted enemy plans an attack | ☐ |
| 6 | Clicking elsewhere plans a "wait" action | ☐ |
| 7 | Can plan orders for all 3 heroes before ending turn | ☐ |
| 8 | "End Turn" button resolves the round | ☐ |
| 9 | Units move along paths (not teleport) | ☐ |
| 10 | Attacks deal damage, HP bars update | ☐ |
| 11 | Units die when HP reaches 0 | ☐ |
| 12 | AI moves enemy heroes toward player and attacks | ☐ |
| 13 | Initiative determines action order (visible in resolution) | ☐ |
| 14 | Win condition triggers when all enemy heroes die | ☐ |
| 15 | Loss condition triggers when all player heroes die | ☐ |
| 16 | Match end overlay shows with restart button | ☐ |
| 17 | Restart resets the game completely | ☐ |
| 18 | Round counter increments correctly | ☐ |

---

## What is NOT in Phase 1

- ❌ Minions / towers / neutrals
- ❌ Fog of war
- ❌ Spells / abilities (only basic attack)
- ❌ Items / upgrades
- ❌ Turn timer
- ❌ Animations (events are logged, not animated)
- ❌ Sound
- ❌ Multiple maps
- ❌ Server / networking

---

## Phase 2 Preview

Once Phase 1 works, Phase 2 adds:
- **Minions** spawning from towers
- **Towers** that auto-attack
- **Fog of war** (simplified radius-based)
- **Turn timer** with AI fallback
- **Event animations** (smooth movement, attack effects)
- **Better AI** (target priority, positioning)
