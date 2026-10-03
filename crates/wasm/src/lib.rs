use hexabellum_core::hex::HexCoord;
use hexabellum_core::orders::{Action, UnitOrder};
use hexabellum_core::tutorial::TutorialScenarioRunner;
use hexabellum_core::GameEngine;
use hexabellum_protocol::tutorial::TutorialOrderResult;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmGame {
    engine: GameEngine,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmGame {
        WasmGame {
            engine: GameEngine::new(),
        }
    }

    pub fn get_state(&self) -> String {
        self.engine.get_state()
    }

    pub fn get_player_state(&self, team: u8) -> String {
        self.engine.get_player_state(team)
    }

    pub fn get_map_hexes(&self) -> String {
        self.engine.get_map_hexes()
    }

    pub fn get_obstacles(&self) -> String {
        self.engine.get_obstacles()
    }

    pub fn get_move_targets(&self, unit_id: u64) -> String {
        self.engine.get_move_targets(unit_id)
    }

    pub fn find_path(&self, from_q: i32, from_r: i32, to_q: i32, to_r: i32) -> String {
        self.engine.find_path(from_q, from_r, to_q, to_r)
    }

    pub fn get_attack_targets(&self, unit_id: u64, from_q: i32, from_r: i32) -> String {
        self.engine.get_attack_targets(unit_id, from_q, from_r)
    }

    pub fn set_move_order(&mut self, unit_id: u64, q: i32, r: i32) -> bool {
        self.engine.set_move_order(unit_id, q, r)
    }

    pub fn set_attack_order(&mut self, unit_id: u64, target_id: u64) -> bool {
        self.engine.set_attack_order(unit_id, target_id)
    }

    pub fn set_wait_order(&mut self, unit_id: u64) -> bool {
        self.engine.set_wait_order(unit_id)
    }

    pub fn get_pending_orders(&self) -> String {
        self.engine.get_pending_orders()
    }

    pub fn all_units_ordered(&self) -> bool {
        self.engine.all_units_ordered()
    }

    pub fn end_turn(&mut self) -> String {
        self.engine.end_turn()
    }

    pub fn get_player_fog(&self) -> String {
        self.engine.get_player_fog()
    }

    pub fn clear_orders(&mut self, unit_id: u64) {
        self.engine.clear_orders(unit_id);
    }

    pub fn restart(&mut self) {
        self.engine.restart();
    }
}

impl Default for WasmGame {
    fn default() -> Self {
        Self::new()
    }
}

/// Offline WebAssembly controller for interactive Tutorial Scenarios.
#[wasm_bindgen]
pub struct WasmTutorialSession {
    runner: TutorialScenarioRunner,
}

#[wasm_bindgen]
impl WasmTutorialSession {
    #[wasm_bindgen(constructor)]
    pub fn new(scenario_id_or_json: &str) -> WasmTutorialSession {
        WasmTutorialSession {
            runner: TutorialScenarioRunner::load(scenario_id_or_json),
        }
    }

    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.runner.state).unwrap_or_default()
    }

    pub fn get_current_step_id(&self) -> String {
        self.runner.current_step().to_string()
    }

    pub fn get_current_step_json(&self) -> String {
        if self.runner.is_completed() {
            "null".to_string()
        } else {
            serde_json::to_string(self.runner.current_step_def()).unwrap_or_default()
        }
    }

    pub fn is_completed(&self) -> bool {
        self.runner.is_completed()
    }

    pub fn advance_dialogue(&mut self) -> bool {
        self.runner.advance_dialogue().unwrap_or(false)
    }

    pub fn skip_to_step(&mut self, step_id: &str) -> bool {
        self.runner.skip_to_step(step_id).is_ok()
    }

    pub fn reset_current_step(&mut self) {
        self.runner.reset_current_step();
    }

    pub fn submit_order(
        &mut self,
        unit_id: u64,
        move_q: Option<i32>,
        move_r: Option<i32>,
        attack_target: Option<u64>,
    ) -> String {
        let action = if let Some(target_id) = attack_target {
            Action::Attack { target_id }
        } else {
            Action::Wait
        };
        let move_target = match (move_q, move_r) {
            (Some(q), Some(r)) => Some(HexCoord::new(q, r)),
            _ => None,
        };
        let order = UnitOrder {
            unit_id,
            move_target,
            action,
        };
        match self.runner.submit_order(order) {
            Ok(res) => serde_json::to_string(&res).unwrap_or_default(),
            Err(err) => serde_json::to_string(&TutorialOrderResult {
                valid: false,
                error_code: Some(err.code),
                completed_step: false,
                archmage_feedback: Some(err.archmage_response),
            })
            .unwrap_or_default(),
        }
    }

    pub fn resolve_round(&mut self) -> String {
        match self.runner.resolve_round() {
            Ok(events) => serde_json::to_string(&events).unwrap_or_default(),
            Err(err) => format!(r#"{{"error":"{}"}}"#, err.message),
        }
    }

    pub fn get_map_hexes(&self) -> String {
        let hexes: Vec<(i32, i32)> = self
            .runner
            .state
            .map
            .all_hexes()
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&hexes).unwrap_or_default()
    }

    pub fn get_obstacles(&self) -> String {
        let obs: Vec<(i32, i32)> = self
            .runner
            .state
            .map
            .obstacles
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&obs).unwrap_or_default()
    }

    pub fn get_player_fog(&self) -> String {
        let mut visible: Vec<(i32, i32)> = self
            .runner
            .state
            .fog
            .visible_hexes(0)
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        visible.sort();
        serde_json::to_string(&visible).unwrap_or_default()
    }
}
