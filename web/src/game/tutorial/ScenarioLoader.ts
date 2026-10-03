export interface HexDto {
  q: number;
  r: number;
}

export interface ArchmageDialogue {
  lines: string[];
  emotion: 'Neutral' | 'Pleased' | 'Stern' | 'Enlightened';
  auto_advance_ms?: number;
}

export interface CalloutConfig {
  target_selector: string;
  title: string;
  description: string;
  placement: 'top' | 'bottom' | 'left' | 'right';
}

export interface InputGateConfig {
  allowed_unit_ids: number[];
  allowed_actions: string[]; // "Move", "Attack", "Wait", "Lock"
  allowed_hex_targets: HexDto[];
  max_ap_spend?: number;
}

export interface ObjectiveConfig {
  text: string;
  spotlight_hexes: HexDto[];
  ring_color: string;
}

export interface SoftFailRule {
  error_code: string;
  archmage_response: string;
}

export type StepCondition =
  | { condition: 'DialogueDismissed' }
  | { condition: 'UnitMovedTo'; unit_id: number; target: HexDto }
  | { condition: 'UnitDamaged'; unit_id: number; min_damage: number }
  | { condition: 'UnitKilled'; unit_id: number }
  | { condition: 'AbilityCast'; unit_id: number; ability_id: string }
  | { condition: 'ItemPurchased'; item_id: string }
  | { condition: 'RoundResolved'; target_round: number };

export interface TutorialStep {
  step_id: string;
  step_type: 'Dialogue' | 'Callout' | 'Objective' | 'ResolutionWatch' | 'Milestone';
  dialogue?: ArchmageDialogue;
  callout?: CalloutConfig;
  input_gate?: InputGateConfig;
  objective?: ObjectiveConfig;
  camera_target?: HexDto;
  completion_condition: StepCondition;
  failure_feedback?: SoftFailRule[];
}

export interface TutorialUnitPreset {
  id: number;
  kind: string;
  team: number;
  pos: HexDto;
  hp: number;
  max_hp: number;
  initiative: number;
  attack_damage: number;
  is_invulnerable: boolean;
}

export interface TutorialScenario {
  id: string;
  lesson_index: number;
  title: string;
  gdd_reference: string;
  map_radius: number;
  initial_state: {
    player_team: number;
    player_hero_kind: string;
    player_hero_pos: HexDto;
    player_gold: number;
    player_xp: number;
    player_energy: number;
    board_units: TutorialUnitPreset[];
    fog_enabled: boolean;
  };
  steps: TutorialStep[];
}

export class ScenarioLoader {
  private static scenarios: Record<string, TutorialScenario> = {
    lesson_00_intro: {
      id: "lesson_00_intro",
      lesson_index: 0,
      title: "The Convergence Calls",
      gdd_reference: "gdd.md §1, §13.1",
      map_radius: 4,
      initial_state: {
        player_team: 0,
        player_hero_kind: "Ranger",
        player_hero_pos: { q: 0, r: 0 },
        player_gold: 0,
        player_xp: 0,
        player_energy: 5,
        board_units: [],
        fog_enabled: false,
      },
      steps: [
        {
          step_id: "l0_s1_welcome",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "Welcome, traveler. You stand upon the Proving Grounds of the Convergence. Six sides to every stone, infinite paths to victory.",
            ],
            emotion: "Enlightened",
          },
          camera_target: { q: 0, r: 0 },
          completion_condition: { condition: "DialogueDismissed" },
        },
        {
          step_id: "l0_s2_hud_callout",
          step_type: "Callout",
          dialogue: {
            lines: [
              "Observe the Top Bar: round indicator, planning timer, and order status. Top-left holds your mission checklist.",
            ],
            emotion: "Neutral",
          },
          callout: {
            target_selector: "#hud",
            title: "Tactical Command HUD",
            description: "Monitor round countdown, status, and turn progression.",
            placement: "bottom",
          },
          objective: {
            text: "Acknowledge the tactical HUD briefing",
            spotlight_hexes: [{ q: 0, r: 0 }],
            ring_color: "#00d2ff",
          },
          camera_target: { q: 0, r: 0 },
          completion_condition: { condition: "DialogueDismissed" },
        },
      ],
    },

    lesson_01_movement: {
      id: "lesson_01_movement",
      lesson_index: 1,
      title: "The Three Steps",
      gdd_reference: "gdd.md §6.2",
      map_radius: 4,
      initial_state: {
        player_team: 0,
        player_hero_kind: "Ranger",
        player_hero_pos: { q: 0, r: 0 },
        player_gold: 0,
        player_xp: 0,
        player_energy: 5,
        board_units: [
          {
            id: 99,
            kind: "Dummy",
            team: 2,
            pos: { q: 0, r: 2 },
            hp: 100,
            max_hp: 100,
            initiative: 0,
            attack_damage: 0,
            is_invulnerable: true,
          },
        ],
        fog_enabled: false,
      },
      steps: [
        {
          step_id: "l1_s1_intro",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "Movement is the foundation of war. Your hero has 3 Action Points each round (1 AP per hex). March 1 hex East to coordinate (1, 0).",
            ],
            emotion: "Neutral",
          },
          camera_target: { q: 0, r: 0 },
          completion_condition: { condition: "DialogueDismissed" },
        },
        {
          step_id: "l1_s2_move_one",
          step_type: "Objective",
          objective: {
            text: "Move 1 hex East to coordinate (1, 0)",
            spotlight_hexes: [{ q: 1, r: 0 }],
            ring_color: "#00e676",
          },
          input_gate: {
            allowed_unit_ids: [1],
            allowed_actions: ["Move"],
            allowed_hex_targets: [{ q: 1, r: 0 }],
            max_ap_spend: 1,
          },
          camera_target: { q: 1, r: 0 },
          failure_feedback: [
            {
              error_code: "ERR_INVALID_TARGET",
              archmage_response: "Look for the emerald ring to the East. Click that hex to plot your march.",
            },
          ],
          completion_condition: {
            condition: "UnitMovedTo",
            unit_id: 1,
            target: { q: 1, r: 0 },
          },
        },
        {
          step_id: "l1_s3_collision_test",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "Well stepped. Two units may never occupy the same hex — try to walk into the dummy's tile at (0, 2).",
            ],
            emotion: "Neutral",
          },
          camera_target: { q: 0, r: 2 },
          completion_condition: { condition: "DialogueDismissed" },
        },
        {
          step_id: "l1_s4_attempt_collision",
          step_type: "Objective",
          objective: {
            text: "Observe tile collision: click dummy at (0, 2), then move to (0, 1)",
            spotlight_hexes: [{ q: 0, r: 2 }, { q: 0, r: 1 }],
            ring_color: "#ff1744",
          },
          input_gate: {
            allowed_unit_ids: [1],
            allowed_actions: ["Move"],
            allowed_hex_targets: [{ q: 0, r: 2 }, { q: 0, r: 1 }],
            max_ap_spend: 2,
          },
          camera_target: { q: 0, r: 2 },
          failure_feedback: [
            {
              error_code: "ERR_OCCUPIED_HEX",
              archmage_response: "Notice how the stone resists. The grid permits no shared space. When pathing, you must skirt around obstacles.",
            },
          ],
          completion_condition: {
            condition: "UnitMovedTo",
            unit_id: 1,
            target: { q: 0, r: 1 },
          },
        },
      ],
    },

    lesson_02_attack: {
      id: "lesson_02_attack",
      lesson_index: 2,
      title: "A Strike Is a Promise",
      gdd_reference: "gdd.md §7.3, §11.2",
      map_radius: 4,
      initial_state: {
        player_team: 0,
        player_hero_kind: "Ranger",
        player_hero_pos: { q: 0, r: 0 },
        player_gold: 0,
        player_xp: 0,
        player_energy: 5,
        board_units: [
          {
            id: 100,
            kind: "Dummy",
            team: 1,
            pos: { q: 1, r: 0 },
            hp: 50,
            max_hp: 50,
            initiative: 0,
            attack_damage: 0,
            is_invulnerable: false,
          },
        ],
        fog_enabled: false,
      },
      steps: [
        {
          step_id: "l2_s1_intro",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "When orders are locked, combat is deterministic. There is no chance, no critical roll, no evasion.",
              "Draw your recurve bow against the training dummy. Hover to read the exact damage promised.",
            ],
            emotion: "Enlightened",
          },
          camera_target: { q: 1, r: 0 },
          completion_condition: { condition: "DialogueDismissed" },
        },
        {
          step_id: "l2_s2_attack_dummy",
          step_type: "Objective",
          objective: {
            text: "Order Ranger to attack dummy at (1, 0) and lock round",
            spotlight_hexes: [{ q: 1, r: 0 }],
            ring_color: "#ff1744",
          },
          input_gate: {
            allowed_unit_ids: [1],
            allowed_actions: ["Attack", "Lock"],
            allowed_hex_targets: [{ q: 1, r: 0 }],
            max_ap_spend: 1,
          },
          camera_target: { q: 1, r: 0 },
          completion_condition: {
            condition: "RoundResolved",
            target_round: 1,
          },
        },
        {
          step_id: "l2_s3_debrief",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "The stone shatter matches the calculation to the digit: 16 damage dealt cleanly.",
              "When you strike in Hexabellum, you know the outcome before it lands.",
            ],
            emotion: "Pleased",
          },
          completion_condition: { condition: "DialogueDismissed" },
        },
      ],
    },

    lesson_03_initiative: {
      id: "lesson_03_initiative",
      lesson_index: 3,
      title: "The Round Resolves",
      gdd_reference: "gdd.md §7.1, §7.3",
      map_radius: 4,
      initial_state: {
        player_team: 0,
        player_hero_kind: "Ranger",
        player_hero_pos: { q: -1, r: 0 },
        player_gold: 0,
        player_xp: 0,
        player_energy: 5,
        board_units: [
          {
            id: 101,
            kind: "Dummy",
            team: 1,
            pos: { q: 0, r: 0 },
            hp: 15,
            max_hp: 50,
            initiative: 2,
            attack_damage: 30,
            is_invulnerable: false,
          },
        ],
        fog_enabled: false,
      },
      steps: [
        {
          step_id: "l3_s1_briefing",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "Simultaneous orders resolve by Initiative. Your Ranger strikes at Initiative 4 before the dummy at Initiative 2 — Death-Before-Acting!",
            ],
            emotion: "Enlightened",
          },
          camera_target: { q: 0, r: 0 },
          completion_condition: { condition: "DialogueDismissed" },
        },
        {
          step_id: "l3_s2_order_attack",
          step_type: "Objective",
          objective: {
            text: "Order Ranger to attack the dummy at (0, 0), then lock round",
            spotlight_hexes: [{ q: 0, r: 0 }],
            ring_color: "#ff1744",
          },
          input_gate: {
            allowed_unit_ids: [1],
            allowed_actions: ["Attack", "Lock"],
            allowed_hex_targets: [{ q: 0, r: 0 }],
          },
          camera_target: { q: 0, r: 0 },
          completion_condition: {
            condition: "RoundResolved",
            target_round: 1,
          },
        },
        {
          step_id: "l3_s3_debrief",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "Death-Before-Acting witnessed! Your photon arrow struck first at Initiative 4, extinguishing the dummy's queued attack. Speed is life!",
            ],
            emotion: "Pleased",
          },
          camera_target: { q: 0, r: 0 },
          completion_condition: { condition: "DialogueDismissed" },
        },
      ],
    },

    lesson_07_fog_of_war: {
      id: "lesson_07_fog_of_war",
      lesson_index: 7,
      title: "The Shadow Knows",
      gdd_reference: "gdd.md §6.3",
      map_radius: 5,
      initial_state: {
        player_team: 0,
        player_hero_kind: "Ranger",
        player_hero_pos: { q: -3, r: 0 },
        player_gold: 0,
        player_xp: 0,
        player_energy: 5,
        board_units: [
          {
            id: 201,
            kind: "Dummy",
            team: 1,
            pos: { q: 2, r: 0 },
            hp: 50,
            max_hp: 50,
            initiative: 1,
            attack_damage: 0,
            is_invulnerable: false,
          },
        ],
        fog_enabled: true,
      },
      steps: [
        {
          step_id: "l7_s1_intro",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "The battlefield is cloaked in the Mist of Convergence.",
              "Hexes exist in three states: Lit, Explored Memory, and Unexplored Darkness.",
              "Your Ranger has a superior vision radius of 4 hexes. Advance toward the crystal wall to dispel the veil.",
            ],
            emotion: "Neutral",
          },
          camera_target: { q: -3, r: 0 },
          completion_condition: { condition: "DialogueDismissed" },
        },
        {
          step_id: "l7_s2_scout_hex",
          step_type: "Objective",
          objective: {
            text: "March 2 hexes East to reveal the hidden sector",
            spotlight_hexes: [{ q: -1, r: 0 }],
            ring_color: "#00e676",
          },
          input_gate: {
            allowed_unit_ids: [1],
            allowed_actions: ["Move", "Lock"],
            allowed_hex_targets: [{ q: -1, r: 0 }],
            max_ap_spend: 2,
          },
          camera_target: { q: -1, r: 0 },
          completion_condition: {
            condition: "UnitMovedTo",
            unit_id: 1,
            target: { q: -1, r: 0 },
          },
        },
        {
          step_id: "l7_s3_reveal_dummy",
          step_type: "Dialogue",
          dialogue: {
            lines: [
              "The mist clears! An enemy sentry dummy rests at (2, 0).",
              "Information is the deadliest weapon on the grid. Reveal them before they reveal you.",
            ],
            emotion: "Enlightened",
          },
          camera_target: { q: 2, r: 0 },
          completion_condition: { condition: "DialogueDismissed" },
        },
      ],
    },
  };

  public static getScenario(id: string): TutorialScenario {
    if (this.scenarios[id]) {
      return this.scenarios[id];
    }
    return this.scenarios.lesson_00_intro;
  }

  public static getAllLessons() {
    return [
      { id: "lesson_00_intro", index: 0, title: "The Convergence Calls", desc: "HUD anatomy, hero identity, and Action Points" },
      { id: "lesson_01_movement", index: 1, title: "The Three Steps", desc: "Hex movement, 1 AP/hex cost, and collision blocking" },
      { id: "lesson_02_attack", index: 2, title: "A Strike Is a Promise", desc: "Basic attack range, hover calculation, and deterministic damage" },
      { id: "lesson_03_initiative", index: 3, title: "The Round Resolves", desc: "Simultaneous resolution, initiative order, and death-before-acting" },
      { id: "lesson_07_fog_of_war", index: 7, title: "The Shadow Knows", desc: "Mist of Convergence, 3-state Fog of War, and vision radius" },
    ];
  }
}
