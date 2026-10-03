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

use crate::hex::{HexCoord, HexMap};
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::{GameState, Phase};
use crate::turn::TurnProcessor;
use crate::unit::{TeamId, Unit, UnitId, UnitKind};
use std::collections::HashSet;

/// Player-controlled team (Blue).
pub const PLAYER_TEAM: u8 = 0;
/// AI-controlled team (Red).
pub const ENEMY_TEAM: u8 = 1;

/// Top-level game engine.
/// This is the main entry point for WASM and client runtime.
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
        state.add_unit(Unit::new_hero(1, PLAYER_TEAM, HexCoord::new(-4, -1), 3));
        state.add_unit(Unit::new_hero(2, PLAYER_TEAM, HexCoord::new(-4, 0), 2));
        state.add_unit(Unit::new_hero(3, PLAYER_TEAM, HexCoord::new(-4, 1), 1));
        state.add_unit(Unit::new_tower(4, PLAYER_TEAM, HexCoord::new(-3, 0)));
        state.add_unit(Unit::new_spawner(5, PLAYER_TEAM, HexCoord::new(-5, 0), 3));

        // Team 1 (AI / Red)
        state.add_unit(Unit::new_hero(6, ENEMY_TEAM, HexCoord::new(4, -1), 3));
        state.add_unit(Unit::new_hero(7, ENEMY_TEAM, HexCoord::new(4, 0), 2));
        state.add_unit(Unit::new_hero(8, ENEMY_TEAM, HexCoord::new(4, 1), 1));
        state.add_unit(Unit::new_tower(9, ENEMY_TEAM, HexCoord::new(3, 0)));
        state.add_unit(Unit::new_spawner(10, ENEMY_TEAM, HexCoord::new(5, 0), 3));

        state.next_unit_id = 11;
        state.update_fog();

        Self {
            state,
            pending_orders: TurnOrders::new(),
        }
    }

    /// Borrow the current game state (read-only).
    pub fn state(&self) -> &GameState {
        &self.state
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

    /// Get valid move targets for a unit (based on AP and mobility).
    /// Stationary structures return an empty list.
    pub fn get_move_targets(&self, unit_id: UnitId) -> String {
        let mut targets: Vec<(i32, i32, u32)> = if let Some(unit) = self.state.get_unit(unit_id) {
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
        targets.sort();
        serde_json::to_string(&targets).unwrap()
    }

    /// Find BFS path between two coordinates avoiding map obstacles.
    pub fn find_path(&self, from_q: i32, from_r: i32, to_q: i32, to_r: i32) -> String {
        let from = HexCoord::new(from_q, from_r);
        let to = HexCoord::new(to_q, to_r);
        let path = self.state.map.find_path(from, to).unwrap_or_default();
        let coords: Vec<(i32, i32)> = path.into_iter().map(|h| (h.q, h.r)).collect();
        serde_json::to_string(&coords).unwrap()
    }

    /// Valid attack targets: must be in range AND visible through Fog of War.
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
                    .filter(|e| {
                        range_set.contains(&e.pos) && self.state.fog.is_visible(unit.team, &e.pos)
                    })
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
    pub fn set_move_order_with(
        &mut self,
        unit_id: UnitId,
        q: i32,
        r: i32,
        reserve_attack_ap: bool,
    ) -> bool {
        let target = HexCoord::new(q, r);

        let unit = match self.state.get_unit(unit_id) {
            Some(u) if u.is_alive() && u.team == PLAYER_TEAM && !u.is_stationary() => u,
            _ => return false,
        };

        if self.state.phase != Phase::Planning {
            return false;
        }

        let mut vacated: HashSet<HexCoord> = HashSet::new();
        for o in &self.pending_orders.orders {
            if o.unit_id == unit_id {
                continue;
            }
            if let Some(mt) = o.move_target {
                if let Some(u) = self.state.get_unit(o.unit_id) {
                    if u.is_alive() && u.team == PLAYER_TEAM && !u.is_stationary() && u.pos != mt {
                        vacated.insert(u.pos);
                    }
                }
            }
        }

        if let Some(cur_move) = self
            .pending_orders
            .get_order(unit_id)
            .and_then(|o| o.move_target)
        {
            if cur_move != target {
                vacated.insert(cur_move);
            }
        }

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
        occupied.retain(|h| !vacated.contains(h));
        occupied.remove(&target);

        let reachable = self
            .state
            .map
            .reachable_hexes(unit.pos, move_budget, &occupied);

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

        false
    }

    /// Set move order for a unit (no attack AP reservation).
    pub fn set_move_order(&mut self, unit_id: UnitId, q: i32, r: i32) -> bool {
        self.set_move_order_with(unit_id, q, r, false)
    }

    /// Set attack order for a unit. Must be within range and visible through Fog of War.
    pub fn set_attack_order(&mut self, unit_id: UnitId, target_id: UnitId) -> bool {
        let (from_pos, own_team) = match self.state.get_unit(unit_id) {
            Some(unit) if unit.is_alive() && unit.team == PLAYER_TEAM => (unit.pos, unit.team),
            _ => return false,
        };

        let target_pos = match self.state.get_unit(target_id) {
            Some(target)
                if target.team != own_team
                    && target.is_alive()
                    && self.state.fog.is_visible(own_team, &target.pos) =>
            {
                target.pos
            }
            _ => return false,
        };

        let unit = self.state.get_unit(unit_id).unwrap();
        let existing_move = self
            .pending_orders
            .get_order(unit_id)
            .and_then(|o| o.move_target);

        let stand_and_attack =
            from_pos.distance(&target_pos) <= unit.attack_range && unit.can_afford(1);

        let mut vacated: HashSet<HexCoord> = HashSet::new();
        for o in &self.pending_orders.orders {
            if o.unit_id == unit_id {
                continue;
            }
            if let Some(mt) = o.move_target {
                if let Some(u) = self.state.get_unit(o.unit_id) {
                    if u.is_alive() && u.team == PLAYER_TEAM && !u.is_stationary() && u.pos != mt {
                        vacated.insert(u.pos);
                    }
                }
            }
        }

        let attack_after_move = match existing_move {
            Some(hex) if hex != from_pos => {
                let mut occ = vacated.clone();
                occ.insert(hex);

                let mut occupied: HashSet<HexCoord> = self
                    .state
                    .units
                    .values()
                    .filter(|u| u.is_alive() && u.id != unit_id)
                    .map(|u| u.pos)
                    .collect();
                occupied.retain(|h| !occ.contains(h));
                let reachable =
                    self.state
                        .map
                        .reachable_hexes(from_pos, unit.ap.saturating_sub(1), &occupied);
                reachable.contains_key(&hex) && hex.distance(&target_pos) <= unit.attack_range
            }
            _ => stand_and_attack,
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

    /// Check if all living player heroes have orders assigned.
    pub fn all_units_ordered(&self) -> bool {
        let player_heroes: Vec<UnitId> = self
            .state
            .team_units(PLAYER_TEAM)
            .iter()
            .filter(|u| u.is_alive() && u.kind == UnitKind::Hero)
            .map(|u| u.id)
            .collect();

        player_heroes
            .iter()
            .all(|id| self.pending_orders.get_order(*id).is_some())
    }

    /// End turn and resolve simultaneous turn with AI order fallbacks.
    /// Returns events as JSON for the client animator.
    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::replace(&mut self.pending_orders, TurnOrders::new());
        let events = TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    /// Visible hex coordinates for Player Team (0) as JSON array of [q, r].
    pub fn get_player_fog(&self) -> String {
        let mut visible: Vec<(i32, i32)> = self
            .state
            .fog
            .visible_hexes(PLAYER_TEAM)
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        visible.sort();
        serde_json::to_string(&visible).unwrap()
    }

    /// Clear all pending orders for a unit.
    pub fn clear_orders(&mut self, unit_id: UnitId) {
        self.pending_orders.orders.retain(|o| o.unit_id != unit_id);
    }

    /// Restart the game match.
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
    use crate::event::GameEvent;

    #[test]
    fn test_engine_new_setup_moba() {
        let engine = GameEngine::new();
        // 6 Heroes + 2 Towers + 2 Spawners = 10 units
        assert_eq!(engine.state.units.len(), 10);
        assert_eq!(engine.state.team_units(PLAYER_TEAM).len(), 5);
        assert_eq!(engine.state.team_units(ENEMY_TEAM).len(), 5);
        assert_eq!(engine.state.round, 0);
        assert_eq!(engine.state.phase, Phase::Planning);
        assert_eq!(engine.state.map.all_hexes().len(), 127);
    }

    #[test]
    fn test_spawner_generates_minion_wave_on_cycle() {
        let mut engine = GameEngine::new();
        assert_eq!(engine.state.round, 0);

        // Round 1
        engine.end_turn();
        assert_eq!(
            engine
                .state
                .units
                .values()
                .filter(|u| u.kind == UnitKind::Minion)
                .count(),
            0
        );

        // Round 2
        engine.end_turn();
        assert_eq!(
            engine
                .state
                .units
                .values()
                .filter(|u| u.kind == UnitKind::Minion)
                .count(),
            0
        );

        // Round 3: Spawners trigger
        let events_json = engine.end_turn();
        let minion_count = engine
            .state
            .units
            .values()
            .filter(|u| u.kind == UnitKind::Minion)
            .count();
        assert_eq!(minion_count, 2);
        assert!(events_json.contains("UnitSpawned"));
    }

    #[test]
    fn test_tower_auto_attacks_and_prioritizes_minions_over_heroes() {
        let mut engine = GameEngine::new();
        // Team 0 Tower at (-3, 0), range 3
        engine
            .state
            .add_unit(Unit::new_hero(99, 1, HexCoord::new(-1, 0), 2));
        engine
            .state
            .add_unit(Unit::new_minion(100, 1, HexCoord::new(-2, 0)));
        engine.state.update_fog();

        let events_json = engine.end_turn();
        // Tower must attack minion (100) first
        assert!(events_json.contains("\"target_id\":100"));
    }

    #[test]
    fn test_fog_of_war_masks_enemy_and_blocks_targeting() {
        let engine = GameEngine::new();
        // Enemy spawner at (5, 0) is well outside Team 0 initial vision
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

        let events_json = engine.end_turn();
        assert!(events_json.contains("RoundEnded"));
        assert_eq!(engine.state.round, 1);
    }

    #[test]
    fn test_destroying_enemy_spawner_wins_game() {
        let mut engine = GameEngine::new();
        let spawner = engine.state.get_unit_mut(10).unwrap();
        spawner.hp = 10;

        engine
            .state
            .add_unit(Unit::new_hero(77, 0, HexCoord::new(5, -1), 10));
        engine.state.update_fog();
        assert!(engine.set_attack_order(77, 10));

        let events_json = engine.end_turn();
        assert!(events_json.contains("MatchEnded"));
        assert_eq!(engine.state.winner, Some(0));
    }

    #[test]
    fn test_stationary_structures_reject_move_orders() {
        let mut engine = GameEngine::new();
        // Tower 4 is stationary
        assert!(!engine.set_move_order(4, -2, 0));
        let targets: Vec<(i32, i32, u32)> =
            serde_json::from_str(&engine.get_move_targets(4)).unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_move_targets_respect_ap() {
        let engine = GameEngine::new();
        let targets: Vec<(i32, i32, u32)> =
            serde_json::from_str(&engine.get_move_targets(2)).unwrap();
        // Hero 2 at (-4, 0) with 3 AP: everything shown costs <= 3
        for (_q, _r, cost) in &targets {
            assert!(*cost >= 1 && *cost <= 3);
        }
        // Adjacent hex (-3, 0) is occupied by tower 4, so it should not be a valid destination
        assert!(!targets.iter().any(|(q, r, _c)| *q == -3 && *r == 0));
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
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::UnitWaited { unit_id: 1 }))
        );
        assert_eq!(engine.state.round, 1);
        assert!(engine.pending_orders.orders.is_empty());
    }

    #[test]
    fn test_full_battle_runs_to_completion_deterministically() {
        let run = || {
            let mut engine = GameEngine::new();
            for _ in 0..60 {
                let hero_ids: Vec<UnitId> = engine
                    .state
                    .team_units(PLAYER_TEAM)
                    .iter()
                    .filter(|u| u.is_alive() && u.kind == UnitKind::Hero)
                    .map(|u| u.id)
                    .collect();
                for id in hero_ids {
                    let pos = engine.state.get_unit(id).unwrap().pos;
                    let enemies: Vec<(u32, UnitId)> = engine
                        .state
                        .enemy_units(PLAYER_TEAM)
                        .iter()
                        .filter(|e| engine.state.fog.is_visible(PLAYER_TEAM, &e.pos))
                        .map(|e| (pos.distance(&e.pos), e.id))
                        .collect();

                    if enemies.is_empty() {
                        continue;
                    }

                    let (_d, target_id) = *enemies.iter().min().unwrap();
                    engine.clear_orders(id);

                    if engine.set_attack_order(id, target_id) {
                        continue;
                    }

                    let tgt_pos = engine.state.get_unit(target_id).unwrap().pos;
                    let mut planned_attack = false;
                    let neighbors = tgt_pos.neighbors();
                    let mut adj: Vec<HexCoord> = neighbors
                        .into_iter()
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
                        engine.clear_orders(id);
                    }

                    if !planned_attack && let Some(path) = engine.state.map.find_path(pos, tgt_pos) {
                        let unit = engine.state.get_unit(id).unwrap();
                        let max_step = (unit.ap as usize).min(path.len().saturating_sub(2));
                        for step_idx in (1..=max_step).rev() {
                            let step = path[step_idx];
                            if engine.set_move_order(id, step.q, step.r) {
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
    }
}
