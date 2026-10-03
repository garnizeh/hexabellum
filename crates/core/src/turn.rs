use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::state::{GameState, Phase};
use crate::unit::UnitId;
use std::collections::HashMap;

/// Player orders collected during planning phase.
#[derive(Debug, Clone, Default)]
pub struct TurnOrders {
    /// unit_id -> target hex for movement
    pub move_orders: HashMap<UnitId, HexCoord>,
}

impl TurnOrders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_move(&mut self, unit_id: UnitId, target: HexCoord) {
        self.move_orders.insert(unit_id, target);
    }
}

/// The turn processor.
/// Phase 0: simple movement resolution.
pub struct TurnProcessor;

impl TurnProcessor {
    /// Resolve the current round.
    /// Returns events for the client to animate.
    pub fn resolve(state: &mut GameState, orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        events.push(GameEvent::RoundStarted {
            round: state.round + 1,
        });

        // Phase 0: simple movement, no initiative, no AP.
        // Sort for deterministic resolution order.
        let mut ordered: Vec<(UnitId, HexCoord)> =
            orders.move_orders.iter().map(|(k, v)| (*k, *v)).collect();
        ordered.sort_by_key(|(id, _)| *id);

        for (unit_id, target) in &ordered {
            // Check existence / alive / validity before taking a mutable borrow
            let from = match state.units.get(unit_id) {
                Some(u) if u.is_alive() => u.pos,
                _ => continue,
            };

            // Validate: target must be walkable and not occupied
            if !state.map.is_walkable(target) {
                continue;
            }
            if state.is_occupied(target) && *target != from {
                continue;
            }

            // Move the unit
            state.units.get_mut(unit_id).unwrap().pos = *target;

            events.push(GameEvent::UnitMoved {
                unit_id: *unit_id,
                from,
                to: *target,
            });
        }

        // Advance round
        state.round += 1;
        state.phase = Phase::Planning;

        events.push(GameEvent::RoundEnded { round: state.round });

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::HexMap;
    use crate::unit::Unit;

    #[test]
    fn test_movement_resolution() {
        let mut state = GameState::new(HexMap::new(4));
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-3, 0)));

        let mut orders = TurnOrders::new();
        orders.set_move(1, HexCoord::new(-2, 0));

        let events = TurnProcessor::resolve(&mut state, &orders);

        assert_eq!(state.round, 1);
        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(-2, 0));
        // RoundStarted + UnitMoved + RoundEnded
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_invalid_move_rejected() {
        let mut state = GameState::new(HexMap::new(4));
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-3, 0)));

        let mut orders = TurnOrders::new();
        orders.set_move(1, HexCoord::new(10, 10)); // off-map

        let events = TurnProcessor::resolve(&mut state, &orders);

        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(-3, 0));
        // Only RoundStarted + RoundEnded
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_occupied_target_rejected() {
        let mut state = GameState::new(HexMap::new(4));
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-3, 0)));
        state.add_unit(Unit::new_hero(2, 1, HexCoord::new(-2, 0)));

        let mut orders = TurnOrders::new();
        orders.set_move(1, HexCoord::new(-2, 0)); // occupied by hero 2

        TurnProcessor::resolve(&mut state, &orders);

        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(-3, 0));
    }
}
