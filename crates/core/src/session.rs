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
}

impl BattleSession {
    /// Initialize a new 3v3 MOBA battle session with lane topology.
    pub fn new(match_id: String, config: BattleConfig) -> Self {
        let mut map = HexMap::new(config.map_radius);

        // Standard Phase 2 obstacle pillars flanking central lane
        map.obstacles.insert(HexCoord::new(0, 2));
        map.obstacles.insert(HexCoord::new(0, -2));
        map.obstacles.insert(HexCoord::new(1, 2));
        map.obstacles.insert(HexCoord::new(-1, -2));
        map.obstacles.insert(HexCoord::new(2, -3));
        map.obstacles.insert(HexCoord::new(-2, 3));

        let mut state = GameState::new(map);
        let mut controllers = HashMap::new();

        // Team 0 Base (Left)
        let t0_spawner = Unit::new_spawner(10, 0, HexCoord::new(-5, 0), config.spawn_interval);
        let t0_tower = Unit::new_tower(11, 0, HexCoord::new(-3, 0));
        controllers.insert(10, Controller::Automatic);
        controllers.insert(11, Controller::Automatic);
        state.add_unit(t0_spawner);
        state.add_unit(t0_tower);

        // Team 0 Heroes
        state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-4, -1), 3));
        state.add_unit(Unit::new_hero(2, 0, HexCoord::new(-4, 0), 2));
        state.add_unit(Unit::new_hero(3, 0, HexCoord::new(-4, 1), 1));
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

        // Team 1 Heroes
        state.add_unit(Unit::new_hero(4, 1, HexCoord::new(4, -1), 3));
        state.add_unit(Unit::new_hero(5, 1, HexCoord::new(4, 0), 2));
        state.add_unit(Unit::new_hero(6, 1, HexCoord::new(4, 1), 1));
        controllers.insert(4, Controller::Ai);
        controllers.insert(5, Controller::Ai);
        controllers.insert(6, Controller::Ai);

        state.next_unit_id = 30;

        // Initialize vision
        state.update_fog();

        Self {
            match_id,
            state,
            controllers,
            staged_orders: HashMap::new(),
            submitted_teams: HashSet::new(),
            config,
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
    ) -> Result<(), ProtocolErrorCode> {
        if self.state.phase != Phase::Planning {
            return Err(ProtocolErrorCode::InvalidPhase);
        }
        if self.state.round != round {
            return Err(ProtocolErrorCode::StaleRound);
        }

        let mut validated_orders = HashMap::new();

        for dto in orders {
            let unit = match self.state.get_unit(dto.unit_id) {
                Some(u) => u,
                None => return Err(ProtocolErrorCode::UnitNotOwned),
            };

            if !unit.is_alive() {
                return Err(ProtocolErrorCode::UnitDead);
            }
            if unit.team != team {
                return Err(ProtocolErrorCode::UnitNotOwned);
            }

            // Verify controller ownership
            match self.controllers.get(&dto.unit_id) {
                Some(Controller::Player(owner)) if owner == player_id => {}
                _ => return Err(ProtocolErrorCode::NotAuthorized),
            }

            let move_target = dto.move_target.map(|h| HexCoord::new(h.q, h.r));
            let action = match dto.action {
                ActionDto::Wait => Action::Wait,
                ActionDto::Attack { target_id } => {
                    let target = match self.state.get_unit(target_id) {
                        Some(t) => t,
                        None => return Err(ProtocolErrorCode::InvalidTarget),
                    };
                    if !target.is_alive() || target.team == team {
                        return Err(ProtocolErrorCode::InvalidTarget);
                    }
                    Action::Attack { target_id }
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
            for unit in self.state.units.values() {
                if unit.team == team && unit.is_alive() && unit.kind == UnitKind::Hero {
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
        for (_, orders) in self.staged_orders.drain() {
            for (id, order) in orders {
                combined_orders.set_order(id, order);
            }
        }

        self.submitted_teams.clear();
        self.state.phase = Phase::Resolution;

        let events = TurnProcessor::resolve_turn(&mut self.state, combined_orders);

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
                    let unit_opt = self.state.get_unit(*unit_id);
                    let is_own_unit = unit_opt.map_or(false, |u| u.team == team);
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
                    let att = self.state.get_unit(*attacker_id);
                    let tgt = self.state.get_unit(*target_id);

                    let att_vis =
                        att.map_or(false, |u| u.team == team || visible_hexes.contains(&u.pos));
                    let tgt_vis =
                        tgt.map_or(false, |u| u.team == team || visible_hexes.contains(&u.pos));

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
                    let tower = self.state.get_unit(*tower_id);
                    let tgt = self.state.get_unit(*target_id);

                    let tower_vis =
                        tower.map_or(false, |u| u.team == team || visible_hexes.contains(&u.pos));
                    let tgt_vis =
                        tgt.map_or(false, |u| u.team == team || visible_hexes.contains(&u.pos));

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
                    sanitized.push(SanitizedGameEvent::UnitDied {
                        unit_id: *unit_id,
                        unit_kind: unit_kind.to_string(),
                        killed_by: *killed_by,
                    });
                }
                GameEvent::UnitWaited { unit_id } => {
                    if let Some(unit) = self.state.get_unit(*unit_id) {
                        if unit.team == team || visible_hexes.contains(&unit.pos) {
                            sanitized.push(SanitizedGameEvent::UnitWaited { unit_id: *unit_id });
                        }
                    }
                }
                GameEvent::RoundEnded { round } => {
                    sanitized.push(SanitizedGameEvent::RoundEnded { round: *round });
                }
                GameEvent::MatchEnded { winner } => {
                    sanitized.push(SanitizedGameEvent::MatchEnded { winner: *winner });
                }
                _ => {}
            }
        }

        sanitized
    }

    /// Generate team-sanitized snapshot strictly withholding concealed enemy positions.
    pub fn snapshot_for_team(&self, team: TeamId, deadline_unix_ms: Option<u64>) -> SnapshotDto {
        let visible_hexes = self.state.fog.visible_hexes(team);

        let visible_units: Vec<UnitDto> = self
            .state
            .units
            .values()
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
                initiative: u.initiative,
                attack_range: u.attack_range,
                vision_range: u.vision_range,
                is_stationary: u.kind.is_stationary(),
            })
            .collect();

        let controlled_units: Vec<UnitId> = self
            .state
            .units
            .values()
            .filter(|u| u.team == team && u.kind == UnitKind::Hero && u.is_alive())
            .map(|u| u.id)
            .collect();

        let walkable: Vec<HexDto> = self
            .state
            .map
            .all_hexes()
            .iter()
            .map(|h| HexDto::new(h.q, h.r))
            .collect();

        let obstacles: Vec<HexDto> = self
            .state
            .map
            .obstacles
            .iter()
            .map(|h| HexDto::new(h.q, h.r))
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
            visible_hexes: visible_hexes
                .iter()
                .map(|h| HexDto::new(h.q, h.r))
                .collect(),
            controlled_units,
            deadline_unix_ms,
            state_hash: self.state_hash(),
        }
    }

    /// Compute cryptographic BLAKE3 state hash for audit and desync detection.
    pub fn state_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.state.round.to_le_bytes());
        hasher.update(&[self.state.winner.unwrap_or(255)]);

        let mut unit_ids: Vec<UnitId> = self.state.units.keys().copied().collect();
        unit_ids.sort_unstable();

        for id in unit_ids {
            if let Some(unit) = self.state.get_unit(id) {
                hasher.update(&unit.id.to_le_bytes());
                hasher.update(&[unit.team]);
                hasher.update(&unit.pos.q.to_le_bytes());
                hasher.update(&unit.pos.r.to_le_bytes());
                hasher.update(&unit.hp.to_le_bytes());
                hasher.update(&unit.ap.to_le_bytes());
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
        assert_eq!(res_stale, Err(ProtocolErrorCode::StaleRound));

        // 3. Reject unit not owned (Hero 4 is Team 1)
        let unowned_order = OrderDto {
            unit_id: 4,
            move_target: None,
            action: ActionDto::Wait,
        };
        let res_unowned = session.submit_player_orders(&player_p1, 0, 0, vec![unowned_order]);
        assert_eq!(res_unowned, Err(ProtocolErrorCode::UnitNotOwned));

        // 4. Reject invalid target (cannot attack own team)
        let friendly_attack = OrderDto {
            unit_id: 1,
            move_target: None,
            action: ActionDto::Attack { target_id: 2 },
        };
        let res_friendly = session.submit_player_orders(&player_p1, 0, 0, vec![friendly_attack]);
        assert_eq!(res_friendly, Err(ProtocolErrorCode::InvalidTarget));
    }
}
