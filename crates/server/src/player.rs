use hexabellum_protocol::{
    HeroDefId, PlayerId, PlayerLobbyDto, ReconnectToken, ServerMessage, TeamId, UnitId,
};
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    AiReplacement,
}

/// Player connection and authentication session.
#[derive(Debug)]
pub struct PlayerConnection {
    pub player_id: PlayerId,
    pub display_name: String,
    pub reconnect_token: ReconnectToken,
    pub team: TeamId,
    pub sender: Option<mpsc::UnboundedSender<ServerMessage>>,
    pub last_seen: Instant,
    pub is_connected: bool,
    pub is_ready: bool,
    pub hero_def_id: Option<HeroDefId>,
    pub hero_unit_id: Option<UnitId>,
    pub is_ai: bool,
    pub connection_state: ConnectionState,
    pub connection_generation: u64,
}

impl PlayerConnection {
    pub fn new(
        player_id: PlayerId,
        team: TeamId,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) -> Self {
        let reconnect_token = uuid::Uuid::new_v4().to_string();
        Self {
            display_name: player_id.clone(),
            player_id,
            reconnect_token,
            team,
            sender: Some(sender),
            last_seen: Instant::now(),
            is_connected: true,
            is_ready: false,
            hero_def_id: None,
            hero_unit_id: None,
            is_ai: false,
            connection_state: ConnectionState::Connected,
            connection_generation: 1,
        }
    }

    pub fn new_ai(hero_def_id: HeroDefId, team: TeamId, unit_id: UnitId) -> Self {
        let player_id = format!("ai_{}_{}", hero_def_id, team);
        Self {
            display_name: format!("AI ({})", hero_def_id),
            player_id,
            reconnect_token: uuid::Uuid::new_v4().to_string(),
            team,
            sender: None,
            last_seen: Instant::now(),
            is_connected: true,
            is_ready: true,
            hero_def_id: Some(hero_def_id),
            hero_unit_id: Some(unit_id),
            is_ai: true,
            connection_state: ConnectionState::AiReplacement,
            connection_generation: 1,
        }
    }

    pub fn send(&self, msg: ServerMessage) {
        if let Some(ref tx) = self.sender {
            let _ = tx.send(msg);
        }
    }

    pub fn to_lobby_dto(&self) -> PlayerLobbyDto {
        PlayerLobbyDto {
            player_id: self.player_id.clone(),
            display_name: self.display_name.clone(),
            team: self.team,
            connected: self.is_connected,
            ready: self.is_ready,
            hero_def_id: self.hero_def_id.clone(),
            is_ai: self.is_ai,
            ping_ms: None,
        }
    }
}
