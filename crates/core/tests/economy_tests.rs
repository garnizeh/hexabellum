use hexabellum_core::hex::HexCoord;
use hexabellum_core::items::get_item_def;
use hexabellum_core::progression::{grant_xp, MAX_LEVEL};
use hexabellum_core::session::{BattleConfig, BattleSession};
use hexabellum_core::shop::execute_purchase;
use hexabellum_core::unit::Unit;
use hexabellum_protocol::ProtocolErrorCode;

fn create_test_session() -> BattleSession {
    let mut config = BattleConfig::default();
    config.map_radius = 8;
    config.heroes_per_team = 5;
    config.players_per_team = 5;
    BattleSession::new("test_economy".into(), config)
}

#[test]
fn test_passive_income_granted_each_round() {
    let mut session = create_test_session();
    let initial_gold = session.get_hero(1).gold;
    assert_eq!(initial_gold, 50);

    session.distribute_passive_income();
    assert_eq!(session.get_hero(1).gold, 56);
}

#[test]
fn test_last_attacker_kill_reward_attribution() {
    let mut session = create_test_session();
    let attacker_id = 1; // Team 0 Hero
    let victim_id = 6;   // Team 1 Hero

    // Attacker damages victim to 0
    session.apply_damage(victim_id, 200, attacker_id);
    assert_eq!(session.get_hero(victim_id).last_attacker, Some(attacker_id));

    session.resolve_fatalities();
    assert_eq!(session.get_hero(attacker_id).gold, 50 + 30);
    assert_eq!(session.get_hero(attacker_id).xp, 30);
}

#[test]
fn test_structure_destruction_team_reward() {
    let mut session = create_test_session(); // Team 0 vs Team 1
    let enemy_tower_id = 201; // Team 1 Tower

    session.destroy_structure(enemy_tower_id);

    // All living Team 0 heroes should receive +25G, +20XP
    for hero in session.get_living_heroes_for_team(0) {
        assert_eq!(hero.gold, 50 + 25);
        assert_eq!(hero.xp, 20);
    }
}

#[test]
fn test_xp_threshold_and_multi_level_leap() {
    let mut hero = Unit::new_hero(1, 0, HexCoord::new(0, 0), 3);
    assert_eq!(hero.level, 1);
    assert_eq!(hero.max_hp, 100);
    assert_eq!(hero.attack_damage, 20);

    let mut events = Vec::new();
    // Award 150 XP (Surpasses Level 2 at 50 XP, and Level 3 at 120 XP)
    grant_xp(&mut hero, 150, &mut events);

    assert_eq!(hero.level, 3);
    assert_eq!(hero.max_hp, 100 + 24);
    assert_eq!(hero.attack_damage, 20 + 6);
    assert_eq!(events.len(), 2); // 2 distinct LevelUp events emitted
}

#[test]
fn test_buy_item_success_and_stat_mutation() {
    let mut hero = Unit::new_hero(1, 0, HexCoord::new(0, 0), 3);
    hero.gold = 100;
    let longblade = get_item_def("longblade").expect("longblade exists");

    let result = execute_purchase(&mut hero, &longblade, 3, false);
    assert!(result.is_ok());
    assert_eq!(hero.gold, 0);
    assert_eq!(hero.items, vec!["longblade".to_string()]);
    assert_eq!(hero.attack_damage, 20 + 6);
}

#[test]
fn test_buy_item_rejects_insufficient_funds() {
    let mut hero = Unit::new_hero(1, 0, HexCoord::new(0, 0), 3);
    hero.gold = 50;
    let longblade = get_item_def("longblade").expect("longblade exists"); // Costs 100

    let result = execute_purchase(&mut hero, &longblade, 3, false);
    assert_eq!(result, Err(ProtocolErrorCode::InsufficientGold));
}

#[test]
fn test_buy_item_rejects_duplicate() {
    let mut hero = Unit::new_hero(1, 0, HexCoord::new(0, 0), 3);
    hero.gold = 300;
    let longblade = get_item_def("longblade").expect("longblade exists");

    execute_purchase(&mut hero, &longblade, 3, false).unwrap();
    let result = execute_purchase(&mut hero, &longblade, 3, false);
    assert_eq!(result, Err(ProtocolErrorCode::ItemAlreadyOwned));
}

#[test]
fn test_buy_item_rejects_full_inventory() {
    let mut hero = Unit::new_hero(1, 0, HexCoord::new(0, 0), 3);
    hero.gold = 1000;
    hero.items = vec!["item1".into(), "item2".into(), "item3".into()];
    let longblade = get_item_def("longblade").expect("longblade exists");

    let result = execute_purchase(&mut hero, &longblade, 3, false);
    assert_eq!(result, Err(ProtocolErrorCode::InventoryFull));
}

#[test]
fn test_plate_armor_instant_heal() {
    let mut hero = Unit::new_hero(1, 0, HexCoord::new(0, 0), 3);
    hero.hp = 50;
    hero.max_hp = 100;
    hero.gold = 200;
    let plate = get_item_def("plate_armor").expect("plate_armor exists");

    let result = execute_purchase(&mut hero, &plate, 3, false);
    assert!(result.is_ok());
    assert_eq!(hero.max_hp, 135);
    assert_eq!(hero.hp, 85);
}

#[test]
fn test_level_cap_enforced() {
    let mut hero = Unit::new_hero(1, 0, HexCoord::new(0, 0), 3);
    let mut events = Vec::new();

    // Award huge XP: 1000 XP
    grant_xp(&mut hero, 1000, &mut events);

    assert_eq!(hero.level, MAX_LEVEL);
    assert_eq!(events.len(), 4); // Lvl 2, 3, 4, 5

    // Further XP does not exceed max level or add stats
    let hp_at_5 = hero.max_hp;
    grant_xp(&mut hero, 500, &mut events);
    assert_eq!(hero.level, MAX_LEVEL);
    assert_eq!(hero.max_hp, hp_at_5);
}

#[test]
fn test_bot_shopping_priority() {
    let mut session = create_test_session();
    // Hero 6 is a bot on Team 1 with starting gold 50.
    // Give hero 6 220 gold: enough for plate_armor (120) and longblade (100)
    session.get_unit_mut(6).unwrap().gold = 220;

    session.execute_ai_bot_shopping();

    let hero = session.get_hero(6);
    assert_eq!(hero.items, vec!["plate_armor".to_string(), "longblade".to_string()]);
    assert_eq!(hero.gold, 0);
}

#[test]
fn test_100_round_economy_soak_determinism() {
    let mut config = BattleConfig::default();
    config.map_radius = 8;
    config.players_per_team = 5;
    config.heroes_per_team = 5;
    config.fill_empty_slots_with_ai = true;

    let run_simulation = || {
        let mut session = BattleSession::new("sim_economy".into(), config.clone());
        let mut state_hashes = Vec::new();

        for _round in 1..=100 {
            if session.state.winner.is_some() {
                break;
            }
            session.resolve_ai_round();
            state_hashes.push(session.state_hash());
        }
        state_hashes
    };

    let run_1 = run_simulation();
    let run_2 = run_simulation();

    assert_eq!(
        run_1, run_2,
        "Non-deterministic simulation detected in 100-round economy soak test!"
    );
    assert!(run_1.len() > 10, "Simulation aborted prematurely!");
}

