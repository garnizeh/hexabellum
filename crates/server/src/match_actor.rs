use crate::player::PlayerConnection;
use hexabellum_core::session::{BattleConfig, BattleSession};
use hexabellum_protocol::{
    OrderDto, PlayerId, ProtocolErrorCode, ReconnectToken, Round,
    ServerMessage, TeamId,
};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::info;

/// Actor commands routed from WebSocket and HTTP handlers.
pub enum MatchCommand {
    PlayerConnect {
        player_id: PlayerId,
        reconnect_token: Option<ReconnectToken>,
        sender: mpsc::Sender<ServerMessage>,
    },
    PlayerDisconnect {
        player_id: PlayerId,
    },
    SubmitOrders {
        player_id: PlayerId,
        round: Round,
        orders: Vec<OrderDto>,
    },
    TurnTimerFired {
        round: Round,
    },
    GracePeriodFired {
        round: Round,
    },
}

/// Actor handle held by the registry.
#[derive(Clone)]
pub struct MatchActorHandle {
    pub tx: mpsc::Sender<MatchCommand>,
}

impl MatchActorHandle {
    pub fn new(match_id: String, config: BattleConfig) -> Self {
        let (tx, rx) = mpsc::channel(128);
        let actor = MatchActor::new(match_id, config, tx.clone());
        tokio::spawn(actor.run(rx));
        Self { tx }
    }

    pub async fn send(&self, cmd: MatchCommand) {
        let _ = self.tx.send(cmd).await;
    }
}

pub struct MatchActor {
    pub match_id: String,
    pub session: BattleSession,
    pub players: HashMap<PlayerId, PlayerConnection>,
    pub human_teams: Vec<TeamId>,
    pub turn_deadline_unix_ms: Option<u64>,
    pub grace_timer_active: bool,
    pub self_tx: mpsc::Sender<MatchCommand>,
}

impl MatchActor {
    pub fn new(
        match_id: String,
        config: BattleConfig,
        self_tx: mpsc::Sender<MatchCommand>,
    ) -> Self {
        let session = BattleSession::new(match_id.clone(), config);
        Self {
            match_id,
            session,
            players: HashMap::new(),
            human_teams: Vec::new(),
            turn_deadline_unix_ms: None,
            grace_timer_active: false,
            self_tx,
        }
    }

    pub async fn run(mut self, mut rx: mpsc::Receiver<MatchCommand>) {
        info!("MatchActor [{}] started", self.match_id);

        while let Some(cmd) = rx.recv().await {
            match cmd {
                MatchCommand::PlayerConnect {
                    player_id,
                    reconnect_token,
                    sender,
                } => {
                    self.handle_player_connect(player_id, reconnect_token, sender)
                        .await;
                }
                MatchCommand::PlayerDisconnect { player_id } => {
                    self.handle_player_disconnect(player_id);
                }
                MatchCommand::SubmitOrders {
                    player_id,
                    round,
                    orders,
                } => {
                    self.handle_submit_orders(player_id, round, orders).await;
                }
                MatchCommand::TurnTimerFired { round } => {
                    if self.session.state.round == round {
                        info!(
                            "MatchActor [{}] Turn timer expired for round {}",
                            self.match_id, round
                        );
                        self.resolve_round().await;
                    }
                }
                MatchCommand::GracePeriodFired { round } => {
                    if self.session.state.round == round && self.grace_timer_active {
                        info!(
                            "MatchActor [{}] Early resolution grace period elapsed for round {}",
                            self.match_id, round
                        );
                        self.resolve_round().await;
                    }
                }
            }
        }

        info!("MatchActor [{}] terminated", self.match_id);
    }

    async fn handle_player_connect(
        &mut self,
        player_id: PlayerId,
        reconnect_token: Option<ReconnectToken>,
        sender: mpsc::Sender<ServerMessage>,
    ) {
        // Reconnection check
        if let Some(conn) = self.players.get_mut(&player_id) {
            let valid_token = reconnect_token.as_ref() == Some(&conn.reconnect_token);
            if valid_token || !conn.is_connected {
                conn.sender = Some(sender.clone());
                conn.is_connected = true;
                conn.last_seen = Instant::now();

                let _ = sender
                    .send(ServerMessage::HelloAck {
                        player_id: player_id.clone(),
                        reconnect_token: conn.reconnect_token.clone(),
                    })
                    .await;

                let snapshot = self
                    .session
                    .snapshot_for_team(conn.team, self.turn_deadline_unix_ms);
                let _ = sender
                    .send(ServerMessage::MatchJoined {
                        match_id: self.match_id.clone(),
                        player_id: player_id.clone(),
                        team: conn.team,
                        is_spectator: false,
                        snapshot: snapshot.clone(),
                    })
                    .await;

                if let Some(deadline) = self.turn_deadline_unix_ms {
                    let _ = sender
                        .send(ServerMessage::RoundStarted {
                            round: self.session.state.round,
                            deadline_unix_ms: deadline,
                            snapshot,
                        })
                        .await;
                }
                return;
            }
        }

        // New player connection
        if self.players.len() >= 2 {
            let _ = sender
                .send(ServerMessage::Error {
                    error_code: ProtocolErrorCode::MatchFull,
                    message: "Match is full".into(),
                })
                .await;
            return;
        }

        let assigned_team = if self.players.is_empty() { 0 } else { 1 };
        let conn = PlayerConnection::new(player_id.clone(), assigned_team, sender.clone());
        let token = conn.reconnect_token.clone();

        self.session
            .assign_team_player(assigned_team, player_id.clone());
        self.human_teams.push(assigned_team);
        self.players.insert(player_id.clone(), conn);

        let _ = sender
            .send(ServerMessage::HelloAck {
                player_id: player_id.clone(),
                reconnect_token: token,
            })
            .await;

        let snapshot = self
            .session
            .snapshot_for_team(assigned_team, self.turn_deadline_unix_ms);
        let _ = sender
            .send(ServerMessage::MatchJoined {
                match_id: self.match_id.clone(),
                player_id,
                team: assigned_team,
                is_spectator: false,
                snapshot,
            })
            .await;

        // Auto-start match if 2 players or PvAI mode
        let ready_to_start = self.players.len() == 2
            || (self.players.len() == 1 && self.session.config.enable_ai_team_1);
        if ready_to_start && self.turn_deadline_unix_ms.is_none() {
            self.start_planning_phase().await;
        }
    }

    fn handle_player_disconnect(&mut self, player_id: PlayerId) {
        if let Some(conn) = self.players.get_mut(&player_id) {
            conn.is_connected = false;
            conn.sender = None;
            info!(
                "Player [{}] disconnected from match [{}]",
                player_id, self.match_id
            );
        }
    }

    async fn start_planning_phase(&mut self) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let deadline_ms = now_ms + (self.session.config.turn_duration_secs * 1000);
        self.turn_deadline_unix_ms = Some(deadline_ms);
        self.grace_timer_active = false;

        let round = self.session.state.round;

        for conn in self.players.values() {
            let snapshot = self.session.snapshot_for_team(conn.team, Some(deadline_ms));
            conn.send(ServerMessage::RoundStarted {
                round,
                deadline_unix_ms: deadline_ms,
                snapshot,
            });
        }

        // Spawn Tokio timer task for the 30-second deadline
        let tx = self.self_tx.clone();
        let duration = Duration::from_secs(self.session.config.turn_duration_secs);
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            let _ = tx.send(MatchCommand::TurnTimerFired { round }).await;
        });
    }

    async fn handle_submit_orders(
        &mut self,
        player_id: PlayerId,
        round: Round,
        orders: Vec<OrderDto>,
    ) {
        let team = match self.players.get(&player_id) {
            Some(conn) => conn.team,
            None => return,
        };

        match self
            .session
            .submit_player_orders(&player_id, team, round, orders)
        {
            Ok(()) => {
                if let Some(conn) = self.players.get(&player_id) {
                    conn.send(ServerMessage::OrdersAccepted { round });
                }

                // Check early resolution
                if self.session.all_human_teams_submitted(&self.human_teams)
                    && !self.grace_timer_active
                {
                    self.grace_timer_active = true;
                    let tx = self.self_tx.clone();
                    let grace_ms = self.session.config.early_resolution_grace_ms;
                    info!(
                        "All human orders submitted for round {}. Triggering {}ms grace period",
                        round, grace_ms
                    );

                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(grace_ms)).await;
                        let _ = tx.send(MatchCommand::GracePeriodFired { round }).await;
                    });
                }
            }
            Err(code) => {
                if let Some(conn) = self.players.get(&player_id) {
                    conn.send(ServerMessage::OrderRejected {
                        round,
                        error_code: code,
                        reason: format!("{:?}", code),
                    });
                }
            }
        }
    }

    async fn resolve_round(&mut self) {
        self.grace_timer_active = false;
        self.turn_deadline_unix_ms = None;

        let raw_events = self.session.resolve_round();
        let round = self.session.state.round;
        let is_ended = self.session.state.winner.is_some()
            || self.session.state.phase == hexabellum_core::state::Phase::MatchEnd;

        for conn in self.players.values() {
            let team = conn.team;
            let sanitized_events = self.session.sanitize_events_for_team(team, &raw_events);
            let snapshot = self.session.snapshot_for_team(team, None);

            if is_ended {
                conn.send(ServerMessage::MatchEnded {
                    winner: self.session.state.winner,
                    snapshot,
                });
            } else {
                conn.send(ServerMessage::RoundResolved {
                    round,
                    events: sanitized_events,
                    snapshot,
                });
            }
        }

        if !is_ended {
            self.start_planning_phase().await;
        }
    }
}
