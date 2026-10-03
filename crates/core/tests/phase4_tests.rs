use hexabellum_core::ability::SpellTarget;
use hexabellum_core::hex::{HexCoord, HexMap};
use hexabellum_core::lane::LaneDef;
use hexabellum_core::neutral::NeutralCamp;
use hexabellum_core::orders::{Action, UnitOrder};
use hexabellum_core::session::{BattleConfig, BattleSession};
use hexabellum_core::turn::TurnProcessor;
use hexabellum_core::unit::{LaneDirection, Unit, UnitKind};
use hexabellum_core::vision::{compute_team_los_fog, has_line_of_sight};
use std::collections::{HashMap, HashSet};

/// 5.1 Test: Line of Sight Raycasting & Terrain Blockers
#[test]
fn test_line_of_sight_raycasting_and_vision_blockers() {
    let mut vision_blockers = HashSet::new();
    vision_blockers.insert(HexCoord::new(0, 2)); // Wall

    // 1. Direct clear sightline
    assert!(has_line_of_sight(&vision_blockers, HexCoord::new(-2, 2), HexCoord::new(-1, 2)));

    // 2. Sightline blocked by intermediate wall at (0, 2)
    assert!(!has_line_of_sight(&vision_blockers, HexCoord::new(-1, 2), HexCoord::new(1, 2)));

    // 3. Raycast ending directly on the blocker itself is permitted
    assert!(has_line_of_sight(&vision_blockers, HexCoord::new(-1, 2), HexCoord::new(0, 2)));

    // 4. Smoke pillar blocks vision
    vision_blockers.insert(HexCoord::new(2, 2));
    assert!(!has_line_of_sight(&vision_blockers, HexCoord::new(1, 2), HexCoord::new(3, 2)));
}

/// 5.2 Test: Ability Cooldown, Energy & Damage Application
#[test]
fn test_ability_cooldown_energy_and_validation() {
    let ranger = Unit::new_ranger(1, 0, HexCoord::new(0, 0), 10);
    let mut enemy = Unit::new_vanguard(2, 1, HexCoord::new(2, 0), 5);
    enemy.hp = 120;
    enemy.max_hp = 120;
    let mut units = vec![ranger, enemy];
    let vision_blockers = HashSet::new();

    // Cast Bolt (Cost: 1 AP, 2 Energy, CD: 2, DMG: 25)
    let mut orders = HashMap::new();
    orders.insert(
        1,
        UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Cast {
                spell_id: "bolt".to_string(),
                target: SpellTarget::Unit(2),
            },
        },
    );

    let _output = TurnProcessor::process_round(
        1,
        &HexMap::new(6),
        &vision_blockers,
        &mut units,
        &mut [],
        &LaneDef::central_lane(),
        orders,
    );

    let updated_ranger = units.iter().find(|u| u.id == 1).unwrap();
    let updated_enemy = units.iter().find(|u| u.id == 2).unwrap();

    assert_eq!(updated_ranger.energy, 4); // 5 - 2 + 1 regen
    assert_eq!(updated_ranger.cooldowns.get("bolt").copied(), Some(1)); // 2 - 1 round tick
    assert_eq!(updated_enemy.hp, 95); // 120 - 25
}

/// 5.3 Test: Structure Repair & AP Validation
#[test]
fn test_repair_mechanics_and_ap_consumption() {
    let hero = Unit::new_vanguard(1, 0, HexCoord::new(-2, 0), 10);
    let damaged_tower = Unit {
        id: 10,
        kind: UnitKind::Tower,
        team: 0,
        hero_id: None,
        pos: HexCoord::new(-3, 0),
        hp: 100,
        max_hp: 150,
        ap: 0,
        max_ap: 0,
        initiative: 0,
        attack_damage: 20,
        attack_range: 3,
        vision_range: 3,
        energy: 0,
        max_energy: 0,
        energy_regen: 0,
        cooldowns: HashMap::new(),
        statuses: Vec::new(),
        lane_id: None,
        waypoint_index: None,
        aggro_range: 0,
        last_attacker: None,
        spawn_interval: None,
        spawn_counter: 0,
        lane_direction: LaneDirection::None,
    };

    let mut units = vec![hero, damaged_tower];
    let mut orders = HashMap::new();
    orders.insert(
        1,
        UnitOrder {
            unit_id: 1,
            move_target: None,
            action: Action::Repair { target_id: 10 },
        },
    );

    TurnProcessor::process_round(
        1,
        &HexMap::new(6),
        &HashSet::new(),
        &mut units,
        &mut [],
        &LaneDef::central_lane(),
        orders,
    );

    let tower = units.iter().find(|u| u.id == 10).unwrap();
    let hero = units.iter().find(|u| u.id == 1).unwrap();

    assert_eq!(tower.hp, 120); // 100 + 20
    assert_eq!(hero.ap, 3); // 3 - 1 spent + round reset
}

/// 5.4 Test: Neutral Camp Aggro, Leash Reset & Team Buff
#[test]
fn test_neutral_camp_aggro_leash_and_team_buff() {
    let camp_pos = HexCoord::new(0, 3);
    let guardian = Unit::new_neutral_guardian(50, camp_pos);
    let hero = Unit::new_vanguard(1, 0, HexCoord::new(0, 5), 10); // Dist 2: triggers aggro

    let mut camp = NeutralCamp::new("camp_alpha".to_string(), camp_pos, 50);
    let mut units = vec![guardian, hero];

    // Verify leash reset when pulled beyond 3 hexes
    units[0].pos = HexCoord::new(0, 7); // Dist 4 > 3
    let (left, right) = units.split_at_mut(1);
    let reset = camp.check_leash_and_reset(&mut left[0], Some(&right[0]));
    assert!(reset);
    assert_eq!(units[0].pos, camp_pos);
    assert_eq!(units[0].hp, 80);

    // Verify kill reward
    units[0].hp = 0;
    units[0].last_attacker = Some(1);
    let mut events = Vec::new();
    camp.handle_guardian_death(0, &mut units, &mut events);

    let hero_buffed = units.iter().find(|u| u.id == 1).unwrap();
    assert_eq!(hero_buffed.effective_attack_damage(), 25); // 20 + 5 buff
}

/// 5.5 Test: Lane Waypoint Minion Progression
#[test]
fn test_lane_waypoint_minion_navigation() {
    let lane = LaneDef::central_lane();
    let mut minion = Unit {
        id: 100,
        kind: UnitKind::Minion,
        team: 0,
        hero_id: None,
        pos: HexCoord::new(-5, 0),
        hp: 40,
        max_hp: 40,
        ap: 1,
        max_ap: 1,
        initiative: 5,
        attack_damage: 8,
        attack_range: 1,
        vision_range: 2,
        energy: 0,
        max_energy: 0,
        energy_regen: 0,
        cooldowns: HashMap::new(),
        statuses: Vec::new(),
        lane_id: Some("mid".to_string()),
        waypoint_index: Some(0),
        aggro_range: 2,
        last_attacker: None,
        spawn_interval: None,
        spawn_counter: 0,
        lane_direction: LaneDirection::None,
    };

    // Minion at (-5, 0) is at waypoint 0. Should advance index to 1
    lane.update_minion_waypoint(&mut minion);
    assert_eq!(minion.waypoint_index, Some(1));
    assert_eq!(lane.waypoints[1], HexCoord::new(-3, 0));
}

/// 5.6 Test: LOS Fog of War Sanitization (No Leakage)
#[test]
fn test_los_fog_snapshot_sanitization_no_leak() {
    let mut vision_blockers = HashSet::new();
    vision_blockers.insert(HexCoord::new(0, 2)); // Dense wall

    let team_0_hero = Unit::new_vanguard(1, 0, HexCoord::new(-1, 2), 10);
    let hidden_enemy = Unit::new_vanguard(2, 1, HexCoord::new(1, 2), 10); // Hidden behind wall

    let units = vec![team_0_hero, hidden_enemy];
    let team_0_visible = compute_team_los_fog(&vision_blockers, 6, &units, 0);

    // Team 0 must not see (1, 2)
    assert!(!team_0_visible.contains(&HexCoord::new(1, 2)));
}

fn run_headless_simulation(rounds: u32, _seed: u64) -> String {
    let mut session = BattleSession::new("headless_sim".into(), BattleConfig::default());
    for _ in 0..rounds {
        if session.state.winner.is_some() {
            break;
        }
        session.resolve_round();
    }
    session.state_hash()
}

/// 5.7 Test: Headless 50-Round AI vs AI Determinism with Abilities
#[test]
fn test_headless_ai_vs_ai_50_rounds_determinism_with_abilities() {
    // Run two identical 50-round simulations from identical seed
    let hash_a = run_headless_simulation(50, 42);
    let hash_b = run_headless_simulation(50, 42);

    assert_eq!(
        hash_a, hash_b,
        "Simulations diverged! State hashing must be 100% deterministic."
    );
}
