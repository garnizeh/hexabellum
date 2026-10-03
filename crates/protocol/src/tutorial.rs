use crate::{HexDto, TeamId, UnitId};
use serde::{Deserialize, Serialize};

/// Complete tutorial scenario definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TutorialScenario {
    pub id: String,
    pub lesson_index: u32,
    pub title: String,
    pub gdd_reference: String,
    pub map_radius: u32,
    pub initial_state: TutorialInitialState,
    pub steps: Vec<TutorialStep>,
}

/// Initial board state injected at the start of a lesson.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TutorialInitialState {
    pub player_team: TeamId,
    pub player_hero_kind: String, // e.g. "Ranger", "Vanguard"
    pub player_hero_pos: HexDto,
    pub player_gold: u32,
    pub player_xp: u32,
    pub player_energy: u32,
    pub board_units: Vec<TutorialUnitPreset>,
    pub fog_enabled: bool,
}

/// Unit preset placed on the tutorial board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TutorialUnitPreset {
    pub id: UnitId,
    pub kind: String, // "Hero", "Minion", "Tower", "Spawner", "Dummy"
    pub team: TeamId,
    pub pos: HexDto,
    pub hp: u32,
    pub max_hp: u32,
    pub initiative: u32,
    pub attack_damage: u32,
    pub is_invulnerable: bool,
}

/// A discrete step inside a tutorial lesson.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TutorialStep {
    pub step_id: String,
    pub step_type: TutorialStepType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialogue: Option<ArchmageDialogue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callout: Option<CalloutConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_gate: Option<InputGateConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective: Option<ObjectiveConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_target: Option<HexDto>,
    pub completion_condition: StepCondition,
    #[serde(default)]
    pub failure_feedback: Vec<SoftFailRule>,
}

/// Step type governing visual presentation and player interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TutorialStepType {
    Dialogue,
    Callout,
    Objective,
    ResolutionWatch,
    Milestone,
}

/// Dialogue speech card delivered by the Archmage of the Convergence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchmageDialogue {
    pub lines: Vec<String>,
    pub emotion: String, // "Neutral", "Pleased", "Stern", "Enlightened"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_advance_ms: Option<u64>,
}

/// Callout targeting a specific UI element with an informative tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalloutConfig {
    pub target_selector: String,
    pub title: String,
    pub description: String,
    pub placement: String, // "top", "bottom", "left", "right"
}

/// Input restriction whitelist enforcing lesson focus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputGateConfig {
    #[serde(default)]
    pub allowed_unit_ids: Vec<UnitId>,
    #[serde(default)]
    pub allowed_actions: Vec<String>, // "Move", "Attack", "Wait", "Lock"
    #[serde(default)]
    pub allowed_hex_targets: Vec<HexDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_ap_spend: Option<u32>,
}

/// Objective pinned to top-left checklist and glowing on canvas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveConfig {
    pub text: String,
    pub spotlight_hexes: Vec<HexDto>,
    pub ring_color: String, // e.g. "#00e676", "#ff1744"
}

/// Pedagogical feedback rule mapping validation errors to Archmage lore explanations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftFailRule {
    pub error_code: String,
    pub archmage_response: String,
}

/// Conditions required to complete a step and advance the lesson.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "condition")]
pub enum StepCondition {
    DialogueDismissed,
    UnitMovedTo {
        unit_id: UnitId,
        target: HexDto,
    },
    UnitDamaged {
        unit_id: UnitId,
        min_damage: u32,
    },
    UnitKilled {
        unit_id: UnitId,
    },
    AbilityCast {
        unit_id: UnitId,
        ability_id: String,
    },
    ItemPurchased {
        item_id: String,
    },
    RoundResolved {
        target_round: u32,
    },
}

/// Result returned after submitting an order in tutorial mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TutorialOrderResult {
    pub valid: bool,
    pub error_code: Option<String>,
    pub completed_step: bool,
    pub archmage_feedback: Option<String>,
}
