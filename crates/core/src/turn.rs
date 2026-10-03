use crate::ai::GameAI;
use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::spawner::SpawnerSystem;
use crate::state::{GameState, Phase};
use crate::unit::{UnitId, UnitKind, ATTACK_AP_COST};
use std::collections::{HashMap, HashSet};

pub struct TurnProcessor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryCheck {
    Free,
    WillLeave,
    Blocked,
}

impl TurnProcessor {
    /// Resolve the simultaneous turn deterministically with full cooperative movement.
    pub fn resolve(state: &mut GameState, player_orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        state.phase = Phase::Resolution;
        events.push(GameEvent::RoundStarted {
            round: state.round + 1,
        });

        // 1. Reset AP for all units
        state.reset_all_ap();

        // 2. Process Spawner waves
        let spawn_events = SpawnerSystem::process_spawns(state);
        events.extend(spawn_events);

        // Pre-resolution snapshot for stable AI planning
        let snapshot = state.clone();

        // 3. Generate AI orders for Team 1
        let team1_orders = GameAI::generate_orders(&snapshot, 1);

        // 4. Auto-fill missing orders for Team 0 heroes (Turn Timer Fallback)
        let mut complete_team0_orders =
            GameAI::generate_fallback_orders(&snapshot, 0, player_orders);

        // 5. Generate automated orders for Team 0 Minions and Towers
        let team0_auto_orders = GameAI::generate_orders(&snapshot, 0);
        for order in team0_auto_orders.orders {
            if let Some(u) = snapshot.get_unit(order.unit_id) {
                if u.kind != UnitKind::Hero {
                    complete_team0_orders.add_order(order);
                }
            }
        }

        // 6. Combine all orders
        let mut all_orders = complete_team0_orders;
        for order in team1_orders.orders {
            all_orders.add_order(order);
        }

        // 7. Sort unit IDs by initiative DESC, unit_id ASC
        let mut unit_ids: Vec<UnitId> = all_orders.orders.iter().map(|o| o.unit_id).collect();
        unit_ids.sort_unstable();
        unit_ids.dedup();
        unit_ids.retain(|id| snapshot.get_unit(*id).is_some());

        unit_ids.sort_by(|a, b| {
            let unit_a = snapshot.get_unit(*a).unwrap();
            let unit_b = snapshot.get_unit(*b).unwrap();
            unit_b
                .initiative
                .cmp(&unit_a.initiative)
                .then_with(|| a.cmp(b))
        });

        // 8. Register Destination Claims & Pending Movers (Cooperative Movement Protocol)
        let mut claims: HashMap<HexCoord, UnitId> = HashMap::new();
        let mut pending_movers: HashSet<UnitId> = HashSet::new();

        for &id in &unit_ids {
            if let Some(order) = all_orders.get_order(id) {
                if let Some(dest) = order.move_target {
                    if let Some(u) = snapshot.get_unit(id) {
                        if u.is_alive() && !u.is_stationary() && dest != u.pos {
                            claims.entry(dest).or_insert(id);
                            pending_movers.insert(id);
                        }
                    }
                }
            }
        }

        // 9. Process each unit in initiative order
        for unit_id in unit_ids {
            let order = all_orders.get_order(unit_id).unwrap().clone();

            // Skip if dead
            if !state.get_unit(unit_id).map(|u| u.is_alive()).unwrap_or(false) {
                if let Some(dest) = order.move_target {
                    if claims.get(&dest) == Some(&unit_id) {
                        claims.remove(&dest);
                    }
                }
                continue;
            }

            // Release own destination claim as it executes
            if let Some(dest) = order.move_target {
                if claims.get(&dest) == Some(&unit_id) {
                    claims.remove(&dest);
                }
            }
            pending_movers.remove(&unit_id);

            let unit_events = Self::process_unit(
                state,
                &snapshot,
                &order,
                &all_orders,
                &mut claims,
                &mut pending_movers,
            );
            events.extend(unit_events);
        }

        // 10. Update Fog of War
        state.update_fog();
        for team in 0..2 {
            let mut visible: Vec<HexCoord> =
                state.fog.visible_hexes(team).iter().copied().collect();
            visible.sort_by_key(|h| (h.q, h.r));
            events.push(GameEvent::FogUpdated {
                team,
                visible_hexes: visible,
            });
        }

        // 11. Evaluate MOBA Dual Victory Conditions
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

    fn check_entry(
        state: &GameState,
        claims: &HashMap<HexCoord, UnitId>,
        pending: &HashSet<UnitId>,
        mover_id: UnitId,
        mover_final_dest: Option<HexCoord>,
        hex: HexCoord,
    ) -> EntryCheck {
        if !state.map.is_walkable(&hex) {
            return EntryCheck::Blocked;
        }

        if let Some(occ) = state.get_unit_at(&hex) {
            if occ.id == mover_id {
                return EntryCheck::Free;
            }

            // Structures can never move or be passed through
            if occ.is_stationary() {
                return EntryCheck::Blocked;
            }

            if let Some(&claimant) = claims.get(&hex) {
                if claimant != mover_id {
                    return EntryCheck::Blocked;
                }
            }

            // Friendly unit stepping aside on its own turn
            if pending.contains(&occ.id) {
                let mover_team = state.get_unit(mover_id).map(|m| m.team);
                if mover_team == Some(occ.team) && mover_final_dest == Some(hex) {
                    return EntryCheck::WillLeave;
                }
                return EntryCheck::Blocked;
            }

            return EntryCheck::Blocked;
        }

        // Cannot enter hex claimed by someone else
        if let Some(&claimant) = claims.get(&hex) {
            if claimant != mover_id {
                return EntryCheck::Blocked;
            }
        }

        EntryCheck::Free
    }

    fn enter_step(
        state: &mut GameState,
        all_orders: &TurnOrders,
        claims: &mut HashMap<HexCoord, UnitId>,
        pending: &mut HashSet<UnitId>,
        mover_id: UnitId,
        mover_final_dest: Option<HexCoord>,
        step: HexCoord,
        ap_cost: u32,
        events: &mut Vec<GameEvent>,
    ) -> bool {
        match Self::check_entry(state, claims, pending, mover_id, mover_final_dest, step) {
            EntryCheck::Free => {}
            EntryCheck::WillLeave => {
                let blocker_id = state.get_unit_at(&step).map(|u| u.id).unwrap();
                let my_pos = state.get_unit(mover_id).unwrap().pos;
                let blocker_dest = all_orders.get_order(blocker_id).and_then(|o| o.move_target);

                if blocker_dest == Some(my_pos) {
                    // Mutual cooperative swap between friendly units
                    if let Some(blocker) = state.get_unit_mut(blocker_id) {
                        blocker.pos = my_pos;
                        blocker.spend_ap(ap_cost);
                    }
                    if let Some(mover) = state.get_unit_mut(mover_id) {
                        mover.pos = step;
                        mover.spend_ap(ap_cost);
                    }
                    claims.remove(&step);
                    claims.remove(&my_pos);
                    pending.remove(&mover_id);
                    pending.remove(&blocker_id);

                    events.push(GameEvent::UnitMoved {
                        unit_id: blocker_id,
                        from: step,
                        to: my_pos,
                        path: vec![step, my_pos],
                        ap_spent: ap_cost,
                    });
                    return true;
                } else if let Some(b_dest) = blocker_dest {
                    let b_pos = state.get_unit(blocker_id).unwrap().pos;
                    if let Some(b_path) = state.map.find_path(b_pos, b_dest) {
                        if b_path.len() > 1 {
                            let next_b_step = b_path[1];
                            if next_b_step != my_pos
                                && Self::enter_step(
                                    state,
                                    all_orders,
                                    claims,
                                    pending,
                                    blocker_id,
                                    Some(b_dest),
                                    next_b_step,
                                    ap_cost,
                                    events,
                                )
                            {
                                // Blocker successfully stepped along its path
                            } else {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }

                if Self::check_entry(state, claims, pending, mover_id, mover_final_dest, step)
                    != EntryCheck::Free
                {
                    return false;
                }
            }
            EntryCheck::Blocked => return false,
        }

        // Commit step
        let from = state.get_unit(mover_id).unwrap().pos;
        if from != step {
            if let Some(u) = state.get_unit_mut(mover_id) {
                u.pos = step;
                u.spend_ap(ap_cost);
            }
            if step == mover_final_dest.unwrap_or(step) {
                if claims.get(&step) == Some(&mover_id) {
                    claims.remove(&step);
                }
                pending.remove(&mover_id);
            } else {
                claims.remove(&from);
                claims.insert(step, mover_id);
            }
        }
        true
    }

    fn process_unit(
        state: &mut GameState,
        snapshot: &GameState,
        order: &UnitOrder,
        all_orders: &TurnOrders,
        claims: &mut HashMap<HexCoord, UnitId>,
        pending_movers: &mut HashSet<UnitId>,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        let unit_id = order.unit_id;
        let unit = match state.get_unit(unit_id) {
            Some(u) if u.is_alive() => u,
            _ => return events,
        };

        let is_stationary = unit.is_stationary();
        let start_pos = unit.pos;

        // Phase A: Movement (Mobile units only)
        if !is_stationary {
            if let Some(dest) = order.move_target {
                if dest != start_pos {
                    if let Some(full_path) = snapshot.map.find_path(start_pos, dest) {
                        let path_cost = (full_path.len() - 1) as u32;
                        let planned_unit = snapshot.get_unit(unit_id).unwrap();
                        let ap_budget = planned_unit.ap.max(unit.ap);

                        if path_cost <= ap_budget {
                            let mut walked = vec![start_pos];
                            for &step in &full_path[1..] {
                                if Self::enter_step(
                                    state,
                                    all_orders,
                                    claims,
                                    pending_movers,
                                    unit_id,
                                    Some(dest),
                                    step,
                                    1,
                                    &mut events,
                                ) {
                                    walked.push(step);
                                } else {
                                    break;
                                }
                            }

                            let final_pos = state.get_unit(unit_id).unwrap().pos;
                            if final_pos != start_pos {
                                let ap_spent = (walked.len() - 1) as u32;
                                events.push(GameEvent::UnitMoved {
                                    unit_id,
                                    from: start_pos,
                                    to: final_pos,
                                    path: walked,
                                    ap_spent,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Phase B: Combat Action
        match order.action {
            Action::Wait => {
                events.push(GameEvent::UnitWaited { unit_id });
            }
            Action::Attack { target_id } => {
                let attack_events = Self::process_attack(state, unit_id, target_id, claims);
                events.extend(attack_events);
            }
        }

        events
    }

    fn process_attack(
        state: &mut GameState,
        attacker_id: UnitId,
        target_id: UnitId,
        claims: &mut HashMap<HexCoord, UnitId>,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        let attacker = match state.get_unit(attacker_id) {
            Some(a) if a.is_alive() => a,
            _ => return events,
        };
        let target = match state.get_unit(target_id) {
            Some(t) if t.is_alive() => t,
            _ => return events,
        };

        if attacker.team == target.team || attacker_id == target_id {
            return events;
        }

        // Range check
        if attacker.pos.distance(&target.pos) > attacker.attack_range {
            return events;
        }

        // AP check
        if !attacker.can_afford(ATTACK_AP_COST) {
            return events;
        }

        let is_tower = attacker.kind == UnitKind::Tower;
        let damage = attacker.attack_damage;

        let attacker_mut = state.get_unit_mut(attacker_id).unwrap();
        attacker_mut.spend_ap(ATTACK_AP_COST);

        let target_mut = state.get_unit_mut(target_id).unwrap();
        target_mut.hp = target_mut.hp.saturating_sub(damage);
        let target_hp_remaining = target_mut.hp;
        let target_dead = target_hp_remaining == 0;
        let target_kind = target_mut.kind;

        if is_tower {
            events.push(GameEvent::TowerAttacked {
                tower_id: attacker_id,
                target_id,
                damage,
                target_hp_remaining,
            });
        } else {
            events.push(GameEvent::UnitAttacked {
                attacker_id,
                target_id,
                damage,
                target_hp_remaining,
            });
        }

        // Immediate Casualty Removal
        if target_dead {
            state.units.remove(&target_id);

            // Clean up destination claim held by dead unit
            claims.retain(|_, &mut claimant| claimant != target_id);

            events.push(GameEvent::UnitDied {
                unit_id: target_id,
                unit_kind: target_kind,
                killed_by: attacker_id,
            });
        }

        events
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hex::{HexCoord, HexMap};
    use crate::unit::Unit;

    fn base_state() -> GameState {
        GameState::new(HexMap::new(6))
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
        assert_eq!(state.get_unit(1).unwrap().ap, 3);
    }

    #[test]
    fn test_dead_unit_cannot_complete_attack_after_move() {
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

        assert!(state.get_unit(2).is_none());
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 2, .. }))
        );
    }

    #[test]
    fn test_initiative_order_determines_who_acts_first() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(0, 0), 5));
        state.add_unit(Unit::new_hero(2, 1, HexCoord::new(1, 0), 10));

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Attack { target_id: 2 },
        });
        orders.add_order(UnitOrder {
            unit_id: 2,
            move_target: None,
            action: Action::Attack { target_id: 1 },
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        let atk2_idx = events
            .iter()
            .position(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 2, .. }))
            .unwrap();
        let atk1_idx = events
            .iter()
            .position(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 1, .. }))
            .unwrap();

        assert!(atk2_idx < atk1_idx);
    }

    #[test]
    fn test_attack_in_range_deals_damage_and_kills() {
        let mut state = base_state();
        let mut u1 = Unit::new_hero(1, 0, HexCoord::new(0, 0), 1);
        u1.attack_damage = 25;
        state.add_unit(u1);
        let mut u2 = Unit::new_hero(2, 1, HexCoord::new(1, 0), 1);
        u2.hp = 20;
        state.add_unit(u2);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Attack { target_id: 2 },
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        assert!(state.get_unit(2).is_none());
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::UnitDied {
                unit_id: 2,
                killed_by: 1,
                ..
            }
        )));
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
    fn test_ai_moves_toward_and_attacks_player() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(0, 0), 1));
        state.add_unit(Unit::new_hero(2, 1, HexCoord::new(3, 0), 2));
        state.update_fog();

        // Player waits
        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Wait,
        });
        let events = TurnProcessor::resolve(&mut state, &orders);

        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::UnitAttacked {
                attacker_id: 2,
                target_id: 1,
                ..
            }
        )));
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
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::MatchEnded { winner: Some(0) }))
        );

        // Normal round increments
        let mut state2 = base_state();
        state2.add_unit(Unit::new_hero(1, 0, HexCoord::new(0, 0), 1));
        state2.add_unit(Unit::new_hero(2, 1, HexCoord::new(4, 0), 1));
        let mut o2 = TurnOrders::new();
        o2.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Wait,
        });
        TurnProcessor::resolve(&mut state2, &o2);
        assert_eq!(state2.round, 1);
        assert_eq!(state2.phase, Phase::Planning);
    }

    #[test]
    fn test_cooperative_position_swap() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(0, 0), 5));
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(1, 0), 3));

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(1, 0)),
            action: Action::Wait,
        });
        orders.add_order(UnitOrder {
            unit_id: 2,
            move_target: Some(HexCoord::new(0, 0)),
            action: Action::Wait,
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(1, 0));
        assert_eq!(state.get_unit(2).unwrap().pos, HexCoord::new(0, 0));
        assert_eq!(state.get_unit(1).unwrap().ap, 2);
        assert_eq!(state.get_unit(2).unwrap().ap, 2);

        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::UnitMoved {
                unit_id: 1,
                from: HexCoord { q: 0, r: 0 },
                to: HexCoord { q: 1, r: 0 },
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::UnitMoved {
                unit_id: 2,
                from: HexCoord { q: 1, r: 0 },
                to: HexCoord { q: 0, r: 0 },
                ..
            }
        )));
    }

    #[test]
    fn test_destination_claims_favor_higher_initiative() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-1, 0), 10));
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(1, 0), 2));

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 2,
            move_target: Some(HexCoord::new(0, 0)),
            action: Action::Wait,
        });
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(0, 0)),
            action: Action::Wait,
        });

        TurnProcessor::resolve(&mut state, &orders);

        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(0, 0));
        assert_eq!(state.get_unit(2).unwrap().pos, HexCoord::new(1, 0));
    }
}
