use crate::match_actor::MatchCommand;
use crate::MatchRegistry;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use hexabellum_protocol::{ClientMessage, MatchId, PlayerId, ReconnectToken, ServerMessage};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{debug, warn};

#[derive(Deserialize)]
pub struct WsQuery {
    pub player_id: PlayerId,
    pub reconnect_token: Option<ReconnectToken>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(match_id): Path<MatchId>,
    Query(query): Query<WsQuery>,
    State(registry): State<MatchRegistry>,
) -> impl IntoResponse {
    let actor_handle = registry.get(&match_id).map(|r| r.clone());

    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            match_id,
            query.player_id,
            query.reconnect_token,
            actor_handle,
        )
    })
}

async fn handle_socket(
    socket: WebSocket,
    match_id: MatchId,
    player_id: PlayerId,
    reconnect_token: Option<ReconnectToken>,
    actor_handle: Option<crate::match_actor::MatchActorHandle>,
) {
    let handle = match actor_handle {
        Some(h) => h,
        None => {
            warn!(
                "Rejected WebSocket connection for unknown match: {}",
                match_id
            );
            return;
        }
    };

    let (mut ws_sink, mut ws_stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(64);

    // Forward outgoing server messages to client WebSocket
    let outgoing_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg)
                && ws_sink.send(Message::Text(json.into())).await.is_err()
            {
                break;
            }
        }
    });

    // Notify actor of connection
    handle
        .send(MatchCommand::PlayerConnect {
            player_id: player_id.clone(),
            reconnect_token,
            sender: tx.clone(),
        })
        .await;

    // Ingest incoming client WebSocket messages
    let p_id = player_id.clone();
    let h = handle.clone();

    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client_msg) => match client_msg {
                    ClientMessage::SubmitOrders { round, orders } => {
                        h.send(MatchCommand::SubmitOrders {
                            player_id: p_id.clone(),
                            round,
                            orders,
                        })
                        .await;
                    }
                    ClientMessage::Ping { client_time_ms } => {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;
                        let _ = tx
                            .send(ServerMessage::Pong {
                                client_time_ms,
                                server_time_ms: now_ms,
                            })
                            .await;
                    }
                    _ => {}
                },
                Err(err) => {
                    debug!("Failed to parse client message: {:?}", err);
                }
            },
            Message::Close(_) => break,
            _ => {}
        }
    }

    outgoing_task.abort();
    handle
        .send(MatchCommand::PlayerDisconnect { player_id })
        .await;
}
