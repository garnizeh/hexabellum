use hexabellum_core::ability::{SpellCatalog, SpellTarget};
use hexabellum_core::controller::Controller;
use hexabellum_core::fog::TeamFogEngine;
use hexabellum_core::hex::{HexCoord, HexMap};
use hexabellum_core::orders::Action;
use hexabellum_core::session::{BattleConfig, BattleSession};
use hexabellum_core::state::GameState;
use hexabellum_core::turn::TurnProcessor;
use hexabellum_core::unit::Unit;
use hexabellum_protocol::{
    ActionDto, ErrorCode, HexCoordDto, OrderDto,
};
use hexabellum_server::{
    build_sanitized_snapshot, ConnectionState, HeroSelectDraft, MatchActor,
};
use std::collections::{HashMap, HashSet};

/// 5.1 Test 1: Order Ownership Permission Enforcement
#[tokio::test]
async fn test_controller_permission_enforcement() {
    let mut actor = MatchActor::new_test_match();
    actor.setup_5v5_session();

    // Player 1 attempts to issue orders for Player 2's hero (Unit 2)
    let unauthorized_order = OrderDto {
        unit_id: 2, // Hero owned by Player 2
        move_target: Some(HexCoordDto { q: 0, r: 0 }),
        action: ActionDto::Wait,
    };

    let result = actor.handle_submit_orders("player_1", 1, vec![unauthorized_order]);
    assert_eq!(result, Err(ErrorCode::NotYourUnit));
}

/// 5.1 Test 2: Hero Select Uniqueness & Timeout Auto-Assignment
#[test]
fn test_hero_select_uniqueness_and_timeout() {
    let mut draft = HeroSelectDraft::new_team_draft(vec![
        "vanguard".into(),
        "ranger".into(),
        "warden".into(),
        "sniper".into(),
        "berserker".into(),
    ]);

    // Player 1 selects Vanguard
    assert!(draft.select_hero("p1", "vanguard").is_ok());

    // Player 2 attempts to select Vanguard on the same team -> Rejection
    assert_eq!(
        draft.select_hero("p2", "vanguard"),
        Err(ErrorCode::HeroAlreadySelected)
    );

    // Register Player 2 in draft
    draft.register_player("p2");

    // Timer expires -> Unassigned players receive remaining pool heroes
    draft.handle_timeout();
    assert!(draft.get_selected_hero("p2").is_some());
    assert_ne!(draft.get_selected_hero("p2").unwrap(), "vanguard");
}

/// 5.2 Dynamic AI Backfill Integration Test
#[tokio::test]
async fn test_ai_backfill_and_reconnection() {
    let mut actor = MatchActor::new_test_match();
    actor.setup_5v5_session();

    // Simulate Player 3 disconnecting during Planning phase
    actor.handle_disconnect("player_3");
    assert_eq!(
        actor.players.get("player_3").unwrap().connection_state,
        ConnectionState::Disconnected
    );

    // Fast-forward turn deadline: Player 3's hero must be transitioned to AI
    actor.handle_turn_timeout();
    let hero_id = actor.players.get("player_3").unwrap().hero_unit_id.unwrap();
    assert_eq!(
        actor.session.as_ref().unwrap().controllers.get(hero_id),
        Some(&Controller::Ai)
    );

    // Player 3 reconnects: ownership must be restored
    actor.handle_reconnect("player_3");
    assert_eq!(
        actor.session.as_ref().unwrap().controllers.get(hero_id),
        Some(&Controller::Player("player_3".into()))
    );
}

/// 5.3 Zero-Knowledge Fog Sanitization Audit
#[test]
fn test_sanitizer_zero_knowledge_leak() {
    let session = BattleSession::new_test_5v5();

    // Team 0 has vision only on their side of the arena.
    // Enemy Berserker (Unit 10) starts at (6, 2) on Team 1 base.
    let snapshot = build_sanitized_snapshot(&session, 0, "player_1");

    // Assert enemy Berserker is completely absent from the units array
    let enemy_berserker = snapshot
        .units
        .iter()
        .find(|u| u.hero_id.as_deref() == Some("berserker") && u.team == 1);
    assert!(
        enemy_berserker.is_none(),
        "Concealed enemy hero leaked in client snapshot!"
    );

    // Assert enemy Berserker HP is masked in the public roster
    let roster_entry = snapshot
        .roster
        .iter()
        .find(|r| r.hero_def_id == "berserker" && r.team == 1)
        .unwrap();
    assert!(
        roster_entry.hp.is_none(),
        "Concealed enemy HP leaked in roster DTO!"
    );
}

/// 5.4 100-Round Headless 5v5 Soak & Determinism Test
#[test]
fn test_5v5_headless_ai_determinism() {
    let config = BattleConfig {
        map_radius: 8,
        players_per_team: 5,
        heroes_per_team: 5,
        fill_empty_slots_with_ai: true,
        ..Default::default()
    };

    let run_simulation = || {
        let mut session = BattleSession::new("sim".into(), config.clone());
        let mut state_hashes = Vec::new();

        for _round in 1..=100 {
            if session.state.winner.is_some() {
                break;
            }
            session.resolve_ai_round();
            state_hashes.push(session.calculate_state_hash());
        }
        state_hashes
    };

    let run_1 = run_simulation();
    let run_2 = run_simulation();

    assert_eq!(
        run_1, run_2,
        "Non-deterministic simulation detected in 5v5 soak test!"
    );
    assert!(run_1.len() > 10, "Simulation aborted prematurely!");
}

/// 5.5 Sniper Longshot & Berserker Fury Gameplay Mechanics Test
#[test]
fn test_sniper_longshot_and_berserker_fury() {
    // 1. Sniper Longshot validation (range 4, min range 2, 30 damage)
    let sniper = Unit::new_sniper(1, 0, HexCoord::new(0, 0), 10);
    let enemy = Unit::new_vanguard(2, 1, HexCoord::new(0, 4), 5); // Distance 4: valid
    let close_enemy = Unit::new_ranger(3, 1, HexCoord::new(0, 1), 5); // Distance 1: invalid min_range

    let longshot = SpellCatalog::get("longshot").expect("Longshot spell missing");
    assert_eq!(longshot.range, 4);
    assert_eq!(longshot.min_range, 2);
    assert_eq!(longshot.effects[0].amount, 30);

    // Valid target at range 4
    let valid_tgt = SpellTarget::Unit(2);

    // Cast longshot
    let mut orders = HashMap::new();
    orders.insert(
        1,
        hexabellum_core::orders::UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Cast {
                spell_id: "longshot".to_string(),
                target: valid_tgt,
            },
        },
    );

    let mut units = vec![sniper, enemy, close_enemy];
    let map = HexMap::new(8);
    let blockers = HashSet::new();
    let lane = hexabellum_core::lane::LaneDef::for_radius(8);
    TurnProcessor::process_round(1, &map, &blockers, &mut units, &mut [], &lane, orders);

    let hit_enemy = units.iter().find(|u| u.id == 2).unwrap();
    assert_eq!(hit_enemy.hp, 140 - 30); // 140 max HP - 30 damage = 110

    // 2. Berserker Fury validation (+8 attack damage buff for 2 rounds)
    let berserker = Unit::new_berserker(10, 0, HexCoord::new(0, 0), 10);
    let mut b_units = vec![berserker];
    let mut b_orders = HashMap::new();
    b_orders.insert(
        10,
        hexabellum_core::orders::UnitOrder {
            unit_id: 10,
            move_target: None,
            action: Action::Cast {
                spell_id: "fury".to_string(),
                target: SpellTarget::None,
            },
        },
    );

    TurnProcessor::process_round(1, &map, &blockers, &mut b_units, &mut [], &lane, b_orders);
    let buffed_berserker = &b_units[0];
    assert_eq!(buffed_berserker.effective_attack_damage(), 20 + 8); // 20 base + 8 fury = 28
    assert!(buffed_berserker.statuses.iter().any(|s| s.def_id == "fury_buff"));
}

/// 5.6 Shared Team Fog of War Composite Union
#[test]
fn test_team_fog_engine_composite_sightlines() {
    let map = HexMap::new(8);
    let mut state = GameState::new(map);

    let h1 = Unit::new_vanguard(1, 0, HexCoord::new(-6, -2), 3);
    let h2 = Unit::new_sniper(2, 0, HexCoord::new(-6, 2), 4);
    let tower = Unit::new_tower(101, 0, HexCoord::new(-5, 0));

    state.add_unit(h1);
    state.add_unit(h2);
    state.add_unit(tower);

    let team_vision = TeamFogEngine::compute_team_vision(&state, 0);

    // Assert that sightlines from all 3 units are represented in the composite team vision
    assert!(team_vision.contains(&HexCoord::new(-6, -2)));
    assert!(team_vision.contains(&HexCoord::new(-6, 2)));
    assert!(team_vision.contains(&HexCoord::new(-5, 0)));
    assert!(team_vision.len() > 40);
}

/// 5.7 10-Player Team Balance & Lobby Management
#[tokio::test]
async fn test_10_player_team_balance() {
    let mut actor = MatchActor::new_test_match();

    for i in 1..=10 {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        actor
            .handle_player_connect(format!("player_{}", i), None, None, tx)
            .await;
    }

    assert_eq!(actor.players.len(), 10);
    let team0_count = actor.players.values().filter(|p| p.team == 0).count();
    let team1_count = actor.players.values().filter(|p| p.team == 1).count();
    assert_eq!(team0_count, 5);
    assert_eq!(team1_count, 5);
}
