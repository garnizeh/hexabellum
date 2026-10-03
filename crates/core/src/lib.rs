pub mod event;
pub mod hex;
pub mod state;
pub mod turn;
pub mod unit;

use event::GameEvent;
use hex::{HexCoord, HexMap};
use state::GameState;
use turn::{TurnOrders, TurnProcessor};
use unit::{Unit, UnitId};

/// Top-level game engine.
/// This is the main entry point for WASM.
pub struct GameEngine {
    state: GameState,
    pending_orders: TurnOrders,
}

impl GameEngine {
    /// Create a new game with default Phase 0 setup.
    pub fn new() -> Self {
        let mut map = HexMap::new(4); // radius 4 = 61 hexes

        // Add a few obstacles for visual interest
        map.obstacles.insert(HexCoord::new(0, 0));
        map.obstacles.insert(HexCoord::new(1, -1));
        map.obstacles.insert(HexCoord::new(-1, 1));

        let mut state = GameState::new(map);

        // Place heroes: team 0 on left, team 1 on right
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-3, 0)));
        state.add_unit(Unit::new_hero(2, 1, HexCoord::new(3, 0)));

        Self {
            state,
            pending_orders: TurnOrders::new(),
        }
    }

    /// Get current game state (serialized for client).
    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap()
    }

    /// Select a unit and set move target.
    pub fn set_move_order(&mut self, unit_id: UnitId, q: i32, r: i32) -> bool {
        let target = HexCoord::new(q, r);

        // Validate: unit exists, is alive, belongs to team 0 (player team for now)
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 {
                self.pending_orders.set_move(unit_id, target);
                return true;
            }
        }
        false
    }

    /// End the turn and resolve.
    /// Returns events as JSON for the client.
    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::replace(&mut self.pending_orders, TurnOrders::new());
        let events: Vec<GameEvent> = TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    /// Get all walkable hexes as JSON (for rendering).
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

    /// Get valid move targets for a unit (Phase 0: adjacent hexes).
    pub fn get_move_targets(&self, unit_id: UnitId) -> String {
        let targets: Vec<(i32, i32)> =
            if let Some(unit) = self.state.get_unit(unit_id) {
                unit.pos
                    .neighbors()
                    .iter()
                    .filter(|h| self.state.map.is_walkable(h) && !self.state.is_occupied(h))
                    .map(|h| (h.q, h.r))
                    .collect()
            } else {
                vec![]
            };
        serde_json::to_string(&targets).unwrap()
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
    fn test_engine_new_setup() {
        let engine = GameEngine::new();
        assert_eq!(engine.state.units.len(), 2);
        assert_eq!(engine.state.round, 0);
    }

    #[test]
    fn test_full_turn_loop() {
        let mut engine = GameEngine::new();

        // Player can order team-0 hero to an adjacent hex
        assert!(engine.set_move_order(1, -2, 0));
        // Enemy hero cannot be ordered
        assert!(!engine.set_move_order(2, 2, 0));

        let events: Vec<GameEvent> =
            serde_json::from_str(&engine.end_turn()).unwrap();
        assert!(events.iter().any(|e| matches!(e, GameEvent::UnitMoved { unit_id: 1, .. })));
        assert_eq!(engine.state.round, 1);

        // Loop repeats
        assert!(engine.set_move_order(1, -1, 0));
        engine.end_turn();
        assert_eq!(engine.state.round, 2);
    }

    #[test]
    fn test_move_targets_are_adjacent_walkable() {
        let engine = GameEngine::new();
        let targets: Vec<(i32, i32)> =
            serde_json::from_str(&engine.get_move_targets(1)).unwrap();
        // Hero at (-3,0): neighbors within map minus occupied/obstacles
        assert!(!targets.contains(&(0, 0)));
        assert!(targets.contains(&(-2, 0)) || targets.contains(&(-3, 1)));
    }
}
