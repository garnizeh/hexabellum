use hexabellum_protocol::{PlayerId, ReconnectToken, ServerMessage, TeamId};
use std::time::Instant;
use tokio::sync::mpsc;

/// Player connection and authentication session.
pub struct PlayerConnection {
    pub player_id: PlayerId,
    pub reconnect_token: ReconnectToken,
    pub team: TeamId,
    pub sender: Option<mpsc::Sender<ServerMessage>>,
    pub last_seen: Instant,
    pub is_connected: bool,
}

impl PlayerConnection {
    pub fn new(player_id: PlayerId, team: TeamId, sender: mpsc::Sender<ServerMessage>) -> Self {
        let reconnect_token = uuid::Uuid::new_v4().to_string();
        Self {
            player_id,
            reconnect_token,
            team,
            sender: Some(sender),
            last_seen: Instant::now(),
            is_connected: true,
        }
    }

    pub fn send(&self, msg: ServerMessage) {
        if let Some(ref tx) = self.sender {
            let tx = tx.clone();
            tokio::spawn(async move {
                let _ = tx.send(msg).await;
            });
        }
    }
}
