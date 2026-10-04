use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use hexabellum_core::controller::Controller;
use hexabellum_core::session::BattleConfig;
use hexabellum_protocol::{
    ClientMessage, MatchPhaseDto, ProtocolErrorCode, ServerMessage,
};
use hexabellum_server::{build_router, MatchActor, MatchActorHandle, MatchRegistry};
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
async fn test_ws_buy_item_lifecycle_and_rejections() {
    let registry: MatchRegistry = Arc::new(DashMap::new());
    let mut config = BattleConfig::default();
    config.skip_draft = true;
    config.enable_ai_team_1 = true;
    config.turn_duration_secs = 60;
    // Set starting gold to 120 so hero starts with 120 + 6 = 126 gold
    config.economy.starting_gold = 120;

    let handle = MatchActorHandle::new("eco_match_1".to_string(), config, Some(registry.clone()));
    registry.insert("eco_match_1".to_string(), handle);

    let (addr, _server_task) = start_test_server(registry).await;
    let ws_url = format!("ws://{}/ws/match/eco_match_1?player_id=p1", addr);

    let (mut ws_stream, _) = connect_async(&ws_url).await.unwrap();

    // 1. Read HelloAck
    let msg1 = ws_stream.next().await.unwrap().unwrap();
    let smsg1: ServerMessage = serde_json::from_str(msg1.to_text().unwrap()).unwrap();
    assert!(matches!(smsg1, ServerMessage::HelloAck { .. }));

    // 2. Read MatchJoined
    let msg2 = ws_stream.next().await.unwrap().unwrap();
    let smsg2: ServerMessage = serde_json::from_str(msg2.to_text().unwrap()).unwrap();
    let mut current_gold = 0;
    if let ServerMessage::MatchJoined { snapshot, .. } = smsg2 {
        if let Some(eco) = snapshot.controlled_hero_economy {
            current_gold = eco.gold;
        }
    }
    assert!(current_gold == 120 || current_gold == 126);

    // 3. Buy non-existent item -> NoSuchItem
    let buy_fake = ClientMessage::BuyItem {
        item_id: "infinity_edge".into(),
    };
    ws_stream
        .send(Message::Text(serde_json::to_string(&buy_fake).unwrap().into()))
        .await
        .unwrap();

    while let Some(msg_res) = ws_stream.next().await {
        let msg = msg_res.unwrap();
        let text = msg.to_text().unwrap();
        let server_msg: ServerMessage = serde_json::from_str(text).unwrap();
        if let ServerMessage::PurchaseResolved {
            success,
            error,
            ..
        } = server_msg
        {
            assert!(!success);
            assert_eq!(error, Some(ProtocolErrorCode::NoSuchItem));
            break;
        }
    }

    // 4. Buy legitimate item: Longblade (cost 100) -> Success!
    let buy_blade = ClientMessage::BuyItem {
        item_id: "longblade".into(),
    };
    ws_stream
        .send(Message::Text(serde_json::to_string(&buy_blade).unwrap().into()))
        .await
        .unwrap();

    let mut got_purchase_ack = false;
    let mut got_economy_update = false;
    while let Some(msg_res) = ws_stream.next().await {
        let msg = msg_res.unwrap();
        let text = msg.to_text().unwrap();
        let server_msg: ServerMessage = serde_json::from_str(text).unwrap();
        match server_msg {
            ServerMessage::PurchaseResolved {
                success,
                gold_remaining,
                error,
                ..
            } => {
                assert!(success);
                assert_eq!(error, None);
                assert_eq!(gold_remaining, 26);
                got_purchase_ack = true;
            }
            ServerMessage::EconomyUpdated { gold, items, .. } => {
                assert_eq!(gold, 26);
                assert_eq!(items, vec!["longblade".to_string()]);
                got_economy_update = true;
            }
            _ => {}
        }
        if got_purchase_ack && got_economy_update {
            break;
        }
    }
    assert!(got_purchase_ack);
    assert!(got_economy_update);

    // 5. Try buying duplicate Longblade -> ItemAlreadyOwned
    let buy_blade_again = ClientMessage::BuyItem {
        item_id: "longblade".into(),
    };
    ws_stream
        .send(Message::Text(serde_json::to_string(&buy_blade_again).unwrap().into()))
        .await
        .unwrap();

    while let Some(msg_res) = ws_stream.next().await {
        let msg = msg_res.unwrap();
        let text = msg.to_text().unwrap();
        let server_msg: ServerMessage = serde_json::from_str(text).unwrap();
        if let ServerMessage::PurchaseResolved {
            success,
            error,
            ..
        } = server_msg
        {
            assert!(!success);
            assert_eq!(error, Some(ProtocolErrorCode::ItemAlreadyOwned));
            break;
        }
    }

    // 6. Try buying Focus Charm (cost 100) with remaining gold (20 or 26) -> InsufficientGold
    let buy_charm = ClientMessage::BuyItem {
        item_id: "focus_charm".into(),
    };
    ws_stream
        .send(Message::Text(serde_json::to_string(&buy_charm).unwrap().into()))
        .await
        .unwrap();

    while let Some(msg_res) = ws_stream.next().await {
        let msg = msg_res.unwrap();
        let text = msg.to_text().unwrap();
        let server_msg: ServerMessage = serde_json::from_str(text).unwrap();
        if let ServerMessage::PurchaseResolved {
            success,
            error,
            ..
        } = server_msg
        {
            assert!(!success);
            assert_eq!(error, Some(ProtocolErrorCode::InsufficientGold));
            break;
        }
    }
}

#[tokio::test]
async fn test_ai_bot_automatic_shopping_and_fog_masking() {
    let mut actor = MatchActor::new_test_match();
    actor.setup_5v5_session();
    actor.config.economy.starting_gold = 120;
    
    // Assign heroes 6..=10 to AI controllers and give them starting gold
    if let Some(session) = actor.session.as_mut() {
        for uid in 6..=10 {
            session.controllers.assign(uid, Controller::Ai);
        }
        for u in session.state.units.values_mut() {
            u.gold = 120;
        }
    }

    // Run start_planning_phase: distributes passive income (+6) and triggers AI shopping
    actor.start_planning_phase().await;

    let session = actor.session.as_ref().unwrap();

    // Verify AI heroes on team 1 purchased Plate Armor (120g)
    let ai_units_with_armor = session
        .state
        .units
        .values()
        .filter(|u| u.team == 1 && u.is_hero() && u.items.contains(&"plate_armor".to_string()))
        .count();
    assert!(
        ai_units_with_armor > 0,
        "Expected AI heroes to have automatically purchased plate_armor"
    );

    // Verify Zero-Knowledge Fog Masking:
    // Team 0 snapshot must NOT reveal gold or xp of enemy heroes,
    // and items of enemies in fog must be masked to empty!
    let t0_snapshot = session.snapshot_for_player(0, None, None);
    let visible_hexes = session.state.fog.visible_hexes(0);

    for u in &t0_snapshot.units {
        if u.team == 1 && u.kind == "Hero" {
            // Enemy gold and XP are strictly None
            assert!(u.gold.is_none(), "Enemy gold must be None in snapshot");
            assert!(u.xp.is_none(), "Enemy xp must be None in snapshot");

            let hex = hexabellum_core::hex::HexCoord::new(u.pos.q, u.pos.r);
            if !visible_hexes.contains(&hex) {
                assert!(u.items.is_empty(), "Enemy items in fog must be masked");
                assert_eq!(u.level, 1, "Enemy level in fog must be default 1");
            }
        }
    }

    for r in &t0_snapshot.roster {
        if r.team == 1 {
            // In roster, enemies not in LOS must also have items masked
            let enemy_unit = session.state.units.get(&r.unit_id).unwrap();
            if !visible_hexes.contains(&enemy_unit.pos) {
                assert!(r.items.is_empty(), "Enemy items in fog roster must be masked");
                assert_eq!(r.level, 1, "Enemy level in fog roster must be default 1");
            }
        }
    }
}

#[tokio::test]
async fn test_cannot_shop_outside_planning_phase() {
    let mut actor = MatchActor::new_test_match();
    actor.setup_5v5_session();
    // Set actor phase to Resolution
    actor.phase = MatchPhaseDto::Resolution;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    actor.players.get_mut("player_1").unwrap().sender = Some(tx);

    actor.handle_buy_item("player_1", "longblade").await;

    let msg = rx.try_recv().expect("Expected error message");
    match msg {
        ServerMessage::Error { error_code, .. } => {
            assert_eq!(error_code, ProtocolErrorCode::CannotShopInPhase);
        }
        other => panic!("Expected Error with CannotShopInPhase, got {:?}", other),
    }
}
