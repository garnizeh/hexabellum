use crate::draft::HeroSelectDraft;
use crate::player::{ConnectionState, PlayerConnection};
use hexabellum_core::controller::Controller;
use hexabellum_core::hero_defs::get_all_hero_defs;
use hexabellum_core::session::{BattleConfig, BattleSession};
use hexabellum_protocol::{
    ErrorCode, HeroDefId, HeroDto, MatchPhaseDto, OrderDto, PlayerId,
    ProtocolErrorCode, ReconnectToken, Round, ServerMessage, TeamId, UnitId,
};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::info;

/// Actor commands routed from WebSocket and HTTP handlers.
#[derive(Debug)]
pub enum MatchCommand {
    PlayerConnect {
        player_id: PlayerId,
        display_name: Option<String>,
        reconnect_token: Option<ReconnectToken>,
        sender: mpsc::UnboundedSender<ServerMessage>,
    },
    PlayerDisconnect {
        player_id: PlayerId,
    },
    PlayerPing {
        player_id: PlayerId,
    },
    SelectHero {
        player_id: PlayerId,
        hero_def_id: HeroDefId,
    },
    SetReady {
        player_id: PlayerId,
        ready: bool,
    },
    SubmitOrders {
        player_id: PlayerId,
        round: Round,
        orders: Vec<OrderDto>,
    },
    CancelOrders {
        player_id: PlayerId,
        round: Round,
    },
    BuyItem {
        player_id: PlayerId,
        item_id: String,
    },
    HeroSelectTimerFired,
    TurnTimerFired {
        round: Round,
    },
    GracePeriodFired {
        round: Round,
        seq: u64,
    },
    Finish,
}

/// Actor handle held by the registry.
#[derive(Clone)]
pub struct MatchActorHandle {
    pub tx: mpsc::Sender<MatchCommand>,
}

impl MatchActorHandle {
    pub fn new(
        match_id: String,
        config: BattleConfig,
        registry: Option<crate::MatchRegistry>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(128);
        let actor = MatchActor::new(match_id, config, tx.clone(), registry);
        tokio::spawn(actor.run(rx));
        Self { tx }
    }

    pub async fn send(&self, cmd: MatchCommand) {
        let _ = self.tx.send(cmd).await;
    }
}

pub struct MatchActor {
    pub match_id: String,
    pub phase: MatchPhaseDto,
    pub session: Option<BattleSession>,
    pub config: BattleConfig,
    pub players: HashMap<PlayerId, PlayerConnection>,
    pub human_teams: Vec<TeamId>,
    pub hero_drafts: HashMap<TeamId, HeroSelectDraft>,
    pub submitted_players: HashSet<PlayerId>,
    pub turn_deadline_unix_ms: Option<u64>,
    pub hero_select_deadline_unix_ms: Option<u64>,
    pub grace_timer_active: bool,
    pub grace_period_seq: u64,
    pub self_tx: mpsc::Sender<MatchCommand>,
    pub registry: Option<crate::MatchRegistry>,
}

impl MatchActor {
    pub fn new(
        match_id: String,
        config: BattleConfig,
        self_tx: mpsc::Sender<MatchCommand>,
        registry: Option<crate::MatchRegistry>,
    ) -> Self {
        let pool = vec![
            "vanguard".to_string(),
            "ranger".to_string(),
            "warden".to_string(),
            "sniper".to_string(),
            "berserker".to_string(),
        ];
        let mut hero_drafts = HashMap::new();
        hero_drafts.insert(0, HeroSelectDraft::new_team_draft(pool.clone()));
        hero_drafts.insert(1, HeroSelectDraft::new_team_draft(pool));

        let initial_phase = if config.skip_draft {
            MatchPhaseDto::Planning
        } else {
            MatchPhaseDto::Lobby
        };

        let session = if config.skip_draft {
            Some(BattleSession::new(match_id.clone(), config.clone()))
        } else {
            None
        };

        Self {
            match_id,
            phase: initial_phase,
            session,
            config,
            players: HashMap::new(),
            human_teams: Vec::new(),
            hero_drafts,
            submitted_players: HashSet::new(),
            turn_deadline_unix_ms: None,
            hero_select_deadline_unix_ms: None,
            grace_timer_active: false,
            grace_period_seq: 0,
            self_tx,
            registry,
        }
    }

    pub fn new_test_match() -> Self {
        let (tx, _rx) = mpsc::channel(128);
        Self::new("test_match".into(), BattleConfig::default(), tx, None)
    }

    pub fn setup_5v5_session(&mut self) {
        let mut config = BattleConfig::default();
        config.map_radius = 8;
        config.heroes_per_team = 5;
        config.players_per_team = 5;
        config.fill_empty_slots_with_ai = true;
        self.config = config.clone();

        let mut session = BattleSession::new(self.match_id.clone(), config);
        session.state.round = 1;

        let heroes = ["vanguard", "ranger", "warden", "sniper", "berserker"];
        for i in 0..5 {
            let p_id = format!("player_{}", i + 1);
            let unit_id = (i + 1) as u64;
            let (tx, _rx) = mpsc::unbounded_channel();
            let mut conn = PlayerConnection::new(p_id.clone(), 0, tx);
            conn.hero_def_id = Some(heroes[i].to_string());
            conn.hero_unit_id = Some(unit_id);
            session.controllers.assign(unit_id, Controller::Player(p_id.clone()));
            self.players.insert(p_id, conn);
        }

        for i in 0..5 {
            let p_id = format!("player_{}", i + 6);
            let unit_id = (i + 6) as u64;
            let (tx, _rx) = mpsc::unbounded_channel();
            let mut conn = PlayerConnection::new(p_id.clone(), 1, tx);
            conn.hero_def_id = Some(heroes[i].to_string());
            conn.hero_unit_id = Some(unit_id);
            session.controllers.assign(unit_id, Controller::Player(p_id.clone()));
            self.players.insert(p_id, conn);
        }

        self.phase = MatchPhaseDto::Planning;
        self.session = Some(session);
    }

    pub async fn run(mut self, mut rx: mpsc::Receiver<MatchCommand>) {
        info!("MatchActor [{}] started", self.match_id);

        while let Some(cmd) = rx.recv().await {
            match cmd {
                MatchCommand::PlayerConnect {
                    player_id,
                    display_name,
                    reconnect_token,
                    sender,
                } => {
                    self.handle_player_connect(player_id, display_name, reconnect_token, sender)
                        .await;
                }
                MatchCommand::PlayerDisconnect { player_id } => {
                    self.handle_disconnect(&player_id);
                }
                MatchCommand::PlayerPing { player_id } => {
                    if let Some(conn) = self.players.get_mut(&player_id) {
                        conn.last_seen = Instant::now();
                    }
                }
                MatchCommand::SelectHero {
                    player_id,
                    hero_def_id,
                } => {
                    let _ = self.handle_select_hero(&player_id, &hero_def_id).await;
                }
                MatchCommand::SetReady { player_id, ready } => {
                    self.handle_set_ready(&player_id, ready).await;
                }
                MatchCommand::SubmitOrders {
                    player_id,
                    round,
                    orders,
                } => {
                    let res = self.handle_submit_orders(&player_id, round, orders.clone());
                    if let Err(err) = res {
                        if let Some(conn) = self.players.get(&player_id) {
                            let reason = if err == ErrorCode::NotYourUnit || err == ErrorCode::UnitNotOwned {
                                if let Some(o) = orders.first() {
                                    format!("Unit #{} is not controlled by player {}", o.unit_id, player_id)
                                } else {
                                    format!("Order rejected: {:?}", err)
                                }
                            } else if err == ErrorCode::RoundMismatch || err == ErrorCode::StaleRound {
                                format!("Stale round {} rejected", round)
                            } else {
                                format!("Order rejected: {:?}", err)
                            };
                            conn.send(ServerMessage::OrderRejected {
                                round,
                                error_code: err,
                                reason,
                            });
                        }
                    }
                }
                MatchCommand::CancelOrders { player_id, round: _ } => {
                    if self.phase == MatchPhaseDto::Planning {
                        if let Some(session) = self.session.as_mut() {
                            if let Some(conn) = self.players.get(&player_id) {
                                if let Some(unit_id) = conn.hero_unit_id {
                                    if let Some(orders) = session.staged_orders.get_mut(&conn.team) {
                                        orders.remove(&unit_id);
                                    }
                                }
                            }
                        }
                        self.submitted_players.remove(&player_id);
                        self.grace_timer_active = false;
                        self.grace_period_seq += 1;
                    }
                }
                MatchCommand::BuyItem { player_id, item_id } => {
                    self.handle_buy_item(&player_id, &item_id).await;
                }
                MatchCommand::HeroSelectTimerFired => {
                    if self.phase == MatchPhaseDto::HeroSelect {
                        self.finalize_hero_draft_and_start_match().await;
                    }
                }
                MatchCommand::TurnTimerFired { round } => {
                    if let Some(ref session) = self.session {
                        if session.state.round == round
                            && session.state.winner.is_none()
                            && self.phase == MatchPhaseDto::Planning
                            && self.turn_deadline_unix_ms.is_some()
                        {
                            info!(
                                "MatchActor [{}] Turn timer expired for round {}",
                                self.match_id, round
                            );
                            self.handle_turn_timeout();
                            self.resolve_round().await;
                        }
                    }
                }
                MatchCommand::GracePeriodFired { round, seq } => {
                    if let Some(ref session) = self.session {
                        if session.state.round == round
                            && self.grace_timer_active
                            && self.grace_period_seq == seq
                            && session.state.winner.is_none()
                            && self.phase == MatchPhaseDto::Planning
                            && self.turn_deadline_unix_ms.is_some()
                        {
                            info!(
                                "MatchActor [{}] Early resolution grace period elapsed for round {} (seq {})",
                                self.match_id, round, seq
                            );
                            self.resolve_round().await;
                        }
                    }
                }
                MatchCommand::Finish => {
                    info!(
                        "MatchActor [{}] Cleaning up match actor and registry",
                        self.match_id
                    );
                    if let Some(ref reg) = self.registry {
                        reg.remove(&self.match_id);
                    }
                    break;
                }
            }
        }

        info!("MatchActor [{}] terminated", self.match_id);
    }

    pub fn handle_disconnect(&mut self, player_id: &str) {
        if let Some(conn) = self.players.get_mut(player_id) {
            conn.connection_state = ConnectionState::Disconnected;
            conn.is_connected = false;
            conn.sender = None;
            let team = conn.team;
            info!(
                "Player [{}] disconnected from match [{}]",
                player_id, self.match_id
            );
            self.broadcast(ServerMessage::PlayerConnectionUpdated {
                player_id: player_id.to_string(),
                connected: false,
                is_ai_controlled: false,
            });
            for other in self.players.values().filter(|o| o.team != team) {
                other.send(ServerMessage::OpponentStatus { online: false });
            }
        }
    }

    pub fn handle_turn_timeout(&mut self) {
        let mut transferred = Vec::new();
        for (pid, conn) in self.players.iter_mut() {
            if conn.connection_state == ConnectionState::Disconnected {
                conn.connection_state = ConnectionState::AiReplacement;
                if let Some(uid) = conn.hero_unit_id {
                    transferred.push((uid, pid.clone()));
                }
            }
        }

        if let Some(session) = self.session.as_mut() {
            for (uid, _) in &transferred {
                session.controllers.transfer_to_ai(*uid);
            }
        }

        for (_, pid) in transferred {
            self.broadcast(ServerMessage::PlayerConnectionUpdated {
                player_id: pid,
                connected: false,
                is_ai_controlled: true,
            });
        }
    }

    pub fn handle_reconnect(&mut self, player_id: &str) {
        if let Some(conn) = self.players.get_mut(player_id) {
            conn.connection_state = ConnectionState::Connected;
            conn.is_connected = true;
            if let Some(uid) = conn.hero_unit_id {
                if let Some(session) = self.session.as_mut() {
                    session
                        .controllers
                        .transfer_to_player(uid, player_id.to_string());
                }
            }
            self.broadcast(ServerMessage::PlayerConnectionUpdated {
                player_id: player_id.to_string(),
                connected: true,
                is_ai_controlled: false,
            });
        }
    }

    pub async fn handle_player_connect(
        &mut self,
        player_id: PlayerId,
        display_name: Option<String>,
        reconnect_token: Option<ReconnectToken>,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) {
        // If match already ended, notify and return
        if self.phase == MatchPhaseDto::MatchEnd {
            if let Some(ref session) = self.session {
                let snapshot = session.snapshot_for_team(0, None);
                let reason = session.state.check_winner_with_reason().map(|(_, r)| r);
                let _ = sender.send(ServerMessage::MatchEnded {
                    winner: session.state.winner,
                    snapshot,
                    state_hash: Some(session.state_hash()),
                    total_rounds: Some(session.state.round),
                    reason,
                });
            }
            return;
        }

        // Reconnection check
        if let Some(conn) = self.players.get_mut(&player_id) {
            let valid_token = reconnect_token.as_ref() == Some(&conn.reconnect_token);
            if valid_token {
                conn.connection_generation += 1;
                conn.sender = Some(sender.clone());
                conn.is_connected = true;
                conn.connection_state = ConnectionState::Connected;
                conn.last_seen = Instant::now();
                let team = conn.team;

                let _ = sender.send(ServerMessage::HelloAck {
                    player_id: player_id.clone(),
                    reconnect_token: conn.reconnect_token.clone(),
                });

                if let Some(ref mut session) = self.session {
                    if let Some(uid) = conn.hero_unit_id {
                        session
                            .controllers
                            .transfer_to_player(uid, player_id.clone());
                    }
                    let snapshot = session.snapshot_for_player(team, Some(&player_id), self.turn_deadline_unix_ms);
                    let _ = sender.send(ServerMessage::MatchJoined {
                        match_id: self.match_id.clone(),
                        player_id: player_id.clone(),
                        team,
                        is_spectator: false,
                        snapshot: snapshot.clone(),
                    });

                    if let Some(deadline) = self.turn_deadline_unix_ms {
                        let _ = sender.send(ServerMessage::RoundStarted {
                            round: session.state.round,
                            deadline_unix_ms: deadline,
                            snapshot,
                            events: Vec::new(),
                        });
                    }
                }

                for other in self.players.values().filter(|o| o.team != team) {
                    other.send(ServerMessage::OpponentStatus { online: true });
                }
                let opp_online = self
                    .players
                    .values()
                    .any(|o| o.team != team && o.is_connected);
                if opp_online {
                    let _ = sender.send(ServerMessage::OpponentStatus { online: true });
                }

                self.broadcast(ServerMessage::PlayerConnectionUpdated {
                    player_id: player_id.clone(),
                    connected: true,
                    is_ai_controlled: false,
                });
                return;
            } else {
                let _ = sender.send(ServerMessage::Error {
                    error_code: ProtocolErrorCode::NotAuthorized,
                    message: "Invalid reconnect token".into(),
                });
                return;
            }
        }

        // Max players check
        let max_players = (self.config.players_per_team * 2) as usize;
        if self.players.len() >= max_players {
            let _ = sender.send(ServerMessage::Error {
                error_code: ProtocolErrorCode::MatchFull,
                message: "Match is full".into(),
            });
            return;
        }

        // Team balancing: team with fewer players, or Team 0 if equal
        let t0_count = self.players.values().filter(|p| p.team == 0).count();
        let t1_count = self.players.values().filter(|p| p.team == 1).count();
        let assigned_team = if t0_count <= t1_count { 0 } else { 1 };

        let mut conn = PlayerConnection::new(player_id.clone(), assigned_team, sender.clone());
        if let Some(name) = display_name {
            conn.display_name = name;
        }
        let token = conn.reconnect_token.clone();

        self.players.insert(player_id.clone(), conn);
        if !self.human_teams.contains(&assigned_team) {
            self.human_teams.push(assigned_team);
        }

        if let Some(draft) = self.hero_drafts.get_mut(&assigned_team) {
            draft.register_player(&player_id);
        }

        let _ = sender.send(ServerMessage::HelloAck {
            player_id: player_id.clone(),
            reconnect_token: token,
        });

        // If in legacy mode / skip draft:
        if self.config.skip_draft {
            if let Some(ref mut session) = self.session {
                session.assign_team_player(assigned_team, player_id.clone());
                let snapshot = session.snapshot_for_player(assigned_team, Some(&player_id), self.turn_deadline_unix_ms);
                let _ = sender.send(ServerMessage::MatchJoined {
                    match_id: self.match_id.clone(),
                    player_id: player_id.clone(),
                    team: assigned_team,
                    is_spectator: false,
                    snapshot,
                });
            }

            for other in self.players.values().filter(|o| o.team != assigned_team) {
                other.send(ServerMessage::OpponentStatus { online: true });
            }
            let opp_online = self
                .players
                .values()
                .any(|o| o.team != assigned_team && o.is_connected);
            if opp_online {
                let _ = sender.send(ServerMessage::OpponentStatus { online: true });
            }

            let ready_to_start = self.players.len() == 2
                || (self.players.len() == 1 && self.config.enable_ai_team_1);
            if ready_to_start && self.turn_deadline_unix_ms.is_none() {
                self.start_planning_phase().await;
            }
        } else {
            // Lobby / HeroSelect mode
            self.broadcast_lobby_state();

            // Auto-advance to HeroSelect if all players ready or lobby full
            if self.players.len() == max_players {
                self.start_hero_select_phase().await;
            }
        }
    }

    pub async fn handle_select_hero(
        &mut self,
        player_id: &str,
        hero_def_id: &str,
    ) -> Result<(), ErrorCode> {
        if self.phase != MatchPhaseDto::HeroSelect && self.phase != MatchPhaseDto::Lobby {
            return Err(ErrorCode::NotInHeroSelectPhase);
        }

        let team = self
            .players
            .get(player_id)
            .ok_or(ErrorCode::PlayerNotAuthenticated)?
            .team;

        let draft = self
            .hero_drafts
            .get_mut(&team)
            .ok_or(ErrorCode::InvalidHeroDef)?;

        draft.select_hero(player_id, hero_def_id)?;

        if let Some(conn) = self.players.get_mut(player_id) {
            conn.hero_def_id = Some(hero_def_id.to_string());
        }

        self.broadcast(ServerMessage::HeroSelected {
            player_id: player_id.to_string(),
            team,
            hero_def_id: hero_def_id.to_string(),
        });
        self.broadcast_lobby_state();

        // Check if all connected players have selected heroes and are ready
        let all_selected_and_ready = self
            .players
            .values()
            .filter(|p| p.is_connected && !p.is_ai)
            .all(|p| p.hero_def_id.is_some() && p.is_ready);

        if all_selected_and_ready && self.phase == MatchPhaseDto::HeroSelect {
            self.finalize_hero_draft_and_start_match().await;
        }

        Ok(())
    }

    pub async fn handle_set_ready(&mut self, player_id: &str, ready: bool) {
        if let Some(conn) = self.players.get_mut(player_id) {
            conn.is_ready = ready;
        }
        self.broadcast_lobby_state();

        let all_ready = self
            .players
            .values()
            .filter(|p| p.is_connected && !p.is_ai)
            .all(|p| p.is_ready);

        if all_ready && self.players.len() >= 2 && self.phase == MatchPhaseDto::Lobby {
            self.start_hero_select_phase().await;
        } else if self.phase == MatchPhaseDto::HeroSelect {
            let all_selected_and_ready = self
                .players
                .values()
                .filter(|p| p.is_connected && !p.is_ai)
                .all(|p| p.hero_def_id.is_some() && p.is_ready);
            if all_selected_and_ready {
                self.finalize_hero_draft_and_start_match().await;
            }
        }
    }

    pub async fn start_hero_select_phase(&mut self) {
        self.phase = MatchPhaseDto::HeroSelect;
        for p in self.players.values_mut() {
            p.is_ready = false;
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let deadline_ms = now_ms + 20_000;
        self.hero_select_deadline_unix_ms = Some(deadline_ms);

        self.broadcast_lobby_state();

        let tx = self.self_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(20)).await;
            let _ = tx.send(MatchCommand::HeroSelectTimerFired).await;
        });
    }

    pub async fn finalize_hero_draft_and_start_match(&mut self) {
        for team in [0, 1] {
            if let Some(draft) = self.hero_drafts.get_mut(&team) {
                draft.handle_timeout();
                for (pid, hero) in &draft.selections {
                    if let Some(conn) = self.players.get_mut(pid) {
                        conn.hero_def_id = Some(hero.clone());
                    }
                }
            }
        }

        let mut session = BattleSession::new(self.match_id.clone(), self.config.clone());

        // Assign heroes to human players in ControllerMap
        for conn in self.players.values_mut() {
            if let Some(ref hero_def_id) = conn.hero_def_id {
                let key = format!("{}_{}", hero_def_id, conn.team);
                if let Some(&unit_id) = session.hero_assignments.get(&key) {
                    conn.hero_unit_id = Some(unit_id);
                    session
                        .controllers
                        .assign(unit_id, Controller::Player(conn.player_id.clone()));
                }
            }
        }

        // Broadcast MatchStarting with initial team snapshot
        for conn in self.players.values() {
            let initial_snapshot = session.snapshot_for_player(conn.team, Some(&conn.player_id), None);
            conn.send(ServerMessage::MatchStarting {
                round: session.state.round,
                initial_snapshot,
            });
        }

        self.session = Some(session);
        self.phase = MatchPhaseDto::Planning;
        self.start_planning_phase().await;
    }

    pub fn handle_submit_orders(
        &mut self,
        player_id: &str,
        round: u32,
        orders: Vec<OrderDto>,
    ) -> Result<(), ErrorCode> {
        // 1. Verify match phase
        if self.phase != MatchPhaseDto::Planning {
            return Err(ErrorCode::NotInPlanningPhase);
        }

        let session = self.session.as_mut().ok_or(ErrorCode::NotInPlanningPhase)?;

        // 2. Verify current round
        if round != session.state.round {
            return Err(ErrorCode::RoundMismatch);
        }

        // 3. Enforce single-hero control: exactly 1 order allowed in multi-player mode
        if self.config.players_per_team > 1 {
            if orders.len() != 1 {
                return Err(ErrorCode::InvalidOrderCount);
            }

            let order = &orders[0];

            // 4. Authoritative controller check: does player_id own this unit_id?
            if !session.controllers.is_controlled_by_player(order.unit_id, &player_id.to_string()) {
                return Err(ErrorCode::NotYourUnit);
            }

            // 5. Verify unit is alive and not dead awaiting respawn
            let unit = session.state.units.get(&order.unit_id).ok_or(ErrorCode::CannotOrderDeadHero)?;
            if !unit.is_alive() || unit.is_dead_awaiting_respawn() {
                return Err(ErrorCode::CannotOrderDeadHero);
            }
        }

        let team = self
            .players
            .get(player_id)
            .ok_or(ErrorCode::PlayerNotAuthenticated)?
            .team;

        // 6. Ingest into session
        let p_id = player_id.to_string();
        session.submit_player_orders(&p_id, team, round, orders).map_err(|e| e.code)?;

        self.submitted_players.insert(player_id.to_string());
        if let Some(conn) = self.players.get(player_id) {
            conn.send(ServerMessage::OrdersAccepted { round });
        }

        // Check early resolution
        let connected_humans: Vec<&PlayerConnection> = self
            .players
            .values()
            .filter(|p| p.is_connected && !p.is_ai)
            .collect();
        let all_submitted = connected_humans
            .iter()
            .all(|p| self.submitted_players.contains(&p.player_id));

        if all_submitted && !connected_humans.is_empty() {
            self.grace_timer_active = true;
            self.grace_period_seq += 1;
            let seq = self.grace_period_seq;
            let tx = self.self_tx.clone();
            let grace_ms = self.config.early_resolution_grace_ms;

            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            self.broadcast(ServerMessage::EarlyResolutionTriggered {
                round,
                resolution_unix_ms: now_ms + grace_ms,
            });

            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(grace_ms)).await;
                let _ = tx.send(MatchCommand::GracePeriodFired { round, seq }).await;
            });
        }

        Ok(())
    }

    pub async fn start_planning_phase(&mut self) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let deadline_ms = now_ms + (self.config.turn_duration_secs * 1000);
        self.turn_deadline_unix_ms = Some(deadline_ms);
        self.grace_timer_active = false;
        self.submitted_players.clear();

        let session = match self.session.as_mut() {
            Some(s) => s,
            None => return,
        };

        // Phase 7 round start pipeline:
        // 1. Decrement hero respawn timers & respawn ready heroes at base
        let mut start_events = Vec::new();
        start_events.extend(session.process_round_start_respawns());
        // 2. Apply base regeneration (+15 HP) to living heroes in base zone
        start_events.extend(session.process_base_regeneration());
        // 3. Distribute passive gold (+6G)
        start_events.extend(session.distribute_passive_income());
        // 4. Greedy AI bot shopping inside base zone
        session.execute_ai_bot_shopping();
        // 5. Update team line-of-sight and fog of war
        session.state.update_fog();

        let round = session.state.round;

        for conn in self.players.values() {
            let snapshot = session.snapshot_for_player(conn.team, Some(&conn.player_id), Some(deadline_ms));
            let team_events = session.sanitize_events_for_team(conn.team, &start_events);
            conn.send(ServerMessage::RoundStarted {
                round,
                deadline_unix_ms: deadline_ms,
                snapshot,
                events: team_events,
            });
        }

        let tx = self.self_tx.clone();
        let duration = Duration::from_secs(self.config.turn_duration_secs);
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            let _ = tx.send(MatchCommand::TurnTimerFired { round }).await;
        });
    }

    pub async fn resolve_round(&mut self) {
        self.grace_timer_active = false;
        self.turn_deadline_unix_ms = None;
        self.phase = MatchPhaseDto::Resolution;

        let session = match self.session.as_mut() {
            Some(s) => s,
            None => return,
        };

        let round_planned = session.state.round;
        let raw_events = session.resolve_round();
        let is_ended = session.state.winner.is_some()
            || session.state.phase == hexabellum_core::state::Phase::MatchEnd;
        let state_hash = session.state_hash();

        for conn in self.players.values() {
            let team = conn.team;
            let sanitized_events = session.sanitize_events_for_team(team, &raw_events);
            let snapshot = session.snapshot_for_player(team, Some(&conn.player_id), None);

            if is_ended {
                let reason = session.state.check_winner_with_reason().map(|(_, r)| r);
                conn.send(ServerMessage::MatchEnded {
                    winner: session.state.winner,
                    snapshot,
                    state_hash: Some(state_hash.clone()),
                    total_rounds: Some(round_planned),
                    reason,
                });
            } else {
                conn.send(ServerMessage::RoundResolved {
                    round: round_planned,
                    events: sanitized_events,
                    snapshot,
                    state_hash: Some(state_hash.clone()),
                });
            }
        }

        if !is_ended {
            self.phase = MatchPhaseDto::Planning;
            self.start_planning_phase().await;
        } else {
            self.phase = MatchPhaseDto::MatchEnd;
            let tx = self.self_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let _ = tx.send(MatchCommand::Finish).await;
            });
        }
    }

    pub fn build_hero_pools_dto(&self) -> HashMap<u8, Vec<HeroDto>> {
        let hero_defs = get_all_hero_defs();
        let mut pools = HashMap::new();
        for team in [0, 1] {
            let dtos: Vec<HeroDto> = hero_defs
                .iter()
                .map(|def| {
                    let spell_desc = match def.spell.id.as_str() {
                        "cleave" => "Cleaves all adjacent enemies in a 1-hex radial sweep for 15 physical damage.",
                        "bolt" => "Fires an energy-infused projectile dealing 25 damage (Range 3, requires LOS).",
                        "mend" => "Channels revitalizing energy to heal an allied unit for +20 HP (Range 2).",
                        "longshot" => "High-caliber artillery round dealing 30 damage (Range 4, Min Range 2, CD 3, requires LOS).",
                        "fury" => "Ignites furious rage, granting +8 Attack Damage buff for 2 rounds (CD 3).",
                        _ => def.spell.name.as_str(),
                    }.to_string();
                    HeroDto {
                        id: def.id.clone(),
                        name: def.name.clone(),
                        role: def.role.clone(),
                        max_hp: def.max_hp,
                        attack_damage: def.attack_damage,
                        attack_range: def.attack_range,
                        vision_range: def.vision_range,
                        max_energy: def.max_energy,
                        spell_id: def.spell.id.clone(),
                        spell_name: def.spell.name.clone(),
                        spell_desc,
                    }
                })
                .collect();
            pools.insert(team, dtos);
        }
        pools
    }

    pub fn send_to_player(&self, player_id: &str, msg: ServerMessage) {
        if let Some(conn) = self.players.get(player_id) {
            conn.send(msg);
        }
    }

    pub fn broadcast_to_team(&self, team: TeamId, msg: ServerMessage) {
        for conn in self.players.values().filter(|p| p.team == team) {
            conn.send(msg.clone());
        }
    }

    pub fn broadcast(&self, msg: ServerMessage) {
        for conn in self.players.values() {
            conn.send(msg.clone());
        }
    }

    pub async fn handle_buy_item(&mut self, player_id: &str, item_id: &str) {
        if self.phase != MatchPhaseDto::Planning {
            self.send_to_player(
                player_id,
                ServerMessage::Error {
                    error_code: ProtocolErrorCode::CannotShopInPhase,
                    message: "Shopping is only allowed during Planning phase".to_string(),
                },
            );
            return;
        }

        let session = match self.session.as_mut() {
            Some(s) => s,
            None => return,
        };

        let conn = match self.players.get(player_id) {
            Some(c) => c,
            None => return,
        };

        let unit_id = if let Some(uid) = conn.hero_unit_id {
            Some(uid)
        } else if let Some(uid) = session
            .controllers
            .get_units_for_player(&player_id.to_string())
            .into_iter()
            .next()
        {
            Some(uid)
        } else {
            let team_heroes: Vec<UnitId> = session
                .state
                .units
                .values()
                .filter(|u| u.team == conn.team && u.is_hero())
                .map(|u| u.id)
                .collect();
            if team_heroes.len() == 1 {
                team_heroes.first().copied()
            } else {
                None
            }
        };

        let Some(unit_id) = unit_id else {
            self.send_to_player(
                player_id,
                ServerMessage::Error {
                    error_code: ProtocolErrorCode::NotYourUnit,
                    message: "No hero assigned to player".to_string(),
                },
            );
            return;
        };

        let item_catalog = hexabellum_core::items::get_canonical_item_catalog();
        let Some(item) = item_catalog.iter().find(|i| i.id == item_id) else {
            let gold = session.state.units.get(&unit_id).map_or(0, |u| u.gold);
            self.send_to_player(
                player_id,
                ServerMessage::PurchaseResolved {
                    unit_id,
                    item_id: item_id.to_string(),
                    success: false,
                    gold_remaining: gold,
                    error: Some(ProtocolErrorCode::NoSuchItem),
                },
            );
            return;
        };

        let (hero_pos, hero_team, is_alive, is_dead_awaiting_respawn, current_gold) = {
            let u = match session.state.units.get(&unit_id) {
                Some(u) => u,
                None => return,
            };
            (u.pos, u.team, u.is_alive(), u.is_dead_awaiting_respawn(), u.gold)
        };

        if is_dead_awaiting_respawn || !is_alive {
            self.send_to_player(
                player_id,
                ServerMessage::PurchaseResolved {
                    unit_id,
                    item_id: item_id.to_string(),
                    success: false,
                    gold_remaining: current_gold,
                    error: Some(ProtocolErrorCode::HeroDeadAwaitingRespawn),
                },
            );
            return;
        }

        let in_base = session
            .base_zones
            .get(&hero_team)
            .map(|bz| bz.contains(hero_pos))
            .unwrap_or(false);

        if !in_base {
            self.send_to_player(
                player_id,
                ServerMessage::PurchaseResolved {
                    unit_id,
                    item_id: item_id.to_string(),
                    success: false,
                    gold_remaining: current_gold,
                    error: Some(ProtocolErrorCode::CannotShopOutsideBase),
                },
            );
            return;
        }

        let max_slots = session.config.economy.max_item_slots;
        let allow_duplicates = session.config.economy.allow_duplicate_items;
        let unit = match session.state.units.get_mut(&unit_id) {
            Some(u) => u,
            None => return,
        };

        match hexabellum_core::shop::execute_purchase(unit, item, max_slots, allow_duplicates) {
            Ok(()) => {
                let gold_remaining = unit.gold;
                let items_cloned = unit.items.clone();
                let xp = unit.xp;
                let level = unit.level;
                let team = unit.team;

                self.send_to_player(
                    player_id,
                    ServerMessage::PurchaseResolved {
                        unit_id,
                        item_id: item_id.to_string(),
                        success: true,
                        gold_remaining,
                        error: None,
                    },
                );

                self.broadcast_to_team(
                    team,
                    ServerMessage::EconomyUpdated {
                        unit_id,
                        gold: gold_remaining,
                        xp,
                        level,
                        items: items_cloned,
                    },
                );
            }
            Err(err) => {
                let gold_remaining = unit.gold;
                self.send_to_player(
                    player_id,
                    ServerMessage::PurchaseResolved {
                        unit_id,
                        item_id: item_id.to_string(),
                        success: false,
                        gold_remaining,
                        error: Some(err),
                    },
                );
            }
        }
    }

    pub fn broadcast_lobby_state(&self) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let countdown_ms = self
            .hero_select_deadline_unix_ms
            .map(|deadline| deadline.saturating_sub(now_ms));

        let msg = ServerMessage::LobbyUpdated {
            match_id: self.match_id.clone(),
            phase: self.phase,
            players: self.players.values().map(|p| p.to_lobby_dto()).collect(),
            hero_pools: self.build_hero_pools_dto(),
            countdown_ms,
        };
        self.broadcast(msg);
    }
}
