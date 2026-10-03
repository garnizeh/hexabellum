use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use hexabellum_core::session::BattleConfig;
use hexabellum_protocol::{
    ActionDto, ClientMessage, HexDto, OrderDto, ServerMessage,
};
use hexabellum_server::{build_router, MatchActorHandle, MatchRegistry};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn start_test_server(registry: MatchRegistry) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = build_router(registry);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, handle)
}

#[tokio::test]
async fn test_rest_api_health_and_match_lifecycle() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let (addr, server_task) = start_test_server(registry).await;
    let client = reqwest::Client::new();

    // 1. Health check
    let resp = client
        .get(format!("http://{}/api/health", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let health: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(health["status"], "ok");

    // 2. Create Match
    let resp = client
        .post(format!("http://{}/api/matches", addr))
        .json(&serde_json::json!({
            "enable_ai_team_1": true,
            "turn_duration_secs": 15
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let match_data: serde_json::Value = resp.json().await.unwrap();
    let match_id = match_data["match_id"].as_str().unwrap().to_string();
    assert!(match_data["ws_url"].as_str().unwrap().contains(&match_id));

    // 3. Match status existing
    let resp = client
        .get(format!("http://{}/api/matches/{}", addr, match_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let status_data: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(status_data["exists"], true);

    // 4. Match status non-existing
    let resp = client
        .get(format!("http://{}/api/matches/unknown-id", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    server_task.abort();
}

#[tokio::test]
async fn test_websocket_pvp_match_and_orders() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let match_id = "test-pvp-match".to_string();

    let mut config = BattleConfig::legacy_3v3();
    config.early_resolution_grace_ms = 100; // fast grace period for test
    config.turn_duration_secs = 5;

    let handle = MatchActorHandle::new(match_id.clone(), config, Some(registry.clone()));
    registry.insert(match_id.clone(), handle);

    let (addr, server_task) = start_test_server(registry).await;

    // Connect Player 1
    let ws_url_p1 = format!("ws://{}/ws/match/{}?player_id=p1", addr, match_id);
    let (mut ws_p1, _) = connect_async(&ws_url_p1).await.unwrap();

    // Player 1 receives HelloAck
    let msg1 = ws_p1.next().await.unwrap().unwrap();
    let smsg1: ServerMessage = serde_json::from_str(msg1.to_text().unwrap()).unwrap();
    let p1_token = match smsg1 {
        ServerMessage::HelloAck {
            player_id,
            reconnect_token,
        } => {
            assert_eq!(player_id, "p1");
            reconnect_token
        }
        other => panic!("Expected HelloAck, got {:?}", other),
    };

    // Player 1 receives MatchJoined for Team 0
    let msg2 = ws_p1.next().await.unwrap().unwrap();
    let smsg2: ServerMessage = serde_json::from_str(msg2.to_text().unwrap()).unwrap();
    match smsg2 {
        ServerMessage::MatchJoined { team, snapshot, .. } => {
            assert_eq!(team, 0);
            assert_eq!(snapshot.match_id, match_id);
            // Verify fog censoring: Enemy hero at (4, 0) should be obscured from Team 0 snapshot
            assert!(!snapshot.units.iter().any(|u| u.id == 5));
        }
        other => panic!("Expected MatchJoined, got {:?}", other),
    }

    // Connect Player 2
    let ws_url_p2 = format!("ws://{}/ws/match/{}?player_id=p2", addr, match_id);
    let (mut ws_p2, _) = connect_async(&ws_url_p2).await.unwrap();

    // Player 2 receives HelloAck
    let msg_p2_1 = ws_p2.next().await.unwrap().unwrap();
    let smsg_p2_1: ServerMessage = serde_json::from_str(msg_p2_1.to_text().unwrap()).unwrap();
    match smsg_p2_1 {
        ServerMessage::HelloAck { player_id, .. } => {
            assert_eq!(player_id, "p2");
        }
        other => panic!("Expected HelloAck, got {:?}", other),
    };

    // Player 2 receives MatchJoined for Team 1
    let msg_p2_2 = ws_p2.next().await.unwrap().unwrap();
    let smsg_p2_2: ServerMessage = serde_json::from_str(msg_p2_2.to_text().unwrap()).unwrap();
    match smsg_p2_2 {
        ServerMessage::MatchJoined { team, snapshot, .. } => {
            assert_eq!(team, 1);
            // Team 1 sees Hero 5
            assert!(snapshot.units.iter().any(|u| u.id == 5));
        }
        other => panic!("Expected MatchJoined, got {:?}", other),
    }

    // Player 1 receives OpponentStatus { online: true }
    let opp_p1 = ws_p1.next().await.unwrap().unwrap();
    let smsg_opp_p1: ServerMessage = serde_json::from_str(opp_p1.to_text().unwrap()).unwrap();
    assert_eq!(smsg_opp_p1, ServerMessage::OpponentStatus { online: true });

    // Player 2 receives OpponentStatus { online: true }
    let opp_p2 = ws_p2.next().await.unwrap().unwrap();
    let smsg_opp_p2: ServerMessage = serde_json::from_str(opp_p2.to_text().unwrap()).unwrap();
    assert_eq!(smsg_opp_p2, ServerMessage::OpponentStatus { online: true });

    // Both players should receive RoundStarted
    let start_p1 = ws_p1.next().await.unwrap().unwrap();
    let smsg_start_p1: ServerMessage =
        serde_json::from_str(start_p1.to_text().unwrap()).unwrap();
    match smsg_start_p1 {
        ServerMessage::RoundStarted { round, .. } => {
            assert_eq!(round, 0);
        }
        other => panic!("Expected RoundStarted for P1, got {:?}", other),
    }

    let start_p2 = ws_p2.next().await.unwrap().unwrap();
    let smsg_start_p2: ServerMessage =
        serde_json::from_str(start_p2.to_text().unwrap()).unwrap();
    match smsg_start_p2 {
        ServerMessage::RoundStarted { round, .. } => {
            assert_eq!(round, 0);
        }
        other => panic!("Expected RoundStarted for P2, got {:?}", other),
    }

    // Ping / Pong test
    let ping_msg = ClientMessage::Ping {
        client_time_ms: 123456,
    };
    ws_p1
        .send(Message::Text(serde_json::to_string(&ping_msg).unwrap().into()))
        .await
        .unwrap();
    let pong_resp = ws_p1.next().await.unwrap().unwrap();
    let smsg_pong: ServerMessage = serde_json::from_str(pong_resp.to_text().unwrap()).unwrap();
    match smsg_pong {
        ServerMessage::Pong { client_time_ms, .. } => {
            assert_eq!(client_time_ms, 123456);
        }
        other => panic!("Expected Pong, got {:?}", other),
    }

    // Submit Orders from Player 1
    let p1_order = ClientMessage::SubmitOrders {
        round: 0,
        orders: vec![OrderDto {
            unit_id: 1,
            move_target: Some(HexDto::new(-3, -1)),
            action: ActionDto::Wait,
        }],
    };
    ws_p1
        .send(Message::Text(serde_json::to_string(&p1_order).unwrap().into()))
        .await
        .unwrap();
    let ack_p1 = ws_p1.next().await.unwrap().unwrap();
    let smsg_ack_p1: ServerMessage = serde_json::from_str(ack_p1.to_text().unwrap()).unwrap();
    match smsg_ack_p1 {
        ServerMessage::OrdersAccepted { round } => assert_eq!(round, 0),
        other => panic!("Expected OrdersAccepted for P1, got {:?}", other),
    }

    // Submit Orders from Player 2 (all human orders submitted -> triggers 100ms grace period)
    let p2_order = ClientMessage::SubmitOrders {
        round: 0,
        orders: vec![OrderDto {
            unit_id: 4,
            move_target: Some(HexDto::new(3, -1)),
            action: ActionDto::Wait,
        }],
    };
    ws_p2
        .send(Message::Text(serde_json::to_string(&p2_order).unwrap().into()))
        .await
        .unwrap();
    let ack_p2 = ws_p2.next().await.unwrap().unwrap();
    let smsg_ack_p2: ServerMessage = serde_json::from_str(ack_p2.to_text().unwrap()).unwrap();
    match smsg_ack_p2 {
        ServerMessage::OrdersAccepted { round } => assert_eq!(round, 0),
        other => panic!("Expected OrdersAccepted for P2, got {:?}", other),
    }

    // Wait for resolution and verify RoundResolved (after possible EarlyResolutionTriggered)
    let mut res_p1 = ws_p1.next().await.unwrap().unwrap();
    let mut smsg_res_p1: ServerMessage = serde_json::from_str(res_p1.to_text().unwrap()).unwrap();
    if matches!(smsg_res_p1, ServerMessage::EarlyResolutionTriggered { .. }) {
        res_p1 = ws_p1.next().await.unwrap().unwrap();
        smsg_res_p1 = serde_json::from_str(res_p1.to_text().unwrap()).unwrap();
    }
    match smsg_res_p1 {
        ServerMessage::RoundResolved {
            round,
            events,
            snapshot,
            state_hash: _,
        } => {
            assert_eq!(round, 0); // Round planned was 0
            assert!(!events.is_empty());
            assert_eq!(snapshot.round, 1);
        }
        other => panic!("Expected RoundResolved for P1, got {:?}", other),
    }

    let mut res_p2 = ws_p2.next().await.unwrap().unwrap();
    let mut smsg_res_p2: ServerMessage = serde_json::from_str(res_p2.to_text().unwrap()).unwrap();
    if matches!(smsg_res_p2, ServerMessage::EarlyResolutionTriggered { .. }) {
        res_p2 = ws_p2.next().await.unwrap().unwrap();
        smsg_res_p2 = serde_json::from_str(res_p2.to_text().unwrap()).unwrap();
    }
    match smsg_res_p2 {
        ServerMessage::RoundResolved {
            round,
            events,
            snapshot,
            state_hash: _,
        } => {
            assert_eq!(round, 0); // Round planned was 0
            assert!(!events.is_empty());
            assert_eq!(snapshot.round, 1);
        }
        other => panic!("Expected RoundResolved for P2, got {:?}", other),
    }

    // Both should receive next RoundStarted for Round 1
    let next_p1 = ws_p1.next().await.unwrap().unwrap();
    let smsg_next_p1: ServerMessage = serde_json::from_str(next_p1.to_text().unwrap()).unwrap();
    assert!(matches!(smsg_next_p1, ServerMessage::RoundStarted { round: 1, .. }));

    let next_p2 = ws_p2.next().await.unwrap().unwrap();
    let smsg_next_p2: ServerMessage = serde_json::from_str(next_p2.to_text().unwrap()).unwrap();
    assert!(matches!(smsg_next_p2, ServerMessage::RoundStarted { round: 1, .. }));

    // Test Reconnection: Close P1 and reconnect
    drop(ws_p1);
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Player 2 receives OpponentStatus { online: false } (after possible PlayerConnectionUpdated)
    let mut opp_drop_p2 = ws_p2.next().await.unwrap().unwrap();
    let mut smsg_opp_drop: ServerMessage = serde_json::from_str(opp_drop_p2.to_text().unwrap()).unwrap();
    if matches!(smsg_opp_drop, ServerMessage::PlayerConnectionUpdated { .. }) {
        opp_drop_p2 = ws_p2.next().await.unwrap().unwrap();
        smsg_opp_drop = serde_json::from_str(opp_drop_p2.to_text().unwrap()).unwrap();
    }
    assert_eq!(smsg_opp_drop, ServerMessage::OpponentStatus { online: false });

    let ws_reconnect_url = format!(
        "ws://{}/ws/match/{}?player_id=p1&reconnect_token={}",
        addr, match_id, p1_token
    );
    let (mut ws_reconn, _) = connect_async(&ws_reconnect_url).await.unwrap();

    let rec_msg1 = ws_reconn.next().await.unwrap().unwrap();
    let smsg_rec1: ServerMessage = serde_json::from_str(rec_msg1.to_text().unwrap()).unwrap();
    assert!(matches!(smsg_rec1, ServerMessage::HelloAck { .. }));

    let rec_msg2 = ws_reconn.next().await.unwrap().unwrap();
    let smsg_rec2: ServerMessage = serde_json::from_str(rec_msg2.to_text().unwrap()).unwrap();
    match smsg_rec2 {
        ServerMessage::MatchJoined { team, snapshot, .. } => {
            assert_eq!(team, 0);
            assert_eq!(snapshot.round, 1);
        }
        other => panic!("Expected MatchJoined on reconnect, got {:?}", other),
    }

    let rec_msg3 = ws_reconn.next().await.unwrap().unwrap();
    let smsg_rec3: ServerMessage = serde_json::from_str(rec_msg3.to_text().unwrap()).unwrap();
    assert!(matches!(smsg_rec3, ServerMessage::RoundStarted { round: 1, .. }));

    // Reconnecting player receives opponent online status
    let mut rec_msg4 = ws_reconn.next().await.unwrap().unwrap();
    let mut smsg_rec4: ServerMessage = serde_json::from_str(rec_msg4.to_text().unwrap()).unwrap();
    if matches!(smsg_rec4, ServerMessage::PlayerConnectionUpdated { .. }) {
        rec_msg4 = ws_reconn.next().await.unwrap().unwrap();
        smsg_rec4 = serde_json::from_str(rec_msg4.to_text().unwrap()).unwrap();
    }
    assert_eq!(smsg_rec4, ServerMessage::OpponentStatus { online: true });

    // Player 2 receives opponent back online status
    let mut opp_back_p2 = ws_p2.next().await.unwrap().unwrap();
    let mut smsg_opp_back: ServerMessage = serde_json::from_str(opp_back_p2.to_text().unwrap()).unwrap();
    if matches!(smsg_opp_back, ServerMessage::PlayerConnectionUpdated { .. }) {
        opp_back_p2 = ws_p2.next().await.unwrap().unwrap();
        smsg_opp_back = serde_json::from_str(opp_back_p2.to_text().unwrap()).unwrap();
    }
    assert_eq!(smsg_opp_back, ServerMessage::OpponentStatus { online: true });

    server_task.abort();
}

#[tokio::test]
async fn test_websocket_pvai_mode() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let match_id = "test-pvai-match".to_string();

    let mut config = BattleConfig::legacy_3v3();
    config.enable_ai_team_1 = true;
    config.early_resolution_grace_ms = 100;
    config.turn_duration_secs = 5;

    let handle = MatchActorHandle::new(match_id.clone(), config, Some(registry.clone()));
    registry.insert(match_id.clone(), handle);

    let (addr, server_task) = start_test_server(registry).await;

    // Connect Player 1 only
    let ws_url = format!("ws://{}/ws/match/{}?player_id=p1", addr, match_id);
    let (mut ws, _) = connect_async(&ws_url).await.unwrap();

    // 1. HelloAck
    let msg1 = ws.next().await.unwrap().unwrap();
    assert!(matches!(
        serde_json::from_str(msg1.to_text().unwrap()).unwrap(),
        ServerMessage::HelloAck { .. }
    ));

    // 2. MatchJoined
    let msg2 = ws.next().await.unwrap().unwrap();
    assert!(matches!(
        serde_json::from_str(msg2.to_text().unwrap()).unwrap(),
        ServerMessage::MatchJoined { team: 0, .. }
    ));

    // 3. Since enable_ai_team_1 is true, RoundStarted fires immediately without waiting for P2
    let msg3 = ws.next().await.unwrap().unwrap();
    assert!(matches!(
        serde_json::from_str(msg3.to_text().unwrap()).unwrap(),
        ServerMessage::RoundStarted { round: 0, .. }
    ));

    // 4. Submit order
    let p1_order = ClientMessage::SubmitOrders {
        round: 0,
        orders: vec![OrderDto {
            unit_id: 1,
            move_target: Some(HexDto::new(-3, -1)),
            action: ActionDto::Wait,
        }],
    };
    ws.send(Message::Text(serde_json::to_string(&p1_order).unwrap().into()))
        .await
        .unwrap();

    let ack = ws.next().await.unwrap().unwrap();
    assert!(matches!(
        serde_json::from_str(ack.to_text().unwrap()).unwrap(),
        ServerMessage::OrdersAccepted { round: 0 }
    ));

    // 5. Resolves after 100ms grace period with Team 1 AI auto-filled
    let mut res = ws.next().await.unwrap().unwrap();
    let mut smsg_res: ServerMessage = serde_json::from_str(res.to_text().unwrap()).unwrap();
    if matches!(smsg_res, ServerMessage::EarlyResolutionTriggered { .. }) {
        res = ws.next().await.unwrap().unwrap();
        smsg_res = serde_json::from_str(res.to_text().unwrap()).unwrap();
    }
    match smsg_res {
        ServerMessage::RoundResolved { round, snapshot, .. } => {
            assert_eq!(round, 0); // Round planned was 0
            assert_eq!(snapshot.round, 1);
        }
        other => panic!("Expected RoundResolved, got {:?}", other),
    }

    server_task.abort();
}

#[tokio::test]
async fn test_reconnect_authorization_and_unknown_match() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let match_id = "test-auth-match".to_string();

    let mut config = BattleConfig::legacy_3v3();
    config.enable_ai_team_1 = true;

    let handle = MatchActorHandle::new(match_id.clone(), config, Some(registry.clone()));
    registry.insert(match_id.clone(), handle);

    let (addr, server_task) = start_test_server(registry.clone()).await;

    // 1. Connect to non-existing match -> expect MatchNotFound error
    let unknown_url = format!("ws://{}/ws/match/nonexistent-match?player_id=p1", addr);
    let (mut ws_unknown, _) = connect_async(&unknown_url).await.unwrap();
    let err_msg = ws_unknown.next().await.unwrap().unwrap();
    let smsg_err: ServerMessage = serde_json::from_str(err_msg.to_text().unwrap()).unwrap();
    match smsg_err {
        ServerMessage::Error { error_code, .. } => {
            assert_eq!(
                error_code,
                hexabellum_protocol::ProtocolErrorCode::MatchNotFound
            );
        }
        other => panic!("Expected Error::MatchNotFound, got {:?}", other),
    }

    // 2. Connect legitimate player to test-auth-match
    let valid_url = format!("ws://{}/ws/match/{}?player_id=p1", addr, match_id);
    let (mut ws_p1, _) = connect_async(&valid_url).await.unwrap();

    let msg1 = ws_p1.next().await.unwrap().unwrap();
    let smsg1: ServerMessage = serde_json::from_str(msg1.to_text().unwrap()).unwrap();
    let _token = match smsg1 {
        ServerMessage::HelloAck {
            reconnect_token, ..
        } => reconnect_token,
        other => panic!("Expected HelloAck, got {:?}", other),
    };

    drop(ws_p1);
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 3. Attempt reconnection with WRONG reconnect_token
    let wrong_token_url = format!(
        "ws://{}/ws/match/{}?player_id=p1&reconnect_token=WRONG-TOKEN",
        addr, match_id
    );
    let (mut ws_wrong, _) = connect_async(&wrong_token_url).await.unwrap();
    let auth_err_msg = ws_wrong.next().await.unwrap().unwrap();
    let smsg_auth_err: ServerMessage =
        serde_json::from_str(auth_err_msg.to_text().unwrap()).unwrap();
    match smsg_auth_err {
        ServerMessage::Error { error_code, .. } => {
            assert_eq!(
                error_code,
                hexabellum_protocol::ProtocolErrorCode::NotAuthorized
            );
        }
        other => panic!("Expected Error::NotAuthorized, got {:?}", other),
    }

    server_task.abort();
}

#[tokio::test]
async fn test_no_double_resolution_after_match_ended() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let match_id = "test-no-double-resolve".to_string();

    let mut config = BattleConfig::legacy_3v3();
    config.enable_ai_team_1 = true;
    config.turn_duration_secs = 1; // 1s timer for fast expiration

    let handle = MatchActorHandle::new(match_id.clone(), config, Some(registry.clone()));
    registry.insert(match_id.clone(), handle.clone());

    let (addr, server_task) = start_test_server(registry).await;

    let ws_url = format!("ws://{}/ws/match/{}?player_id=p1", addr, match_id);
    let (mut ws, _) = connect_async(&ws_url).await.unwrap();

    // Consume HelloAck, MatchJoined, RoundStarted
    let _ = ws.next().await.unwrap().unwrap(); // HelloAck
    let _ = ws.next().await.unwrap().unwrap(); // MatchJoined
    let _ = ws.next().await.unwrap().unwrap(); // RoundStarted

    // Force win condition directly on actor's session state by destroying Spawner (id 20)
    // Send TurnTimerFired to trigger resolve
    // TurnProcessor checks if spawner 20 is destroyed -> Team 0 wins
    handle
        .send(hexabellum_server::MatchCommand::SubmitOrders {
            player_id: "p1".into(),
            round: 0,
            orders: vec![],
        })
        .await;

    // Send TurnTimerFired for Round 0
    handle
        .send(hexabellum_server::MatchCommand::TurnTimerFired { round: 0 })
        .await;

    // Receive OrdersAccepted
    let _ack = ws.next().await.unwrap().unwrap();

    // Wait for resolution
    let mut resolved_or_ended = ws.next().await.unwrap().unwrap();
    let mut smsg: ServerMessage = serde_json::from_str(resolved_or_ended.to_text().unwrap()).unwrap();
    if matches!(smsg, ServerMessage::EarlyResolutionTriggered { .. }) {
        resolved_or_ended = ws.next().await.unwrap().unwrap();
        smsg = serde_json::from_str(resolved_or_ended.to_text().unwrap()).unwrap();
    }
    assert!(
        matches!(smsg, ServerMessage::RoundResolved { .. } | ServerMessage::MatchEnded { .. })
    );

    // If TurnTimerFired for round 0 or next round is sent again after round has resolved,
    // verify no duplicate resolution or errors occur
    handle
        .send(hexabellum_server::MatchCommand::TurnTimerFired { round: 0 })
        .await;

    // Allow time for any errant timer message
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify socket remains healthy and no unexpected extra messages arrived
    let timeout_res = tokio::time::timeout(Duration::from_millis(300), ws.next()).await;
    // Should either be next RoundStarted (if match not ended) or timeout (no double resolved event)
    if let Ok(Some(Ok(msg))) = timeout_res {
        let text = msg.to_text().unwrap();
        let smsg_extra: ServerMessage = serde_json::from_str(text).unwrap();
        assert!(
            matches!(smsg_extra, ServerMessage::RoundStarted { round: 1, .. }),
            "Expected RoundStarted for Round 1, got {:?}",
            smsg_extra
        );
    }

    server_task.abort();
}

#[tokio::test]
async fn test_delete_match_and_order_rejection_over_websocket() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let (addr, server_task) = start_test_server(registry.clone()).await;
    let client = reqwest::Client::new();

    // 1. Create Match via REST
    let resp = client
        .post(format!("http://{}/api/matches", addr))
        .json(&serde_json::json!({
            "enable_ai_team_1": true,
            "turn_duration_secs": 20
        }))
        .send()
        .await
        .unwrap();
    let match_data: serde_json::Value = resp.json().await.unwrap();
    let match_id = match_data["match_id"].as_str().unwrap().to_string();

    // 2. Connect WebSocket
    let ws_url = format!("ws://{}/ws/match/{}?player_id=p1", addr, match_id);
    let (mut ws, _) = connect_async(&ws_url).await.unwrap();

    let _ = ws.next().await.unwrap().unwrap(); // HelloAck
    let _ = ws.next().await.unwrap().unwrap(); // MatchJoined
    let _ = ws.next().await.unwrap().unwrap(); // RoundStarted

    // 3. Submit invalid order: StaleRound
    let stale_order = ClientMessage::SubmitOrders {
        round: 99,
        orders: vec![],
    };
    ws.send(Message::Text(serde_json::to_string(&stale_order).unwrap().into()))
        .await
        .unwrap();
    let reject_msg = ws.next().await.unwrap().unwrap();
    let smsg_rej: ServerMessage = serde_json::from_str(reject_msg.to_text().unwrap()).unwrap();
    match smsg_rej {
        ServerMessage::OrderRejected { error_code, reason: _, .. } => {
            assert!(
                error_code == hexabellum_protocol::ProtocolErrorCode::StaleRound
                    || error_code == hexabellum_protocol::ProtocolErrorCode::RoundMismatch
            );
        }
        other => panic!("Expected OrderRejected, got {:?}", other),
    }

    // 4. Submit invalid order: UnitNotOwned
    let unowned_order = ClientMessage::SubmitOrders {
        round: 0,
        orders: vec![OrderDto {
            unit_id: 4, // Hero 4 belongs to Team 1, not Team 0
            move_target: None,
            action: ActionDto::Wait,
        }],
    };
    ws.send(Message::Text(serde_json::to_string(&unowned_order).unwrap().into()))
        .await
        .unwrap();
    let reject_msg2 = ws.next().await.unwrap().unwrap();
    let smsg_rej2: ServerMessage = serde_json::from_str(reject_msg2.to_text().unwrap()).unwrap();
    match smsg_rej2 {
        ServerMessage::OrderRejected { error_code, reason, .. } => {
            assert!(
                error_code == hexabellum_protocol::ProtocolErrorCode::UnitNotOwned
                    || error_code == hexabellum_protocol::ProtocolErrorCode::NotYourUnit
            );
            assert!(reason.contains("Unit #4"));
        }
        other => panic!("Expected OrderRejected, got {:?}", other),
    }

    // 5. DELETE /api/matches/:id
    let del_resp = client
        .delete(format!("http://{}/api/matches/{}", addr, match_id))
        .send()
        .await
        .unwrap();
    assert_eq!(del_resp.status(), 204);

    // Verify GET returns 404 now
    let get_resp = client
        .get(format!("http://{}/api/matches/{}", addr, match_id))
        .send()
        .await
        .unwrap();
    assert_eq!(get_resp.status(), 404);

    server_task.abort();
}
