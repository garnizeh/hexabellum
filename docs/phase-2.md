# Phase 2 Technical Spec — MOBA Elements Vertical Slice

## Phase 2 Goal

> **Transform the tactical battle into a MOBA-like experience: minions spawn and push, towers defend and auto-attack, fog of war hides enemy positions, a turn timer keeps the game moving, and events animate smoothly.**

Phase 1 proved combat works. Phase 2 makes it feel like a **MOBA**.

### What Phase 2 Adds

| System | Description |
|--------|-------------|
| Minions | Spawn from spawner towers, push toward enemy, fight |
| Towers | Stationary defenses that auto-attack enemies in range |
| Spawner Towers | Generate minion waves every N rounds |
| Fog of War | Radius-based vision; enemy units hidden in fog |
| Turn Timer | Countdown per planning phase; AI fills missing orders |
| Animations | Smooth movement, attack flash, damage numbers, death fade |
| Lane Pathing | Minions follow a lane toward the enemy side |
| Target Priority | Towers/minions prefer minions over heroes |

### Definition of Done

- [ ] Each team has a Tower and a Spawner Tower on the map
- [ ] Spawner Towers generate minions every 3 rounds
- [ ] Minions walk toward enemy side using pathfinding
- [ ] Minions attack enemies in range (prefer minions > heroes > towers)
- [ ] Towers auto-attack nearest enemy in range (prefer minions > heroes)
- [ ] Towers and Spawners can be attacked and destroyed
- [ ] Fog of war hides enemy units outside player vision
- [ ] Vision radius from heroes and towers updates fog
- [ ] Turn timer counts down during planning phase
- [ ] When timer expires, AI fills missing orders automatically
- [ ] Movement animates smoothly along path
- [ ] Attacks show visual feedback (flash + damage number)
- [ ] Deaths show fade-out animation
- [ ] Game remains playable and winnable

---

## Architecture Delta from Phase 1

```
Phase 1                          Phase 2
─────────                        ─────────
Heroes only                 →    Heroes + Minions + Towers
No constructions            →    Towers + Spawner Towers
No fog                      →    Radius-based fog of war
No timer                    →    Turn timer + AI fallback
Instant state updates       →    Animated event playback
Simple AI (move + attack)   →    Lane AI, target priority, tower AI
3v3 heroes                  →    3v3 heroes + minion waves
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
    SpawnerTower,
    Neutral,
}

impl UnitKind {
    pub fn is_stationary(&self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::SpawnerTower)
    }

    pub fn is_structure(&self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::SpawnerTower)
    }
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

    // Phase 2 additions
    pub vision_range: u32,
    pub spawn_interval: Option<u32>,  // For SpawnerTower
    pub spawn_counter: u32,           // Counts rounds since last spawn
    pub lane_direction: LaneDirection, // For minions
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneDirection {
    None,
    TowardEnemy,  // Minions moving toward enemy side
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
            attack_range: 1,
            vision_range: 3,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
        }
    }

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
            initiative: 5, // Towers act early
            attack_damage: 30,
            attack_range: 3,
            vision_range: 4,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
        }
    }

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
            initiative: 0, // Spawners don't attack
            attack_damage: 0,
            attack_range: 0,
            vision_range: 2,
            spawn_interval: Some(spawn_interval),
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    pub fn is_stationary(&self) -> bool {
        self.kind.is_stationary()
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

    /// Check if this unit should spawn minions this round.
    pub fn should_spawn(&self) -> bool {
        if let Some(interval) = self.spawn_interval {
            self.spawn_counter >= interval
        } else {
            false
        }
    }

    /// Increment spawn counter, reset if spawned.
    pub fn tick_spawn(&mut self) -> bool {
        if self.spawn_interval.is_none() {
            return false;
        }
        self.spawn_counter += 1;
        if self.should_spawn() {
            self.spawn_counter = 0;
            return true;
        }
        false
    }
}
```

### `crates/core/src/fog.rs` — New Module

```rust
use crate::hex::HexCoord;
use crate::state::GameState;
use crate::unit::TeamId;
use std::collections::HashSet;
use serde::{Serialize, Deserialize};

/// Fog of war state per team.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FogState {
    /// Hexes visible to each team.
    pub visible: Vec<HashSet<HexCoord>>, // indexed by team_id
}

impl FogState {
    pub fn new(team_count: usize) -> Self {
        Self {
            visible: vec![HashSet::new(); team_count],
        }
    }

    /// Compute visibility for all teams based on current state.
    pub fn update(&mut self, state: &GameState) {
        for team in 0..self.visible.len() {
            self.visible[team].clear();

            for unit in state.team_units(team as TeamId) {
                if !unit.is_alive() {
                    continue;
                }

                // Add all hexes within vision range
                let center = unit.pos;
                let range = unit.vision_range;

                // Add center
                self.visible[team].insert(center);

                // Add all hexes in range
                for r in 1..=range {
                    for hex in center.ring(r) {
                        if state.map.is_walkable(&hex) {
                            self.visible[team].insert(hex);
                        }
                    }
                }
            }
        }
    }

    /// Check if a hex is visible to a team.
    pub fn is_visible(&self, team: TeamId, hex: &HexCoord) -> bool {
        self.visible.get(team as usize)
            .map(|v| v.contains(hex))
            .unwrap_or(false)
    }

    /// Get all visible hexes for a team.
    pub fn visible_hexes(&self, team: TeamId) -> &HashSet<HexCoord> {
        &self.visible[team as usize]
    }
}
```

### `crates/core/src/state.rs` — Updated

Add fog state and update relevant methods:

```rust
use crate::hex::{HexCoord, HexMap};
use crate::unit::{Unit, UnitId, TeamId, UnitKind};
use crate::fog::FogState;
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
            fog: FogState::new(2), // 2 teams
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

    pub fn occupied_hexes(&self) -> HashSet<HexCoord> {
        self.units.values()
            .filter(|u| u.is_alive())
            .map(|u| u.pos)
            .collect()
    }

    pub fn alive_units(&self) -> Vec<&Unit> {
        self.units.values().filter(|u| u.is_alive()).collect()
    }

    pub fn team_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units.values()
            .filter(|u| u.team == team && u.is_alive())
            .collect()
    }

    pub fn enemy_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units.values()
            .filter(|u| u.team != team && u.is_alive())
            .collect()
    }

    /// Get enemy units visible to a team.
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

    pub fn check_winner(&self) -> Option<TeamId> {
        let team0_alive = self.team_has_heroes(0);
        let team1_alive = self.team_has_heroes(1);

        if !team0_alive && !team1_alive {
            None
        } else if !team0_alive {
            Some(1)
        } else if !team1_alive {
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

    /// Update fog of war for all teams.
    pub fn update_fog(&mut self) {
        self.fog.update(self);
    }

    /// Get the next available unit ID.
    pub fn alloc_unit_id(&mut self) -> UnitId {
        let id = self.next_unit_id;
        self.next_unit_id += 1;
        id
    }

    /// Get spawn positions adjacent to a spawner.
    pub fn spawn_positions(&self, spawner_pos: &HexCoord) -> Vec<HexCoord> {
        spawner_pos.neighbors()
            .iter()
            .filter(|h| {
                self.map.is_walkable(h) && !self.is_occupied(h)
            })
            .copied()
            .collect()
    }
}
```

### `crates/core/src/event.rs` — Updated

```rust
use crate::hex::HexCoord;
use crate::unit::{UnitId, TeamId, UnitKind};
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
        unit_kind: UnitKind,
        killed_by: UnitId,
    },

    UnitWaited {
        unit_id: UnitId,
    },

    UnitSpawned {
        unit_id: UnitId,
        unit_kind: UnitKind,
        team: TeamId,
        pos: HexCoord,
        spawner_id: UnitId,
    },

    TowerAttacked {
        tower_id: UnitId,
        target_id: UnitId,
        damage: u32,
        target_hp_remaining: u32,
    },

    FogUpdated {
        team: TeamId,
        visible_hexes: Vec<HexCoord>,
    },

    RoundEnded { round: u32 },

    MatchEnded {
        winner: Option<TeamId>,
    },
}
```

---

## New Module: Minion AI

### `crates/core/src/minion_ai.rs`

```rust
use crate::state::GameState;
use crate::orders::{UnitOrder, Action};
use crate::hex::HexCoord;
use crate::unit::{UnitId, TeamId, UnitKind};
use std::collections::HashSet;

/// AI behavior for minions.
/// Minions push toward the enemy side, attacking enemies in range.
pub struct MinionAI;

impl MinionAI {
    /// Generate an order for a minion.
    pub fn generate_order(
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

        // Find target with priority: Minion > Hero > Structure
        let target = Self::find_target(state, unit_id, &enemies);

        if let Some(target_unit) = target {
            let distance = unit.pos.distance(&target_unit.pos);

            // If in attack range, attack
            if distance <= unit.attack_range {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Attack { target_id: target_unit.id },
                };
            }

            // Try to move toward target
            let reachable = state.map.reachable_hexes(
                unit.pos,
                unit.ap,
                occupied,
            );

            // Find hex closest to target
            let best_move = reachable.iter()
                .min_by_key(|(hex, _)| hex.distance(&target_unit.pos));

            if let Some((best_hex, cost)) = best_move {
                let new_distance = best_hex.distance(&target_unit.pos);

                if new_distance <= unit.attack_range && *cost < unit.ap {
                    // Can move and attack
                    return UnitOrder {
                        unit_id,
                        move_target: Some(*best_hex),
                        action: Action::Attack { target_id: target_unit.id },
                    };
                } else {
                    // Just move closer
                    return UnitOrder {
                        unit_id,
                        move_target: Some(*best_hex),
                        action: Action::Wait,
                    };
                }
            }
        }

        // No target found, move toward enemy side
        Self::move_toward_enemy(state, unit_id, occupied)
    }

    /// Find the best target based on priority.
    fn find_target<'a>(
        state: &GameState,
        unit_id: UnitId,
        enemies: &[&'a crate::unit::Unit],
    ) -> Option<&'a crate::unit::Unit> {
        let unit = state.get_unit(unit_id)?;

        // Priority: Minion > Hero > Tower/Spawner
        let mut minion_targets: Vec<_> = enemies.iter()
            .filter(|e| e.kind == UnitKind::Minion)
            .collect();
        let mut hero_targets: Vec<_> = enemies.iter()
            .filter(|e| e.kind == UnitKind::Hero)
            .collect();
        let mut structure_targets: Vec<_> = enemies.iter()
            .filter(|e| e.kind.is_structure())
            .collect();

        // Sort each group by distance
        minion_targets.sort_by_key(|e| unit.pos.distance(&e.pos));
        hero_targets.sort_by_key(|e| unit.pos.distance(&e.pos));
        structure_targets.sort_by_key(|e| unit.pos.distance(&e.pos));

        minion_targets.first()
            .or(hero_targets.first())
            .or(structure_targets.first())
            .copied()
    }

    /// Move toward the enemy side of the map.
    fn move_toward_enemy(
        state: &GameState,
        unit_id: UnitId,
        occupied: &HashSet<HexCoord>,
    ) -> UnitOrder {
        let unit = state.get_unit(unit_id).unwrap();

        // Determine direction based on team
        // Team 0 moves right (positive q), Team 1 moves left (negative q)
        let target_q = if unit.team == 0 {
            state.map.radius as i32
        } else {
            -(state.map.radius as i32)
        };
        let target_pos = HexCoord::new(target_q, 0);

        let reachable = state.map.reachable_hexes(
            unit.pos,
            unit.ap,
            occupied,
        );

        let best_move = reachable.iter()
            .min_by_key(|(hex, _)| hex.distance(&target_pos));

        if let Some((best_hex, _)) = best_move {
            UnitOrder {
                unit_id,
                move_target: Some(*best_hex),
                action: Action::Wait,
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

## New Module: Tower AI

### `crates/core/src/tower_ai.rs`

```rust
use crate::state::GameState;
use crate::orders::{UnitOrder, Action};
use crate::unit::{UnitId, UnitKind};

/// AI behavior for towers.
/// Towers auto-attack the nearest enemy in range.
pub struct TowerAI;

impl TowerAI {
    /// Generate an order for a tower.
    pub fn generate_order(state: &GameState, unit_id: UnitId) -> UnitOrder {
        let unit = state.get_unit(unit_id).unwrap();
        let enemies = state.enemy_units(unit.team);

        if enemies.is_empty() || unit.attack_range == 0 {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            };
        }

        // Find target with priority: Minion > Hero > Structure
        let target = Self::find_target(state, unit_id, &enemies);

        if let Some(target_unit) = target {
            let distance = unit.pos.distance(&target_unit.pos);

            if distance <= unit.attack_range {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Attack { target_id: target_unit.id },
                };
            }
        }

        UnitOrder {
            unit_id,
            move_target: None,
            action: Action::Wait,
        }
    }

    /// Find the best target based on priority and distance.
    fn find_target<'a>(
        state: &GameState,
        unit_id: UnitId,
        enemies: &[&'a crate::unit::Unit],
    ) -> Option<&'a crate::unit::Unit> {
        let unit = state.get_unit(unit_id)?;

        // Filter enemies in range
        let in_range: Vec<_> = enemies.iter()
            .filter(|e| unit.pos.distance(&e.pos) <= unit.attack_range)
            .collect();

        if in_range.is_empty() {
            return None;
        }

        // Priority: Minion > Hero > Structure
        let mut minion_targets: Vec<_> = in_range.iter()
            .filter(|e| e.kind == UnitKind::Minion)
            .collect();
        let mut hero_targets: Vec<_> = in_range.iter()
            .filter(|e| e.kind == UnitKind::Hero)
            .collect();
        let mut structure_targets: Vec<_> = in_range.iter()
            .filter(|e| e.kind.is_structure())
            .collect();

        // Sort by distance within each group
        minion_targets.sort_by_key(|e| unit.pos.distance(&e.pos));
        hero_targets.sort_by_key(|e| unit.pos.distance(&e.pos));
        structure_targets.sort_by_key(|e| unit.pos.distance(&e.pos));

        minion_targets.first()
            .or(hero_targets.first())
            .or(structure_targets.first())
            .copied()
    }
}
```

---

## New Module: Spawner System

### `crates/core/src/spawner.rs`

```rust
use crate::state::GameState;
use crate::unit::{Unit, UnitId, TeamId, UnitKind};
use crate::hex::HexCoord;
use crate::event::GameEvent;

/// Handles minion spawning from SpawnerTowers.
pub struct SpawnerSystem;

impl SpawnerSystem {
    /// Process all spawners at the start of a round.
    /// Returns spawn events.
    pub fn process_spawns(state: &mut GameState) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut new_units: Vec<Unit> = Vec::new();

        // Collect spawner IDs to avoid borrow issues
        let spawner_ids: Vec<UnitId> = state.units.values()
            .filter(|u| u.kind == UnitKind::SpawnerTower && u.is_alive())
            .map(|u| u.id)
            .collect();

        for spawner_id in spawner_ids {
            let should_spawn = {
                let spawner = state.get_unit_mut(spawner_id).unwrap();
                spawner.tick_spawn()
            };

            if should_spawn {
                let spawner = state.get_unit(spawner_id).unwrap();
                let team = spawner.team;
                let spawner_pos = spawner.pos;

                // Find a free adjacent hex to spawn
                let spawn_positions = state.spawn_positions(&spawner_pos);

                if let Some(spawn_pos) = spawn_positions.first() {
                    let new_id = state.alloc_unit_id();
                    let minion = Unit::new_minion(new_id, team, *spawn_pos);

                    new_units.push(minion.clone());

                    events.push(GameEvent::UnitSpawned {
                        unit_id: new_id,
                        unit_kind: UnitKind::Minion,
                        team,
                        pos: *spawn_pos,
                        spawner_id,
                    });
                }
            }
        }

        // Add new units to state
        for unit in new_units {
            state.add_unit(unit);
        }

        events
    }
}
```

---

## Updated AI Module

### `crates/core/src/ai.rs` — Updated

```rust
use crate::state::GameState;
use crate::orders::{TurnOrders, UnitOrder, Action};
use crate::hex::HexCoord;
use crate::unit::{UnitId, TeamId, UnitKind};
use crate::minion_ai::MinionAI;
use crate::tower_ai::TowerAI;
use std::collections::HashSet;

/// Unified AI system for Phase 2.
pub struct GameAI;

impl GameAI {
    /// Generate orders for all units of a team.
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
                UnitKind::Tower | UnitKind::SpawnerTower => {
                    TowerAI::generate_order(state, unit.id)
                }
                UnitKind::Neutral => {
                    // Neutral AI TBD
                    UnitOrder {
                        unit_id: unit.id,
                        move_target: None,
                        action: Action::Wait,
                    }
                }
            };

            orders.add_order(order);
        }

        orders
    }

    /// Generate order for a hero (same as Phase 1 SimpleAI).
    fn generate_hero_order(
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

        // Find nearest enemy (prefer visible enemies)
        let visible_enemies: Vec<_> = enemies.iter()
            .filter(|e| state.fog.is_visible(unit.team, &e.pos))
            .collect();

        let target_pool = if visible_enemies.is_empty() {
            &enemies
        } else {
            &visible_enemies
        };

        let nearest_enemy = target_pool.iter()
            .min_by_key(|e| unit.pos.distance(&e.pos));

        if let Some(nearest) = nearest_enemy {
            let distance = unit.pos.distance(&nearest.pos);

            // If in attack range, attack
            if distance <= unit.attack_range {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Attack { target_id: nearest.id },
                };
            }

            // Try to move toward enemy
            let ap_for_movement = if distance <= unit.attack_range + unit.max_ap {
                unit.max_ap - 1 // Reserve AP for attack
            } else {
                unit.max_ap
            };

            let reachable = state.map.reachable_hexes(
                unit.pos,
                ap_for_movement,
                occupied,
            );

            let best_move = reachable.iter()
                .min_by_key(|(hex, _)| hex.distance(&nearest.pos));

            if let Some((best_hex, _)) = best_move {
                let new_distance = best_hex.distance(&nearest.pos);

                if new_distance <= unit.attack_range {
                    return UnitOrder {
                        unit_id,
                        move_target: Some(*best_hex),
                        action: Action::Attack { target_id: nearest.id },
                    };
                } else {
                    return UnitOrder {
                        unit_id,
                        move_target: Some(*best_hex),
                        action: Action::Wait,
                    };
                }
            }
        }

        UnitOrder {
            unit_id,
            move_target: None,
            action: Action::Wait,
        }
    }

    /// Generate fallback orders for units without player input.
    /// Used when timer expires.
    pub fn generate_fallback_orders(
        state: &GameState,
        team: TeamId,
        existing_orders: &TurnOrders,
    ) -> TurnOrders {
        let mut orders = existing_orders.clone();

        for unit in state.team_units(team) {
            if !unit.is_alive() || unit.kind != UnitKind::Hero {
                continue;
            }

            // Check if unit already has an order
            if orders.get_order(unit.id).is_some() {
                continue;
            }

            // Generate AI order for this unit
            let occupied = state.occupied_hexes();
            let order = Self::generate_hero_order(state, unit.id, &occupied);
            orders.add_order(order);
        }

        orders
    }
}
```

---

## Updated Turn Processor

### `crates/core/src/turn.rs` — Updated

```rust
use crate::state::{GameState, Phase};
use crate::event::GameEvent;
use crate::orders::{TurnOrders, UnitOrder, Action};
use crate::hex::HexCoord;
use crate::unit::{UnitId, UnitKind};
use crate::ai::GameAI;
use crate::spawner::SpawnerSystem;
use std::collections::HashSet;

pub struct TurnProcessor;

impl TurnProcessor {
    /// Resolve the current round.
    pub fn resolve(state: &mut GameState, player_orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        events.push(GameEvent::RoundStarted { round: state.round + 1 });

        // Reset AP for all units
        state.reset_all_ap();

        // Process spawners (spawn minions)
        let spawn_events = SpawnerSystem::process_spawns(state);
        events.extend(spawn_events);

        // Generate AI orders for team 1 (enemy)
        let ai_orders = GameAI::generate_orders(state, 1);

        // Generate AI orders for minions and towers on team 0
        let team0_ai_orders = GameAI::generate_orders(state, 0);

        // Combine all orders
        let mut all_orders = player_orders.clone();

        // Add team 0 AI orders (for minions, towers, and fallback heroes)
        for order in team0_ai_orders.orders {
            let unit = state.get_unit(order.unit_id);
            if let Some(u) = unit {
                // Only add AI orders for non-hero units or units without player orders
                if u.kind != UnitKind::Hero || player_orders.get_order(order.unit_id).is_none() {
                    all_orders.add_order(order);
                }
            }
        }

        // Add team 1 AI orders
        for order in ai_orders.orders {
            all_orders.add_order(order);
        }

        // Collect all unit IDs that have orders, sorted by initiative
        let mut unit_ids: Vec<UnitId> = all_orders.orders.iter()
            .map(|o| o.unit_id)
            .collect();

        // Sort by initiative (descending), then by unit_id for determinism
        unit_ids.sort_by(|a, b| {
            let unit_a = state.get_unit(*a);
            let unit_b = state.get_unit(*b);

            match (unit_a, unit_b) {
                (Some(ua), Some(ub)) => {
                    ub.initiative.cmp(&ua.initiative)
                        .then_with(|| a.cmp(b))
                }
                _ => a.cmp(b),
            }
        });

        // Process each unit in initiative order
        for unit_id in unit_ids {
            // Check if unit still exists and is alive
            let is_alive = state.get_unit(unit_id)
                .map(|u| u.is_alive())
                .unwrap_or(false);

            if !is_alive {
                continue;
            }

            let order = all_orders.get_order(unit_id).unwrap().clone();
            let unit_events = Self::process_unit(state, &order);
            events.extend(unit_events);

            // Check for deaths and remove dead units
            Self::cleanup_dead(state, &mut events);
        }

        // Update fog of war
        state.update_fog();

        // Emit fog update events
        for team in 0..2 {
            let visible: Vec<HexCoord> = state.fog.visible_hexes(team).iter().copied().collect();
            events.push(GameEvent::FogUpdated { team, visible_hexes: visible });
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

        let unit = match state.get_unit(order.unit_id) {
            Some(u) if u.is_alive() => u,
            _ => return events,
        };

        let unit_id = order.unit_id;
        let is_stationary = unit.is_stationary();
        let start_pos = unit.pos;
        let ap_budget = unit.ap;

        // Movement (skip for stationary units)
        if !is_stationary {
            if let Some(move_target) = order.move_target {
                if move_target != start_pos {
                    let occupied = state.occupied_hexes();

                    if let Some(path) = state.map.find_path(start_pos, move_target, &occupied) {
                        let path_cost = (path.len() - 1) as u32;

                        if path_cost <= ap_budget {
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

        // Action
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

        let (attacker, target) = match (state.get_unit(attacker_id), state.get_unit(target_id)) {
            (Some(a), Some(t)) => (a, t),
            _ => return events,
        };

        if !attacker.is_alive() || !target.is_alive() {
            return events;
        }

        let distance = attacker.pos.distance(&target.pos);
        if distance > attacker.attack_range {
            return events;
        }

        let attack_cost = 1;
        if !attacker.can_afford(attack_cost) {
            return events;
        }

        if let Some(attacker) = state.get_unit_mut(attacker_id) {
            attacker.spend_ap(attack_cost);
        }

        let damage = attacker.attack_damage;
        let target_hp_remaining;

        if let Some(target) = state.get_unit_mut(target_id) {
            target.hp = target.hp.saturating_sub(damage);
            target_hp_remaining = target.hp;
        } else {
            return events;
        }

        // Use different event type for towers
        let is_tower = state.get_unit(attacker_id)
            .map(|u| u.kind == UnitKind::Tower)
            .unwrap_or(false);

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

        events
    }

    /// Remove dead units and emit death events.
    fn cleanup_dead(state: &mut GameState, events: &mut Vec<GameEvent>) {
        let dead_units: Vec<(UnitId, UnitKind)> = state.units.values()
            .filter(|u| !u.is_alive())
            .map(|u| (u.id, u.kind))
            .collect();

        for (unit_id, unit_kind) in dead_units {
            state.units.remove(&unit_id);

            events.push(GameEvent::UnitDied {
                unit_id,
                unit_kind,
                killed_by: 0, // TODO: track killer
            });
        }
    }
}
```

---

## Updated Game Engine

### `crates/core/src/lib.rs` — Updated

```rust
pub mod hex;
pub mod unit;
pub mod state;
pub mod turn;
pub mod event;
pub mod orders;
pub mod ai;
pub mod fog;
pub mod minion_ai;
pub mod tower_ai;
pub mod spawner;

use hex::{HexCoord, HexMap};
use unit::{Unit, UnitId, TeamId, UnitKind};
use state::{GameState, Phase};
use orders::{TurnOrders, UnitOrder, Action};
use turn::TurnProcessor;
use ai::GameAI;
use std::collections::HashSet;

pub struct GameEngine {
    state: GameState,
    pending_orders: TurnOrders,
}

impl GameEngine {
    /// Create a new MOBA-style game.
    pub fn new() -> Self {
        let mut map = HexMap::new(6); // radius 6 = 127 hexes

        // Add obstacles (avoid the main lane at r=0)
        map.obstacles.insert(HexCoord::new(0, 2));
        map.obstacles.insert(HexCoord::new(0, -2));
        map.obstacles.insert(HexCoord::new(1, 2));
        map.obstacles.insert(HexCoord::new(-1, -2));
        map.obstacles.insert(HexCoord::new(2, -3));
        map.obstacles.insert(HexCoord::new(-2, 3));

        let mut state = GameState::new(map);

        // Team 0 (player) - left side
        // Heroes
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-4, -1), 3));
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(-4, 0), 2));
        state.add_unit(Unit::new_hero(3, 0, HexCoord::new(-4, 1), 1));

        // Tower
        state.add_unit(Unit::new_tower(4, 0, HexCoord::new(-3, 0)));

        // Spawner
        state.add_unit(Unit::new_spawner(5, 0, HexCoord::new(-5, 0), 3));

        // Team 1 (enemy) - right side
        // Heroes
        state.add_unit(Unit::new_hero(6, 1, HexCoord::new(4, -1), 3));
        state.add_unit(Unit::new_hero(7, 1, HexCoord::new(4, 0), 2));
        state.add_unit(Unit::new_hero(8, 1, HexCoord::new(4, 1), 1));

        // Tower
        state.add_unit(Unit::new_tower(9, 1, HexCoord::new(3, 0)));

        // Spawner
        state.add_unit(Unit::new_spawner(10, 1, HexCoord::new(5, 0), 3));

        state.next_unit_id = 11;

        // Initialize fog
        state.update_fog();

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

    /// Get valid move targets for a unit.
    pub fn get_move_targets(&self, unit_id: UnitId) -> String {
        let targets: Vec<(i32, i32, u32)> = if let Some(unit) = self.state.get_unit(unit_id) {
            if !unit.is_alive() || unit.is_stationary() {
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

        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 && !unit.is_stationary() {
                let occupied = self.state.occupied_hexes();
                let reachable = self.state.map.reachable_hexes(unit.pos, unit.ap, &occupied);

                if reachable.contains_key(&target) || target == unit.pos {
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
                if let Some(target) = self.state.get_unit(target_id) {
                    if target.team != unit.team && target.is_alive() {
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

    /// Get current orders as JSON.
    pub fn get_pending_orders(&self) -> String {
        serde_json::to_string(&self.pending_orders).unwrap()
    }

    /// Check if all player heroes have orders.
    pub fn all_units_ordered(&self) -> bool {
        let player_heroes: Vec<UnitId> = self.state.team_units(0)
            .iter()
            .filter(|u| u.is_alive() && u.kind == UnitKind::Hero)
            .map(|u| u.id)
            .collect();

        player_heroes.iter().all(|id| {
            self.pending_orders.get_order(*id).is_some()
        })
    }

    /// End turn and resolve.
    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::replace(&mut self.pending_orders, TurnOrders::new());
        let events = TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    /// Get fog visibility for player team.
    pub fn get_player_fog(&self) -> String {
        let visible: Vec<(i32, i32)> = self.state.fog.visible_hexes(0)
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&visible).unwrap()
    }

    /// Restart the game.
    pub fn restart(&mut self) {
        *self = GameEngine::new();
    }
}
```

---

## Updated WASM Bindings

### `crates/wasm/src/lib.rs` — Updated

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

    pub fn get_player_fog(&self) -> String {
        self.engine.get_player_fog()
    }

    pub fn restart(&mut self) {
        self.engine.restart();
    }
}
```

---

## Updated Client — Animation System

### `web/src/game/animator.ts` — New

```typescript
import * as PIXI from 'pixi.js';
import { HexRenderer } from './renderer';
import { GameEvent, GameState, HexCoord } from './bridge';

interface AnimationItem {
  type: string;
  data: any;
  startTime: number;
  duration: number;
}

export class Animator {
  private renderer: HexRenderer;
  private queue: AnimationItem[] = [];
  private isPlaying: boolean = false;
  private onComplete: (() => void) | null = null;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
  }

  /// Queue events for animation playback.
  playEvents(events: GameEvent[], onComplete: () => void): void {
    this.queue = events.map(event => ({
      type: event.type,
      data: event,
      startTime: 0, // Will be set when played
      duration: this.getEventDuration(event.type),
    }));
    this.onComplete = onComplete;
    this.isPlaying = true;

    // Start playback
    this.playNext();
  }

  private getEventDuration(type: string): number {
    switch (type) {
      case 'UnitMoved': return 500;
      case 'UnitAttacked':
      case 'TowerAttacked': return 300;
      case 'UnitDied': return 400;
      case 'UnitSpawned': return 300;
      default: return 100;
    }
  }

  private playNext(): void {
    if (this.queue.length === 0) {
      this.isPlaying = false;
      if (this.onComplete) {
        this.onComplete();
      }
      return;
    }

    const item = this.queue.shift()!;
    item.startTime = performance.now();

    this.animateEvent(item);
  }

  private animateEvent(item: AnimationItem): void {
    const { type, data, duration } = item;

    switch (type) {
      case 'UnitMoved':
        this.animateMovement(data, duration);
        break;
      case 'UnitAttacked':
      case 'TowerAttacked':
        this.animateAttack(data, duration);
        break;
      case 'UnitDied':
        this.animateDeath(data, duration);
        break;
      case 'UnitSpawned':
        this.animateSpawn(data, duration);
        break;
      default:
        // No animation for this event type
        setTimeout(() => this.playNext(), duration);
        break;
    }
  }

  private animateMovement(data: any, duration: number): void {
    const { unit_id, path } = data;
    const sprite = this.renderer.getUnitSprite(unit_id);

    if (!sprite || !path || path.length < 2) {
      setTimeout(() => this.playNext(), duration);
      return;
    }

    // Animate along path
    const points = path.map((hex: HexCoord) => {
      const { x, y } = this.renderer.hexToPixel(hex.q, hex.r);
      return new PIXI.Point(x, y);
    });

    const startTime = performance.now();
    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      // Find position along path
      const totalSegments = points.length - 1;
      const segmentProgress = progress * totalSegments;
      const segmentIndex = Math.min(Math.floor(segmentProgress), totalSegments - 1);
      const segmentT = segmentProgress - segmentIndex;

      const from = points[segmentIndex];
      const to = points[segmentIndex + 1];

      sprite.x = from.x + (to.x - from.x) * segmentT;
      sprite.y = from.y + (to.y - from.y) * segmentT;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        setTimeout(() => this.playNext(), 50);
      }
    };

    requestAnimationFrame(animate);
  }

  private animateAttack(data: any, duration: number): void {
    const targetId = data.target_id;
    const sprite = this.renderer.getUnitSprite(targetId);

    if (sprite) {
      // Flash effect
      const originalAlpha = sprite.alpha;
      sprite.alpha = 0.5;

      // Show damage number
      const damage = data.damage;
      this.showDamageNumber(targetId, damage);

      setTimeout(() => {
        sprite.alpha = originalAlpha;
        setTimeout(() => this.playNext(), 50);
      }, duration);
    } else {
      setTimeout(() => this.playNext(), duration);
    }
  }

  private animateDeath(data: any, duration: number): void {
    const { unit_id } = data;
    const sprite = this.renderer.getUnitSprite(unit_id);

    if (sprite) {
      const startTime = performance.now();
      const animate = () => {
        const elapsed = performance.now() - startTime;
        const progress = Math.min(elapsed / duration, 1);

        sprite.alpha = 1 - progress;
        sprite.scale.set(1 - progress * 0.5);

        if (progress < 1) {
          requestAnimationFrame(animate);
        } else {
          this.renderer.removeUnitSprite(unit_id);
          setTimeout(() => this.playNext(), 50);
        }
      };

      requestAnimationFrame(animate);
    } else {
      setTimeout(() => this.playNext(), duration);
    }
  }

  private animateSpawn(data: any, duration: number): void {
    // Spawn animation is handled by re-rendering
    setTimeout(() => this.playNext(), duration);
  }

  private showDamageNumber(unitId: number, damage: number): void {
    const sprite = this.renderer.getUnitSprite(unitId);
    if (!sprite) return;

    const text = new PIXI.Text({
      text: `-${damage}`,
      style: {
        fontSize: 16,
        fill: 0xff5252,
        fontWeight: 'bold',
        fontFamily: 'Arial',
      },
    });

    text.anchor.set(0.5);
    text.x = sprite.x;
    text.y = sprite.y - 40;

    const stage = this.renderer.getStage();
    stage.addChild(text);

    // Animate floating up
    const startTime = performance.now();
    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / 1000, 1);

      text.y = sprite.y - 40 - progress * 30;
      text.alpha = 1 - progress;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        stage.removeChild(text);
      }
    };

    requestAnimationFrame(animate);
  }
}
```

### `web/src/game/renderer.ts` — Add methods for animation

Add these methods to the existing `HexRenderer` class:

```typescript
// Add to HexRenderer class:

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

drawFog(visibleHexes: HexCoord[], allHexes: HexCoord[]): void {
  // Create fog overlay layer if not exists
  let fogLayer = this.app.stage.getChildByName('fogLayer') as PIXI.Container;
  if (!fogLayer) {
    fogLayer = new PIXI.Container();
    fogLayer.name = 'fogLayer';
    this.app.stage.addChild(fogLayer);
  }
  fogLayer.removeChildren();

  const visibleSet = new Set(visibleHexes.map(h => `${h.q},${h.r}`));

  for (const hex of allHexes) {
    if (!visibleSet.has(`${hex.q},${hex.r}`)) {
      const { x, y } = this.hexToPixel(hex.q, hex.r);

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
      g.fill({ color: 0x000000, alpha: 0.7 });

      fogLayer.addChild(g);
    }
  }
}

hideFog(): void {
  const fogLayer = this.app.stage.getChildByName('fogLayer');
  if (fogLayer) {
    fogLayer.removeChildren();
  }
}
```

### `web/src/game/timer.ts` — New

```typescript
export class TurnTimer {
  private duration: number; // in seconds
  private remaining: number;
  private interval: number | null = null;
  private onExpire: (() => void) | null = null;
  private displayElement: HTMLElement | null = null;

  constructor(duration: number = 30) {
    this.duration = duration;
    this.remaining = duration;
  }

  setDisplay(element: HTMLElement): void {
    this.displayElement = element;
  }

  start(onExpire: () => void): void {
    this.stop(); // Clear any existing timer
    this.remaining = this.duration;
    this.onExpire = onExpire;

    this.updateDisplay();

    this.interval = window.setInterval(() => {
      this.remaining--;
      this.updateDisplay();

      if (this.remaining <= 0) {
        this.stop();
        if (this.onExpire) {
          this.onExpire();
        }
      }
    }, 1000);
  }

  stop(): void {
    if (this.interval !== null) {
      clearInterval(this.interval);
      this.interval = null;
    }
  }

  private updateDisplay(): void {
    if (this.displayElement) {
      const minutes = Math.floor(this.remaining / 60);
      const seconds = this.remaining % 60;
      this.displayElement.textContent = `${minutes}:${seconds.toString().padStart(2, '0')}`;

      // Color coding
      if (this.remaining <= 5) {
        this.displayElement.style.color = '#ff5252';
      } else if (this.remaining <= 10) {
        this.displayElement.style.color = '#ff9800';
      } else {
        this.displayElement.style.color = '#4fc3f7';
      }
    }
  }
}
```

### `web/src/main.ts` — Updated

```typescript
import {
  initGame, getState, getMapHexes, getObstacles,
  get_player_fog, endTurn, allUnitsOrdered, restart,
} from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';
import { Animator } from './game/animator';
import { TurnTimer } from './game/timer';

async function main() {
  await initGame();

  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  const renderer = new HexRenderer(canvas);
  const animator = new Animator(renderer);
  const timer = new TurnTimer(30); // 30 second turns

  // Set up timer display
  const timerDisplay = document.getElementById('timer') as HTMLElement;
  timer.setDisplay(timerDisplay);

  // Draw initial state
  const hexes = getMapHexes();
  const obstacles = getObstacles();
  const state = getState();

  renderer.drawMap(hexes, obstacles);
  renderer.drawUnits(state);

  // Draw initial fog
  const fogHexes = JSON.parse(get_player_fog());
  renderer.drawFog(fogHexes, hexes);

  // Create input handler
  const input = new InputHandler(renderer);

  // End Turn button
  const endTurnBtn = document.getElementById('end-turn') as HTMLButtonElement;
  endTurnBtn.addEventListener('click', () => {
    timer.stop();
    input.endTurnWithAnimation(animator, () => {
      // After animation completes
      const newState = getState();
      renderer.drawUnits(newState);

      // Update fog
      const newFog = JSON.parse(get_player_fog());
      renderer.drawFog(newFog, hexes);

      if (newState.phase === 'MatchEnd') {
        const winner = newState.winner;
        const message = winner === 0 ? 'Victory!' : winner === 1 ? 'Defeat!' : 'Draw!';
        updateStatus(`Match ended: ${message}`);
        showMatchEndOverlay(winner);
      } else {
        updateStatus(`Round ${newState.round} - Select your units`);
        updateRound();
        timer.start(() => {
          // Timer expired - auto end turn
          endTurnBtn.click();
        });
      }
    });
  });

  // Restart button
  const restartBtn = document.getElementById('restart') as HTMLButtonElement;
  restartBtn.addEventListener('click', () => {
    input.restartGame();
    const newState = getState();
    const newFog = JSON.parse(get_player_fog());
    renderer.drawFog(newFog, hexes);
    updateStatus('Round 1 - Select your units');
    updateRound();
    timer.start(() => endTurnBtn.click());
  });

  // Round counter
  const roundDisplay = document.getElementById('round') as HTMLElement;
  const updateRound = () => {
    const currentState = getState();
    roundDisplay.textContent = `Round ${currentState.round}`;
  };

  const updateStatus = (message: string) => {
    const status = document.getElementById('status') as HTMLElement;
    if (status) status.textContent = message;
  };

  const showMatchEndOverlay = (winner: number | null) => {
    const restartBtn = document.getElementById('restart') as HTMLButtonElement;
    if (restartBtn) restartBtn.style.display = 'block';
  };

  // Start timer for first round
  timer.start(() => endTurnBtn.click());

  console.log("Hexabellum Phase 2 initialized!");
}

main().catch(console.error);
```

### `web/index.html` — Updated

Add timer display and update styles:

```html
<!-- Add to HUD section -->
<div id="hud">
  <div id="round">Round 0</div>
  <div id="timer" style="font-size: 24px; font-weight: bold;">0:30</div>
  <div id="status">Select your units</div>
  <button id="end-turn" class="btn">End Turn</button>
  <button id="restart" class="btn">Restart</button>
</div>

<!-- Update instructions -->
<div id="instructions">
  1. Click a blue hero to select<br>
  2. Click a highlighted hex to move (shows AP cost)<br>
  3. Click a red-outlined enemy to attack, or click elsewhere to wait<br>
  4. Repeat for all 3 heroes<br>
  5. Click "End Turn" or wait for timer<br>
  <br>
  <strong>Legend:</strong><br>
  🔵 Heroes &nbsp; 🔺 Minions &nbsp; 🏰 Towers &nbsp; ⭐ Spawners
</div>
```

---

## Acceptance Criteria

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Each team has a Tower and Spawner Tower on the map | ☐ |
| 2 | Spawner Towers generate minions every 3 rounds | ☐ |
| 3 | Spawned minions appear adjacent to spawner | ☐ |
| 4 | Minions move toward enemy side using pathfinding | ☐ |
| 5 | Minions attack enemies in range (prefer minions > heroes) | ☐ |
| 6 | Towers auto-attack nearest enemy in range | ☐ |
| 7 | Towers prefer minion targets over hero targets | ☐ |
| 8 | Towers and Spawners can be attacked and destroyed | ☐ |
| 9 | Destroying a Spawner stops minion generation | ☐ |
| 10 | Fog of war hides enemy units outside vision range | ☐ |
| 11 | Vision updates when units move | ☐ |
| 12 | Turn timer counts down from 30 seconds | ☐ |
| 13 | Timer expiration auto-submits orders with AI fallback | ☐ |
| 14 | Movement animates smoothly along path | ☐ |
| 15 | Attacks show flash effect and damage number | ☐ |
| 16 | Deaths show fade-out animation | ☐ |
| 17 | Spawn events show new units appearing | ☐ |
| 18 | Game remains winnable (all enemy heroes dead) | ☐ |
| 19 | Round counter increments correctly | ☐ |
| 20 | Restart resets everything | ☐ |

---

## What is NOT in Phase 2

- ❌ Line-of-sight blocking (fog is radius-only)
- ❌ Repair mechanic for towers
- ❌ Neutral units / jungle camps
- ❌ Multiple lanes
- ❌ Gold / XP / items
- ❌ Spells / abilities
- ❌ Server / networking
- ❌ Sound effects
- ❌ Multiple maps

---

## Phase 3 Preview

Once Phase 2 works, Phase 3 adds:
- **Repair mechanic** — heroes can repair towers/spawners
- **Neutral camps** — jungle units that can be fought or recruited
- **Line of sight** — obstacles block vision
- **Spells / abilities** — heroes get special attacks
- **Better pathfinding** — minions use lane waypoints
- **Server integration** — move to authoritative server model
- **Reconnection** — handle disconnects gracefully
