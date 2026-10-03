use crate::ai::SimpleAI;
use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::{GameState, Phase};
use crate::unit::{ATTACK_AP_COST, UnitId};
use std::collections::{HashMap, HashSet};

/// The turn processor.
/// Phase 1: initiative-ordered resolution with AP, movement (A*), and combat.
pub struct TurnProcessor;

/// Result of checking whether a unit may enter `dest` at the moment it acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryCheck {
    /// The hex is free right now — step in immediately.
    Free,
    /// A living unit sits on the hex but its own order moves it elsewhere
    /// *this same round*, so it will be gone by the end of the round.
    WillLeave,
    /// The hex is permanently blocked for this mover (the occupant stays, or
    /// another unit has already committed to landing here).
    Blocked,
}

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
        unit_ids.retain(|id| snapshot.get_unit(*id).is_some());

        // Sort by initiative (descending), then by unit_id for determinism
        unit_ids.sort_by(|a, b| {
            let unit_a = snapshot.get_unit(*a).unwrap();
            let unit_b = snapshot.get_unit(*b).unwrap();
            unit_b
                .initiative
                .cmp(&unit_a.initiative)
                .then_with(|| a.cmp(b))
        });

        // Destination claims: every planned move destination is claimed in
        // initiative order (higher initiative gets priority on contention).
        let mut claims: HashMap<HexCoord, UnitId> = HashMap::new();
        let mut pending_movers: HashSet<UnitId> = HashSet::new();
        for &id in &unit_ids {
            if let Some(o) = all_orders.get_order(id)
                && let Some(dest) = o.move_target
                && let Some(u) = snapshot.get_unit(id)
                && u.is_alive()
                && dest != u.pos
            {
                claims.entry(dest).or_insert(id);
                pending_movers.insert(id);
            }
        }

        // Process each unit in initiative order
        for unit_id in unit_ids {
            let order = all_orders.get_order(unit_id).unwrap().clone();
            // A unit that was alive at planning time may have been killed by an
            // earlier (higher-initiative) unit this round. Drop its *entire*
            // order — movement included — so corpses never "walk" across the
            // board, and release any claim it still holds on a destination.
            if !state
                .get_unit(unit_id)
                .map(|u| u.is_alive())
                .unwrap_or(false)
            {
                if let Some(dest) = order.move_target
                    && claims.get(&dest) == Some(&unit_id)
                {
                    claims.remove(&dest);
                }
                continue;
            }
            // Release this unit's claim on its own destination: it is about to
            // try to occupy it anyway, and if its move fails the claim must
            // not linger and block someone else's entry check.
            if let Some(dest) = order.move_target
                && claims.get(&dest) == Some(&unit_id)
            {
                claims.remove(&dest);
            }
            // Once a unit takes its turn, whatever hex it still stands on is
            // no longer "on loan" to anyone: any later mover that was told it
            // could land there once this unit stepped aside must now treat it
            // as a normal occupied (and blocking) hex.
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

    /// Can `mover_id` pass through / land on `hex` right now?
    ///
    /// The moving unit's own current hex never blocks it (it is standing
    /// there). A hex held by a living unit that still has to relocate this
    /// round (`pending`) is only passable for a mover whose *final*
    /// destination is that very hex: the blocker will step aside (into the
    /// mover's start hex or wherever its own path leads) and the mover will
    /// settle into the freed spot, so both plans resolve exactly as ordered.
    /// Any other use of a pending blocker's hex — merely walking through it —
    /// would require the blocker to move before it is its turn, which could
    /// scramble AP spending and initiative semantics; such steps are refused
    /// and the mover waits for the blocker to clear the way on its own turn.
    fn check_entry(
        state: &GameState,
        claims: &HashMap<HexCoord, UnitId>,
        pending: &HashSet<UnitId>,
        mover_id: UnitId,
        mover_final_dest: Option<HexCoord>,
        hex: &HexCoord,
    ) -> EntryCheck {
        if let Some(u) = state.get_unit_at(hex) {
            if u.id == mover_id {
                return EntryCheck::Free;
            }
            // Claimed as somebody else's destination means that unit is on
            // its way here; treat the hex as blocked even if the occupant is
            // also leaving, so paths never cross a committed landing zone.
            if let Some(&claimer) = claims.get(hex)
                && claimer != mover_id
            {
                return EntryCheck::Blocked;
            }
            if pending.contains(&u.id) {
                let mover_team = state.get_unit(mover_id).map(|m| m.team);
                if mover_team == Some(u.team) && mover_final_dest == Some(*hex) {
                    return EntryCheck::WillLeave;
                }
                return EntryCheck::Blocked;
            }
            return EntryCheck::Blocked;
        }
        if let Some(&claimer) = claims.get(hex)
            && claimer != mover_id
        {
            return EntryCheck::Blocked;
        }
        EntryCheck::Free
    }

    /// Attempt one step of a planned walk: `mover_id` (heading to
    /// `mover_final_dest`) tries to move from its current position onto
    /// `step`. Returns true if the step happened.
    #[allow(clippy::too_many_arguments)]
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
        match Self::check_entry(state, claims, pending, mover_id, mover_final_dest, &step) {
            EntryCheck::Free => {}
            EntryCheck::WillLeave => {
                let blocker_id = state.get_unit_at(&step).map(|u| u.id).unwrap();
                let my_pos = state.get_unit(mover_id).unwrap().pos;
                let blocker_dest = all_orders.get_order(blocker_id).and_then(|o| o.move_target);

                if blocker_dest == Some(my_pos) {
                    // Mutual cooperative swap between friendly units:
                    // Trade positions atomically.
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
                                // Blocker stepped along its path
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

                // Re-check ourselves after shuffling the blocker away.
                if Self::check_entry(state, claims, pending, mover_id, mover_final_dest, &step)
                    != EntryCheck::Free
                {
                    return false;
                }
            }
            EntryCheck::Blocked => return false,
        }

        // Commit the step.
        let from = state.get_unit(mover_id).unwrap().pos;
        if from != step {
            if let Some(u) = state.get_unit_mut(mover_id) {
                u.pos = step;
                u.spend_ap(ap_cost);
            }
            if step == mover_final_dest.unwrap_or(step) {
                // Arrived: drop any lingering claim/pending flag for us here.
                if claims.get(&step) == Some(&mover_id) {
                    claims.remove(&step);
                }
                pending.remove(&mover_id);
            } else {
                // Mid-path: our old start hex is free again for others.
                claims.remove(&from);
                claims.insert(step, mover_id);
            }
        }
        true
    }

    /// Process a single unit's orders.
    /// `snapshot` is the pre-resolution state used to validate orders planned
    /// earlier. `claims` maps each still-unresolved landing destination to
    /// the unit that will occupy it, and `pending` tracks units that have not
    /// finished relocating yet (see `resolve`).
    #[allow(clippy::too_many_arguments)]
    fn process_unit(
        state: &mut GameState,
        snapshot: &GameState,
        order: &UnitOrder,
        all_orders: &TurnOrders,
        claims: &mut HashMap<HexCoord, UnitId>,
        pending: &mut HashSet<UnitId>,
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
        if let Some(move_target) = order.move_target
            && move_target != start_pos
        {
            // Walk the geometric shortest path one hex at a time. Each
            // intermediate hex must either be free now or become free by
            // the end of the round (its occupant is moving away too).
            // Because movement costs exactly one AP per step, executing
            // the walk incrementally is equivalent to validating the whole
            // path up front — and it lets a blocked step fall back to
            // walking around instead of cancelling the entire move.
            if let Some(path) = state.map.find_path(planned_unit.pos, move_target) {
                let path_cost = (path.len() - 1) as u32; // -1 because path includes start

                if path_cost <= ap_budget {
                    let mut walked: Vec<HexCoord> = vec![planned_unit.pos];
                    for &step in &path[1..] {
                        if Self::enter_step(
                            state,
                            all_orders,
                            claims,
                            pending,
                            unit_id,
                            Some(move_target),
                            step,
                            1,
                            &mut events,
                        ) {
                            walked.push(step);
                        } else {
                            break;
                        }
                    }
                    // Report whatever progress was legally made (each step
                    // was independently valid) rather than teleporting
                    // back to the start.
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
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::UnitAttacked { .. }))
        );
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
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 1, .. }))
        );
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
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 2, .. }))
        );
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
        TurnProcessor::resolve(&mut state2, &TurnOrders::new());
        assert_eq!(state2.round, 1);
        assert_eq!(state2.phase, Phase::Planning);
    }

    #[test]
    fn test_cooperative_position_swap() {
        let mut state = base_state();
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(0, 0), 5));
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(1, 0), 3));

        let mut orders = TurnOrders::new();
        // Unit 1 wants to move to Unit 2's hex (1, 0)
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(1, 0)),
            action: Action::Wait,
        });
        // Unit 2 wants to move to Unit 1's hex (0, 0)
        orders.add_order(UnitOrder {
            unit_id: 2,
            move_target: Some(HexCoord::new(0, 0)),
            action: Action::Wait,
        });

        let events = TurnProcessor::resolve(&mut state, &orders);

        // Both units should have successfully swapped places
        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(1, 0));
        assert_eq!(state.get_unit(2).unwrap().pos, HexCoord::new(0, 0));
        assert_eq!(state.get_unit(1).unwrap().ap, 2);
        assert_eq!(state.get_unit(2).unwrap().ap, 2);

        // Both UnitMoved events should be emitted
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
        // Unit 1 has higher initiative (10 > 2)
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-1, 0), 10));
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(1, 0), 2));

        let mut orders = TurnOrders::new();
        // Both want to move to (0, 0)
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

        // Unit 1 should land at (0, 0) due to higher initiative; Unit 2 must be blocked
        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(0, 0));
        assert_eq!(state.get_unit(2).unwrap().pos, HexCoord::new(1, 0));
    }
}
