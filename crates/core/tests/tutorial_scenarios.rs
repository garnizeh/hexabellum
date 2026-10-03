use hexabellum_core::hex::HexCoord;
use hexabellum_core::orders::{Action, UnitOrder};
use hexabellum_core::tutorial::TutorialScenarioRunner;

#[test]
fn test_tutorial_lesson_01_movement_golden_path() {
    let mut session = TutorialScenarioRunner::load("lesson_01_movement");
    assert_eq!(session.current_step(), "l1_s1_intro");

    // Dismiss dialogue
    let advanced = session.advance_dialogue().expect("Dialogue should advance");
    assert!(advanced);
    assert_eq!(session.current_step(), "l1_s2_move_one");

    // Execute valid 1 AP move to (1, 0)
    let order = UnitOrder {
        unit_id: 1,
        move_target: Some(HexCoord::new(1, 0)),
        action: Action::Wait,
    };
    let result = session.submit_order(order).expect("Order should succeed");
    assert!(result.completed_step);
    assert_eq!(session.current_step(), "l1_s3_collision_test");
}

#[test]
fn test_tutorial_lesson_03_initiative_death_before_acting() {
    let mut session = TutorialScenarioRunner::load("lesson_03_initiative");
    session
        .skip_to_step("l3_s2_order_attack")
        .expect("Skip to attack step");

    let order = UnitOrder {
        unit_id: 1,
        move_target: None,
        action: Action::Attack { target_id: 101 },
    };
    session.submit_order(order).expect("Attack order valid");

    // Resolve simultaneous round
    let _events = session.resolve_round().expect("Round resolves");

    // Assert dummy died at initiative 4, and dummy's attack at initiative 2 was cancelled
    assert!(session.unit(101).is_none() || !session.unit(101).unwrap().is_alive(), "Dummy should be dead");
    assert_eq!(
        session.unit(1).unwrap().hp,
        90,
        "Hero should take 0 damage due to death-before-acting"
    );
    assert_eq!(session.current_step(), "l3_s3_debrief");
}

#[test]
fn test_tutorial_lesson_01_soft_fail_occupied_hex() {
    let mut session = TutorialScenarioRunner::load("lesson_01_movement");
    session
        .skip_to_step("l1_s4_attempt_collision")
        .expect("Skip to collision step");

    // Try to move into dummy at (0, 2)
    let order = UnitOrder {
        unit_id: 1,
        move_target: Some(HexCoord::new(0, 2)),
        action: Action::Wait,
    };
    let err = session.submit_order(order).unwrap_err();
    assert_eq!(err.code, "ERR_OCCUPIED_HEX");
    assert!(
        err.archmage_response.contains("no shared space") || err.archmage_response.contains("crowding"),
        "Archmage should provide pedagogical feedback"
    );
    assert_eq!(
        session.unit(1).unwrap().pos,
        HexCoord::new(0, 0),
        "Hero must not move on error"
    );
}

#[test]
fn test_tutorial_lesson_00_dialogue_progression() {
    let mut session = TutorialScenarioRunner::load("lesson_00_intro");
    assert_eq!(session.current_step(), "l0_s1_welcome");
    assert!(session.advance_dialogue().unwrap());
    assert_eq!(session.current_step(), "l0_s2_hud_callout");
    assert!(session.advance_dialogue().unwrap());
    assert!(session.is_completed());
}

#[test]
fn test_tutorial_lesson_02_attack_resolution() {
    let mut session = TutorialScenarioRunner::load("lesson_02_attack");
    session.advance_dialogue().unwrap();
    assert_eq!(session.current_step(), "l2_s2_attack_dummy");

    let order = UnitOrder {
        unit_id: 1,
        move_target: None,
        action: Action::Attack { target_id: 100 },
    };
    session.submit_order(order).expect("Attack order valid");

    session.resolve_round().expect("Round resolves");
    // Dummy had 50 HP, hero deals 16 damage -> 34 HP remaining
    assert_eq!(session.unit(100).unwrap().hp, 34);
    assert!(session.is_completed());
}
