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
            // `orders` already contains the decisions made this round for
            // units with lower ids; a friendly unit planning to move away
            // vacates its hex, so later units can path through it.
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

        // Exclude this unit's own hex from occupancy so it can "move" through,
        // and treat hexes held by friendly units that are themselves planning
        // to move as vacated (they will free up during resolution). Without
        // the vacated-hex model the AI would steer around friendly units that
        // are about to step aside anyway, which could permanently stall the
        // front line when both sides shuffle in lockstep.
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

        // Find best hex to move to (closest to enemy within AP budget)
        let reachable = state
            .map
            .reachable_hexes(unit.pos, ap_for_movement, &blocked);

        let mut candidates: Vec<(&HexCoord, u32)> = reachable.iter().map(|(h, c)| (h, *c)).collect();
        candidates.sort_by_key(|(hex, cost)| (hex.distance(&nearest_enemy.pos), *cost, hex.q, hex.r));

        // Destination must still be free once every planned move this round
        // has completed. A friendly unit that was ordered before this one and
        // plans to step onto our chosen hex will get there first (and its
        // order is already committed), so we must pick a different hex —
        // otherwise both units would end the round stacked on one tile,
        // corrupting occupancy and permanently stalling combat.
        // Enemy units never vacate during our own planning pass, so any hex
        // they currently stand on (or are seen stepping onto) stays occupied.
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
            .filter(|(hex, _)| !final_occupancy(hex))
            .next()
        {
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
