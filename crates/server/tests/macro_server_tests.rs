use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use hexabellum_core::session::BattleConfig;
use hexabellum_protocol::{
    ActionDto, ClientMessage, OrderDto, ProtocolErrorCode, ServerMessage,
};
use hexabellum_server::{build_router, MatchActorHandle, MatchRegistry};
use std::net::SocketAddr;
use std::sync::Arc;
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
async fn test_ws_phase7_macro_shopping_rules() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let mut config = BattleConfig::default();
    config.skip_draft = true;
    config.enable_ai_team_1 = true;
    config.turn_duration_secs = 60;
    config.economy.starting_gold = 200;

    let handle = MatchActorHandle::new("macro_match_1".to_string(), config, Some(registry.clone()));
    registry.insert("macro_match_1".to_string(), handle);

    let (addr, _server_task) = start_test_server(registry).await;
    let ws_url = format!("ws://{}/ws/match/macro_match_1?player_id=p1", addr);

    let (mut ws_stream, _) = connect_async(&ws_url).await.unwrap();

    // 1. Read HelloAck
    let msg1 = ws_stream.next().await.unwrap().unwrap();
    let smsg1: ServerMessage = serde_json::from_str(msg1.to_text().unwrap()).unwrap();
    assert!(matches!(smsg1, ServerMessage::HelloAck { .. }));

    // 2. Read MatchJoined
    let msg2 = ws_stream.next().await.unwrap().unwrap();
    let smsg2: ServerMessage = serde_json::from_str(msg2.to_text().unwrap()).unwrap();
    if let ServerMessage::MatchJoined { snapshot, .. } = smsg2 {
        assert_eq!(snapshot.victory_mode, Some("CoreDestruction".to_string()));
        assert!(!snapshot.base_zones.is_empty());
        assert!(!snapshot.core_hp.is_empty());
        assert!(snapshot.objective.is_some());
    }

    // Read RoundStarted
    let msg3 = ws_stream.next().await.unwrap().unwrap();
    let smsg3: ServerMessage = serde_json::from_str(msg3.to_text().unwrap()).unwrap();
    assert!(matches!(smsg3, ServerMessage::RoundStarted { .. }));

    // Hero starts at (-6, 0), inside BaseZone (-7, 0) radius 2.
    // Purchasing longblade (cost 100) should succeed!
    let buy_blade = ClientMessage::BuyItem {
        item_id: "longblade".into(),
    };
    ws_stream
        .send(Message::Text(serde_json::to_string(&buy_blade).unwrap().into()))
        .await
        .unwrap();

    while let Some(msg_res) = ws_stream.next().await {
        let msg = msg_res.unwrap();
        let text = msg.to_text().unwrap();
        let server_msg: ServerMessage = serde_json::from_str(text).unwrap();
        if let ServerMessage::PurchaseResolved {
            success,
            error,
            item_id,
            ..
        } = server_msg
        {
            assert!(success);
            assert_eq!(error, None);
            assert_eq!(item_id, "longblade");
            break;
        }
    }
}

#[tokio::test]
async fn test_ws_phase7_order_validation_dead_hero_and_repair() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let mut config = BattleConfig::default();
    config.skip_draft = true;
    config.enable_ai_team_1 = true;
    config.turn_duration_secs = 60;

    let handle = MatchActorHandle::new("macro_match_2".to_string(), config, Some(registry.clone()));
    registry.insert("macro_match_2".to_string(), handle);

    let (addr, _server_task) = start_test_server(registry.clone()).await;
    let ws_url = format!("ws://{}/ws/match/macro_match_2?player_id=p1", addr);

    let (mut ws_stream, _) = connect_async(&ws_url).await.unwrap();

    let _hello = ws_stream.next().await.unwrap().unwrap();
    let _joined = ws_stream.next().await.unwrap().unwrap();
    let round_msg = ws_stream.next().await.unwrap().unwrap();
    let smsg: ServerMessage = serde_json::from_str(round_msg.to_text().unwrap()).unwrap();
    let mut hero_id = 1;
    let mut round = 1;
    if let ServerMessage::RoundStarted { round: r, snapshot, .. } = smsg {
        round = r;
        if let Some(uid) = snapshot.controlled_units.first() {
            hero_id = *uid;
        }
    }

    // Try to submit a repair order targeting Team 0 Core (id 500)
    let repair_core = ClientMessage::SubmitOrders {
        round,
        orders: vec![OrderDto {
            unit_id: hero_id,
            move_target: None,
            action: ActionDto::Repair { target_id: 500 },
        }],
    };

    ws_stream
        .send(Message::Text(serde_json::to_string(&repair_core).unwrap().into()))
        .await
        .unwrap();

    while let Some(msg_res) = ws_stream.next().await {
        let msg = msg_res.unwrap();
        let text = msg.to_text().unwrap();
        let server_msg: ServerMessage = serde_json::from_str(text).unwrap();
        if let ServerMessage::OrderRejected { error_code, .. } = server_msg {
            assert_eq!(error_code, ProtocolErrorCode::CoreCannotBeRepaired);
            break;
        }
    }
}
