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

        // Perimeter of the board: hexes at maximum radius. A unit pushed onto
        // one of these has no forward space left and would oscillate against
        // the edge forever; keeping our own team off them (when free interior
        // hexes exist) lets the front line reform and combat resolve.
        let center = HexCoord::new(0, 0);
        let max_dist = state.map.radius as i32;

        // Find best hex to move to (closest to enemy within AP budget).
        // Friendly units that are themselves planning to move do not block
        // routes (they will step aside during resolution), but every other
        // occupied hex - friendlies holding position *and* enemies - does:
        // a body never moves out of the way for an intruder, so ordering a
        // move into an occupied hex would fail at resolution and leave the
        // whole team shuffling against the same wall forever instead of
        // closing in to attack.
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
        let final_occupancy = |hex: &HexCoord| -> bool {
            orders.orders.iter().any(|o| {
                o.unit_id != unit_id && o.move_target == Some(*hex)
            })
        };

        // Prefer interior landing zones over the board's outer ring so the
        // team never corners itself against the map edge.
        let on_perimeter =
            |hex: &HexCoord| -> bool { hex.q.abs().max(hex.r.abs()).max((hex.q + hex.r).abs()) >= max_dist };

        let best = candidates
            .iter()
            .filter(|(hex, _)| !final_occupancy(hex))
            .find(|(hex, _)| !on_perimeter(hex))
            .or_else(|| {
                candidates
                    .iter()
                    .find(|(hex, _)| !final_occupancy(hex))
                    .or_else(|| candidates.first())
            });

        let (move_target, action) = if let Some((best_hex, _)) = best {
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
