use hexabellum_core::hex::HexCoord;
use hexabellum_core::session::{BattleConfig, BattleSession};

#[test]
fn test_core_destruction_triggers_victory() {
    let mut session = BattleSession::new_test_session_5v5();
    let team1_core_id = session.get_core_id(1).unwrap();

    // Inflict lethal damage to Team 1 Core
    session.inflict_damage(team1_core_id, 700, 101); // 101 = Team 0 Vanguard

    let victory_event = session.check_core_victory();
    assert!(victory_event.is_some());
    assert_eq!(session.is_match_over, true);
    assert_eq!(session.winning_team, Some(0));
}

#[test]
fn test_hero_death_enters_respawn_state_instead_of_removal() {
    let mut session = BattleSession::new_test_session_5v5();
    let hero_id = 101; // Team 0 Hero

    session.inflict_damage(hero_id, 200, 201); // Inflict lethal damage

    let hero = session.units.get(&hero_id).unwrap();
    assert_eq!(hero.is_alive(), false);
    assert!(hero.is_dead_awaiting_respawn());
    assert_eq!(hero.respawn_rounds, Some(3));
    assert_eq!(session.is_hex_occupied(hero.pos), false); // Cleared board occupancy!
}

#[test]
fn test_hero_respawn_countdown_and_placement_at_base() {
    let mut session = BattleSession::new_test_session_5v5();
    let hero_id = 101;
    session.kill_hero_for_test(hero_id);

    // Round 1 Start: Timer drops 3 -> 2
    session.process_round_start_respawns();
    assert_eq!(session.units.get(&hero_id).unwrap().respawn_rounds, Some(2));

    // Round 2 Start: Timer drops 2 -> 1
    session.process_round_start_respawns();
    assert_eq!(session.units.get(&hero_id).unwrap().respawn_rounds, Some(1));

    // Round 3 Start: Timer drops 1 -> 0 -> Respawns!
    let _events = session.process_round_start_respawns();
    let hero = session.units.get(&hero_id).unwrap();
    assert_eq!(hero.is_alive(), true);
    assert_eq!(hero.respawn_rounds, None);
    assert_eq!(hero.hp, hero.max_hp);
    assert_eq!(hero.ap, hero.max_ap);
    assert_eq!(hero.energy, hero.max_energy);

    // Verify placement is within Team 0 base zone
    let base_zone = session.get_base_zone(0).unwrap();
    assert!(base_zone.contains(hero.pos));
}

#[test]
fn test_hero_respawn_preserves_gold_xp_level_items() {
    let mut session = BattleSession::new_test_session_5v5();
    let hero_id = 101;

    // Give hero progression
    {
        let hero = session.units.get_mut(&hero_id).unwrap();
        hero.gold = 350;
        hero.xp = 180;
        hero.level = 3;
        hero.items = vec!["longblade".into(), "plate_armor".into()];
    }

    session.kill_hero_for_test(hero_id);
    session.fast_forward_respawn(hero_id);

    let hero = session.units.get(&hero_id).unwrap();
    assert_eq!(hero.gold, 350);
    assert_eq!(hero.xp, 180);
    assert_eq!(hero.level, 3);
    assert_eq!(hero.items.len(), 2);
}

#[test]
fn test_base_shop_enforces_base_zone_and_alive_state() {
    let mut session = BattleSession::new_test_session_5v5();
    let hero_id = 101; // In Team 0 base

    assert_eq!(session.can_hero_shop(hero_id), true);

    // Move hero outside base zone
    session.set_unit_pos(hero_id, HexCoord::new(0, 0));
    assert_eq!(session.can_hero_shop(hero_id), false);

    // Move back to base, then kill hero
    session.set_unit_pos(hero_id, HexCoord::new(-6, 0));
    session.kill_hero_for_test(hero_id);
    assert_eq!(session.can_hero_shop(hero_id), false);
}

#[test]
fn test_base_regeneration_applies_only_to_alive_heroes_in_base() {
    let mut session = BattleSession::new_test_session_5v5();
    let hero_in_base = 101;
    let hero_in_field = 102;

    session.set_unit_pos(hero_in_base, HexCoord::new(-6, 0)); // Inside base
    session.set_unit_pos(hero_in_field, HexCoord::new(0, 0)); // Outside base

    session.set_unit_hp(hero_in_base, 50);
    session.set_unit_hp(hero_in_field, 50);

    session.process_base_regeneration();

    assert_eq!(session.units.get(&hero_in_base).unwrap().hp, 65); // 50 + 15
    assert_eq!(session.units.get(&hero_in_field).unwrap().hp, 50); // Unchanged!
}

#[test]
fn test_objective_vault_destruction_awards_team_bounty_and_buff() {
    let mut session = BattleSession::new_test_session_5v5();
    let _vault_id = session.get_vault_id().unwrap();
    let attacker_id = 101; // Team 0 hero

    let initial_gold = session.units.get(&attacker_id).unwrap().gold;
    session.destroy_vault(attacker_id);

    // All alive Team 0 heroes gain +50G, +40XP
    let hero = session.units.get(&attacker_id).unwrap();
    assert_eq!(hero.gold, initial_gold + 50);
    assert!(session.has_active_buff(attacker_id, "attack_damage_buff"));
}

#[test]
fn test_minions_route_to_enemy_core_and_ignore_vault() {
    let mut session = BattleSession::new_test_session_5v5();
    let minion_id = session.spawn_test_minion(0, HexCoord::new(0, 1));
    let targets = session.get_valid_attack_targets(minion_id);

    let vault_id = session.get_vault_id().unwrap();
    assert!(!targets.contains(&vault_id)); // Vault is strictly ignored by minions!
}

#[test]
fn test_core_cannot_be_repaired() {
    let session = BattleSession::new_test_session_5v5();
    let core0_id = session.get_core_id(0).unwrap();
    let core = session.units.get(&core0_id).unwrap();
    assert!(!core.kind.is_repairable());

    let _vault_id = session.get_vault_id().unwrap();
    let vault = session.units.get(&_vault_id).unwrap();
    assert!(!vault.kind.is_repairable());
}

#[test]
fn test_100_round_macro_soak_determinism() {
    let run_sim = || {
        let mut session = BattleSession::new("soak".into(), BattleConfig::default());
        let mut hashes = Vec::new();
        for _ in 1..=100 {
            if session.is_match_over || session.state.winner.is_some() {
                break;
            }
            session.resolve_ai_round();
            hashes.push(session.state_hash());
        }
        hashes
    };

    let run1 = run_sim();
    let run2 = run_sim();
    assert!(!run1.is_empty());
    assert_eq!(run1, run2, "Macro simulation must be 100% bitwise BLAKE3 deterministic across runs!");
}
