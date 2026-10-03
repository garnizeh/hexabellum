pub mod ai;
pub mod event;
pub mod hex;
pub mod orders;
pub mod state;
pub mod turn;
pub mod unit;

use crate::event::GameEvent;
use crate::hex::{HexCoord, HexMap};
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::{GameState, Phase};
use crate::turn::TurnProcessor;
use crate::unit::{Unit, UnitId};
use std::collections::HashSet;

/// Player-controlled team.
pub const PLAYER_TEAM: u8 = 0;
/// AI-controlled team.
pub const ENEMY_TEAM: u8 = 1;

/// Top-level game engine.
/// This is the main entry point for WASM.
pub struct GameEngine {
    pub(crate) state: GameState,
    pub(crate) pending_orders: TurnOrders,
}

impl GameEngine {
    /// Create a new 3v3 game (Phase 1 setup).
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
        state.add_unit(Unit::new_hero(1, PLAYER_TEAM, HexCoord::new(-4, 0), 3));
        state.add_unit(Unit::new_hero(2, PLAYER_TEAM, HexCoord::new(-4, 1), 2));
        state.add_unit(Unit::new_hero(3, PLAYER_TEAM, HexCoord::new(-4, -1), 1));

        // Team 1 (enemy AI) - right side
        state.add_unit(Unit::new_hero(4, ENEMY_TEAM, HexCoord::new(4, 0), 3));
        state.add_unit(Unit::new_hero(5, ENEMY_TEAM, HexCoord::new(4, 1), 2));
        state.add_unit(Unit::new_hero(6, ENEMY_TEAM, HexCoord::new(4, -1), 1));

        Self {
            state,
            pending_orders: TurnOrders::new(),
        }
    }

    /// Get current game state as JSON.
    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap()
    }

    /// Borrow the current game state (read-only).
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// Get all walkable hexes as JSON.
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

    /// Get obstacles as JSON.
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

    /// Get valid move targets for a unit (based on AP).
    /// Returns JSON array of [q, r, ap_cost].
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
        // Deterministic output ordering
        targets.sort();
        serde_json::to_string(&targets).unwrap()
    }

    /// Get valid attack targets for a unit from a given position.
    /// Returns JSON array of unit ids.
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

    /// Set move order for a unit.
    /// `reserve_attack_ap` indicates whether an attack action may be planned
    /// alongside this move; when true, 1 AP is reserved for the attack so the
    /// move validation stays consistent with attack-after-move validation.
    pub fn set_move_order_with(
        &mut self,
        unit_id: UnitId,
        q: i32,
        r: i32,
        reserve_attack_ap: bool,
    ) -> bool {
        let target = HexCoord::new(q, r);

        // Vacated-hex model (matches how resolution validates moves against
        // planning-time positions): a hex currently held by a friendly unit
        // that is itself planning to move does not block pathfinding, and the
        // mover's own start hex never blocks its path. The final collision
        // check happens at resolution time via A*.
        let mut vacated: HashSet<HexCoord> = HashSet::new();
        for o in &self.pending_orders.orders {
            if o.unit_id == unit_id {
                continue;
            }
            if let Some(mt) = o.move_target {
                if let Some(u) = self.state.get_unit(o.unit_id) {
                    if u.is_alive() && u.team == PLAYER_TEAM && u.pos != mt {
                        vacated.insert(u.pos);
                    }
                }
            }
        }
        if let Some(cur_move) = self.pending_orders.get_order(unit_id).and_then(|o| o.move_target) {
            vacated.insert(cur_move); // previous plan's destination stays free
        }

        // Validate unit exists, is alive, and belongs to the player team
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == PLAYER_TEAM && self.state.phase == Phase::Planning {
                // Reserve 1 AP for an attack when one is already planned or may
                // still be added alongside this move, so that move + attack
                // orders are valid no matter which one was set first.
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

                // Check if target is reachable within the AP budget.
                // The mover's own hex must not block its path, and the
                // destination may currently be held by another unit that will
                // itself move this round (all moves are validated against
                // planning-time positions, matching the AI), so entering a
                // contested hex is allowed at order time; resolution performs
                // the final occupancy check via A*.
                let mut occupied: HashSet<HexCoord> = self
                    .state
                    .units
                    .values()
                    .filter(|u| u.is_alive() && u.id != unit_id)
                    .map(|u| u.pos)
                    .collect();
                occupied.retain(|h| !vacated.contains(h));
                occupied.remove(&target); // may be entered even if contested
                let reachable = self
                    .state
                    .map
                    .reachable_hexes(unit.pos, move_budget, &occupied);

                if reachable.contains_key(&target) || target == unit.pos {
                    // Update or create order (preserving any planned action)
                    if let Some(order) =
                        self.pending_orders.orders.iter_mut().find(|o| o.unit_id == unit_id)
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
        // The ordered unit must be a living player unit
        let (from_pos, own_team) = match self.state.get_unit(unit_id) {
            Some(unit) if unit.is_alive() && unit.team == PLAYER_TEAM => (unit.pos, unit.team),
            _ => return false,
        };

        // Target must be a living enemy
        let target_pos = match self.state.get_unit(target_id) {
            Some(target) if target.team != own_team && target.is_alive() => target.pos,
            _ => return false,
        };

        // Validate reachability: the attack must be executable after the
        // planned move (if any), given AP costs (move = path length, attack = 1).
        let unit = self.state.get_unit(unit_id).unwrap();
        let existing_move = self
            .pending_orders
            .get_order(unit_id)
            .and_then(|o| o.move_target);

        let stand_and_attack =
            from_pos.distance(&target_pos) <= unit.attack_range && unit.can_afford(1);

        let attack_after_move = match existing_move {
            Some(hex) if hex != from_pos => {
                // Same vacated-hex model as move validation: hexes held by
                // units that are themselves planning to move do not block the
                // path, and the attacker's own start hex never blocks it.
                let mut vacated: HashSet<HexCoord> = HashSet::new();
                for o in &self.pending_orders.orders {
                    if o.unit_id == unit_id {
                        continue;
                    }
                    if let Some(mt) = o.move_target {
                        if let Some(u) = self.state.get_unit(o.unit_id) {
                            if u.is_alive() && u.team == PLAYER_TEAM && u.pos != mt {
                                vacated.insert(u.pos);
                            }
                        }
                    }
                }
                vacated.insert(hex);

                let mut occupied: HashSet<HexCoord> = self
                    .state
                    .units
                    .values()
                    .filter(|u| u.is_alive() && u.id != unit_id)
                    .map(|u| u.pos)
                    .collect();
                occupied.retain(|h| !vacated.contains(h));
                let reachable = self
                    .state
                    .map
                    .reachable_hexes(from_pos, unit.ap.saturating_sub(1), &occupied);
                reachable.contains_key(&hex)
                    && hex.distance(&target_pos) <= unit.attack_range
            }
            _ => false,
        };

        if !(stand_and_attack || attack_after_move) {
            return false;
        }

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
        true
    }

    /// Set move order for a unit (no attack AP reservation).
    pub fn set_move_order(&mut self, unit_id: UnitId, q: i32, r: i32) -> bool {
        self.set_move_order_with(unit_id, q, r, false)
    }

    /// Set wait order for a unit.
    pub fn set_wait_order(&mut self, unit_id: UnitId) -> bool {
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == PLAYER_TEAM {
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

    /// Get current orders as JSON (for UI display).
    pub fn get_pending_orders(&self) -> String {
        serde_json::to_string(&self.pending_orders).unwrap()
    }

    /// Check if all player units have orders.
    pub fn all_units_ordered(&self) -> bool {
        let mut player_units: Vec<UnitId> = self
            .state
            .team_units(PLAYER_TEAM)
            .iter()
            .filter(|u| u.is_alive())
            .map(|u| u.id)
            .collect();
        player_units.sort_unstable();

        player_units
            .iter()
            .all(|id| self.pending_orders.get_order(*id).is_some())
    }

    /// End turn and resolve.
    /// Returns events as JSON for the client.
    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::replace(&mut self.pending_orders, TurnOrders::new());
        let events: Vec<GameEvent> = TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    /// Clear all pending orders for a unit (move target and action).
    /// Used by clients/tests to reset a unit's plan before re-ordering it.
    pub fn clear_orders(&mut self, unit_id: UnitId) {
        self.pending_orders.orders.retain(|o| o.unit_id != unit_id);
    }

    /// Restart the game.
    pub fn restart(&mut self) {
        *self = GameEngine::new();
    }
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_new_setup_3v3() {
        let engine = GameEngine::new();
        assert_eq!(engine.state.units.len(), 6);
        assert_eq!(engine.state.team_units(PLAYER_TEAM).len(), 3);
        assert_eq!(engine.state.team_units(ENEMY_TEAM).len(), 3);
        assert_eq!(engine.state.round, 0);
        assert_eq!(engine.state.phase, Phase::Planning);
    }

    #[test]
    fn test_move_targets_respect_ap() {
        let engine = GameEngine::new();
        let targets: Vec<(i32, i32, u32)> =
            serde_json::from_str(&engine.get_move_targets(1)).unwrap();
        // Hero 1 at (-4, 0) with 3 AP: everything shown costs <= 3
        for (_q, _r, cost) in &targets {
            assert!(*cost >= 1 && *cost <= 3);
        }
        // A hex at distance 4 is not reachable
        assert!(!targets.iter().any(|(q, r, _c)| *q == 0 && *r == 0));
        // Adjacent hex (-3,0) reachable with cost 1
        assert!(targets.contains(&(-3, 0, 1)));
    }

    #[test]
    fn test_set_move_order_validation() {
        let mut engine = GameEngine::new();
        // Reachable within AP
        assert!(engine.set_move_order(1, -2, 0));
        // Too far (distance 7 > AP 3)
        assert!(!engine.set_move_order(1, 3, 0));
        // Enemy unit cannot be ordered
        assert!(!engine.set_move_order(4, 3, 0));
        // Obstacle cannot be entered
        assert!(!engine.set_move_order(1, -2, 2)); // (-2,2) is... check walkable below
    }

    #[test]
    fn test_attack_order_requires_range_or_planned_move() {
        let mut engine = GameEngine::new();
        // Enemies are far away: no attack possible from start
        assert!(!engine.set_attack_order(1, 4));

        // Plan a move that puts hero 1 adjacent to an enemy? Not possible in one round here.
        // Instead directly place units adjacent via a fresh engine state manipulation:
        let mut e2 = GameEngine::new();
        e2.state.add_unit(Unit::new_hero(7, ENEMY_TEAM, HexCoord::new(-3, 0), 1));
        assert!(e2.set_attack_order(1, 7)); // adjacent, can afford
        // Attack after planned move
        let mut e3 = GameEngine::new();
        e3.state.add_unit(Unit::new_hero(8, ENEMY_TEAM, HexCoord::new(-1, 0), 1));
        assert!(e3.set_move_order(1, -2, 0)); // move 2 AP, leaves 1 AP
        assert!(e3.set_attack_order(1, 8)); // adjacent to -2,0 -> valid
    }

    #[test]
    fn test_all_units_ordered_and_end_turn() {
        let mut engine = GameEngine::new();
        assert!(!engine.all_units_ordered());
        assert!(engine.set_wait_order(1));
        assert!(engine.set_wait_order(2));
        assert!(!engine.all_units_ordered());
        assert!(engine.set_wait_order(3));
        assert!(engine.all_units_ordered());

        let events: Vec<GameEvent> = serde_json::from_str(&engine.end_turn()).unwrap();
        // AI should have acted; player waited
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::UnitWaited { unit_id: 1 })));
        assert_eq!(engine.state.round, 1);
        assert!(engine.pending_orders.orders.is_empty());
    }

    #[test]
    fn test_full_battle_runs_to_completion_deterministically() {
        // Scripted player aggression vs AI; run until match end (bounded rounds).
        let run = || {
            let mut engine = GameEngine::new();
            for _ in 0..60 {
                // Order all living player heroes to attack nearest enemy or advance
                let hero_ids: Vec<UnitId> = engine
                    .state
                    .team_units(PLAYER_TEAM)
                    .iter()
                    .map(|u| u.id)
                    .collect();
                for id in hero_ids {
                    let pos = engine.state.get_unit(id).unwrap().pos;
                    let enemies: Vec<(u32, UnitId)> = engine
                        .state
                        .enemy_units(PLAYER_TEAM)
                        .iter()
                        .map(|e| (pos.distance(&e.pos), e.id))
                        .collect();
                    let (_d, target_id) = *enemies.iter().min().unwrap();

                    // Fresh plan every round: drop any stale orders first
                    engine.clear_orders(id);

                    // Prefer: stand and attack
                    if engine.set_attack_order(id, target_id) {
                        continue;
                    }

                    // Then: move adjacent to enemy and attack from there
                    let tgt_pos = engine.state.get_unit(target_id).unwrap().pos;
                    let mut planned_attack = false;
                    let neighbors = tgt_pos.neighbors();
                    let mut adj: Vec<&HexCoord> = neighbors
                        .iter()
                        .filter(|h| engine.state.map.is_walkable(h))
                        .collect();
                    adj.sort_by_key(|h| (h.distance(&pos), h.q, h.r));
                    for hex in adj {
                        if engine.set_move_order_with(id, hex.q, hex.r, true)
                            && engine.set_attack_order(id, target_id)
                        {
                            planned_attack = true;
                            break;
                        }
                        // Failed attack plan -> clear the move we just set
                        engine.pending_orders.orders.iter_mut().for_each(|o| {
                            if o.unit_id == id {
                                o.move_target = None;
                            }
                        });
                    }

                    // Fallback: just advance toward the enemy
                    if !planned_attack {
                        let dq = (tgt_pos.q - pos.q).signum();
                        let dr = (tgt_pos.r - pos.r).signum();
                        let candidates = [
                            (pos.q + dq * 2, pos.r + dr * 2),
                            (pos.q + dq, pos.r + dr),
                            (pos.q + dq, pos.r),
                            (pos.q, pos.r + dr),
                        ];
                        for (q, r) in candidates {
                            if engine.set_move_order(id, q, r) {
                                break;
                            }
                        }
                    }
                }
                engine.end_turn();
                if engine.state.phase == Phase::MatchEnd {
                    break;
                }
            }
            (engine.state.winner, engine.state.round)
        };

        let (winner_a, rounds_a) = run();
        let (winner_b, rounds_b) = run();
        assert_eq!(winner_a, winner_b, "match outcome must be deterministic");
        assert_eq!(rounds_a, rounds_b, "match length must be deterministic");
        // The match must actually finish (someone wins) within the round cap.
        assert!(
            winner_a.is_some(),
            "scripted battle should reach a winner within the round cap"
        );
    }
}
