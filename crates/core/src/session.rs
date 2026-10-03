use crate::ai::GameAI;
use crate::event::GameEvent;
use crate::hex::{HexCoord, HexMap};
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::{GameState, Phase};
use crate::turn::TurnProcessor;
use crate::unit::{Unit, UnitId, UnitKind};
use hexabellum_protocol::{
    ActionDto, HexDto, MapDto, OrderDto, PlayerId, ProtocolErrorCode,
    SanitizedGameEvent, SnapshotDto, TeamId, UnitDto,
};
use std::collections::{HashMap, HashSet};

/// Entity controller designation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Controller {
    Player(PlayerId),
    Ai,
    Automatic,
}

/// Detailed error information for rejected order submissions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderSubmissionError {
    pub code: ProtocolErrorCode,
    pub unit_id: Option<UnitId>,
    pub reason: String,
}

impl PartialEq<ProtocolErrorCode> for OrderSubmissionError {
    fn eq(&self, other: &ProtocolErrorCode) -> bool {
        self.code == *other
    }
}

/// Match configuration parameters.
#[derive(Debug, Clone)]
pub struct BattleConfig {
    pub map_radius: u32,
    pub turn_duration_secs: u64,
    pub spawn_interval: u32,
    pub enable_ai_team_1: bool,
    pub early_resolution_grace_ms: u64,
}

impl Default for BattleConfig {
    fn default() -> Self {
        Self {
            map_radius: 6,
            turn_duration_secs: 30,
            spawn_interval: 3,
            enable_ai_team_1: false,
            early_resolution_grace_ms: 1000,
        }
    }
}

/// Headless authoritative match session.
pub struct BattleSession {
    pub match_id: String,
    pub state: GameState,
    pub controllers: HashMap<UnitId, Controller>,
    pub staged_orders: HashMap<TeamId, HashMap<UnitId, UnitOrder>>,
    pub submitted_teams: HashSet<TeamId>,
    pub config: BattleConfig,
    pub unit_registry: HashMap<UnitId, (TeamId, UnitKind, HexCoord)>,
}

impl BattleSession {
    /// Initialize a new 3v3 MOBA battle session with lane topology.
    pub fn new(match_id: String, config: BattleConfig) -> Self {
        let mut map = HexMap::new(config.map_radius);

        // Phase 4 Terrain Topology
        // Dense Stone Wall: (0, 2) & (0, -2) (blocks move, blocks vision)
        map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(0, 2)));
        map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(0, -2)));

        // Smoke Pillar: (2, 2) & (-2, -2) (blocks vision only, allows movement)
        map.add_obstacle(crate::vision::Obstacle::smoke(HexCoord::new(2, 2)));
        map.add_obstacle(crate::vision::Obstacle::smoke(HexCoord::new(-2, -2)));

        // Low Boulders: (0, 1) & (0, -1) (blocks move only, allows sight)
        map.add_obstacle(crate::vision::Obstacle::boulder(HexCoord::new(0, 1)));
        map.add_obstacle(crate::vision::Obstacle::boulder(HexCoord::new(0, -1)));

        let mut state = GameState::new(map);
        let mut controllers = HashMap::new();

        // Team 0 Base (Left)
        let t0_spawner = Unit::new_spawner(10, 0, HexCoord::new(-5, 0), config.spawn_interval);
        let t0_tower = Unit::new_tower(11, 0, HexCoord::new(-3, 0));
        controllers.insert(10, Controller::Automatic);
        controllers.insert(11, Controller::Automatic);
        state.add_unit(t0_spawner);
        state.add_unit(t0_tower);

        // Team 0 Heroes: Vanguard, Ranger, Warden
        state.add_unit(Unit::new_vanguard(1, 0, HexCoord::new(-4, -1), 3));
        state.add_unit(Unit::new_ranger(2, 0, HexCoord::new(-4, 0), 2));
        state.add_unit(Unit::new_warden(3, 0, HexCoord::new(-4, 1), 1));
        controllers.insert(1, Controller::Ai);
        controllers.insert(2, Controller::Ai);
        controllers.insert(3, Controller::Ai);

        // Team 1 Base (Right)
        let t1_spawner = Unit::new_spawner(20, 1, HexCoord::new(5, 0), config.spawn_interval);
        let t1_tower = Unit::new_tower(21, 1, HexCoord::new(3, 0));
        controllers.insert(20, Controller::Automatic);
        controllers.insert(21, Controller::Automatic);
        state.add_unit(t1_spawner);
        state.add_unit(t1_tower);

        // Team 1 Heroes: Vanguard, Ranger, Warden
        state.add_unit(Unit::new_vanguard(4, 1, HexCoord::new(4, -1), 3));
        state.add_unit(Unit::new_ranger(5, 1, HexCoord::new(4, 0), 2));
        state.add_unit(Unit::new_warden(6, 1, HexCoord::new(4, 1), 1));
        controllers.insert(4, Controller::Ai);
        controllers.insert(5, Controller::Ai);
        controllers.insert(6, Controller::Ai);

        // Neutral Camps & Guardians
        let guardian_alpha = Unit::new_neutral_guardian(31, HexCoord::new(0, 3));
        let guardian_beta = Unit::new_neutral_guardian(32, HexCoord::new(0, -3));
        controllers.insert(31, Controller::Automatic);
        controllers.insert(32, Controller::Automatic);
        state.add_unit(guardian_alpha);
        state.add_unit(guardian_beta);

        state.neutral_camps.push(crate::neutral::NeutralCamp::new(
            "camp_alpha".to_string(),
            HexCoord::new(0, 3),
            31,
        ));
        state.neutral_camps.push(crate::neutral::NeutralCamp::new(
            "camp_beta".to_string(),
            HexCoord::new(0, -3),
            32,
        ));

        state.next_unit_id = 35;

        // Initialize vision
        state.update_fog();

        let mut unit_registry = HashMap::new();
        for unit in state.units.values() {
            unit_registry.insert(unit.id, (unit.team, unit.kind, unit.pos));
        }

        Self {
            match_id,
            state,
            controllers,
            staged_orders: HashMap::new(),
            submitted_teams: HashSet::new(),
            config,
            unit_registry,
        }
    }

    /// Assign player to all living heroes on a team.
    pub fn assign_team_player(&mut self, team: TeamId, player_id: PlayerId) {
        for unit in self.state.units.values() {
            if unit.team == team && unit.kind == UnitKind::Hero {
                self.controllers
                    .insert(unit.id, Controller::Player(player_id.clone()));
            }
        }
    }

    /// Submit turn orders from a player.
    pub fn submit_player_orders(
        &mut self,
        player_id: &PlayerId,
        team: TeamId,
        round: u32,
        orders: Vec<OrderDto>,
    ) -> Result<(), OrderSubmissionError> {
        if self.state.phase != Phase::Planning {
            return Err(OrderSubmissionError {
                code: ProtocolErrorCode::InvalidPhase,
                unit_id: None,
                reason: "Orders can only be submitted during Planning phase".into(),
            });
        }
        if self.state.round != round {
            return Err(OrderSubmissionError {
                code: ProtocolErrorCode::StaleRound,
                unit_id: None,
                reason: format!(
                    "Stale round: match is at round {}, received orders for round {}",
                    self.state.round, round
                ),
            });
        }

        let mut validated_orders = HashMap::new();

        for dto in orders {
            let unit = match self.state.get_unit(dto.unit_id) {
                Some(u) => u,
                None => {
                    return Err(OrderSubmissionError {
                        code: ProtocolErrorCode::UnitNotOwned,
                        unit_id: Some(dto.unit_id),
                        reason: format!("Unit #{} does not exist", dto.unit_id),
                    });
                }
            };

            if !unit.is_alive() {
                return Err(OrderSubmissionError {
                    code: ProtocolErrorCode::UnitDead,
                    unit_id: Some(dto.unit_id),
                    reason: format!("Unit #{} is dead", dto.unit_id),
                });
            }
            if unit.team != team {
                return Err(OrderSubmissionError {
                    code: ProtocolErrorCode::UnitNotOwned,
                    unit_id: Some(dto.unit_id),
                    reason: format!("Unit #{} does not belong to team {}", dto.unit_id, team),
                });
            }

            // Verify controller ownership
            match self.controllers.get(&dto.unit_id) {
                Some(Controller::Player(owner)) if owner == player_id => {}
                _ => {
                    return Err(OrderSubmissionError {
                        code: ProtocolErrorCode::NotAuthorized,
                        unit_id: Some(dto.unit_id),
                        reason: format!(
                            "Player {} is not authorized to order unit #{}",
                            player_id, dto.unit_id
                        ),
                    });
                }
            }

            let move_target = dto.move_target.map(|h| HexCoord::new(h.q, h.r));
            let action = match dto.action {
                ActionDto::Wait => Action::Wait,
                ActionDto::Attack { target_id } => {
                    let target = match self.state.get_unit(target_id) {
                        Some(t) => t,
                        None => {
                            return Err(OrderSubmissionError {
                                code: ProtocolErrorCode::InvalidTarget,
                                unit_id: Some(dto.unit_id),
                                reason: format!("Attack target #{} does not exist", target_id),
                            });
                        }
                    };
                    if !target.is_alive() || target.team == team {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: format!(
                                "Invalid attack target #{}: dead or friendly unit",
                                target_id
                            ),
                        });
                    }
                    Action::Attack { target_id }
                }
                ActionDto::Cast {
                    spell_id,
                    target: target_dto,
                } => {
                    let spell = match crate::ability::SpellCatalog::get(&spell_id) {
                        Some(s) => s,
                        None => {
                            return Err(OrderSubmissionError {
                                code: ProtocolErrorCode::InvalidMessage,
                                unit_id: Some(dto.unit_id),
                                reason: format!("Unknown spell '{}'", spell_id),
                            });
                        }
                    };
                    if unit.ap < spell.ap_cost {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InsufficientResources,
                            unit_id: Some(dto.unit_id),
                            reason: format!(
                                "Insufficient AP for spell '{}' (costs {}, has {})",
                                spell_id, spell.ap_cost, unit.ap
                            ),
                        });
                    }
                    if unit.energy < spell.energy_cost {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InsufficientResources,
                            unit_id: Some(dto.unit_id),
                            reason: format!(
                                "Insufficient Energy for spell '{}' (costs {}, has {})",
                                spell_id, spell.energy_cost, unit.energy
                            ),
                        });
                    }
                    if !unit.is_spell_ready(&spell_id) {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::CooldownActive,
                            unit_id: Some(dto.unit_id),
                            reason: format!("Spell '{}' is on cooldown", spell_id),
                        });
                    }
                    let spell_target = match target_dto {
                        hexabellum_protocol::SpellTargetDto::None => {
                            if spell.targeting != crate::ability::TargetingMode::SelfOnly {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Spell requires a target".into(),
                                });
                            }
                            crate::ability::SpellTarget::None
                        }
                        hexabellum_protocol::SpellTargetDto::Hex { hex } => {
                            crate::ability::SpellTarget::Hex(HexCoord::new(hex.q, hex.r))
                        }
                        hexabellum_protocol::SpellTargetDto::Unit { unit_id: tid } => {
                            let t = match self.state.get_unit(tid) {
                                Some(u) if u.is_alive() => u,
                                _ => {
                                    return Err(OrderSubmissionError {
                                        code: ProtocolErrorCode::InvalidTarget,
                                        unit_id: Some(dto.unit_id),
                                        reason: format!("Target unit #{} not found or dead", tid),
                                    });
                                }
                            };
                            if spell.targeting == crate::ability::TargetingMode::EnemyUnit
                                && t.team == team
                            {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Spell requires an enemy target".into(),
                                });
                            }
                            if spell.targeting == crate::ability::TargetingMode::AllyUnit
                                && t.team != team
                            {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Spell requires an allied target".into(),
                                });
                            }
                            let effective_caster_pos = move_target.unwrap_or(unit.pos);
                            let dist = effective_caster_pos.distance(&t.pos);
                            if dist < spell.min_range || dist > spell.range {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: format!(
                                        "Target out of range (dist {}, range {})",
                                        dist, spell.range
                                    ),
                                });
                            }
                            // Anti-maphack: enemy must be visible to player team
                            if t.team != team && !self.state.fog.is_visible(team, &t.pos) {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Target is hidden in fog of war".into(),
                                });
                            }
                            if spell.requires_line_of_sight
                                && !crate::vision::has_line_of_sight(
                                    &self.state.map.vision_blockers(),
                                    effective_caster_pos,
                                    t.pos,
                                )
                            {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::MissingLineOfSight,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Line of sight is obstructed by terrain".into(),
                                });
                            }
                            crate::ability::SpellTarget::Unit(tid)
                        }
                    };
                    Action::Cast {
                        spell_id,
                        target: spell_target,
                    }
                }
                ActionDto::Repair { target_id } => {
                    if !unit.is_hero() {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidMessage,
                            unit_id: Some(dto.unit_id),
                            reason: "Only Heroes can repair structures".into(),
                        });
                    }
                    if unit.ap < crate::repair::REPAIR_AP_COST {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InsufficientResources,
                            unit_id: Some(dto.unit_id),
                            reason: "Insufficient AP for repair".into(),
                        });
                    }
                    let target = match self.state.get_unit(target_id) {
                        Some(t) if t.is_alive() => t,
                        _ => {
                            return Err(OrderSubmissionError {
                                code: ProtocolErrorCode::InvalidTarget,
                                unit_id: Some(dto.unit_id),
                                reason: format!("Repair target #{} is dead or not found", target_id),
                            });
                        }
                    };
                    if !target.is_structure() {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: "Target is not a structure".into(),
                        });
                    }
                    if target.team != team {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: "Cannot repair enemy structures".into(),
                        });
                    }
                    if target.hp >= target.max_hp {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: "Structure is already at full health".into(),
                        });
                    }
                    let effective_hero_pos = move_target.unwrap_or(unit.pos);
                    if effective_hero_pos.distance(&target.pos) > crate::repair::REPAIR_RANGE {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: "Hero must be adjacent to repair structure".into(),
                        });
                    }
                    Action::Repair { target_id }
                }
            };

            validated_orders.insert(
                dto.unit_id,
                UnitOrder {
                    unit_id: dto.unit_id,
                    move_target,
                    action,
                },
            );
        }

        self.staged_orders.insert(team, validated_orders);
        self.submitted_teams.insert(team);
        Ok(())
    }

    /// Check if all human-controlled teams have submitted orders.
    pub fn all_human_teams_submitted(&self, active_human_teams: &[TeamId]) -> bool {
        active_human_teams
            .iter()
            .all(|t| self.submitted_teams.contains(t))
    }

    /// Fill missing orders for unmanaged or timed-out heroes with tactical AI.
    pub fn fill_missing_orders_with_ai(&mut self) {
        for team in [0, 1] {
            let team_orders = self.staged_orders.entry(team).or_default();
            for unit in self.state.team_units(team) {
                if unit.kind == UnitKind::Hero {
                    if !team_orders.contains_key(&unit.id) {
                        let fallback = GameAI::generate_fallback_order(&self.state, unit.id);
                        team_orders.insert(unit.id, fallback);
                    }
                }
            }
        }
    }

    /// Authoritatively resolve the current round and advance the match state.
    pub fn resolve_round(&mut self) -> Vec<GameEvent> {
        self.fill_missing_orders_with_ai();

        let mut combined_orders = TurnOrders::new();
        for team in [0, 1] {
            if let Some(mut orders) = self.staged_orders.remove(&team) {
                let mut unit_ids: Vec<UnitId> = orders.keys().copied().collect();
                unit_ids.sort_unstable();
                for id in unit_ids {
                    if let Some(order) = orders.remove(&id) {
                        combined_orders.set_order(id, order);
                    }
                }
            }
        }
        self.staged_orders.clear();

        self.submitted_teams.clear();
        self.state.phase = Phase::Resolution;

        let events = TurnProcessor::resolve_turn(&mut self.state, combined_orders);

        // Update unit_registry with events from this turn
        for event in &events {
            match event {
                GameEvent::UnitSpawned {
                    unit_id,
                    unit_kind,
                    team,
                    pos,
                    ..
                } => {
                    self.unit_registry
                        .insert(*unit_id, (*team, *unit_kind, *pos));
                }
                GameEvent::UnitMoved { unit_id, to, .. } => {
                    if let Some(meta) = self.unit_registry.get_mut(unit_id) {
                        meta.2 = *to;
                    }
                }
                _ => {}
            }
        }

        if self.state.winner.is_none() {
            self.state.phase = Phase::Planning;
        } else {
            self.state.phase = Phase::MatchEnd;
        }

        events
    }

    /// Filter simulation events so coordinates and actions inside fog are concealed.
    pub fn sanitize_events_for_team(
        &self,
        team: TeamId,
        events: &[GameEvent],
    ) -> Vec<SanitizedGameEvent> {
        let visible_hexes = self.state.fog.visible_hexes(team);
        let mut sanitized = Vec::new();

        let get_unit_meta = |id: &UnitId| {
            if let Some(m) = self.unit_registry.get(id) {
                Some((m.0, m.1, m.2))
            } else if let Some(u) = self.state.get_unit(*id) {
                Some((u.team, u.kind, u.pos))
            } else {
                None
            }
        };

        for event in events {
            match event {
                GameEvent::RoundStarted { round } => {
                    sanitized.push(SanitizedGameEvent::RoundStarted { round: *round });
                }
                GameEvent::UnitSpawned {
                    unit_id,
                    unit_kind,
                    team: u_team,
                    pos,
                    spawner_id,
                } => {
                    if *u_team == team || visible_hexes.contains(pos) {
                        sanitized.push(SanitizedGameEvent::UnitSpawned {
                            unit_id: *unit_id,
                            unit_kind: unit_kind.to_string(),
                            team: *u_team,
                            pos: HexDto::new(pos.q, pos.r),
                            spawner_id: *spawner_id,
                        });
                    }
                }
                GameEvent::UnitMoved {
                    unit_id,
                    from,
                    to,
                    path,
                    ap_spent,
                } => {
                    let is_own_unit = get_unit_meta(unit_id).map_or(false, |m| m.0 == team);
                    let from_vis = visible_hexes.contains(from);
                    let to_vis = visible_hexes.contains(to);

                    if is_own_unit || from_vis || to_vis {
                        let path_dto = path.iter().map(|h| HexDto::new(h.q, h.r)).collect();
                        sanitized.push(SanitizedGameEvent::UnitMoved {
                            unit_id: *unit_id,
                            from: HexDto::new(from.q, from.r),
                            to: HexDto::new(to.q, to.r),
                            path: path_dto,
                            ap_spent: *ap_spent,
                        });
                    }
                }
                GameEvent::UnitAttacked {
                    attacker_id,
                    target_id,
                    damage,
                    target_hp_remaining,
                } => {
                    let att_meta = get_unit_meta(attacker_id);
                    let tgt_meta = get_unit_meta(target_id);

                    let att_vis =
                        att_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let tgt_vis =
                        tgt_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));

                    if att_vis || tgt_vis {
                        sanitized.push(SanitizedGameEvent::UnitAttacked {
                            attacker_id: *attacker_id,
                            target_id: *target_id,
                            damage: *damage,
                            target_hp_remaining: *target_hp_remaining,
                        });
                    }
                }
                GameEvent::TowerAttacked {
                    tower_id,
                    target_id,
                    damage,
                    target_hp_remaining,
                } => {
                    let tower_meta = get_unit_meta(tower_id);
                    let tgt_meta = get_unit_meta(target_id);

                    let tower_vis =
                        tower_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let tgt_vis =
                        tgt_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));

                    if tower_vis || tgt_vis {
                        sanitized.push(SanitizedGameEvent::TowerAttacked {
                            tower_id: *tower_id,
                            target_id: *target_id,
                            damage: *damage,
                            target_hp_remaining: *target_hp_remaining,
                        });
                    }
                }
                GameEvent::UnitDied {
                    unit_id,
                    unit_kind,
                    killed_by,
                } => {
                    let meta = get_unit_meta(unit_id);
                    let is_own = meta.map_or(false, |m| m.0 == team);
                    let is_vis = meta.map_or(false, |m| visible_hexes.contains(&m.2));

                    if is_own || is_vis {
                        sanitized.push(SanitizedGameEvent::UnitDied {
                            unit_id: *unit_id,
                            unit_kind: unit_kind.to_string(),
                            killed_by: *killed_by,
                        });
                    }
                }
                GameEvent::UnitWaited { unit_id } => {
                    let meta = get_unit_meta(unit_id);
                    let is_own = meta.map_or(false, |m| m.0 == team);
                    let is_vis = meta.map_or(false, |m| visible_hexes.contains(&m.2));

                    if is_own || is_vis {
                        sanitized.push(SanitizedGameEvent::UnitWaited { unit_id: *unit_id });
                    }
                }
                GameEvent::RoundEnded { round } => {
                    sanitized.push(SanitizedGameEvent::RoundEnded { round: *round });
                }
                GameEvent::MatchEnded { winner } => {
                    sanitized.push(SanitizedGameEvent::MatchEnded { winner: *winner });
                }
                GameEvent::SpellCast {
                    caster_id,
                    spell_id,
                    target,
                } => {
                    let caster_meta = get_unit_meta(caster_id);
                    let caster_vis = caster_meta
                        .map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let target_vis = match target {
                        crate::ability::SpellTarget::Unit(tid) => {
                            let tm = get_unit_meta(tid);
                            tm.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2))
                        }
                        crate::ability::SpellTarget::Hex(h) => visible_hexes.contains(h),
                        crate::ability::SpellTarget::None => false,
                    };

                    if caster_vis || target_vis {
                        let target_dto = match target {
                            crate::ability::SpellTarget::None => {
                                hexabellum_protocol::SpellTargetDto::None
                            }
                            crate::ability::SpellTarget::Hex(h) => {
                                hexabellum_protocol::SpellTargetDto::Hex {
                                    hex: HexDto::new(h.q, h.r),
                                }
                            }
                            crate::ability::SpellTarget::Unit(tid) => {
                                hexabellum_protocol::SpellTargetDto::Unit { unit_id: *tid }
                            }
                        };
                        sanitized.push(SanitizedGameEvent::SpellCast {
                            caster_id: *caster_id,
                            spell_id: spell_id.clone(),
                            target: target_dto,
                        });
                    }
                }
                GameEvent::HealApplied {
                    caster_id,
                    target_id,
                    amount,
                    target_hp_remaining,
                } => {
                    let c_meta = get_unit_meta(caster_id);
                    let t_meta = get_unit_meta(target_id);
                    let c_vis =
                        c_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let t_vis =
                        t_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if c_vis || t_vis {
                        sanitized.push(SanitizedGameEvent::HealApplied {
                            caster_id: *caster_id,
                            target_id: *target_id,
                            amount: *amount,
                            target_hp_remaining: *target_hp_remaining,
                        });
                    }
                }
                GameEvent::StructureRepaired {
                    repairer_id,
                    target_id,
                    amount,
                    target_hp_remaining,
                } => {
                    let r_meta = get_unit_meta(repairer_id);
                    let t_meta = get_unit_meta(target_id);
                    let r_vis =
                        r_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let t_vis =
                        t_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if r_vis || t_vis {
                        sanitized.push(SanitizedGameEvent::StructureRepaired {
                            repairer_id: *repairer_id,
                            target_id: *target_id,
                            amount: *amount,
                            target_hp_remaining: *target_hp_remaining,
                        });
                    }
                }
                GameEvent::StatusApplied {
                    unit_id,
                    status_id,
                    duration_rounds,
                } => {
                    let meta = get_unit_meta(unit_id);
                    let is_vis = meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if is_vis {
                        sanitized.push(SanitizedGameEvent::StatusApplied {
                            unit_id: *unit_id,
                            status_id: status_id.clone(),
                            duration_rounds: *duration_rounds,
                        });
                    }
                }
                GameEvent::StatusExpired { unit_id, status_id } => {
                    let meta = get_unit_meta(unit_id);
                    let is_vis = meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if is_vis {
                        sanitized.push(SanitizedGameEvent::StatusExpired {
                            unit_id: *unit_id,
                            status_id: status_id.clone(),
                        });
                    }
                }
                GameEvent::NeutralCampCleared { camp_id, killer_team } => {
                    sanitized.push(SanitizedGameEvent::NeutralCampCleared {
                        camp_id: camp_id.clone(),
                        killer_team: *killer_team,
                    });
                }
                GameEvent::TeamBuffApplied {
                    team: b_team,
                    buff_id,
                    duration_rounds,
                } => {
                    sanitized.push(SanitizedGameEvent::TeamBuffApplied {
                        team: *b_team,
                        buff_id: buff_id.clone(),
                        duration_rounds: *duration_rounds,
                    });
                }
                _ => {}
            }
        }

        sanitized
    }

    /// Generate team-sanitized snapshot strictly withholding concealed enemy positions.
    pub fn snapshot_for_team(&self, team: TeamId, deadline_unix_ms: Option<u64>) -> SnapshotDto {
        let visible_hexes = self.state.fog.visible_hexes(team);

        let mut sorted_unit_ids: Vec<UnitId> = self.state.units.keys().copied().collect();
        sorted_unit_ids.sort_unstable();

        let visible_units: Vec<UnitDto> = sorted_unit_ids
            .iter()
            .filter_map(|id| self.state.get_unit(*id))
            .filter(|unit| unit.team == team || visible_hexes.contains(&unit.pos))
            .map(|u| UnitDto {
                id: u.id,
                kind: u.kind.to_string(),
                team: u.team,
                pos: HexDto::new(u.pos.q, u.pos.r),
                hp: u.hp,
                max_hp: u.max_hp,
                ap: u.ap,
                max_ap: u.max_ap,
                energy: u.energy,
                max_energy: u.max_energy,
                initiative: u.initiative,
                attack_damage: u.attack_damage,
                attack_range: u.attack_range,
                vision_range: u.vision_range,
                is_stationary: u.kind.is_stationary(),
                cooldowns: u.cooldowns.clone(),
                statuses: u
                    .statuses
                    .iter()
                    .map(|s| hexabellum_protocol::StatusDto {
                        id: s.def_id.clone(),
                        remaining_rounds: s.remaining_rounds,
                        attack_damage_mod: s
                            .modifiers
                            .iter()
                            .filter(|m| m.stat == crate::status::StatKind::AttackDamage)
                            .map(|m| m.value)
                            .sum(),
                    })
                    .collect(),
                lane_id: u.lane_id.clone(),
            })
            .collect();

        let controlled_units: Vec<UnitId> = sorted_unit_ids
            .iter()
            .filter_map(|id| self.state.get_unit(*id))
            .filter(|u| u.team == team && u.kind == UnitKind::Hero && u.is_alive())
            .map(|u| u.id)
            .collect();

        let mut walkable: Vec<HexDto> = self
            .state
            .map
            .all_hexes()
            .iter()
            .filter(|h| !self.state.map.movement_blockers().contains(h))
            .map(|h| HexDto::new(h.q, h.r))
            .collect();
        walkable.sort_by_key(|h| (h.q, h.r));

        let mut obstacles: Vec<HexDto> = self
            .state
            .map
            .movement_blockers()
            .into_iter()
            .map(|h| HexDto::new(h.q, h.r))
            .collect();
        obstacles.sort_by_key(|h| (h.q, h.r));

        let mut visible_hexes_dto: Vec<HexDto> = visible_hexes
            .iter()
            .map(|h| HexDto::new(h.q, h.r))
            .collect();
        visible_hexes_dto.sort_by_key(|h| (h.q, h.r));

        let neutral_camps: Vec<hexabellum_protocol::NeutralCampDto> = self
            .state
            .neutral_camps
            .iter()
            .map(|c| {
                let is_alive = self
                    .state
                    .get_unit(c.guardian_id)
                    .map(|u| u.is_alive())
                    .unwrap_or(false);
                hexabellum_protocol::NeutralCampDto {
                    id: c.id.clone(),
                    pos: HexDto::new(c.camp_pos.q, c.camp_pos.r),
                    is_alive,
                    guardian_unit_id: if is_alive {
                        Some(c.guardian_id)
                    } else {
                        None
                    },
                }
            })
            .collect();

        SnapshotDto {
            match_id: self.match_id.clone(),
            round: self.state.round,
            phase: format!("{:?}", self.state.phase),
            winner: self.state.winner,
            map: MapDto {
                radius: self.config.map_radius,
                walkable,
                obstacles,
            },
            units: visible_units,
            visible_hexes: visible_hexes_dto,
            controlled_units,
            deadline_unix_ms,
            state_hash: self.state_hash(),
            neutral_camps,
        }
    }

    /// Compute cryptographic BLAKE3 state hash for audit and desync detection.
    pub fn state_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.state.round.to_le_bytes());
        hasher.update(&[self.state.phase as u8]);
        hasher.update(&[self.state.winner.unwrap_or(255)]);

        let mut unit_ids: Vec<UnitId> = self.state.units.keys().copied().collect();
        unit_ids.sort_unstable();

        for id in unit_ids {
            if let Some(unit) = self.state.get_unit(id) {
                hasher.update(&unit.id.to_le_bytes());
                hasher.update(&[unit.team]);
                hasher.update(&[unit.kind as u8]);
                hasher.update(&unit.pos.q.to_le_bytes());
                hasher.update(&unit.pos.r.to_le_bytes());
                hasher.update(&unit.hp.to_le_bytes());
                hasher.update(&unit.max_hp.to_le_bytes());
                hasher.update(&unit.ap.to_le_bytes());
                hasher.update(&unit.max_ap.to_le_bytes());
                hasher.update(&unit.energy.to_le_bytes());
                hasher.update(&unit.max_energy.to_le_bytes());
                hasher.update(&unit.attack_damage.to_le_bytes());
                hasher.update(&unit.attack_range.to_le_bytes());
                hasher.update(&unit.vision_range.to_le_bytes());
                hasher.update(&unit.initiative.to_le_bytes());

                let mut cd_keys: Vec<&String> = unit.cooldowns.keys().collect();
                cd_keys.sort();
                for k in cd_keys {
                    hasher.update(k.as_bytes());
                    hasher.update(&unit.cooldowns[k].to_le_bytes());
                }

                for s in &unit.statuses {
                    hasher.update(s.def_id.as_bytes());
                    hasher.update(&s.remaining_rounds.to_le_bytes());
                }
            }
        }

        hasher.finalize().to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_snapshot_censors_hidden_enemy_units() {
        let mut session = BattleSession::new("test_match".into(), BattleConfig::default());

        // Add concealed enemy scout in distant fog hex (0, 5)
        let scout = Unit::new_hero(99, 1, HexCoord::new(0, 5), 1);
        session.state.add_unit(scout);
        session.state.update_fog();

        // Team 0 Snapshot
        let snap_t0 = session.snapshot_for_team(0, None);
        assert!(
            !snap_t0.units.iter().any(|u| u.id == 99),
            "Concealed enemy leaked to Team 0 snapshot!"
        );

        // Team 1 Snapshot
        let snap_t1 = session.snapshot_for_team(1, None);
        assert!(
            snap_t1.units.iter().any(|u| u.id == 99),
            "Friendly unit missing from Team 1 snapshot!"
        );
    }

    #[test]
    fn test_fog_event_sanitization_suppresses_hidden_movements() {
        let mut session = BattleSession::new("test_match".into(), BattleConfig::default());
        session.state.update_fog();

        let hidden_move = GameEvent::UnitMoved {
            unit_id: 4, // Team 1 Hero
            from: HexCoord::new(4, 0),
            to: HexCoord::new(5, -1),
            path: vec![HexCoord::new(4, 0), HexCoord::new(5, -1)],
            ap_spent: 1,
        };

        let sanitized = session.sanitize_events_for_team(0, &[hidden_move]);
        assert!(
            sanitized.is_empty(),
            "Concealed movement leaked across fog of war in event stream!"
        );
    }

    #[test]
    fn test_blake3_state_hash_determinism() {
        let session1 = BattleSession::new("m1".into(), BattleConfig::default());
        let session2 = BattleSession::new("m2".into(), BattleConfig::default());

        assert_eq!(
            session1.state_hash(),
            session2.state_hash(),
            "Identical match setups produced divergent BLAKE3 state hashes!"
        );

        let mut session3 = BattleSession::new("m3".into(), BattleConfig::default());
        if let Some(unit) = session3.state.get_unit_mut(1) {
            unit.attack_damage += 2;
        }
        assert_ne!(
            session1.state_hash(),
            session3.state_hash(),
            "Mutating unit attack damage must change BLAKE3 state hash!"
        );
    }

    #[test]
    fn test_headless_ai_vs_ai_server_simulation_terminates() {
        let mut session = BattleSession::new("sim".into(), BattleConfig::default());

        for round in 1..=50 {
            if session.state.winner.is_some() {
                break;
            }
            let events = session.resolve_round();
            assert!(
                !events.is_empty(),
                "Round resolution produced zero events in round {}",
                round
            );
        }

        assert!(session.state.round > 1, "Simulation failed to advance rounds");
    }

    #[test]
    fn test_submit_orders_validation() {
        let mut session = BattleSession::new("order_test".into(), BattleConfig::default());
        let player_p1 = "player-1".to_string();
        session.assign_team_player(0, player_p1.clone());

        // 1. Submit orders for hero 1 with valid move
        let valid_order = OrderDto {
            unit_id: 1,
            move_target: Some(HexDto::new(-3, -1)),
            action: ActionDto::Wait,
        };
        let res = session.submit_player_orders(&player_p1, 0, 0, vec![valid_order]);
        assert!(res.is_ok());

        // 2. Reject stale round
        let res_stale = session.submit_player_orders(&player_p1, 0, 99, vec![]);
        assert_eq!(res_stale.unwrap_err().code, ProtocolErrorCode::StaleRound);

        // 3. Reject unit not owned (Hero 4 is Team 1)
        let unowned_order = OrderDto {
            unit_id: 4,
            move_target: None,
            action: ActionDto::Wait,
        };
        let err_unowned = session
            .submit_player_orders(&player_p1, 0, 0, vec![unowned_order])
            .unwrap_err();
        assert_eq!(err_unowned.code, ProtocolErrorCode::UnitNotOwned);
        assert_eq!(err_unowned.unit_id, Some(4));
        assert!(err_unowned.reason.contains("Unit #4"));

        // 4. Reject invalid target (cannot attack own team)
        let friendly_attack = OrderDto {
            unit_id: 1,
            move_target: None,
            action: ActionDto::Attack { target_id: 2 },
        };
        let err_friendly = session
            .submit_player_orders(&player_p1, 0, 0, vec![friendly_attack])
            .unwrap_err();
        assert_eq!(err_friendly.code, ProtocolErrorCode::InvalidTarget);
        assert_eq!(err_friendly.unit_id, Some(1));

        // 5. Valid Cleave Cast order for Vanguard (1 AP, 3 Energy)
        let cleave_order = OrderDto {
            unit_id: 1,
            move_target: None,
            action: ActionDto::Cast {
                spell_id: "cleave".to_string(),
                target: hexabellum_protocol::SpellTargetDto::None,
            },
        };
        let res_cleave = session.submit_player_orders(&player_p1, 0, 0, vec![cleave_order]);
        assert!(res_cleave.is_ok());

        // 6. Reject Cast if insufficient Energy
        let mut session_no_energy = BattleSession::new("energy_test".into(), BattleConfig::default());
        session_no_energy.assign_team_player(0, player_p1.clone());
        session_no_energy.state.get_unit_mut(1).unwrap().energy = 0;
        let err_energy = session_no_energy
            .submit_player_orders(
                &player_p1,
                0,
                0,
                vec![OrderDto {
                    unit_id: 1,
                    move_target: None,
                    action: ActionDto::Cast {
                        spell_id: "cleave".to_string(),
                        target: hexabellum_protocol::SpellTargetDto::None,
                    },
                }],
            )
            .unwrap_err();
        assert_eq!(err_energy.code, ProtocolErrorCode::InsufficientResources);

        // 7. Reject Cast if on cooldown
        let mut session_cd = BattleSession::new("cd_test".into(), BattleConfig::default());
        session_cd.assign_team_player(0, player_p1.clone());
        session_cd.state.get_unit_mut(1).unwrap().cooldowns.insert("cleave".to_string(), 2);
        let err_cd = session_cd
            .submit_player_orders(
                &player_p1,
                0,
                0,
                vec![OrderDto {
                    unit_id: 1,
                    move_target: None,
                    action: ActionDto::Cast {
                        spell_id: "cleave".to_string(),
                        target: hexabellum_protocol::SpellTargetDto::None,
                    },
                }],
            )
            .unwrap_err();
        assert_eq!(err_cd.code, ProtocolErrorCode::CooldownActive);

        // 8. Reject Repair if structure is already at full health
        let err_full_repair = session
            .submit_player_orders(
                &player_p1,
                0,
                0,
                vec![OrderDto {
                    unit_id: 1,
                    move_target: Some(HexDto::new(-4, 0)),
                    action: ActionDto::Repair { target_id: 11 },
                }],
            )
            .unwrap_err();
        assert_eq!(err_full_repair.code, ProtocolErrorCode::InvalidTarget);
        assert!(err_full_repair.reason.contains("full health"));

        // 9. Accept Repair if structure is damaged and adjacent
        session.state.get_unit_mut(11).unwrap().hp = 50;
        let valid_repair = session.submit_player_orders(
            &player_p1,
            0,
            0,
            vec![OrderDto {
                unit_id: 1,
                move_target: Some(HexDto::new(-4, 0)),
                action: ActionDto::Repair { target_id: 11 },
            }],
        );
        assert!(valid_repair.is_ok());
    }
}
