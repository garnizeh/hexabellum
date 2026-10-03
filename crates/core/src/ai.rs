use crate::hex::HexCoord;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::GameState;
use crate::unit::{TeamId, UnitId};
use std::collections::HashSet;

/// Simple AI for Phase 1.
/// Strategy: move toward nearest enemy, attack if in range.
pub struct SimpleAI;

impl SimpleAI {
    /// Generate orders for all units of a team.
    pub fn generate_orders(state: &GameState, team: TeamId) -> TurnOrders {
        let mut orders = TurnOrders::new();
        let occupied = state.occupied_hexes();

        let mut unit_ids: Vec<UnitId> = state
            .team_units(team)
            .iter()
            .filter(|u| u.is_alive())
            .map(|u| u.id)
            .collect();
        unit_ids.sort_unstable(); // deterministic iteration (HashMap order is not)

        for unit_id in unit_ids {
            let order = Self::generate_unit_order(state, unit_id, &occupied);
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

        // Find nearest enemy (deterministic tie-break by id)
        let nearest_enemy = enemies
            .iter()
            .min_by_key(|e| (unit.pos.distance(&e.pos), e.id))
            .unwrap();

        let distance_to_enemy = unit.pos.distance(&nearest_enemy.pos);

        // If enemy is in attack range, attack without moving
        if distance_to_enemy <= unit.attack_range {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Attack {
                    target_id: nearest_enemy.id,
                },
            };
        }

        // Try to move toward enemy.
        // Reserve 1 AP for attack if we'll be in range after moving.
        let ap_for_movement = if distance_to_enemy <= unit.attack_range + unit.max_ap {
            unit.max_ap.saturating_sub(1) // Reserve AP for attack
        } else {
            unit.max_ap // Just move, can't attack this round anyway
        };

        // Exclude this unit's own hex from occupancy so it can "move" through
        let mut blocked = occupied.clone();
        blocked.remove(&unit.pos);

        // Find best hex to move to (closest to enemy within AP budget)
        let reachable = state
            .map
            .reachable_hexes(unit.pos, ap_for_movement, &blocked);

        let mut candidates: Vec<(&HexCoord, u32)> = reachable.iter().map(|(h, c)| (h, *c)).collect();
        candidates.sort_by_key(|(hex, cost)| (hex.distance(&nearest_enemy.pos), *cost, hex.q, hex.r));

        let (move_target, action) = if let Some((best_hex, _)) = candidates.first() {
            let new_distance = best_hex.distance(&nearest_enemy.pos);

            if new_distance <= unit.attack_range && ap_for_movement > 0 {
                // Can attack after moving
                (
                    Some(**best_hex),
                    Action::Attack {
                        target_id: nearest_enemy.id,
                    },
                )
            } else {
                // Just move closer
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
