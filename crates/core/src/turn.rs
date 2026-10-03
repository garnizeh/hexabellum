use crate::ai::SimpleAI;
use crate::event::GameEvent;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::{GameState, Phase};
use crate::unit::{UnitId, ATTACK_AP_COST};

/// The turn processor.
/// Phase 1: initiative-ordered resolution with AP, movement (A*), and combat.
pub struct TurnProcessor;

impl TurnProcessor {
    /// Resolve the current round.
    pub fn resolve(state: &mut GameState, player_orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        state.phase = Phase::Resolution;
        events.push(GameEvent::RoundStarted {
            round: state.round + 1,
        });

        // Reset AP for all units
        state.reset_all_ap();

        // Snapshot the pre-resolution state so order validation and resolution
        // use identical assumptions (positions / AP as they were during planning).
        let snapshot = state.clone();

        // Generate AI orders for team 1
        let ai_orders = SimpleAI::generate_orders(&snapshot, 1);

        // Combine all orders
        let mut all_orders = player_orders.clone();
        for order in ai_orders.orders {
            all_orders.add_order(order);
        }

        // Collect all unit IDs that have orders, sorted by initiative.
        // Dedup + sort by id first so the result is independent of HashMap order.
        let mut unit_ids: Vec<UnitId> = all_orders.orders.iter().map(|o| o.unit_id).collect();
        unit_ids.sort_unstable();
        unit_ids.dedup();

        // Sort by initiative (descending), then by unit_id for determinism
        unit_ids.sort_by(|a, b| {
            let unit_a = snapshot.get_unit(*a).unwrap();
            let unit_b = snapshot.get_unit(*b).unwrap();
            unit_b
                .initiative
                .cmp(&unit_a.initiative)
                .then_with(|| a.cmp(b))
        });

        // Process each unit in initiative order
        for unit_id in unit_ids {
            let order = all_orders.get_order(unit_id).unwrap().clone();
            let unit_events = Self::process_unit(state, &snapshot, &order);
            events.extend(unit_events);
        }

        // Check win condition
        if let Some(winner) = state.check_winner() {
            state.winner = Some(winner);
            state.phase = Phase::MatchEnd;
            events.push(GameEvent::MatchEnded {
                winner: Some(winner),
            });
        } else {
            state.round += 1;
            state.phase = Phase::Planning;
            events.push(GameEvent::RoundEnded { round: state.round });
        }

        events
    }

    /// Process a single unit's orders.
    /// `snapshot` is the pre-resolution state used to validate orders planned earlier.
    fn process_unit(
        state: &mut GameState,
        snapshot: &GameState,
        order: &UnitOrder,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        // Check if unit is alive
        let unit = match state.get_unit(order.unit_id) {
            Some(u) if u.is_alive() => u,
            _ => return events, // Dead units skip
        };

        let unit_id = order.unit_id;
        let start_pos = unit.pos;
        // Validate movement against the planning-time position/AP (snapshot):
        // an earlier move this round must not invalidate a later unit's order.
        let planned_unit = match snapshot.get_unit(unit_id) {
            Some(u) if u.is_alive() => u,
            _ => return events,
        };
        let ap_budget = planned_unit.ap.max(unit.ap);

        // Phase 1: Movement
        if let Some(move_target) = order.move_target {
            if move_target != start_pos {
                // The moving unit's own hex must not block its path
                let mut occupied = state.occupied_hexes();
                occupied.remove(&start_pos);

                let from_planned = planned_unit.pos == start_pos;
                let path_start = if from_planned { start_pos } else { planned_unit.pos };
                let mut occupied_planned = occupied.clone();
                occupied_planned.remove(&planned_unit.pos);
                let occ_ref = if from_planned { &occupied } else { &occupied_planned };

                if let Some(path) = state.map.find_path(path_start, move_target, occ_ref) {
                    let path_cost = (path.len() - 1) as u32; // -1 because path includes start

                    if path_cost <= ap_budget {
                        // Move the unit
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

        // Phase 2: Action
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

        // Get attacker and target
        let (attacker, target) = match (state.get_unit(attacker_id), state.get_unit(target_id)) {
            (Some(a), Some(t)) => (a, t),
            _ => return events,
        };

        if !attacker.is_alive() || !target.is_alive() {
            return events;
        }

        // Cannot attack allies / self
        if attacker.team == target.team || attacker_id == target_id {
            return events;
        }

        // Check range from the attacker's current (possibly moved) position
        let distance = attacker.pos.distance(&target.pos);
        if distance > attacker.attack_range {
            return events; // Out of range, attack fails
        }

        // Check AP (use post-move AP; snapshot guarantees budget was validated at order time)
        if !attacker.can_afford(ATTACK_AP_COST) {
            return events; // Not enough AP
        }

        // Capture damage before taking a mutable borrow
        let damage = attacker.attack_damage;

        // Spend AP
        if let Some(attacker) = state.get_unit_mut(attacker_id) {
            attacker.spend_ap(ATTACK_AP_COST);
        }

        // Apply damage
        let target_hp_remaining;

        if let Some(target) = state.get_unit_mut(target_id) {
            target.hp = target.hp.saturating_sub(damage);
            target_hp_remaining = target.hp;
        } else {
            return events;
        }

        events.push(GameEvent::UnitAttacked {
            attacker_id,
            target_id,
            damage,
            target_hp_remaining,
        });

        // Handle death immediately so later units see the updated board
        if target_hp_remaining == 0 {
            state.units.remove(&target_id);
            events.push(GameEvent::UnitDied {
                unit_id: target_id,
                killed_by: attacker_id,
            });
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::{HexCoord, HexMap};
    use crate::unit::Unit;

    fn base_state() -> GameState {
        GameState::new(HexMap::new(4))
    }

    #[test]
    fn test_movement_along_path_costs_ap() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-3, 0), 3));

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(-1, 0)),
            action: Action::Wait,
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(-1, 0));
        assert_eq!(state.get_unit(1).unwrap().ap, 1); // 3 - 2 move cost
        let moved = events
            .iter()
            .find(|e| matches!(e, GameEvent::UnitMoved { .. }))
            .unwrap();
        match moved {
            GameEvent::UnitMoved { path, ap_spent, .. } => {
                assert_eq!(*ap_spent, 2);
                assert_eq!(path.len(), 3); // includes start
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_move_beyond_ap_budget_rejected() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-3, 0), 3));

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(3, 0)), // distance 6 > AP 3
            action: Action::Wait,
        });

        TurnProcessor::resolve(&mut state, &orders);
        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(-3, 0));
    }

    #[test]
    fn test_attack_in_range_deals_damage_and_kills() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(0, 0), 5));
        let mut victim = Unit::new_hero(2, 1, HexCoord::new(1, 0), 1);
        victim.hp = 20; // one hit from dying
        state.add_unit(victim);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Attack { target_id: 2 },
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::UnitAttacked {
                attacker_id: 1,
                target_id: 2,
                damage: 20,
                target_hp_remaining: 0
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::UnitDied {
                unit_id: 2,
                killed_by: 1
            }
        )));
        assert!(state.get_unit(2).is_none()); // removed from board
    }

    #[test]
    fn test_attack_out_of_range_fails() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-3, 0), 5));
        state.add_unit(Unit::new_hero(2, 1, HexCoord::new(3, 0), 1));

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Attack { target_id: 2 },
        });

        let events = TurnProcessor::resolve(&mut state, &orders);
        assert!(!events.iter().any(|e| matches!(e, GameEvent::UnitAttacked { .. })));
        assert_eq!(state.get_unit(2).unwrap().hp, 100);
    }

    #[test]
    fn test_initiative_order_determines_who_acts_first() {
        let mut state = base_state();
        // Unit 2 has higher initiative; it must act first and kill unit 1
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(0, 0), 10));
        let mut victim = Unit::new_hero(1, 1, HexCoord::new(1, 0), 1);
        victim.hp = 20;
        state.add_unit(victim);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 2,
            move_target: None,
            action: Action::Attack { target_id: 1 },
        });
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Attack { target_id: 2 },
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        // Unit 2 acts first (initiative 10 > 1) and kills unit 1
        let attacked_idx = events
            .iter()
            .position(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 2, .. }))
            .unwrap();
        let died_idx = events
            .iter()
            .position(|e| matches!(e, GameEvent::UnitDied { unit_id: 1, .. }))
            .unwrap();
        assert!(attacked_idx < died_idx);
        // Unit 1 dies before acting -> no attack event from it
        assert!(!events
            .iter()
            .any(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 1, .. })));
        assert_eq!(state.get_unit(2).unwrap().hp, 100);
    }

    #[test]
    fn test_dead_unit_cannot_complete_attack_after_move() {
        let mut state = base_state();
        let mut killer = Unit::new_hero(1, 0, HexCoord::new(-2, 0), 10);
        killer.attack_damage = 100;
        state.add_unit(killer);
        let mut victim = Unit::new_hero(2, 1, HexCoord::new(-1, 0), 1);
        victim.hp = 20;
        state.add_unit(victim);

        // Victim plans to move away and attack the killer (but will die first)
        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Attack { target_id: 2 },
        });
        orders.add_order(UnitOrder {
            unit_id: 2,
            move_target: Some(HexCoord::new(-3, 1)),
            action: Action::Attack { target_id: 1 },
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        assert!(state.get_unit(2).is_none());
        assert!(!events
            .iter()
            .any(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 2, .. })));
    }

    #[test]
    fn test_ai_moves_toward_and_attacks_player() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(0, 0), 1));
        state.add_unit(Unit::new_hero(2, 1, HexCoord::new(3, 0), 2));

        // Player does nothing
        let orders = TurnOrders::new();
        let events = TurnProcessor::resolve(&mut state, &orders);

        // AI unit 2 should have moved closer and attacked unit 1
        assert!(events.iter().any(
            |e| matches!(e, GameEvent::UnitAttacked { attacker_id: 2, target_id: 1, .. })
        ));
        assert_eq!(state.get_unit(1).unwrap().hp, 80);
    }

    #[test]
    fn test_win_condition_and_round_increment() {
        let mut state = base_state();
        let mut killer = Unit::new_hero(1, 0, HexCoord::new(0, 0), 10);
        killer.attack_damage = 100;
        state.add_unit(killer);
        let mut victim = Unit::new_hero(2, 1, HexCoord::new(1, 0), 1);
        victim.hp = 20;
        state.add_unit(victim);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Attack { target_id: 2 },
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        assert_eq!(state.winner, Some(0));
        assert_eq!(state.phase, Phase::MatchEnd);
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::MatchEnded { winner: Some(0) })));

        // Normal round increments
        let mut state2 = base_state();
        state2.add_unit(Unit::new_hero(1, 0, HexCoord::new(0, 0), 1));
        state2.add_unit(Unit::new_hero(2, 1, HexCoord::new(4, 0), 1));
        TurnProcessor::resolve(&mut state2, &TurnOrders::new());
        assert_eq!(state2.round, 1);
        assert_eq!(state2.phase, Phase::Planning);
    }
}
