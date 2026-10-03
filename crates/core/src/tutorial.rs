use crate::event::GameEvent;
use crate::hex::{HexCoord, HexMap};
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::GameState;
use crate::turn::TurnProcessor;
use crate::unit::{LaneDirection, Unit, UnitId, UnitKind};
use hexabellum_protocol::tutorial::*;
use hexabellum_protocol::HexDto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TutorialError {
    pub code: String,
    pub message: String,
    pub archmage_response: String,
}

impl std::fmt::Display for TutorialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]: {}", self.code, self.message)
    }
}

impl std::error::Error for TutorialError {}

/// Headless simulation runner for Tutorial Scenarios.
pub struct TutorialScenarioRunner {
    scenario: TutorialScenario,
    current_step_index: usize,
    pub state: GameState,
    pub pending_orders: TurnOrders,
    step_initial_state: Option<GameState>,
}

impl TutorialScenarioRunner {
    /// Load a built-in scenario by ID or parse arbitrary JSON.
    pub fn load(id_or_json: &str) -> Self {
        let scenario = match id_or_json {
            "lesson_01_movement" => Self::preset_lesson_01(),
            "lesson_02_attack" => Self::preset_lesson_02(),
            "lesson_03_initiative" => Self::preset_lesson_03(),
            "lesson_00_intro" => Self::preset_lesson_00(),
            "lesson_07_fog_of_war" => Self::preset_lesson_07(),
            other => serde_json::from_str::<TutorialScenario>(other)
                .unwrap_or_else(|_| Self::preset_lesson_01()),
        };
        Self::from_scenario(scenario)
    }

    /// Construct runner from a declarative `TutorialScenario`.
    pub fn from_scenario(scenario: TutorialScenario) -> Self {
        let map = HexMap::new(scenario.map_radius);
        let mut state = GameState::new(map);

        // Add player champion
        let mut player_hero = Unit::new_hero(
            1,
            scenario.initial_state.player_team,
            HexCoord::new(
                scenario.initial_state.player_hero_pos.q,
                scenario.initial_state.player_hero_pos.r,
            ),
            4,
        );
        player_hero.hp = 90;
        player_hero.max_hp = 90;
        player_hero.ap = 3;
        player_hero.max_ap = 3;
        player_hero.attack_damage = 16;
        player_hero.attack_range = 2;
        player_hero.vision_range = 4;
        state.add_unit(player_hero);

        // Add preset units
        for preset in &scenario.initial_state.board_units {
            let kind = match preset.kind.as_str() {
                "Minion" => UnitKind::Minion,
                "Tower" => UnitKind::Tower,
                "Spawner" => UnitKind::Spawner,
                "Hero" => UnitKind::Hero,
                "Dummy" if preset.team == 1 => UnitKind::Hero,
                _ => UnitKind::NeutralGuardian,
            };
            let mut unit = Unit {
                id: preset.id,
                kind,
                team: preset.team,
                pos: HexCoord::new(preset.pos.q, preset.pos.r),
                hp: preset.hp,
                max_hp: preset.max_hp,
                ap: if kind.is_stationary() { 0 } else { 2 },
                max_ap: if kind.is_stationary() { 0 } else { 2 },
                initiative: preset.initiative,
                attack_damage: preset.attack_damage,
                attack_range: 1,
                vision_range: 3,
                energy: 0,
                max_energy: 0,
                energy_regen: 0,
                cooldowns: std::collections::HashMap::new(),
                statuses: Vec::new(),
                lane_id: None,
                waypoint_index: None,
                aggro_range: 2,
                last_attacker: None,
                spawn_interval: None,
                spawn_counter: 0,
                lane_direction: LaneDirection::None,
            };
            if preset.is_invulnerable {
                unit.hp = preset.hp.max(100);
                unit.max_hp = preset.max_hp.max(100);
            }
            state.add_unit(unit);
        }

        if scenario.initial_state.fog_enabled {
            state.update_fog();
        } else {
            let all = state.map.all_hexes();
            for team_set in &mut state.fog.visible {
                team_set.extend(all.iter().cloned());
            }
        }

        let step_snapshot = state.clone();

        Self {
            scenario,
            current_step_index: 0,
            state,
            pending_orders: TurnOrders::new(),
            step_initial_state: Some(step_snapshot),
        }
    }

    pub fn scenario(&self) -> &TutorialScenario {
        &self.scenario
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut GameState {
        &mut self.state
    }

    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.state.get_unit(id)
    }

    pub fn current_step(&self) -> &str {
        self.scenario
            .steps
            .get(self.current_step_index)
            .map(|s| s.step_id.as_str())
            .unwrap_or("")
    }

    pub fn current_step_def(&self) -> &TutorialStep {
        &self.scenario.steps[self.current_step_index]
    }

    pub fn is_completed(&self) -> bool {
        self.current_step_index >= self.scenario.steps.len()
    }

    pub fn skip_to_step(&mut self, step_id: &str) -> Result<(), TutorialError> {
        if let Some(idx) = self.scenario.steps.iter().position(|s| s.step_id == step_id) {
            self.current_step_index = idx;
            self.step_initial_state = Some(self.state.clone());
            Ok(())
        } else {
            Err(TutorialError {
                code: "ERR_STEP_NOT_FOUND".into(),
                message: format!("Step '{}' not found in scenario", step_id),
                archmage_response: "The path diverges into the unknown.".into(),
            })
        }
    }

    pub fn advance_dialogue(&mut self) -> Result<bool, TutorialError> {
        if self.is_completed() {
            return Ok(false);
        }
        let step = &self.scenario.steps[self.current_step_index];
        if matches!(step.completion_condition, StepCondition::DialogueDismissed) {
            self.current_step_index += 1;
            self.step_initial_state = Some(self.state.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn reset_current_step(&mut self) {
        if let Some(snap) = &self.step_initial_state {
            self.state = snap.clone();
            self.pending_orders.orders.clear();
        }
    }

    /// Submit a unit order, checking input gates, core invariants, and step conditions.
    pub fn submit_order(&mut self, order: UnitOrder) -> Result<TutorialOrderResult, TutorialError> {
        if self.is_completed() {
            return Err(TutorialError {
                code: "ERR_SCENARIO_COMPLETED".into(),
                message: "Lesson is already complete".into(),
                archmage_response: "The trial is concluded.".into(),
            });
        }

        let step = &self.scenario.steps[self.current_step_index];

        // 1. Input Gate Whitelist Validation
        if let Some(gate) = &step.input_gate {
            if !gate.allowed_unit_ids.is_empty() && !gate.allowed_unit_ids.contains(&order.unit_id) {
                return Err(self.make_error(
                    "ERR_INVALID_UNIT",
                    "Unit is not allowed for this objective",
                    step,
                ));
            }

            match &order.action {
                Action::Attack { .. } => {
                    if !gate.allowed_actions.iter().any(|a| a.eq_ignore_ascii_case("attack")) {
                        return Err(self.make_error(
                            "ERR_ACTION_NOT_ALLOWED",
                            "Attack action is not permitted in this step",
                            step,
                        ));
                    }
                }
                Action::Wait => {
                    if order.move_target.is_none()
                        && !gate.allowed_actions.iter().any(|a| a.eq_ignore_ascii_case("wait") || a.eq_ignore_ascii_case("lock"))
                    {
                        return Err(self.make_error(
                            "ERR_ACTION_NOT_ALLOWED",
                            "Waiting is not permitted in this step",
                            step,
                        ));
                    }
                }
                Action::Cast { .. } => {
                    if !gate.allowed_actions.iter().any(|a| a.eq_ignore_ascii_case("cast")) {
                        return Err(self.make_error(
                            "ERR_ACTION_NOT_ALLOWED",
                            "Casting is not permitted in this step",
                            step,
                        ));
                    }
                }
                Action::Repair { .. } => {
                    if !gate.allowed_actions.iter().any(|a| a.eq_ignore_ascii_case("repair")) {
                        return Err(self.make_error(
                            "ERR_ACTION_NOT_ALLOWED",
                            "Repairing is not permitted in this step",
                            step,
                        ));
                    }
                }
            }

            if let Some(target) = order.move_target {
                if !gate.allowed_actions.iter().any(|a| a.eq_ignore_ascii_case("move")) {
                    return Err(self.make_error(
                        "ERR_ACTION_NOT_ALLOWED",
                        "Movement is not permitted in this step",
                        step,
                    ));
                }
                if !gate.allowed_hex_targets.is_empty() {
                    let matches_target = gate
                        .allowed_hex_targets
                        .iter()
                        .any(|h| h.q == target.q && h.r == target.r);
                    if !matches_target {
                        return Err(self.make_error(
                            "ERR_INVALID_TARGET",
                            "Target hex is outside the current objective area",
                            step,
                        ));
                    }
                }
            }
        }

        // 2. Engine Invariants Validation
        let unit = match self.state.get_unit(order.unit_id) {
            Some(u) if u.is_alive() => u,
            _ => {
                return Err(self.make_error(
                    "ERR_UNIT_DEAD",
                    "Unit does not exist or is fallen",
                    step,
                ));
            }
        };

        if let Some(target_hex) = order.move_target {
            // Collision with another unit
            if let Some(occ) = self.state.units.values().find(|u| u.is_alive() && u.id != order.unit_id && u.pos == target_hex) {
                return Err(self.make_error(
                    "ERR_OCCUPIED_HEX",
                    &format!("Hex ({}, {}) is occupied by unit {}", target_hex.q, target_hex.r, occ.id),
                    step,
                ));
            }

            // Walkable & AP budget
            let dist = unit.pos.distance(&target_hex);
            if dist > unit.ap {
                return Err(self.make_error(
                    "ERR_INSUFFICIENT_AP",
                    &format!("Movement distance {} exceeds unit AP {}", dist, unit.ap),
                    step,
                ));
            }
            if !self.state.map.is_walkable(&target_hex) {
                return Err(self.make_error(
                    "ERR_OBSTACLE_BLOCKED",
                    "Hex contains an impenetrable obstacle",
                    step,
                ));
            }
        }

        if let Action::Attack { target_id } = order.action {
            let target_unit = match self.state.get_unit(target_id) {
                Some(t) if t.is_alive() => t,
                _ => {
                    return Err(self.make_error(
                        "ERR_INVALID_TARGET",
                        "Target unit does not exist or is dead",
                        step,
                    ));
                }
            };

            let origin = order.move_target.unwrap_or(unit.pos);
            let dist = origin.distance(&target_unit.pos);
            if dist > unit.attack_range {
                return Err(self.make_error(
                    "ERR_OUT_OF_RANGE",
                    &format!("Distance {} exceeds attack range {}", dist, unit.attack_range),
                    step,
                ));
            }
        }

        // 3. Queue Order & Evaluate Step Condition
        let target_move = order.move_target;
        self.pending_orders.add_order(order.clone());

        // Apply immediate movement for Interactive Movement Objectives if no combat resolution is required
        let mut step_completed = false;
        if let StepCondition::UnitMovedTo { unit_id, target } = &step.completion_condition {
            if *unit_id == order.unit_id && target_move == Some(HexCoord::new(target.q, target.r)) {
                if let Some(u) = self.state.get_unit_mut(order.unit_id) {
                    u.pos = HexCoord::new(target.q, target.r);
                    u.spend_ap(target_move.map(|t| u.pos.distance(&t)).unwrap_or(0));
                }
                step_completed = true;
            }
        }

        if step_completed {
            self.current_step_index += 1;
            self.step_initial_state = Some(self.state.clone());
            self.pending_orders.orders.clear();
        }

        Ok(TutorialOrderResult {
            valid: true,
            error_code: None,
            completed_step: step_completed,
            archmage_feedback: None,
        })
    }

    /// Resolve simultaneous turn deterministically and evaluate post-resolution invariants.
    pub fn resolve_round(&mut self) -> Result<Vec<GameEvent>, TutorialError> {
        let orders = std::mem::replace(&mut self.pending_orders, TurnOrders::new());
        let events = TurnProcessor::resolve(&mut self.state, &orders);
        if self.state.round == 0 {
            self.state.round = 1;
        }

        if !self.is_completed() {
            let step = &self.scenario.steps[self.current_step_index];
            let mut completed = false;

            match &step.completion_condition {
                StepCondition::RoundResolved { target_round } => {
                    if self.state.round >= *target_round {
                        completed = true;
                    }
                }
                StepCondition::UnitKilled { unit_id } => {
                    if self.state.get_unit(*unit_id).map(|u| !u.is_alive()).unwrap_or(true) {
                        completed = true;
                    }
                }
                StepCondition::UnitDamaged { unit_id, min_damage } => {
                    if let Some(u) = self.state.get_unit(*unit_id) {
                        if u.max_hp.saturating_sub(u.hp) >= *min_damage {
                            completed = true;
                        }
                    }
                }
                _ => {}
            }

            if completed {
                self.current_step_index += 1;
                self.step_initial_state = Some(self.state.clone());
            }
        }

        Ok(events)
    }

    fn make_error(&self, code: &str, default_msg: &str, step: &TutorialStep) -> TutorialError {
        let feedback = step
            .failure_feedback
            .iter()
            .find(|f| f.error_code == code)
            .map(|f| f.archmage_response.clone())
            .unwrap_or_else(|| match code {
                "ERR_OCCUPIED_HEX" => "The grid admits no crowding. Two units cannot occupy one stone; chart your course around.".into(),
                "ERR_INSUFFICIENT_AP" => "Your hero's breath is spent for this round. You have 3 AP each turn — pace your advance.".into(),
                "ERR_OUT_OF_RANGE" => "Your bowstring cannot reach across such distances. Advance closer before drawing.".into(),
                "ERR_COOLDOWN_ACTIVE" => "The runes are still cooling. Inscribe your patience; the spell awakens in turns.".into(),
                "ERR_INSUFFICIENT_ENERGY" => "Your spirit reservoir is dry. Conserve your energy; it restores +1 each round.".into(),
                "ERR_TARGET_IN_FOG" => "You cannot strike what the mist conceals. Advance to reveal your quarry before attacking.".into(),
                _ => "Observe the runes and hone your intent.".into(),
            });

        TutorialError {
            code: code.to_string(),
            message: default_msg.to_string(),
            archmage_response: feedback,
        }
    }

    // --- Built-in Scenario Presets ---

    fn preset_lesson_00() -> TutorialScenario {
        TutorialScenario {
            id: "lesson_00_intro".into(),
            lesson_index: 0,
            title: "The Convergence Calls".into(),
            gdd_reference: "gdd.md §1, §13.1".into(),
            map_radius: 4,
            initial_state: TutorialInitialState {
                player_team: 0,
                player_hero_kind: "Ranger".into(),
                player_hero_pos: HexDto::new(0, 0),
                player_gold: 0,
                player_xp: 0,
                player_energy: 5,
                board_units: vec![],
                fog_enabled: false,
            },
            steps: vec![
                TutorialStep {
                    step_id: "l0_s1_welcome".into(),
                    step_type: TutorialStepType::Dialogue,
                    dialogue: Some(ArchmageDialogue {
                        lines: vec![
                            "Welcome, traveler. You stand upon the Proving Grounds of the Convergence.".into(),
                            "Six sides to every stone, infinite paths to victory.".into(),
                        ],
                        emotion: "Enlightened".into(),
                        auto_advance_ms: None,
                    }),
                    callout: None,
                    input_gate: None,
                    objective: None,
                    camera_target: Some(HexDto::new(0, 0)),
                    completion_condition: StepCondition::DialogueDismissed,
                    failure_feedback: vec![],
                },
                TutorialStep {
                    step_id: "l0_s2_hud_callout".into(),
                    step_type: TutorialStepType::Callout,
                    dialogue: Some(ArchmageDialogue {
                        lines: vec![
                            "Observe your HUD: top-left holds your mission checklist; top-center marks the turn timer.".into(),
                            "Select your hero to inspect your Action Points and Initiative.".into(),
                        ],
                        emotion: "Neutral".into(),
                        auto_advance_ms: None,
                    }),
                    callout: Some(CalloutConfig {
                        target_selector: "#hud".into(),
                        title: "Tactical HUD".into(),
                        description: "Monitor round countdown, status, and turn progression.".into(),
                        placement: "bottom".into(),
                    }),
                    input_gate: None,
                    objective: Some(ObjectiveConfig {
                        text: "Acknowledge the tactical HUD briefing".into(),
                        spotlight_hexes: vec![HexDto::new(0, 0)],
                        ring_color: "#00d2ff".into(),
                    }),
                    camera_target: Some(HexDto::new(0, 0)),
                    completion_condition: StepCondition::DialogueDismissed,
                    failure_feedback: vec![],
                },
            ],
        }
    }

    fn preset_lesson_01() -> TutorialScenario {
        TutorialScenario {
            id: "lesson_01_movement".into(),
            lesson_index: 1,
            title: "The Three Steps".into(),
            gdd_reference: "gdd.md §6.2".into(),
            map_radius: 4,
            initial_state: TutorialInitialState {
                player_team: 0,
                player_hero_kind: "Ranger".into(),
                player_hero_pos: HexDto::new(0, 0),
                player_gold: 0,
                player_xp: 0,
                player_energy: 5,
                board_units: vec![TutorialUnitPreset {
                    id: 99,
                    kind: "Dummy".into(),
                    team: 2,
                    pos: HexDto::new(0, 2),
                    hp: 100,
                    max_hp: 100,
                    initiative: 0,
                    attack_damage: 0,
                    is_invulnerable: true,
                }],
                fog_enabled: false,
            },
            steps: vec![
                TutorialStep {
                    step_id: "l1_s1_intro".into(),
                    step_type: TutorialStepType::Dialogue,
                    dialogue: Some(ArchmageDialogue {
                        lines: vec![
                            "Every march begins with a single step across the grid.".into(),
                            "Your hero possesses 3 Action Points each round. Each hex traversed consumes exactly 1 AP.".into(),
                            "Take your first step. March 1 hex East.".into(),
                        ],
                        emotion: "Neutral".into(),
                        auto_advance_ms: None,
                    }),
                    callout: None,
                    input_gate: None,
                    objective: None,
                    camera_target: Some(HexDto::new(0, 0)),
                    completion_condition: StepCondition::DialogueDismissed,
                    failure_feedback: vec![],
                },
                TutorialStep {
                    step_id: "l1_s2_move_one".into(),
                    step_type: TutorialStepType::Objective,
                    dialogue: None,
                    callout: None,
                    input_gate: Some(InputGateConfig {
                        allowed_unit_ids: vec![1],
                        allowed_actions: vec!["Move".into()],
                        allowed_hex_targets: vec![HexDto::new(1, 0)],
                        max_ap_spend: Some(1),
                    }),
                    objective: Some(ObjectiveConfig {
                        text: "Move 1 hex East to coordinate (1, 0)".into(),
                        spotlight_hexes: vec![HexDto::new(1, 0)],
                        ring_color: "#00e676".into(),
                    }),
                    camera_target: Some(HexDto::new(1, 0)),
                    completion_condition: StepCondition::UnitMovedTo {
                        unit_id: 1,
                        target: HexDto::new(1, 0),
                    },
                    failure_feedback: vec![SoftFailRule {
                        error_code: "ERR_INVALID_TARGET".into(),
                        archmage_response: "Look for the emerald ring to the East. Click that hex to plot your march.".into(),
                    }],
                },
                TutorialStep {
                    step_id: "l1_s3_collision_test".into(),
                    step_type: TutorialStepType::Dialogue,
                    dialogue: Some(ArchmageDialogue {
                        lines: vec![
                            "Well stepped. Two units may never occupy the same hex — try to walk into the dummy's tile at (0, 2).".into(),
                        ],
                        emotion: "Neutral".into(),
                        auto_advance_ms: None,
                    }),
                    callout: None,
                    input_gate: None,
                    objective: None,
                    camera_target: Some(HexDto::new(0, 2)),
                    completion_condition: StepCondition::DialogueDismissed,
                    failure_feedback: vec![],
                },
                TutorialStep {
                    step_id: "l1_s4_attempt_collision".into(),
                    step_type: TutorialStepType::Objective,
                    dialogue: None,
                    callout: None,
                    input_gate: Some(InputGateConfig {
                        allowed_unit_ids: vec![1],
                        allowed_actions: vec!["Move".into()],
                        allowed_hex_targets: vec![HexDto::new(0, 2), HexDto::new(0, 1)],
                        max_ap_spend: Some(2),
                    }),
                    objective: Some(ObjectiveConfig {
                        text: "Attempt to move into the occupied hex at (0, 2)".into(),
                        spotlight_hexes: vec![HexDto::new(0, 2)],
                        ring_color: "#ff1744".into(),
                    }),
                    camera_target: Some(HexDto::new(0, 2)),
                    completion_condition: StepCondition::UnitMovedTo {
                        unit_id: 1,
                        target: HexDto::new(0, 1),
                    },
                    failure_feedback: vec![SoftFailRule {
                        error_code: "ERR_OCCUPIED_HEX".into(),
                        archmage_response: "Notice how the stone resists. The grid permits no shared space. When pathing, you must skirt around obstacles.".into(),
                    }],
                },
            ],
        }
    }

    fn preset_lesson_02() -> TutorialScenario {
        TutorialScenario {
            id: "lesson_02_attack".into(),
            lesson_index: 2,
            title: "A Strike Is a Promise".into(),
            gdd_reference: "gdd.md §7.3, §11.2".into(),
            map_radius: 4,
            initial_state: TutorialInitialState {
                player_team: 0,
                player_hero_kind: "Ranger".into(),
                player_hero_pos: HexDto::new(0, 0),
                player_gold: 0,
                player_xp: 0,
                player_energy: 5,
                board_units: vec![TutorialUnitPreset {
                    id: 100,
                    kind: "Dummy".into(),
                    team: 1,
                    pos: HexDto::new(1, 0),
                    hp: 50,
                    max_hp: 50,
                    initiative: 0,
                    attack_damage: 0,
                    is_invulnerable: false,
                }],
                fog_enabled: false,
            },
            steps: vec![
                TutorialStep {
                    step_id: "l2_s1_intro".into(),
                    step_type: TutorialStepType::Dialogue,
                    dialogue: Some(ArchmageDialogue {
                        lines: vec![
                            "When orders are locked, combat is deterministic. There is no chance, no critical roll, no evasion.".into(),
                            "Hover over your target to calculate the damage promised.".into(),
                        ],
                        emotion: "Enlightened".into(),
                        auto_advance_ms: None,
                    }),
                    callout: None,
                    input_gate: None,
                    objective: None,
                    camera_target: Some(HexDto::new(1, 0)),
                    completion_condition: StepCondition::DialogueDismissed,
                    failure_feedback: vec![],
                },
                TutorialStep {
                    step_id: "l2_s2_attack_dummy".into(),
                    step_type: TutorialStepType::Objective,
                    dialogue: None,
                    callout: None,
                    input_gate: Some(InputGateConfig {
                        allowed_unit_ids: vec![1],
                        allowed_actions: vec!["Attack".into(), "Lock".into()],
                        allowed_hex_targets: vec![HexDto::new(1, 0)],
                        max_ap_spend: Some(1),
                    }),
                    objective: Some(ObjectiveConfig {
                        text: "Attack the dummy at (1, 0) and resolve round".into(),
                        spotlight_hexes: vec![HexDto::new(1, 0)],
                        ring_color: "#ff1744".into(),
                    }),
                    camera_target: Some(HexDto::new(1, 0)),
                    completion_condition: StepCondition::RoundResolved { target_round: 1 },
                    failure_feedback: vec![],
                },
            ],
        }
    }

    fn preset_lesson_03() -> TutorialScenario {
        TutorialScenario {
            id: "lesson_03_initiative".into(),
            lesson_index: 3,
            title: "The Round Resolves".into(),
            gdd_reference: "gdd.md §7.1, §7.3".into(),
            map_radius: 4,
            initial_state: TutorialInitialState {
                player_team: 0,
                player_hero_kind: "Ranger".into(),
                player_hero_pos: HexDto::new(-1, 0),
                player_gold: 0,
                player_xp: 0,
                player_energy: 5,
                board_units: vec![TutorialUnitPreset {
                    id: 101,
                    kind: "Dummy".into(),
                    team: 1,
                    pos: HexDto::new(0, 0),
                    hp: 15,
                    max_hp: 50,
                    initiative: 2,
                    attack_damage: 30,
                    is_invulnerable: false,
                }],
                fog_enabled: false,
            },
            steps: vec![
                TutorialStep {
                    step_id: "l3_s1_briefing".into(),
                    step_type: TutorialStepType::Dialogue,
                    dialogue: Some(ArchmageDialogue {
                        lines: vec![
                            "Simultaneous orders resolve by Initiative. Your Ranger strikes at Initiative 4 before the dummy at Initiative 2 — Death-Before-Acting!".into(),
                        ],
                        emotion: "Enlightened".into(),
                        auto_advance_ms: None,
                    }),
                    callout: None,
                    input_gate: None,
                    objective: None,
                    camera_target: Some(HexDto::new(0, 0)),
                    completion_condition: StepCondition::DialogueDismissed,
                    failure_feedback: vec![],
                },
                TutorialStep {
                    step_id: "l3_s2_order_attack".into(),
                    step_type: TutorialStepType::Objective,
                    dialogue: None,
                    callout: None,
                    input_gate: Some(InputGateConfig {
                        allowed_unit_ids: vec![1],
                        allowed_actions: vec!["Attack".into(), "Lock".into()],
                        allowed_hex_targets: vec![HexDto::new(0, 0)],
                        max_ap_spend: Some(1),
                    }),
                    objective: Some(ObjectiveConfig {
                        text: "Order Ranger to attack the dummy at (0, 0), then lock round".into(),
                        spotlight_hexes: vec![HexDto::new(0, 0)],
                        ring_color: "#ff1744".into(),
                    }),
                    camera_target: Some(HexDto::new(0, 0)),
                    completion_condition: StepCondition::RoundResolved { target_round: 1 },
                    failure_feedback: vec![],
                },
                TutorialStep {
                    step_id: "l3_s3_debrief".into(),
                    step_type: TutorialStepType::Dialogue,
                    dialogue: Some(ArchmageDialogue {
                        lines: vec![
                            "Witness the outcome. The dummy prepared a 30 damage strike against you.".into(),
                            "Yet your photon arrow struck first at Initiative 4, dealing 16 lethal damage.".into(),
                            "The dummy perished instantly. Its queued strike dissolved into dust. Speed is life.".into(),
                        ],
                        emotion: "Pleased".into(),
                        auto_advance_ms: None,
                    }),
                    callout: None,
                    input_gate: None,
                    objective: None,
                    camera_target: Some(HexDto::new(0, 0)),
                    completion_condition: StepCondition::DialogueDismissed,
                    failure_feedback: vec![],
                },
            ],
        }
    }

    fn preset_lesson_07() -> TutorialScenario {
        TutorialScenario {
            id: "lesson_07_fog_of_war".into(),
            lesson_index: 7,
            title: "The Shadow Knows".into(),
            gdd_reference: "gdd.md §6.3".into(),
            map_radius: 5,
            initial_state: TutorialInitialState {
                player_team: 0,
                player_hero_kind: "Ranger".into(),
                player_hero_pos: HexDto::new(-3, 0),
                player_gold: 0,
                player_xp: 0,
                player_energy: 5,
                board_units: vec![TutorialUnitPreset {
                    id: 201,
                    kind: "Dummy".into(),
                    team: 1,
                    pos: HexDto::new(2, 0),
                    hp: 50,
                    max_hp: 50,
                    initiative: 1,
                    attack_damage: 0,
                    is_invulnerable: false,
                }],
                fog_enabled: true,
            },
            steps: vec![
                TutorialStep {
                    step_id: "l7_s1_intro".into(),
                    step_type: TutorialStepType::Dialogue,
                    dialogue: Some(ArchmageDialogue {
                        lines: vec![
                            "The battlefield is cloaked in the Mist of Convergence.".into(),
                            "Hexes exist in three states: Lit, Explored Memory, and Unexplored Darkness.".into(),
                            "Your Ranger has a superior vision radius of 4 hexes. Advance toward the crystal wall to dispel the veil.".into(),
                        ],
                        emotion: "Neutral".into(),
                        auto_advance_ms: None,
                    }),
                    callout: None,
                    input_gate: None,
                    objective: None,
                    camera_target: Some(HexDto::new(-3, 0)),
                    completion_condition: StepCondition::DialogueDismissed,
                    failure_feedback: vec![],
                },
                TutorialStep {
                    step_id: "l7_s2_scout_hex".into(),
                    step_type: TutorialStepType::Objective,
                    dialogue: None,
                    callout: None,
                    input_gate: Some(InputGateConfig {
                        allowed_unit_ids: vec![1],
                        allowed_actions: vec!["Move".into(), "Lock".into()],
                        allowed_hex_targets: vec![HexDto::new(-1, 0)],
                        max_ap_spend: Some(2),
                    }),
                    objective: Some(ObjectiveConfig {
                        text: "March 2 hexes East to reveal the hidden sector".into(),
                        spotlight_hexes: vec![HexDto::new(-1, 0)],
                        ring_color: "#00e676".into(),
                    }),
                    camera_target: Some(HexDto::new(-1, 0)),
                    completion_condition: StepCondition::UnitMovedTo {
                        unit_id: 1,
                        target: HexDto::new(-1, 0),
                    },
                    failure_feedback: vec![],
                },
            ],
        }
    }
}
