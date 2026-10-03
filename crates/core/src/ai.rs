use crate::hex::HexCoord;
use crate::minion_ai::MinionAI;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::GameState;
use crate::tower_ai::TowerAI;
use crate::unit::{TeamId, UnitId, UnitKind};
use std::collections::HashSet;

pub struct GameAI;

impl GameAI {
    /// Generate orders for all units belonging to `team`.
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
                UnitKind::Tower => TowerAI::generate_order(state, unit.id),
                UnitKind::Spawner | UnitKind::NeutralGuardian => UnitOrder {
                    unit_id: unit.id,
                    move_target: None,
                    action: Action::Wait,
                },
            };

            orders.add_order(order);
        }

        orders
    }

    /// Autonomous hero AI: seeks visible enemies, advances along shortest path, attacks.
    pub fn generate_hero_order(
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

        let visible_enemies = state.visible_enemy_units(unit.team);
        let target_pool = if !visible_enemies.is_empty() {
            visible_enemies
        } else {
            state.enemy_units(unit.team)
        };

        if target_pool.is_empty() {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            };
        }

        let mut sorted_targets = target_pool;
        sorted_targets.sort_by(|a, b| {
            let dist_a = unit.pos.distance(&a.pos);
            let dist_b = unit.pos.distance(&b.pos);
            dist_a.cmp(&dist_b).then_with(|| a.id.cmp(&b.id))
        });

        let target = sorted_targets[0];
        let distance = unit.pos.distance(&target.pos);

        // In range -> attack
        if distance <= unit.attack_range {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Attack { target_id: target.id },
            };
        }

        // Maneuver towards target
        let ap_for_move = if distance <= unit.attack_range + unit.max_ap {
            unit.max_ap.saturating_sub(1)
        } else {
            unit.max_ap
        };

        let reachable = state.map.reachable_hexes(unit.pos, ap_for_move, occupied);
        let mut moves: Vec<(HexCoord, u32)> = reachable.into_iter().collect();
        moves.sort_by(|(hex_a, cost_a), (hex_b, cost_b)| {
            let dist_a = hex_a.distance(&target.pos);
            let dist_b = hex_b.distance(&target.pos);
            dist_a
                .cmp(&dist_b)
                .then_with(|| cost_a.cmp(cost_b))
                .then_with(|| (hex_a.q, hex_a.r).cmp(&(hex_b.q, hex_b.r)))
        });

        if let Some((best_hex, _)) = moves.first() {
            let new_dist = best_hex.distance(&target.pos);
            if new_dist <= unit.attack_range {
                UnitOrder {
                    unit_id,
                    move_target: Some(*best_hex),
                    action: Action::Attack { target_id: target.id },
                }
            } else {
                UnitOrder {
                    unit_id,
                    move_target: Some(*best_hex),
                    action: Action::Wait,
                }
            }
        } else {
            UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            }
        }
    }

    /// Auto-fill missing orders when timer expires. Preserves already submitted player orders.
    pub fn generate_fallback_orders(
        state: &GameState,
        team: TeamId,
        existing_orders: &TurnOrders,
    ) -> TurnOrders {
        let mut orders = existing_orders.clone();
        let occupied = state.occupied_hexes();

        for unit in state.team_units(team) {
            if !unit.is_alive() || unit.kind != UnitKind::Hero {
                continue;
            }

            if orders.get_order(unit.id).is_none() {
                let fallback = Self::generate_hero_order(state, unit.id, &occupied);
                orders.add_order(fallback);
            }
        }

        orders
    }

    /// Generate a tactical fallback order for a single hero.
    pub fn generate_fallback_order(state: &GameState, unit_id: UnitId) -> UnitOrder {
        let occupied = state.occupied_hexes();
        Self::generate_hero_order(state, unit_id, &occupied)
    }
}

pub type SimpleAI = GameAI;
