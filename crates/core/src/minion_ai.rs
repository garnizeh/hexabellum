use crate::hex::HexCoord;
use crate::orders::{Action, UnitOrder};
use crate::state::GameState;
use crate::unit::{Unit, UnitId};
use std::collections::HashSet;

pub struct MinionAI;

impl MinionAI {
    /// Generate autonomous tactical order for a minion.
    pub fn generate_order(
        state: &GameState,
        unit_id: UnitId,
        occupied: &HashSet<HexCoord>,
    ) -> UnitOrder {
        let unit = match state.get_unit(unit_id) {
            Some(u) if u.is_alive() => u,
            _ => {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Wait,
                }
            }
        };

        // Minions only target enemies visible to their team's Fog of War
        let visible_enemies = state.visible_enemy_units(unit.team);

        if let Some(target) = Self::select_target(unit, &visible_enemies) {
            let distance = unit.pos.distance(&target.pos);

            // 1. In attack range immediately -> attack without moving
            if distance <= unit.attack_range {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Attack { target_id: target.id },
                };
            }

            // 2. Move towards target within AP budget (reserve 1 AP for attack if reachable)
            let reachable = state.map.reachable_hexes(unit.pos, unit.ap, occupied);
            let mut moves: Vec<(HexCoord, u32)> = reachable.into_iter().collect();

            // Find closest approach hex to the target
            moves.sort_by(|(hex_a, cost_a), (hex_b, cost_b)| {
                let dist_a = hex_a.distance(&target.pos);
                let dist_b = hex_b.distance(&target.pos);
                dist_a
                    .cmp(&dist_b)
                    .then_with(|| cost_a.cmp(cost_b))
                    .then_with(|| (hex_a.q, hex_a.r).cmp(&(hex_b.q, hex_b.r)))
            });

            if let Some((best_hex, cost)) = moves.first() {
                let new_dist = best_hex.distance(&target.pos);
                if new_dist <= unit.attack_range && *cost < unit.ap {
                    // Move and attack in the same round
                    return UnitOrder {
                        unit_id,
                        move_target: Some(*best_hex),
                        action: Action::Attack { target_id: target.id },
                    };
                } else {
                    // Just maneuver closer
                    return UnitOrder {
                        unit_id,
                        move_target: Some(*best_hex),
                        action: Action::Wait,
                    };
                }
            }
        }

        // 3. No enemies in range/vision -> push down the lane toward enemy base
        Self::advance_down_lane(state, unit, occupied)
    }

    /// Multi-tier target priority: Minions > Heroes > Structures, with (PriorityClass, dist, hp, unit_id) tie-breaking.
    pub fn select_target<'a>(unit: &Unit, enemies: &[&'a Unit]) -> Option<&'a Unit> {
        let mut sorted: Vec<&'a Unit> = enemies.to_vec();
        sorted.sort_by(|a, b| {
            let prio_a = crate::priority::evaluate_minion_target_priority(a);
            let prio_b = crate::priority::evaluate_minion_target_priority(b);
            let dist_a = unit.pos.distance(&a.pos);
            let dist_b = unit.pos.distance(&b.pos);
            prio_a
                .cmp(&prio_b)
                .then_with(|| dist_a.cmp(&dist_b))
                .then_with(|| a.hp.cmp(&b.hp))
                .then_with(|| a.id.cmp(&b.id))
        });
        sorted.first().copied()
    }

    /// Move toward the next lane waypoint along the central corridor, falling back to enemy base.
    fn advance_down_lane(
        state: &GameState,
        unit: &Unit,
        occupied: &HashSet<HexCoord>,
    ) -> UnitOrder {
        let lane = crate::lane::LaneDef::central_lane();
        let goal_pos = if let Some(idx) = unit.waypoint_index {
            lane.waypoints.get(idx).copied().unwrap_or_else(|| {
                let goal_q = if unit.team == 0 {
                    state.map.radius as i32
                } else {
                    -(state.map.radius as i32)
                };
                HexCoord::new(goal_q, 0)
            })
        } else {
            let goal_q = if unit.team == 0 {
                state.map.radius as i32
            } else {
                -(state.map.radius as i32)
            };
            HexCoord::new(goal_q, 0)
        };

        let reachable = state.map.reachable_hexes(unit.pos, unit.ap, occupied);
        let mut moves: Vec<(HexCoord, u32)> = reachable.into_iter().collect();

        moves.sort_by(|(hex_a, cost_a), (hex_b, cost_b)| {
            let dist_a = hex_a.distance(&goal_pos);
            let dist_b = hex_b.distance(&goal_pos);
            dist_a
                .cmp(&dist_b)
                .then_with(|| cost_a.cmp(cost_b))
                .then_with(|| (hex_a.q, hex_a.r).cmp(&(hex_b.q, hex_b.r)))
        });

        if let Some((best_hex, _)) = moves.first() {
            UnitOrder {
                unit_id: unit.id,
                move_target: Some(*best_hex),
                action: Action::Wait,
            }
        } else {
            UnitOrder {
                unit_id: unit.id,
                move_target: None,
                action: Action::Wait,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::Unit;

    #[test]
    fn test_minion_target_priority_and_tie_breaking() {
        let minion = Unit::new_minion(1, 0, HexCoord::new(0, 0));

        let enemy_structure = Unit::new_tower(10, 1, HexCoord::new(1, 0)); // dist 1
        let enemy_hero = Unit::new_hero(20, 1, HexCoord::new(1, 0), 2); // dist 1
        let _enemy_minion_far = Unit::new_minion(30, 1, HexCoord::new(2, 0)); // dist 2
        let enemy_minion_near1 = Unit::new_minion(32, 1, HexCoord::new(1, 0)); // dist 1, id 32
        let enemy_minion_near2 = Unit::new_minion(31, 1, HexCoord::new(1, 0)); // dist 1, id 31

        // 1. Minions prioritized over Heroes and Structures even if equidistant
        let pool = vec![&enemy_structure, &enemy_hero, &enemy_minion_near1];
        let target = MinionAI::select_target(&minion, &pool);
        assert_eq!(target.unwrap().id, 32);

        // 2. Deterministic tie-breaking on (dist, id ASC)
        let pool2 = vec![&enemy_minion_near1, &enemy_minion_near2];
        let target2 = MinionAI::select_target(&minion, &pool2);
        assert_eq!(target2.unwrap().id, 31); // id 31 < 32

        // 3. Fallback to Heroes when no minions present
        let pool3 = vec![&enemy_structure, &enemy_hero];
        let target3 = MinionAI::select_target(&minion, &pool3);
        assert_eq!(target3.unwrap().id, 20);

        // 4. Fallback to Structures when no heroes or minions present
        let pool4 = vec![&enemy_structure];
        let target4 = MinionAI::select_target(&minion, &pool4);
        assert_eq!(target4.unwrap().id, 10);
    }
}
