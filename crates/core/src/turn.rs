use crate::ability::{EffectKind, SpellCatalog, SpellTarget, TargetingMode};
use crate::ai::GameAI;
use crate::event::GameEvent;
use crate::hex::{HexCoord, HexMap};
use crate::lane::LaneDef;
use crate::neutral::NeutralCamp;
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::repair::execute_repair;
use crate::spawner::SpawnerSystem;
use crate::state::{GameState, Phase};
use crate::status::StatusInstance;
use crate::unit::{TeamId, Unit, UnitId, UnitKind, ATTACK_AP_COST};
use crate::vision::{compute_team_los_fog, has_line_of_sight};
use std::collections::{HashMap, HashSet};

pub struct TurnResolutionOutput {
    pub events: Vec<GameEvent>,
    pub team_0_visible_hexes: HashSet<HexCoord>,
    pub team_1_visible_hexes: HashSet<HexCoord>,
}

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

        // 1. Process Spawner waves
        let spawn_events = SpawnerSystem::process_spawns(state);
        events.extend(spawn_events);

        // Update lane waypoints for all living minions
        let lane = LaneDef::for_radius(state.map.radius);
        for unit in state.units.values_mut() {
            if unit.is_alive() && unit.kind == UnitKind::Minion {
                lane.update_minion_waypoint(unit);
            }
        }

        // Pre-resolution snapshot for stable AI planning
        let snapshot = state.clone();

        // 2. Generate AI orders for Team 1
        let team1_orders = GameAI::generate_orders(&snapshot, 1);

        // 3. Auto-fill missing orders for Team 0 heroes (Turn Timer Fallback)
        let mut complete_team0_orders =
            GameAI::generate_fallback_orders(&snapshot, 0, player_orders);

        // 4. Generate automated orders for Team 0 Minions and Towers
        let team0_auto_orders = GameAI::generate_orders(&snapshot, 0);
        for order in team0_auto_orders.orders {
            if let Some(u) = snapshot.get_unit(order.unit_id) {
                if u.kind != UnitKind::Hero {
                    complete_team0_orders.add_order(order);
                }
            }
        }

        // 5. Combine all orders
        let mut all_orders = complete_team0_orders;
        for order in team1_orders.orders {
            all_orders.add_order(order);
        }

        Self::execute_orders(state, &snapshot, all_orders, &mut events);
        events
    }

    /// Resolve simultaneous turn with authoritative multi-team orders.
    pub fn resolve_turn(state: &mut GameState, mut combined_orders: TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        state.phase = Phase::Resolution;
        events.push(GameEvent::RoundStarted {
            round: state.round + 1,
        });

        // 1. Process Spawner waves
        let spawn_events = SpawnerSystem::process_spawns(state);
        events.extend(spawn_events);

        // Update lane waypoints for all living minions
        let lane = LaneDef::for_radius(state.map.radius);
        for unit in state.units.values_mut() {
            if unit.is_alive() && unit.kind == UnitKind::Minion {
                lane.update_minion_waypoint(unit);
            }
        }

        // Pre-resolution snapshot for stable AI planning
        let snapshot = state.clone();
        let occupied = state.occupied_hexes();

        // Auto-fill automated orders for Minions and Towers for both teams if not already present
        for team in [0, 1] {
            let auto_orders = GameAI::generate_orders(&snapshot, team);
            for order in auto_orders.orders {
                if let Some(u) = snapshot.get_unit(order.unit_id) {
                    if u.kind != UnitKind::Hero && combined_orders.get_order(order.unit_id).is_none() {
                        combined_orders.add_order(order);
                    }
                }
            }
        }

        // Auto-fill orders for Neutral Guardians
        for camp in &state.neutral_camps {
            if let Some(guardian) = snapshot.get_unit(camp.guardian_id) {
                if guardian.is_alive() && combined_orders.get_order(guardian.id).is_none() {
                    let candidate_enemies: Vec<&Unit> = snapshot
                        .units
                        .values()
                        .filter(|e| {
                            e.is_alive()
                                && e.team != guardian.team
                                && (guardian.pos.distance(&e.pos) <= guardian.aggro_range
                                    || Some(e.id) == guardian.last_attacker)
                        })
                        .collect();

                    let mut sorted = candidate_enemies;
                    sorted.sort_by(|a, b| {
                        let prio_a = crate::priority::evaluate_neutral_target_priority(
                            a,
                            guardian.last_attacker,
                        );
                        let prio_b = crate::priority::evaluate_neutral_target_priority(
                            b,
                            guardian.last_attacker,
                        );
                        prio_a
                            .cmp(&prio_b)
                            .then_with(|| {
                                guardian.pos.distance(&a.pos).cmp(&guardian.pos.distance(&b.pos))
                            })
                            .then_with(|| a.hp.cmp(&b.hp))
                            .then_with(|| a.id.cmp(&b.id))
                    });

                    if let Some(target) = sorted.first() {
                        let dist = guardian.pos.distance(&target.pos);
                        if dist <= guardian.attack_range {
                            combined_orders.add_order(UnitOrder {
                                unit_id: guardian.id,
                                move_target: None,
                                action: Action::Attack {
                                    target_id: target.id,
                                },
                            });
                        } else {
                            let reachable =
                                state.map.reachable_hexes(guardian.pos, guardian.ap, &occupied);
                            let mut moves: Vec<(HexCoord, u32)> = reachable.into_iter().collect();
                            moves.sort_by(|(hex_a, cost_a), (hex_b, cost_b)| {
                                hex_a
                                    .distance(&target.pos)
                                    .cmp(&hex_b.distance(&target.pos))
                                    .then_with(|| cost_a.cmp(cost_b))
                                    .then_with(|| (hex_a.q, hex_a.r).cmp(&(hex_b.q, hex_b.r)))
                            });
                            let best = moves.first().map(|(h, _)| *h);
                            combined_orders.add_order(UnitOrder {
                                unit_id: guardian.id,
                                move_target: best,
                                action: Action::Wait,
                            });
                        }
                    } else {
                        combined_orders.add_order(UnitOrder {
                            unit_id: guardian.id,
                            move_target: None,
                            action: Action::Wait,
                        });
                    }
                }
            }
        }

        // Auto-fill fallback orders for any living heroes without an order
        for team in [0, 1] {
            for unit in snapshot.team_units(team) {
                if unit.kind == UnitKind::Hero {
                    if combined_orders.get_order(unit.id).is_none() {
                        combined_orders
                            .add_order(GameAI::generate_fallback_order(&snapshot, unit.id));
                    }
                }
            }
        }

        Self::execute_orders(state, &snapshot, combined_orders, &mut events);
        events
    }

    /// Process round deterministically over explicit unit slice, neutral camps, and lane definition.
    pub fn process_round(
        round: u32,
        map: &HexMap,
        vision_blockers: &HashSet<HexCoord>,
        units: &mut Vec<Unit>,
        camps: &mut [NeutralCamp],
        lane: &LaneDef,
        staged_orders: HashMap<UnitId, UnitOrder>,
    ) -> TurnResolutionOutput {
        let mut events = Vec::new();
        events.push(GameEvent::RoundStarted { round });

        // Stage 1: Lane Waypoint Progression for Minions
        for unit in units.iter_mut() {
            if unit.is_alive() && unit.kind == UnitKind::Minion {
                lane.update_minion_waypoint(unit);
            }
        }

        // Stage 2: Initiative Sorting for Execution
        let mut execution_order: Vec<UnitId> = units
            .iter()
            .filter(|u| u.is_alive())
            .map(|u| u.id)
            .collect();

        execution_order.sort_by(|&a_id, &b_id| {
            let u_a = units.iter().find(|u| u.id == a_id).unwrap();
            let u_b = units.iter().find(|u| u.id == b_id).unwrap();
            u_b.initiative
                .cmp(&u_a.initiative)
                .then_with(|| a_id.cmp(&b_id))
        });

        // Stage 3: Order Execution Pipeline
        let mut dead_ids = HashSet::new();
        for unit_id in execution_order {
            if dead_ids.contains(&unit_id) {
                continue;
            }

            let unit_idx = match units.iter().position(|u| u.id == unit_id) {
                Some(idx) => idx,
                None => continue,
            };

            if !units[unit_idx].is_alive() {
                continue;
            }

            let order = staged_orders.get(&unit_id).cloned().unwrap_or(UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            });

            // 3.1 Movement Resolution
            if let Some(target_pos) = order.move_target {
                let current_pos = units[unit_idx].pos;
                if target_pos != current_pos {
                    let move_cost = current_pos.distance(&target_pos);
                    let is_tile_free = !units.iter().any(|u| u.is_alive() && u.pos == target_pos);
                    let is_walkable = map.is_walkable(&target_pos);

                    if is_walkable && is_tile_free && units[unit_idx].ap >= move_cost {
                        units[unit_idx].spend_ap(move_cost);
                        units[unit_idx].pos = target_pos;

                        events.push(GameEvent::UnitMoved {
                            unit_id,
                            from: current_pos,
                            to: target_pos,
                            path: vec![current_pos, target_pos],
                            ap_spent: move_cost,
                        });
                    }
                }
            }

            // 3.2 Action Resolution
            match order.action {
                Action::Wait => {
                    events.push(GameEvent::UnitWaited { unit_id });
                }
                Action::Attack { target_id } => {
                    Self::resolve_attack(
                        unit_idx,
                        target_id,
                        units,
                        vision_blockers,
                        camps,
                        &mut dead_ids,
                        &mut events,
                    );
                }
                Action::Cast { spell_id, target } => {
                    Self::resolve_cast(
                        unit_idx,
                        &spell_id,
                        target,
                        units,
                        vision_blockers,
                        camps,
                        &mut dead_ids,
                        &mut events,
                    );
                }
                Action::Repair { target_id } => {
                    Self::resolve_repair(unit_idx, target_id, units, &mut events);
                }
            }
        }

        // Stage 4: Neutral Camp Leash & Reset Verification
        for camp in camps.iter_mut() {
            let target_pos = {
                let guardian = match units.iter().find(|u| u.id == camp.guardian_id) {
                    Some(g) => g,
                    None => continue,
                };
                guardian
                    .last_attacker
                    .and_then(|t_id| units.iter().find(|u| u.id == t_id))
                    .map(|t| t.pos)
            };
            if let Some(guardian) = units.iter_mut().find(|u| u.id == camp.guardian_id) {
                let guardian_dist = guardian.pos.distance(&camp.camp_pos);
                let target_dist = target_pos.map(|p| p.distance(&camp.camp_pos)).unwrap_or(0);
                if guardian_dist > crate::neutral::NEUTRAL_LEASH_RADIUS
                    || (target_pos.is_some() && target_dist > crate::neutral::NEUTRAL_LEASH_RADIUS)
                {
                    guardian.pos = camp.camp_pos;
                    guardian.hp = guardian.max_hp;
                    guardian.last_attacker = None;
                }
            }
        }

        // Stage 5: Round Maintenance (AP, Energy, Cooldowns, Statuses)
        for unit in units.iter_mut() {
            if !unit.is_alive() {
                continue;
            }

            unit.reset_ap();
            unit.regenerate_energy();
            unit.decrement_cooldowns();

            // Status Ticking
            let mut expired_statuses = Vec::new();
            unit.statuses.retain_mut(|status| {
                if status.tick() {
                    expired_statuses.push(status.def_id.clone());
                    false
                } else {
                    true
                }
            });

            for expired in expired_statuses {
                events.push(GameEvent::StatusExpired {
                    unit_id: unit.id,
                    status_id: expired,
                });
            }
        }

        // Stage 6: Line-of-Sight Fog Calculation
        let team_0_fog = compute_team_los_fog(vision_blockers, map.radius, units, 0);
        let team_1_fog = compute_team_los_fog(vision_blockers, map.radius, units, 1);

        TurnResolutionOutput {
            events,
            team_0_visible_hexes: team_0_fog,
            team_1_visible_hexes: team_1_fog,
        }
    }

    fn resolve_attack(
        attacker_idx: usize,
        target_id: UnitId,
        units: &mut [Unit],
        vision_blockers: &HashSet<HexCoord>,
        camps: &mut [NeutralCamp],
        dead_ids: &mut HashSet<UnitId>,
        events: &mut Vec<GameEvent>,
    ) {
        let attacker = &units[attacker_idx];
        let target_idx = match units.iter().position(|u| u.id == target_id) {
            Some(idx) => idx,
            None => return,
        };

        if !units[target_idx].is_alive() || attacker.team == units[target_idx].team {
            return;
        }

        let dist = attacker.pos.distance(&units[target_idx].pos);
        if dist > attacker.effective_attack_range() || attacker.ap < ATTACK_AP_COST {
            return;
        }

        // Check LOS for ranged attacks (range > 1)
        if dist > 1 && !has_line_of_sight(vision_blockers, attacker.pos, units[target_idx].pos) {
            return;
        }

        let attacker_id = attacker.id;
        let attacker_kind = attacker.kind;
        let attacker_team = attacker.team;
        let damage = attacker.effective_attack_damage();
        units[attacker_idx].spend_ap(ATTACK_AP_COST);

        units[target_idx].hp = units[target_idx].hp.saturating_sub(damage);
        units[target_idx].last_attacker = Some(attacker_id);
        let target_remaining_hp = units[target_idx].hp;
        let target_kind = units[target_idx].kind;

        if attacker_kind == UnitKind::Tower {
            events.push(GameEvent::TowerAttacked {
                tower_id: attacker_id,
                target_id,
                damage,
                target_hp_remaining: target_remaining_hp,
            });
        } else {
            events.push(GameEvent::UnitAttacked {
                attacker_id,
                target_id,
                damage,
                target_hp_remaining: target_remaining_hp,
            });
        }

        if target_remaining_hp == 0 && !dead_ids.contains(&target_id) {
            dead_ids.insert(target_id);
            let target_team = units[target_idx].team;
            events.push(GameEvent::UnitDied {
                unit_id: target_id,
                unit_kind: target_kind,
                killed_by: attacker_id,
            });

            handle_kill_rewards_in_slice(units, target_kind, target_team, attacker_id, events);

            if target_kind == UnitKind::NeutralGuardian {
                for camp in camps.iter_mut() {
                    if camp.guardian_id == target_id {
                        camp.handle_guardian_death(attacker_team, units, events);
                    }
                }
            }
        }
    }

    fn resolve_cast(
        caster_idx: usize,
        spell_id: &str,
        target: SpellTarget,
        units: &mut [Unit],
        vision_blockers: &HashSet<HexCoord>,
        camps: &mut [NeutralCamp],
        dead_ids: &mut HashSet<UnitId>,
        events: &mut Vec<GameEvent>,
    ) {
        let spell = match SpellCatalog::get(spell_id) {
            Some(s) => s,
            None => return,
        };

        let caster = &units[caster_idx];
        if caster.ap < spell.ap_cost
            || caster.energy < spell.energy_cost
            || !caster.is_spell_ready(spell_id)
        {
            return;
        }

        match target {
            SpellTarget::None | SpellTarget::Hex(_)
                if spell.targeting == TargetingMode::SelfOnly =>
            {
                let caster_id = caster.id;
                let caster_pos = caster.pos;
                let caster_team = caster.team;

                units[caster_idx].spend_ap(spell.ap_cost);
                units[caster_idx].spend_energy(spell.energy_cost);
                units[caster_idx].trigger_cooldown(spell_id, spell.cooldown);

                events.push(GameEvent::SpellCast {
                    caster_id,
                    spell_id: spell_id.to_string(),
                    target,
                });

                // Resolve Cleave AOE
                for effect in &spell.effects {
                    if effect.kind == EffectKind::Damage && effect.radius == Some(1) {
                        for i in 0..units.len() {
                            if units[i].is_alive()
                                && units[i].team != caster_team
                                && caster_pos.distance(&units[i].pos) == 1
                            {
                                let other_id = units[i].id;
                                let other_kind = units[i].kind;
                                units[i].hp = units[i].hp.saturating_sub(effect.amount);
                                units[i].last_attacker = Some(caster_id);
                                let other_hp = units[i].hp;

                                events.push(GameEvent::UnitAttacked {
                                    attacker_id: caster_id,
                                    target_id: other_id,
                                    damage: effect.amount,
                                    target_hp_remaining: other_hp,
                                });

                                if other_hp == 0 && !dead_ids.contains(&other_id) {
                                    dead_ids.insert(other_id);
                                    let other_team = units[i].team;
                                    events.push(GameEvent::UnitDied {
                                        unit_id: other_id,
                                        unit_kind: other_kind,
                                        killed_by: caster_id,
                                    });

                                    handle_kill_rewards_in_slice(units, other_kind, other_team, caster_id, events);

                                    if other_kind == UnitKind::NeutralGuardian {
                                        for camp in camps.iter_mut() {
                                            if camp.guardian_id == other_id {
                                                camp.handle_guardian_death(
                                                    caster_team,
                                                    units,
                                                    events,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if effect.kind == EffectKind::ApplyStatus {
                        if let Some(ref status_def) = effect.status {
                            units[caster_idx]
                                .statuses
                                .retain(|s| s.def_id != status_def.id);
                            units[caster_idx]
                                .statuses
                                .push(crate::status::StatusInstance::from_def(status_def));

                            events.push(GameEvent::StatusApplied {
                                unit_id: caster_id,
                                status_id: status_def.id.clone(),
                                duration_rounds: status_def.duration_rounds,
                            });
                        }
                    }
                }
            }
            SpellTarget::Unit(target_id) => {
                let target_idx = match units.iter().position(|u| u.id == target_id) {
                    Some(idx) => idx,
                    None => return,
                };

                if !units[target_idx].is_alive() {
                    return;
                }

                let caster_pos = units[caster_idx].pos;
                let target_pos = units[target_idx].pos;
                let caster_team = units[caster_idx].team;
                let target_team = units[target_idx].team;
                let dist = caster_pos.distance(&target_pos);

                if dist < spell.min_range || dist > spell.range {
                    return;
                }

                if spell.targeting == TargetingMode::EnemyUnit && target_team == caster_team {
                    return;
                }
                if spell.targeting == TargetingMode::AllyUnit && target_team != caster_team {
                    return;
                }

                if spell.requires_line_of_sight
                    && !has_line_of_sight(vision_blockers, caster_pos, target_pos)
                {
                    return;
                }

                let caster_id = units[caster_idx].id;
                units[caster_idx].spend_ap(spell.ap_cost);
                units[caster_idx].spend_energy(spell.energy_cost);
                units[caster_idx].trigger_cooldown(spell_id, spell.cooldown);

                events.push(GameEvent::SpellCast {
                    caster_id,
                    spell_id: spell_id.to_string(),
                    target,
                });

                for effect in &spell.effects {
                    match effect.kind {
                        EffectKind::Damage => {
                            let target_kind = units[target_idx].kind;
                            units[target_idx].hp =
                                units[target_idx].hp.saturating_sub(effect.amount);
                            units[target_idx].last_attacker = Some(caster_id);
                            let target_hp = units[target_idx].hp;

                            events.push(GameEvent::UnitAttacked {
                                attacker_id: caster_id,
                                target_id,
                                damage: effect.amount,
                                target_hp_remaining: target_hp,
                            });

                            if target_hp == 0 && !dead_ids.contains(&target_id) {
                                dead_ids.insert(target_id);
                                let target_team = units[target_idx].team;
                                events.push(GameEvent::UnitDied {
                                    unit_id: target_id,
                                    unit_kind: target_kind,
                                    killed_by: caster_id,
                                });

                                handle_kill_rewards_in_slice(units, target_kind, target_team, caster_id, events);

                                if target_kind == UnitKind::NeutralGuardian {
                                    for camp in camps.iter_mut() {
                                        if camp.guardian_id == target_id {
                                            camp.handle_guardian_death(caster_team, units, events);
                                        }
                                    }
                                }
                            }
                        }
                        EffectKind::Heal => {
                            let original = units[target_idx].hp;
                            units[target_idx].hp =
                                (units[target_idx].hp + effect.amount).min(units[target_idx].max_hp);
                            let healed = units[target_idx].hp - original;

                            events.push(GameEvent::HealApplied {
                                caster_id,
                                target_id,
                                amount: healed,
                                target_hp_remaining: units[target_idx].hp,
                            });
                        }
                        EffectKind::ApplyStatus => {
                            if let Some(ref status_def) = effect.status {
                                units[target_idx]
                                    .statuses
                                    .retain(|s| s.def_id != status_def.id);
                                units[target_idx]
                                    .statuses
                                    .push(StatusInstance::from_def(status_def));

                                events.push(GameEvent::StatusApplied {
                                    unit_id: target_id,
                                    status_id: status_def.id.clone(),
                                    duration_rounds: status_def.duration_rounds,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn resolve_repair(
        repairer_idx: usize,
        target_id: UnitId,
        units: &mut [Unit],
        events: &mut Vec<GameEvent>,
    ) {
        let target_idx = match units.iter().position(|u| u.id == target_id) {
            Some(idx) => idx,
            None => return,
        };

        if repairer_idx == target_idx {
            return;
        }

        let (repairer, target) = if repairer_idx < target_idx {
            let (left, right) = units.split_at_mut(target_idx);
            (&mut left[repairer_idx], &mut right[0])
        } else {
            let (left, right) = units.split_at_mut(repairer_idx);
            (&mut right[0], &mut left[target_idx])
        };

        if let Some(event) = execute_repair(repairer, target) {
            events.push(event);
        }
    }

    fn execute_orders(
        state: &mut GameState,
        snapshot: &GameState,
        all_orders: TurnOrders,
        events: &mut Vec<GameEvent>,
    ) {
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

        for unit_id in unit_ids {
            let order = all_orders.get_order(unit_id).unwrap().clone();

            if !state.get_unit(unit_id).map(|u| u.is_alive()).unwrap_or(false) {
                if let Some(dest) = order.move_target {
                    if claims.get(&dest) == Some(&unit_id) {
                        claims.remove(&dest);
                    }
                }
                continue;
            }

            if let Some(dest) = order.move_target {
                if claims.get(&dest) == Some(&unit_id) {
                    claims.remove(&dest);
                }
            }
            pending_movers.remove(&unit_id);

            let unit_events = Self::process_unit(
                state,
                snapshot,
                &order,
                &all_orders,
                &mut claims,
                &mut pending_movers,
            );
            events.extend(unit_events);
        }

        // Leash check for neutral camps
        for camp in state.neutral_camps.iter_mut() {
            let target_pos = state
                .units
                .get(&camp.guardian_id)
                .and_then(|g| g.last_attacker)
                .and_then(|t_id| state.units.get(&t_id))
                .map(|t| t.pos);

            if let Some(guardian) = state.units.get_mut(&camp.guardian_id) {
                let guardian_dist = guardian.pos.distance(&camp.camp_pos);
                let target_dist = target_pos.map(|p| p.distance(&camp.camp_pos)).unwrap_or(0);
                if guardian_dist > crate::neutral::NEUTRAL_LEASH_RADIUS
                    || (target_pos.is_some() && target_dist > crate::neutral::NEUTRAL_LEASH_RADIUS)
                {
                    guardian.pos = camp.camp_pos;
                    guardian.hp = guardian.max_hp;
                    guardian.last_attacker = None;
                }
            }
        }

        // Round Maintenance
        let mut sorted_unit_ids: Vec<UnitId> = state.units.keys().copied().collect();
        sorted_unit_ids.sort_unstable();
        for id in sorted_unit_ids {
            if let Some(unit) = state.units.get_mut(&id) {
                if unit.is_alive() {
                    unit.reset_ap();
                    unit.regenerate_energy();
                    unit.decrement_cooldowns();

                    let mut expired = Vec::new();
                    unit.statuses.retain_mut(|s| {
                        if s.tick() {
                            expired.push(s.def_id.clone());
                            false
                        } else {
                            true
                        }
                    });

                    for exp in expired {
                        events.push(GameEvent::StatusExpired {
                            unit_id: unit.id,
                            status_id: exp,
                        });
                    }
                }
            }
        }

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

            if occ.is_stationary() {
                return EntryCheck::Blocked;
            }

            if let Some(&claimant) = claims.get(&hex) {
                if claimant != mover_id {
                    return EntryCheck::Blocked;
                }
            }

            if pending.contains(&occ.id) {
                let mover_team = state.get_unit(mover_id).map(|m| m.team);
                if mover_team == Some(occ.team) && mover_final_dest == Some(hex) {
                    return EntryCheck::WillLeave;
                }
                return EntryCheck::Blocked;
            }

            return EntryCheck::Blocked;
        }

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
                if dest != start_pos
                    && let Some(full_path) = snapshot.map.find_path(start_pos, dest)
                {
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

        // Phase B: Action Resolution
        let blockers = state.map.vision_blockers();
        match &order.action {
            Action::Wait => {
                events.push(GameEvent::UnitWaited { unit_id });
            }
            Action::Attack { target_id } => {
                let attack_events = Self::process_attack(state, unit_id, *target_id, &blockers, claims);
                events.extend(attack_events);
            }
            Action::Cast { spell_id, target } => {
                Self::process_cast(state, unit_id, spell_id, target, &blockers, &mut events);
            }
            Action::Repair { target_id } => {
                Self::process_repair(state, unit_id, *target_id, &mut events);
            }
        }

        events
    }

    fn process_attack(
        state: &mut GameState,
        attacker_id: UnitId,
        target_id: UnitId,
        vision_blockers: &HashSet<HexCoord>,
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

        if attacker.team == target.team {
            return events;
        }

        let dist = attacker.pos.distance(&target.pos);
        if dist > attacker.effective_attack_range() || attacker.ap < ATTACK_AP_COST {
            return events;
        }

        // Check LOS for ranged attacks (dist > 1)
        if dist > 1 && !has_line_of_sight(vision_blockers, attacker.pos, target.pos) {
            return events;
        }

        let damage = attacker.effective_attack_damage();
        let attacker_team = attacker.team;
        let is_tower = attacker.kind == UnitKind::Tower;

        let attacker_mut = state.get_unit_mut(attacker_id).unwrap();
        attacker_mut.spend_ap(ATTACK_AP_COST);

        let (target_remaining_hp, target_dead, target_kind, target_team) = {
            let target_mut = state.get_unit_mut(target_id).unwrap();
            target_mut.hp = target_mut.hp.saturating_sub(damage);
            target_mut.last_attacker = Some(attacker_id);
            (target_mut.hp, target_mut.hp == 0, target_mut.kind, target_mut.team)
        };

        if is_tower {
            events.push(GameEvent::TowerAttacked {
                tower_id: attacker_id,
                target_id,
                damage,
                target_hp_remaining: target_remaining_hp,
            });
        } else {
            events.push(GameEvent::UnitAttacked {
                attacker_id,
                target_id,
                damage,
                target_hp_remaining: target_remaining_hp,
            });
        }

        if target_dead {
            state.units.remove(&target_id);
            claims.retain(|_, &mut claimant| claimant != target_id);

            events.push(GameEvent::UnitDied {
                unit_id: target_id,
                unit_kind: target_kind,
                killed_by: attacker_id,
            });

            handle_kill_rewards_in_state(state, target_kind, target_team, attacker_id, &mut events);

            if target_kind == UnitKind::NeutralGuardian {
                for camp in state.neutral_camps.iter_mut() {
                    if camp.guardian_id == target_id {
                        let mut all_units: Vec<Unit> = state.units.values().cloned().collect();
                        camp.handle_guardian_death(attacker_team, &mut all_units, &mut events);
                        for u in all_units {
                            state.units.insert(u.id, u);
                        }
                    }
                }
            }
        }

        events
    }

    fn process_cast(
        state: &mut GameState,
        caster_id: UnitId,
        spell_id: &str,
        target: &SpellTarget,
        vision_blockers: &HashSet<HexCoord>,
        events: &mut Vec<GameEvent>,
    ) {
        let spell = match SpellCatalog::get(spell_id) {
            Some(s) => s,
            None => return,
        };

        let caster = match state.get_unit(caster_id) {
            Some(c) if c.is_alive() => c,
            _ => return,
        };

        if caster.ap < spell.ap_cost
            || caster.energy < spell.energy_cost
            || !caster.is_spell_ready(spell_id)
        {
            return;
        }

        match target {
            SpellTarget::None | SpellTarget::Hex(_)
                if spell.targeting == TargetingMode::SelfOnly =>
            {
                let caster_pos = caster.pos;
                let caster_team = caster.team;

                let caster_mut = state.get_unit_mut(caster_id).unwrap();
                caster_mut.spend_ap(spell.ap_cost);
                caster_mut.spend_energy(spell.energy_cost);
                caster_mut.trigger_cooldown(spell_id, spell.cooldown);

                events.push(GameEvent::SpellCast {
                    caster_id,
                    spell_id: spell_id.to_string(),
                    target: target.clone(),
                });

                for effect in &spell.effects {
                    if effect.kind == EffectKind::Damage && effect.radius == Some(1) {
                        let candidate_ids: Vec<UnitId> = state
                            .units
                            .values()
                            .filter(|u| {
                                u.is_alive()
                                    && u.team != caster_team
                                    && caster_pos.distance(&u.pos) == 1
                            })
                            .map(|u| u.id)
                            .collect();

                        for other_id in candidate_ids {
                            let (dead, kind, other_team) = {
                                let other = state.get_unit_mut(other_id).unwrap();
                                other.hp = other.hp.saturating_sub(effect.amount);
                                other.last_attacker = Some(caster_id);
                                (other.hp == 0, other.kind, other.team)
                            };

                            let other_hp = state.get_unit(other_id).map(|u| u.hp).unwrap_or(0);
                            events.push(GameEvent::UnitAttacked {
                                attacker_id: caster_id,
                                target_id: other_id,
                                damage: effect.amount,
                                target_hp_remaining: other_hp,
                            });

                            if dead {
                                state.units.remove(&other_id);
                                events.push(GameEvent::UnitDied {
                                    unit_id: other_id,
                                    unit_kind: kind,
                                    killed_by: caster_id,
                                });

                                handle_kill_rewards_in_state(state, kind, other_team, caster_id, events);

                                if kind == UnitKind::NeutralGuardian {
                                    for camp in state.neutral_camps.iter_mut() {
                                        if camp.guardian_id == other_id {
                                            let mut all_units: Vec<Unit> =
                                                state.units.values().cloned().collect();
                                            camp.handle_guardian_death(caster_team, &mut all_units, events);
                                            for u in all_units {
                                                state.units.insert(u.id, u);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if effect.kind == EffectKind::ApplyStatus {
                        if let Some(ref status_def) = effect.status {
                            let caster = state.get_unit_mut(caster_id).unwrap();
                            caster.statuses.retain(|s| s.def_id != status_def.id);
                            caster.statuses.push(crate::status::StatusInstance::from_def(status_def));

                            events.push(GameEvent::StatusApplied {
                                unit_id: caster_id,
                                status_id: status_def.id.clone(),
                                duration_rounds: status_def.duration_rounds,
                            });
                        }
                    }
                }
            }
            SpellTarget::Unit(target_id) => {
                let target_unit = match state.get_unit(*target_id) {
                    Some(t) if t.is_alive() => t,
                    _ => return,
                };

                let caster_pos = caster.pos;
                let target_pos = target_unit.pos;
                let caster_team = caster.team;
                let target_team = target_unit.team;
                let target_kind = target_unit.kind;
                let dist = caster_pos.distance(&target_pos);

                if dist < spell.min_range || dist > spell.range {
                    return;
                }

                if spell.targeting == TargetingMode::EnemyUnit && target_team == caster_team {
                    return;
                }
                if spell.targeting == TargetingMode::AllyUnit && target_team != caster_team {
                    return;
                }

                if spell.requires_line_of_sight
                    && !has_line_of_sight(vision_blockers, caster_pos, target_pos)
                {
                    return;
                }

                let caster_mut = state.get_unit_mut(caster_id).unwrap();
                caster_mut.spend_ap(spell.ap_cost);
                caster_mut.spend_energy(spell.energy_cost);
                caster_mut.trigger_cooldown(spell_id, spell.cooldown);

                events.push(GameEvent::SpellCast {
                    caster_id,
                    spell_id: spell_id.to_string(),
                    target: target.clone(),
                });

                for effect in &spell.effects {
                    match effect.kind {
                        EffectKind::Damage => {
                            let dead = {
                                let t_mut = state.get_unit_mut(*target_id).unwrap();
                                t_mut.hp = t_mut.hp.saturating_sub(effect.amount);
                                t_mut.last_attacker = Some(caster_id);
                                t_mut.hp == 0
                            };

                            let t_hp = state.get_unit(*target_id).map(|u| u.hp).unwrap_or(0);
                            events.push(GameEvent::UnitAttacked {
                                attacker_id: caster_id,
                                target_id: *target_id,
                                damage: effect.amount,
                                target_hp_remaining: t_hp,
                            });

                            if dead {
                                state.units.remove(target_id);
                                events.push(GameEvent::UnitDied {
                                    unit_id: *target_id,
                                    unit_kind: target_kind,
                                    killed_by: caster_id,
                                });

                                handle_kill_rewards_in_state(state, target_kind, target_team, caster_id, events);

                                if target_kind == UnitKind::NeutralGuardian {
                                    for camp in state.neutral_camps.iter_mut() {
                                        if camp.guardian_id == *target_id {
                                            let mut all_units: Vec<Unit> =
                                                state.units.values().cloned().collect();
                                            camp.handle_guardian_death(caster_team, &mut all_units, events);
                                            for u in all_units {
                                                state.units.insert(u.id, u);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        EffectKind::Heal => {
                            let (healed, new_hp) = {
                                let t_mut = state.get_unit_mut(*target_id).unwrap();
                                let old_hp = t_mut.hp;
                                t_mut.hp = (t_mut.hp + effect.amount).min(t_mut.max_hp);
                                (t_mut.hp - old_hp, t_mut.hp)
                            };

                            events.push(GameEvent::HealApplied {
                                caster_id,
                                target_id: *target_id,
                                amount: healed,
                                target_hp_remaining: new_hp,
                            });
                        }
                        EffectKind::ApplyStatus => {
                            if let Some(ref status_def) = effect.status {
                                let t_mut = state.get_unit_mut(*target_id).unwrap();
                                t_mut.statuses.retain(|s| s.def_id != status_def.id);
                                t_mut.statuses.push(StatusInstance::from_def(status_def));

                                events.push(GameEvent::StatusApplied {
                                    unit_id: *target_id,
                                    status_id: status_def.id.clone(),
                                    duration_rounds: status_def.duration_rounds,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn process_repair(
        state: &mut GameState,
        repairer_id: UnitId,
        target_id: UnitId,
        events: &mut Vec<GameEvent>,
    ) {
        if repairer_id == target_id {
            return;
        }

        let repairer = match state.get_unit(repairer_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let target = match state.get_unit(target_id) {
            Some(t) => t.clone(),
            None => return,
        };

        if crate::repair::validate_repair(&repairer, &target).is_err() {
            return;
        }

        let repairer_mut = state.get_unit_mut(repairer_id).unwrap();
        repairer_mut.spend_ap(crate::repair::REPAIR_AP_COST);

        let target_mut = state.get_unit_mut(target_id).unwrap();
        let old_hp = target_mut.hp;
        target_mut.hp = (target_mut.hp + crate::repair::REPAIR_AMOUNT).min(target_mut.max_hp);
        let amount = target_mut.hp - old_hp;

        events.push(GameEvent::StructureRepaired {
            repairer_id,
            target_id,
            amount,
            target_hp_remaining: target_mut.hp,
        });
    }
}

pub fn handle_kill_rewards_in_state(
    state: &mut GameState,
    victim_kind: UnitKind,
    victim_team: TeamId,
    killer_id: UnitId,
    events: &mut Vec<GameEvent>,
) {
    if victim_kind == UnitKind::Tower || victim_kind == UnitKind::Spawner {
        let beneficiary_team = 1 - victim_team;
        let (gold, xp, reason) = if victim_kind == UnitKind::Tower {
            (25, 20, hexabellum_protocol::RewardReason::TowerDestroyed)
        } else {
            (30, 25, hexabellum_protocol::RewardReason::SpawnerDestroyed)
        };

        let mut living_allies: Vec<UnitId> = state
            .units
            .values()
            .filter(|u| u.team == beneficiary_team && u.is_hero() && u.is_alive())
            .map(|u| u.id)
            .collect();
        living_allies.sort_unstable();

        for ally_id in living_allies {
            if let Some(hero) = state.units.get_mut(&ally_id) {
                hero.gold += gold;
                crate::progression::grant_xp(hero, xp, events);
                events.push(GameEvent::RewardGranted {
                    unit_id: ally_id,
                    gold,
                    xp,
                    reason,
                });
            }
        }
    }

    let (gold, xp, reason) = match victim_kind {
        UnitKind::Hero => (30, 30, hexabellum_protocol::RewardReason::HeroKill),
        UnitKind::Minion => (10, 10, hexabellum_protocol::RewardReason::MinionKill),
        UnitKind::NeutralGuardian => (40, 25, hexabellum_protocol::RewardReason::NeutralKill),
        _ => (0, 0, hexabellum_protocol::RewardReason::PassiveIncome),
    };

    if (gold > 0 || xp > 0) && let Some(killer) = state.units.get_mut(&killer_id) {
        if killer.is_alive() && killer.is_hero() {
            killer.gold += gold;
            crate::progression::grant_xp(killer, xp, events);
            events.push(GameEvent::RewardGranted {
                unit_id: killer.id,
                gold,
                xp,
                reason,
            });
        }
    }
}

pub fn handle_kill_rewards_in_slice(
    units: &mut [Unit],
    victim_kind: UnitKind,
    victim_team: TeamId,
    killer_id: UnitId,
    events: &mut Vec<GameEvent>,
) {
    if victim_kind == UnitKind::Tower || victim_kind == UnitKind::Spawner {
        let beneficiary_team = 1 - victim_team;
        let (gold, xp, reason) = if victim_kind == UnitKind::Tower {
            (25, 20, hexabellum_protocol::RewardReason::TowerDestroyed)
        } else {
            (30, 25, hexabellum_protocol::RewardReason::SpawnerDestroyed)
        };

        for i in 0..units.len() {
            if units[i].team == beneficiary_team && units[i].is_hero() && units[i].is_alive() {
                units[i].gold += gold;
                let uid = units[i].id;
                crate::progression::grant_xp(&mut units[i], xp, events);
                events.push(GameEvent::RewardGranted {
                    unit_id: uid,
                    gold,
                    xp,
                    reason,
                });
            }
        }
    }

    let (gold, xp, reason) = match victim_kind {
        UnitKind::Hero => (30, 30, hexabellum_protocol::RewardReason::HeroKill),
        UnitKind::Minion => (10, 10, hexabellum_protocol::RewardReason::MinionKill),
        UnitKind::NeutralGuardian => (40, 25, hexabellum_protocol::RewardReason::NeutralKill),
        _ => (0, 0, hexabellum_protocol::RewardReason::PassiveIncome),
    };

    if (gold > 0 || xp > 0) && let Some(killer) = units.iter_mut().find(|u| u.id == killer_id) {
        if killer.is_alive() && killer.is_hero() {
            killer.gold += gold;
            let uid = killer.id;
            crate::progression::grant_xp(killer, xp, events);
            events.push(GameEvent::RewardGranted {
                unit_id: uid,
                gold,
                xp,
                reason,
            });
        }
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
        let u1 = Unit::new_hero(1, 0, HexCoord::new(0, 0), 10);
        state.add_unit(u1);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(2, 0)),
            action: Action::Wait,
        });

        let events = TurnProcessor::resolve(&mut state, &orders);
        let u = state.get_unit(1).unwrap();
        assert_eq!(u.pos, HexCoord::new(2, 0));

        let moved_event = events
            .iter()
            .find(|e| matches!(e, GameEvent::UnitMoved { .. }));
        assert!(moved_event.is_some());
    }

    #[test]
    fn test_destination_claims_favor_higher_initiative() {
        let mut state = base_state();
        let u1 = Unit::new_hero(1, 0, HexCoord::new(0, 1), 10);
        let u2 = Unit::new_hero(2, 1, HexCoord::new(0, -1), 5);
        state.add_unit(u1);
        state.add_unit(u2);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(0, 0)),
            action: Action::Wait,
        });
        orders.add_order(UnitOrder {
            unit_id: 2,
            move_target: Some(HexCoord::new(0, 0)),
            action: Action::Wait,
        });

        TurnProcessor::resolve(&mut state, &orders);
        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(0, 0));
        assert_eq!(state.get_unit(2).unwrap().pos, HexCoord::new(0, -1));
    }

    #[test]
    fn test_cooperative_position_swap() {
        let mut state = base_state();
        let u1 = Unit::new_hero(1, 0, HexCoord::new(0, 0), 10);
        let u2 = Unit::new_hero(2, 0, HexCoord::new(1, 0), 5);
        state.add_unit(u1);
        state.add_unit(u2);

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

        let move_count = events
            .iter()
            .filter(|e| matches!(e, GameEvent::UnitMoved { .. }))
            .count();
        assert_eq!(move_count, 2);
    }

    #[test]
    fn test_dead_unit_cannot_complete_attack_after_move() {
        let mut state = base_state();
        let mut fast_killer = Unit::new_hero(1, 0, HexCoord::new(0, 0), 10);
        fast_killer.attack_damage = 100;
        let slow_victim = Unit::new_hero(2, 1, HexCoord::new(1, 0), 5);
        state.add_unit(fast_killer);
        state.add_unit(slow_victim);

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
        assert!(state.get_unit(2).is_none());
        assert_eq!(state.get_unit(1).unwrap().hp, 100);

        let died = events
            .iter()
            .find(|e| matches!(e, GameEvent::UnitDied { unit_id: 2, .. }));
        assert!(died.is_some());
    }

    #[test]
    fn test_attack_in_range_deals_damage_and_kills() {
        let mut state = base_state();
        let u1 = Unit::new_hero(1, 0, HexCoord::new(0, 0), 10);
        let u2 = Unit::new_hero(2, 1, HexCoord::new(1, 0), 5);
        state.add_unit(u1);
        state.add_unit(u2);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Attack { target_id: 2 },
        });

        let events = TurnProcessor::resolve(&mut state, &orders);
        assert_eq!(state.get_unit(2).unwrap().hp, 80);

        let attack_event = events
            .iter()
            .find(|e| matches!(e, GameEvent::UnitAttacked { .. }));
        assert!(attack_event.is_some());
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
        assert_eq!(state.get_unit(2).unwrap().hp, 100);
        assert!(!events
            .iter()
            .any(|e| matches!(e, GameEvent::UnitAttacked { .. })));
    }

    #[test]
    fn test_ai_moves_toward_and_attacks_player() {
        let mut state = base_state();
        let player = Unit::new_hero(1, 0, HexCoord::new(0, 0), 5);
        let ai = Unit::new_hero(2, 1, HexCoord::new(2, 0), 10);
        state.add_unit(player);
        state.add_unit(ai);

        let orders = TurnOrders::new();
        let events = TurnProcessor::resolve(&mut state, &orders);

        let ai_moved = events
            .iter()
            .any(|e| matches!(e, GameEvent::UnitMoved { unit_id: 2, .. }));
        let ai_attacked = events
            .iter()
            .any(|e| matches!(e, GameEvent::UnitAttacked { attacker_id: 2, .. }));

        assert!(ai_moved || ai_attacked);
    }

    #[test]
    fn test_move_beyond_ap_budget_rejected() {
        let mut state = base_state();
        let u1 = Unit::new_hero(1, 0, HexCoord::new(0, 0), 10);
        state.add_unit(u1);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(5, 0)),
            action: Action::Wait,
        });

        let events = TurnProcessor::resolve(&mut state, &orders);
        assert_eq!(state.get_unit(1).unwrap().pos, HexCoord::new(0, 0));
        assert!(!events
            .iter()
            .any(|e| matches!(e, GameEvent::UnitMoved { .. })));
    }

    #[test]
    fn test_initiative_order_determines_who_acts_first() {
        let mut state = base_state();
        let u1 = Unit::new_hero(1, 0, HexCoord::new(0, 0), 1);
        let u2 = Unit::new_hero(2, 1, HexCoord::new(2, 0), 100);
        state.add_unit(u1);
        state.add_unit(u2);

        let mut orders = TurnOrders::new();
        orders.add_order(UnitOrder {
            unit_id: 1,
            move_target: Some(HexCoord::new(1, 0)),
            action: Action::Wait,
        });
        orders.add_order(UnitOrder {
            unit_id: 2,
            move_target: Some(HexCoord::new(1, 0)),
            action: Action::Wait,
        });

        let mut state2 = state.clone();
        let o2 = orders.clone();

        TurnProcessor::resolve(&mut state2, &o2);
        assert_eq!(state2.get_unit(2).unwrap().pos, HexCoord::new(1, 0));
        assert_eq!(state2.get_unit(1).unwrap().pos, HexCoord::new(0, 0));
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
}
