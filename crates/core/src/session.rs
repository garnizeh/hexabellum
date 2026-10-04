use crate::ai::GameAI;
use crate::event::GameEvent;
use crate::hex::{HexCoord, HexMap};
use crate::orders::{Action, TurnOrders, UnitOrder};
use crate::state::{GameState, Phase};
use crate::turn::TurnProcessor;
use crate::unit::{Unit, UnitId, UnitKind};
use hexabellum_protocol::{
    ActionDto, BaseZoneDto, HexDto, LifeStateDto, MapDto, MatchPhaseDto, ObjectiveStatusDto,
    OrderDto, ProtocolErrorCode, RosterEntryDto, SanitizedGameEvent, ShopDisabledReasonDto,
    SnapshotDto, TeamId, UnitDto,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::base_zone::BaseZone;
use crate::unit::LifeState;

pub use crate::controller::{Controller, ControllerMap, PlayerId};

/// Detailed error information for rejected order submissions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderSubmissionError {
    pub code: ProtocolErrorCode,
    pub unit_id: Option<UnitId>,
    pub reason: String,
}

impl PartialEq<ProtocolErrorCode> for OrderSubmissionError {
    fn eq(&self, other: &ProtocolErrorCode) -> bool {
        self.code == *other
            || (*other == ProtocolErrorCode::UnitNotOwned
                && self.code == ProtocolErrorCode::NotYourUnit)
            || (*other == ProtocolErrorCode::NotYourUnit
                && self.code == ProtocolErrorCode::UnitNotOwned)
    }
}

/// Match configuration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleConfig {
    pub map_radius: u32,
    pub players_per_team: u32,
    pub heroes_per_team: u32,
    pub turn_duration_secs: u64,
    pub early_resolution_grace_ms: u64,
    pub spawn_interval: u32,
    pub fill_empty_slots_with_ai: bool,
    #[serde(default)]
    pub enable_ai_team_1: bool,
    #[serde(default)]
    pub skip_draft: bool,
    #[serde(default)]
    pub economy: crate::economy::EconomyConfig,
    #[serde(default)]
    pub phase7: crate::macro_config::Phase7Config,
}

impl Default for BattleConfig {
    fn default() -> Self {
        Self {
            map_radius: 8,
            players_per_team: 5,
            heroes_per_team: 5,
            turn_duration_secs: 30,
            early_resolution_grace_ms: 1000,
            spawn_interval: 3,
            fill_empty_slots_with_ai: true,
            enable_ai_team_1: false,
            skip_draft: false,
            economy: crate::economy::EconomyConfig::default(),
            phase7: crate::macro_config::Phase7Config::default(),
        }
    }
}

impl BattleConfig {
    pub fn legacy_3v3() -> Self {
        Self {
            map_radius: 6,
            players_per_team: 1,
            heroes_per_team: 3,
            turn_duration_secs: 30,
            early_resolution_grace_ms: 1000,
            spawn_interval: 3,
            fill_empty_slots_with_ai: false,
            enable_ai_team_1: false,
            skip_draft: true,
            economy: crate::economy::EconomyConfig::default(),
            phase7: crate::macro_config::Phase7Config::default(),
        }
    }
}

/// Headless authoritative match session.
pub struct BattleSession {
    pub match_id: String,
    pub state: GameState,
    pub controllers: ControllerMap,
    pub staged_orders: HashMap<TeamId, HashMap<UnitId, UnitOrder>>,
    pub submitted_teams: HashSet<TeamId>,
    pub config: BattleConfig,
    pub unit_registry: HashMap<UnitId, (TeamId, UnitKind, HexCoord)>,
    pub hero_assignments: HashMap<String, UnitId>,
    pub base_zones: HashMap<TeamId, BaseZone>,
    pub is_match_over: bool,
    pub winning_team: Option<TeamId>,
}

impl std::ops::Deref for BattleSession {
    type Target = GameState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl std::ops::DerefMut for BattleSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl BattleSession {
    pub fn new_test_5v5() -> Self {
        let mut config = BattleConfig::default();
        config.map_radius = 8;
        config.heroes_per_team = 5;
        config.players_per_team = 5;
        Self::new("test_5v5".into(), config)
    }

    pub fn new_with_seed(config: BattleConfig, _seed: u64) -> Self {
        Self::new("sim".into(), config)
    }

    /// Initialize a battle session supporting 1v1, 3v3, or 5v5 MOBA configurations.
    pub fn new(match_id: String, config: BattleConfig) -> Self {
        let mut map = HexMap::new(config.map_radius);
        let mut state;
        let mut controllers = ControllerMap::new();
        let mut hero_assignments = HashMap::new();
        let mut base_zones = HashMap::new();

        if config.heroes_per_team >= 5 || config.map_radius >= 8 {
            base_zones.insert(
                0,
                BaseZone::new(0, HexCoord::new(-7, 0), config.phase7.base_zone_radius),
            );
            base_zones.insert(
                1,
                BaseZone::new(1, HexCoord::new(7, 0), config.phase7.base_zone_radius),
            );

            // ==========================================
            // Phase 5 Scaled Arena Topology (Radius 8, 217 Hexes)
            // ==========================================
            // 6 Tactical Vision Blockers (both movement and sight blockers)
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(0, -2)));
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(0, 2)));
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(-2, -3)));
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(2, -3)));
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(-3, 2)));
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(3, -2)));

            state = GameState::new(map);

            // Team 0 Core
            let t0_core = Unit::new_core(
                500,
                0,
                HexCoord::new(-7, 0),
                config.phase7.core_hp,
                config.phase7.core_vision_range,
            );
            controllers.assign(500, Controller::Automatic);
            state.add_unit(t0_core);

            // Team 0 Spawners and Towers
            let mut t0_spawner_north =
                Unit::new_spawner(100, 0, HexCoord::new(-6, -1), config.spawn_interval);
            t0_spawner_north.hp = 200;
            t0_spawner_north.max_hp = 200;
            let mut t0_spawner_south =
                Unit::new_spawner(102, 0, HexCoord::new(-6, 1), config.spawn_interval);
            t0_spawner_south.hp = 200;
            t0_spawner_south.max_hp = 200;
            let t0_tower_north = Unit::new_tower(101, 0, HexCoord::new(-4, -1));
            let t0_tower_south = Unit::new_tower(103, 0, HexCoord::new(-4, 1));
            controllers.assign(100, Controller::Automatic);
            controllers.assign(101, Controller::Automatic);
            controllers.assign(102, Controller::Automatic);
            controllers.assign(103, Controller::Automatic);
            state.add_unit(t0_spawner_north);
            state.add_unit(t0_spawner_south);
            state.add_unit(t0_tower_north);
            state.add_unit(t0_tower_south);

            // Team 0 Heroes: Vanguard, Ranger, Warden, Sniper, Berserker
            state.add_unit(Unit::new_vanguard(1, 0, HexCoord::new(-6, 0), 3));
            state.add_unit(Unit::new_ranger(2, 0, HexCoord::new(-7, -1), 4));
            state.add_unit(Unit::new_warden(3, 0, HexCoord::new(-8, 0), 2));
            state.add_unit(Unit::new_sniper(4, 0, HexCoord::new(-7, 1), 4));
            state.add_unit(Unit::new_berserker(5, 0, HexCoord::new(-8, 1), 3));
            for uid in 1..=5 {
                controllers.assign(uid, Controller::Ai);
            }
            hero_assignments.insert("vanguard_0".to_string(), 1);
            hero_assignments.insert("ranger_0".to_string(), 2);
            hero_assignments.insert("warden_0".to_string(), 3);
            hero_assignments.insert("sniper_0".to_string(), 4);
            hero_assignments.insert("berserker_0".to_string(), 5);

            // Team 1 Core
            let t1_core = Unit::new_core(
                501,
                1,
                HexCoord::new(7, 0),
                config.phase7.core_hp,
                config.phase7.core_vision_range,
            );
            controllers.assign(501, Controller::Automatic);
            state.add_unit(t1_core);

            // Team 1 Spawners and Towers
            let mut t1_spawner_north =
                Unit::new_spawner(200, 1, HexCoord::new(6, -1), config.spawn_interval);
            t1_spawner_north.hp = 200;
            t1_spawner_north.max_hp = 200;
            let mut t1_spawner_south =
                Unit::new_spawner(202, 1, HexCoord::new(6, 1), config.spawn_interval);
            t1_spawner_south.hp = 200;
            t1_spawner_south.max_hp = 200;
            let t1_tower_north = Unit::new_tower(201, 1, HexCoord::new(4, -1));
            let t1_tower_south = Unit::new_tower(203, 1, HexCoord::new(4, 1));
            controllers.assign(200, Controller::Automatic);
            controllers.assign(201, Controller::Automatic);
            controllers.assign(202, Controller::Automatic);
            controllers.assign(203, Controller::Automatic);
            state.add_unit(t1_spawner_north);
            state.add_unit(t1_spawner_south);
            state.add_unit(t1_tower_north);
            state.add_unit(t1_tower_south);

            // Team 1 Heroes: Vanguard, Ranger, Warden, Sniper, Berserker
            state.add_unit(Unit::new_vanguard(6, 1, HexCoord::new(6, 0), 3));
            state.add_unit(Unit::new_ranger(7, 1, HexCoord::new(7, -1), 4));
            state.add_unit(Unit::new_warden(8, 1, HexCoord::new(8, 0), 2));
            state.add_unit(Unit::new_sniper(9, 1, HexCoord::new(7, 1), 4));
            state.add_unit(Unit::new_berserker(10, 1, HexCoord::new(8, 1), 3));
            for uid in 6..=10 {
                controllers.assign(uid, Controller::Ai);
            }
            hero_assignments.insert("vanguard_1".to_string(), 6);
            hero_assignments.insert("ranger_1".to_string(), 7);
            hero_assignments.insert("warden_1".to_string(), 8);
            hero_assignments.insert("sniper_1".to_string(), 9);
            hero_assignments.insert("berserker_1".to_string(), 10);

            // Central Objective Vault at (0, 0)
            let vault = Unit::new_vault(300, HexCoord::new(0, 0), config.phase7.objective_hp);
            controllers.assign(300, Controller::Automatic);
            state.add_unit(vault);

            // Neutral Camps: North (0, -4) and South (0, 4)
            let guardian_north = Unit::new_neutral_guardian(301, HexCoord::new(0, -4));
            let guardian_south = Unit::new_neutral_guardian(302, HexCoord::new(0, 4));
            controllers.assign(301, Controller::Automatic);
            controllers.assign(302, Controller::Automatic);
            state.add_unit(guardian_north);
            state.add_unit(guardian_south);

            state.neutral_camps.push(crate::neutral::NeutralCamp::new(
                "camp_north".to_string(),
                HexCoord::new(0, -4),
                301,
            ));
            state.neutral_camps.push(crate::neutral::NeutralCamp::new(
                "camp_south".to_string(),
                HexCoord::new(0, 4),
                302,
            ));

            state.next_unit_id = 600;
        } else if config.heroes_per_team == 3 {
            base_zones.insert(
                0,
                BaseZone::new(0, HexCoord::new(-5, 0), config.phase7.base_zone_radius),
            );
            base_zones.insert(
                1,
                BaseZone::new(1, HexCoord::new(5, 0), config.phase7.base_zone_radius),
            );

            // Phase 4 Terrain Topology (Radius 6/7 3v3)
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(0, 2)));
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(0, -2)));
            map.add_obstacle(crate::vision::Obstacle::smoke(HexCoord::new(2, 2)));
            map.add_obstacle(crate::vision::Obstacle::smoke(HexCoord::new(-2, -2)));
            map.add_obstacle(crate::vision::Obstacle::boulder(HexCoord::new(0, 1)));
            map.add_obstacle(crate::vision::Obstacle::boulder(HexCoord::new(0, -1)));

            state = GameState::new(map);

            let t0_core = Unit::new_core(500, 0, HexCoord::new(-5, 0), config.phase7.core_hp, config.phase7.core_vision_range);
            controllers.assign(500, Controller::Automatic);
            state.add_unit(t0_core);

            // Team 0 Base (Left)
            let t0_spawner = Unit::new_spawner(10, 0, HexCoord::new(-4, -1), config.spawn_interval);
            let t0_tower = Unit::new_tower(11, 0, HexCoord::new(-3, 0));
            controllers.assign(10, Controller::Automatic);
            controllers.assign(11, Controller::Automatic);
            state.add_unit(t0_spawner);
            state.add_unit(t0_tower);

            // Team 0 Heroes: Vanguard, Ranger, Warden
            state.add_unit(Unit::new_vanguard(1, 0, HexCoord::new(-4, 0), 3));
            state.add_unit(Unit::new_ranger(2, 0, HexCoord::new(-5, -1), 2));
            state.add_unit(Unit::new_warden(3, 0, HexCoord::new(-4, 1), 1));
            controllers.assign(1, Controller::Ai);
            controllers.assign(2, Controller::Ai);
            controllers.assign(3, Controller::Ai);
            hero_assignments.insert("vanguard_0".to_string(), 1);
            hero_assignments.insert("ranger_0".to_string(), 2);
            hero_assignments.insert("warden_0".to_string(), 3);

            let t1_core = Unit::new_core(501, 1, HexCoord::new(5, 0), config.phase7.core_hp, config.phase7.core_vision_range);
            controllers.assign(501, Controller::Automatic);
            state.add_unit(t1_core);

            // Team 1 Base (Right)
            let t1_spawner = Unit::new_spawner(20, 1, HexCoord::new(4, -1), config.spawn_interval);
            let t1_tower = Unit::new_tower(21, 1, HexCoord::new(3, 0));
            controllers.assign(20, Controller::Automatic);
            controllers.assign(21, Controller::Automatic);
            state.add_unit(t1_spawner);
            state.add_unit(t1_tower);

            // Team 1 Heroes: Vanguard, Ranger, Warden
            state.add_unit(Unit::new_vanguard(4, 1, HexCoord::new(4, 0), 3));
            state.add_unit(Unit::new_ranger(5, 1, HexCoord::new(5, -1), 2));
            state.add_unit(Unit::new_warden(6, 1, HexCoord::new(4, 1), 1));
            controllers.assign(4, Controller::Ai);
            controllers.assign(5, Controller::Ai);
            controllers.assign(6, Controller::Ai);
            hero_assignments.insert("vanguard_1".to_string(), 4);
            hero_assignments.insert("ranger_1".to_string(), 5);
            hero_assignments.insert("warden_1".to_string(), 6);

            // Vault at (0, 0)
            let vault = Unit::new_vault(300, HexCoord::new(0, 0), config.phase7.objective_hp);
            controllers.assign(300, Controller::Automatic);
            state.add_unit(vault);

            // Neutral Camps & Guardians
            let guardian_alpha = Unit::new_neutral_guardian(31, HexCoord::new(0, 3));
            let guardian_beta = Unit::new_neutral_guardian(32, HexCoord::new(0, -3));
            controllers.assign(31, Controller::Automatic);
            controllers.assign(32, Controller::Automatic);
            state.add_unit(guardian_alpha);
            state.add_unit(guardian_beta);

            state.neutral_camps.push(crate::neutral::NeutralCamp::new(
                "camp_alpha".to_string(),
                HexCoord::new(0, 3),
                31,
            ));
            state.neutral_camps.push(crate::neutral::NeutralCamp::new(
                "camp_beta".to_string(),
                HexCoord::new(0, -3),
                32,
            ));

            state.next_unit_id = 600;
        } else {
            base_zones.insert(
                0,
                BaseZone::new(0, HexCoord::new(-5, 0), config.phase7.base_zone_radius),
            );
            base_zones.insert(
                1,
                BaseZone::new(1, HexCoord::new(5, 0), config.phase7.base_zone_radius),
            );

            // 1v1 Configuration
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(0, 2)));
            map.add_obstacle(crate::vision::Obstacle::wall(HexCoord::new(0, -2)));
            state = GameState::new(map);

            let t0_core = Unit::new_core(500, 0, HexCoord::new(-5, 0), config.phase7.core_hp, config.phase7.core_vision_range);
            controllers.assign(500, Controller::Automatic);
            state.add_unit(t0_core);

            let t0_spawner = Unit::new_spawner(10, 0, HexCoord::new(-4, -1), config.spawn_interval);
            let t0_tower = Unit::new_tower(11, 0, HexCoord::new(-3, 0));
            controllers.assign(10, Controller::Automatic);
            controllers.assign(11, Controller::Automatic);
            state.add_unit(t0_spawner);
            state.add_unit(t0_tower);

            state.add_unit(Unit::new_vanguard(1, 0, HexCoord::new(-4, 0), 3));
            controllers.assign(1, Controller::Ai);
            hero_assignments.insert("vanguard_0".to_string(), 1);

            let t1_core = Unit::new_core(501, 1, HexCoord::new(5, 0), config.phase7.core_hp, config.phase7.core_vision_range);
            controllers.assign(501, Controller::Automatic);
            state.add_unit(t1_core);

            let t1_spawner = Unit::new_spawner(20, 1, HexCoord::new(4, -1), config.spawn_interval);
            let t1_tower = Unit::new_tower(21, 1, HexCoord::new(3, 0));
            controllers.assign(20, Controller::Automatic);
            controllers.assign(21, Controller::Automatic);
            state.add_unit(t1_spawner);
            state.add_unit(t1_tower);

            state.add_unit(Unit::new_vanguard(4, 1, HexCoord::new(4, 0), 3));
            controllers.assign(4, Controller::Ai);
            hero_assignments.insert("vanguard_1".to_string(), 4);

            let vault = Unit::new_vault(300, HexCoord::new(0, 0), config.phase7.objective_hp);
            controllers.assign(300, Controller::Automatic);
            state.add_unit(vault);

            state.next_unit_id = 600;
        }

        // Initialize vision
        state.update_fog();

        for unit in state.units.values_mut() {
            if unit.is_hero() {
                unit.gold = config.economy.starting_gold;
            }
        }

        let mut unit_registry = HashMap::new();
        for unit in state.units.values() {
            unit_registry.insert(unit.id, (unit.team, unit.kind, unit.pos));
        }

        Self {
            match_id,
            state,
            controllers,
            staged_orders: HashMap::new(),
            submitted_teams: HashSet::new(),
            config,
            unit_registry,
            hero_assignments,
            base_zones,
            is_match_over: false,
            winning_team: None,
        }
    }

    /// Assign player to all living heroes on a team.
    pub fn assign_team_player(&mut self, team: TeamId, player_id: PlayerId) {
        for unit in self.state.units.values() {
            if unit.team == team && unit.kind == UnitKind::Hero {
                self.controllers
                    .assign(unit.id, Controller::Player(player_id.clone()));
            }
        }
    }

    /// Assign player to a specific hero avatar.
    pub fn assign_hero_player(&mut self, unit_id: UnitId, player_id: PlayerId) {
        self.controllers
            .assign(unit_id, Controller::Player(player_id));
    }

    /// Submit turn orders from a player.
    pub fn submit_player_orders(
        &mut self,
        player_id: &PlayerId,
        team: TeamId,
        round: u32,
        orders: Vec<OrderDto>,
    ) -> Result<(), OrderSubmissionError> {
        if self.state.phase != Phase::Planning {
            return Err(OrderSubmissionError {
                code: ProtocolErrorCode::InvalidPhase,
                unit_id: None,
                reason: "Orders can only be submitted during Planning phase".into(),
            });
        }
        if self.state.round != round {
            return Err(OrderSubmissionError {
                code: ProtocolErrorCode::StaleRound,
                unit_id: None,
                reason: format!(
                    "Stale round: match is at round {}, received orders for round {}",
                    self.state.round, round
                ),
            });
        }

        if self.config.players_per_team == 5 && orders.len() != 1 {
            return Err(OrderSubmissionError {
                code: ProtocolErrorCode::InvalidOrderCount,
                unit_id: None,
                reason: "Exactly one order must be submitted per player in single-hero mode".into(),
            });
        }

        let mut validated_orders = HashMap::new();

        for dto in orders {
            let unit = match self.state.get_unit(dto.unit_id) {
                Some(u) => u,
                None => {
                    return Err(OrderSubmissionError {
                        code: ProtocolErrorCode::UnitNotOwned,
                        unit_id: Some(dto.unit_id),
                        reason: format!("Unit #{} does not exist", dto.unit_id),
                    });
                }
            };

            if !unit.is_alive() || unit.is_dead_awaiting_respawn() {
                return Err(OrderSubmissionError {
                    code: ProtocolErrorCode::CannotOrderDeadHero,
                    unit_id: Some(dto.unit_id),
                    reason: format!("Unit #{} is dead awaiting respawn", dto.unit_id),
                });
            }
            if unit.team != team {
                return Err(OrderSubmissionError {
                    code: ProtocolErrorCode::NotYourUnit,
                    unit_id: Some(dto.unit_id),
                    reason: format!("Unit #{} does not belong to team {}", dto.unit_id, team),
                });
            }

            // Verify controller ownership
            if !self.controllers.is_controlled_by_player(dto.unit_id, player_id) {
                return Err(OrderSubmissionError {
                    code: ProtocolErrorCode::NotYourUnit,
                    unit_id: Some(dto.unit_id),
                    reason: format!(
                        "Player {} is not authorized to order unit #{}",
                        player_id, dto.unit_id
                    ),
                });
            }

            let move_target = dto.move_target.map(|h| HexCoord::new(h.q, h.r));
            if move_target.is_some() && unit.is_stationary() {
                return Err(OrderSubmissionError {
                    code: ProtocolErrorCode::UnauthorizedAction,
                    unit_id: Some(dto.unit_id),
                    reason: format!("Unit #{} is stationary and cannot move", dto.unit_id),
                });
            }
            let action = match dto.action {
                ActionDto::Wait => Action::Wait,
                ActionDto::Attack { target_id } => {
                    if unit.attack_damage == 0 {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::UnauthorizedAction,
                            unit_id: Some(dto.unit_id),
                            reason: format!(
                                "Unit #{} has 0 attack damage and cannot attack",
                                dto.unit_id
                            ),
                        });
                    }
                    let target = match self.state.get_unit(target_id) {
                        Some(t) => t,
                        None => {
                            return Err(OrderSubmissionError {
                                code: ProtocolErrorCode::InvalidTarget,
                                unit_id: Some(dto.unit_id),
                                reason: format!("Attack target #{} does not exist", target_id),
                            });
                        }
                    };
                    if target.team == team {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: format!(
                                "Invalid attack target #{}: friendly unit",
                                target_id
                            ),
                        });
                    }
                    if !target.is_alive() || target.is_dead_awaiting_respawn() {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::TargetUntargetable,
                            unit_id: Some(dto.unit_id),
                            reason: format!(
                                "Invalid attack target #{}: dead or untargetable",
                                target_id
                            ),
                        });
                    }
                    Action::Attack { target_id }
                }
                ActionDto::Cast {
                    spell_id,
                    target: target_dto,
                } => {
                    let spell = match crate::ability::SpellCatalog::get(&spell_id) {
                        Some(s) => s,
                        None => {
                            return Err(OrderSubmissionError {
                                code: ProtocolErrorCode::InvalidMessage,
                                unit_id: Some(dto.unit_id),
                                reason: format!("Unknown spell '{}'", spell_id),
                            });
                        }
                    };
                    if unit.ap < spell.ap_cost {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InsufficientResources,
                            unit_id: Some(dto.unit_id),
                            reason: format!(
                                "Insufficient AP for spell '{}' (costs {}, has {})",
                                spell_id, spell.ap_cost, unit.ap
                            ),
                        });
                    }
                    if unit.energy < spell.energy_cost {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InsufficientResources,
                            unit_id: Some(dto.unit_id),
                            reason: format!(
                                "Insufficient Energy for spell '{}' (costs {}, has {})",
                                spell_id, spell.energy_cost, unit.energy
                            ),
                        });
                    }
                    if !unit.is_spell_ready(&spell_id) {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::CooldownActive,
                            unit_id: Some(dto.unit_id),
                            reason: format!("Spell '{}' is on cooldown", spell_id),
                        });
                    }
                    let spell_target = match target_dto {
                        hexabellum_protocol::SpellTargetDto::None => {
                            if spell.targeting != crate::ability::TargetingMode::SelfOnly {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Spell requires a target".into(),
                                });
                            }
                            crate::ability::SpellTarget::None
                        }
                        hexabellum_protocol::SpellTargetDto::Hex { hex } => {
                            crate::ability::SpellTarget::Hex(HexCoord::new(hex.q, hex.r))
                        }
                        hexabellum_protocol::SpellTargetDto::Unit { unit_id: tid } => {
                            let t = match self.state.get_unit(tid) {
                                Some(u) => {
                                    if !u.is_alive() || u.is_dead_awaiting_respawn() {
                                        return Err(OrderSubmissionError {
                                            code: ProtocolErrorCode::TargetUntargetable,
                                            unit_id: Some(dto.unit_id),
                                            reason: format!(
                                                "Target unit #{} is dead or untargetable",
                                                tid
                                            ),
                                        });
                                    }
                                    u
                                }
                                None => {
                                    return Err(OrderSubmissionError {
                                        code: ProtocolErrorCode::InvalidTarget,
                                        unit_id: Some(dto.unit_id),
                                        reason: format!("Target unit #{} not found", tid),
                                    });
                                }
                            };
                            if spell
                                .effects
                                .iter()
                                .any(|e| e.kind == crate::ability::EffectKind::Heal)
                                && t.is_structure()
                            {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Healing spells cannot target structures".into(),
                                });
                            }
                            if spell.targeting == crate::ability::TargetingMode::EnemyUnit
                                && t.team == team
                            {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Spell requires an enemy target".into(),
                                });
                            }
                            if spell.targeting == crate::ability::TargetingMode::AllyUnit
                                && t.team != team
                            {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Spell requires an allied target".into(),
                                });
                            }
                            let effective_caster_pos = move_target.unwrap_or(unit.pos);
                            let dist = effective_caster_pos.distance(&t.pos);
                            if dist < spell.min_range || dist > spell.range {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: format!(
                                        "Target out of range (dist {}, range {})",
                                        dist, spell.range
                                    ),
                                });
                            }
                            // Anti-maphack: enemy must be visible to player team
                            if t.team != team && !self.state.fog.is_visible(team, &t.pos) {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::InvalidTarget,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Target is hidden in fog of war".into(),
                                });
                            }
                            if spell.requires_line_of_sight
                                && !crate::vision::has_line_of_sight(
                                    &self.state.map.vision_blockers(),
                                    effective_caster_pos,
                                    t.pos,
                                )
                            {
                                return Err(OrderSubmissionError {
                                    code: ProtocolErrorCode::MissingLineOfSight,
                                    unit_id: Some(dto.unit_id),
                                    reason: "Line of sight is obstructed by terrain".into(),
                                });
                            }
                            crate::ability::SpellTarget::Unit(tid)
                        }
                    };
                    Action::Cast {
                        spell_id,
                        target: spell_target,
                    }
                }
                ActionDto::Repair { target_id } => {
                    if !unit.is_hero() {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidMessage,
                            unit_id: Some(dto.unit_id),
                            reason: "Only Heroes can repair structures".into(),
                        });
                    }
                    if unit.ap < crate::repair::REPAIR_AP_COST {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InsufficientResources,
                            unit_id: Some(dto.unit_id),
                            reason: "Insufficient AP for repair".into(),
                        });
                    }
                    let target = match self.state.get_unit(target_id) {
                        Some(t) if t.is_alive() => t,
                        _ => {
                            return Err(OrderSubmissionError {
                                code: ProtocolErrorCode::InvalidTarget,
                                unit_id: Some(dto.unit_id),
                                reason: format!("Repair target #{} is dead or not found", target_id),
                            });
                        }
                    };
                    if !target.is_structure() {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: "Target is not a structure".into(),
                        });
                    }
                    if !target.kind.is_repairable() {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::CoreCannotBeRepaired,
                            unit_id: Some(dto.unit_id),
                            reason: "Cores and neutral objectives cannot be repaired".into(),
                        });
                    }
                    if target.team != team {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: "Cannot repair enemy structures".into(),
                        });
                    }
                    if target.hp >= target.max_hp {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: "Structure is already at full health".into(),
                        });
                    }
                    let effective_hero_pos = move_target.unwrap_or(unit.pos);
                    if effective_hero_pos.distance(&target.pos) > crate::repair::REPAIR_RANGE {
                        return Err(OrderSubmissionError {
                            code: ProtocolErrorCode::InvalidTarget,
                            unit_id: Some(dto.unit_id),
                            reason: "Hero must be adjacent to repair structure".into(),
                        });
                    }
                    Action::Repair { target_id }
                }
            };

            validated_orders.insert(
                dto.unit_id,
                UnitOrder {
                    unit_id: dto.unit_id,
                    move_target,
                    action,
                },
            );
        }

        let team_orders = self.staged_orders.entry(team).or_default();
        for (uid, order) in validated_orders {
            team_orders.insert(uid, order);
        }
        self.submitted_teams.insert(team);
        Ok(())
    }

    /// Check if all human-controlled teams have submitted orders.
    pub fn all_human_teams_submitted(&self, active_human_teams: &[TeamId]) -> bool {
        active_human_teams
            .iter()
            .all(|t| self.submitted_teams.contains(t))
    }

    /// Fill missing orders for unmanaged or timed-out heroes with tactical AI.
    pub fn fill_missing_orders_with_ai(&mut self) {
        for team in [0, 1] {
            let team_orders = self.staged_orders.entry(team).or_default();
            for unit in self.state.team_units(team) {
                if unit.kind == UnitKind::Hero {
                    if !team_orders.contains_key(&unit.id) {
                        let fallback = GameAI::generate_fallback_order(&self.state, unit.id);
                        team_orders.insert(unit.id, fallback);
                    }
                }
            }
        }
    }

    /// Authoritatively resolve the current round and advance the match state.
    pub fn resolve_round(&mut self) -> Vec<GameEvent> {
        self.fill_missing_orders_with_ai();

        let mut combined_orders = TurnOrders::new();
        for team in [0, 1] {
            if let Some(mut orders) = self.staged_orders.remove(&team) {
                let mut unit_ids: Vec<UnitId> = orders.keys().copied().collect();
                unit_ids.sort_unstable();
                for id in unit_ids {
                    if let Some(order) = orders.remove(&id) {
                        combined_orders.set_order(id, order);
                    }
                }
            }
        }
        self.staged_orders.clear();

        self.submitted_teams.clear();
        self.state.phase = Phase::Resolution;

        let events = TurnProcessor::resolve_turn(&mut self.state, combined_orders);

        // Update unit_registry with events from this turn
        for event in &events {
            match event {
                GameEvent::UnitSpawned {
                    unit_id,
                    unit_kind,
                    team,
                    pos,
                    ..
                } => {
                    self.unit_registry
                        .insert(*unit_id, (*team, *unit_kind, *pos));
                    self.controllers.assign(*unit_id, Controller::Automatic);
                }
                GameEvent::UnitMoved { unit_id, to, .. } => {
                    if let Some(meta) = self.unit_registry.get_mut(unit_id) {
                        meta.2 = *to;
                    }
                }
                _ => {}
            }
        }

        if self.state.winner.is_none() {
            self.state.phase = Phase::Planning;
        } else {
            self.state.phase = Phase::MatchEnd;
            self.is_match_over = true;
            self.winning_team = self.state.winner;
        }

        events
    }

    /// Filter simulation events so coordinates and actions inside fog are concealed.
    pub fn sanitize_events_for_team(
        &self,
        team: TeamId,
        events: &[GameEvent],
    ) -> Vec<SanitizedGameEvent> {
        let visible_hexes = self.state.fog.visible_hexes(team);
        let mut sanitized = Vec::new();

        let get_unit_meta = |id: &UnitId| {
            if let Some(m) = self.unit_registry.get(id) {
                Some((m.0, m.1, m.2))
            } else if let Some(u) = self.state.get_unit(*id) {
                Some((u.team, u.kind, u.pos))
            } else {
                None
            }
        };

        for event in events {
            match event {
                GameEvent::RoundStarted { round } => {
                    sanitized.push(SanitizedGameEvent::RoundStarted { round: *round });
                }
                GameEvent::UnitSpawned {
                    unit_id,
                    unit_kind,
                    team: u_team,
                    pos,
                    spawner_id,
                } => {
                    if *u_team == team || visible_hexes.contains(pos) {
                        sanitized.push(SanitizedGameEvent::UnitSpawned {
                            unit_id: *unit_id,
                            unit_kind: unit_kind.to_string(),
                            team: *u_team,
                            pos: HexDto::new(pos.q, pos.r),
                            spawner_id: *spawner_id,
                        });
                    }
                }
                GameEvent::UnitMoved {
                    unit_id,
                    from,
                    to,
                    path,
                    ap_spent,
                } => {
                    let is_own_unit = get_unit_meta(unit_id).map_or(false, |m| m.0 == team);
                    let from_vis = visible_hexes.contains(from);
                    let to_vis = visible_hexes.contains(to);

                    if is_own_unit || from_vis || to_vis {
                        let path_dto = path.iter().map(|h| HexDto::new(h.q, h.r)).collect();
                        sanitized.push(SanitizedGameEvent::UnitMoved {
                            unit_id: *unit_id,
                            from: HexDto::new(from.q, from.r),
                            to: HexDto::new(to.q, to.r),
                            path: path_dto,
                            ap_spent: *ap_spent,
                        });
                    }
                }
                GameEvent::UnitAttacked {
                    attacker_id,
                    target_id,
                    damage,
                    target_hp_remaining,
                } => {
                    let att_meta = get_unit_meta(attacker_id);
                    let tgt_meta = get_unit_meta(target_id);

                    let att_vis =
                        att_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let tgt_vis =
                        tgt_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));

                    if att_vis || tgt_vis {
                        sanitized.push(SanitizedGameEvent::UnitAttacked {
                            attacker_id: *attacker_id,
                            target_id: *target_id,
                            damage: *damage,
                            target_hp_remaining: *target_hp_remaining,
                        });
                    }
                }
                GameEvent::TowerAttacked {
                    tower_id,
                    target_id,
                    damage,
                    target_hp_remaining,
                } => {
                    let tower_meta = get_unit_meta(tower_id);
                    let tgt_meta = get_unit_meta(target_id);

                    let tower_vis =
                        tower_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let tgt_vis =
                        tgt_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));

                    if tower_vis || tgt_vis {
                        sanitized.push(SanitizedGameEvent::TowerAttacked {
                            tower_id: *tower_id,
                            target_id: *target_id,
                            damage: *damage,
                            target_hp_remaining: *target_hp_remaining,
                        });
                    }
                }
                GameEvent::UnitDied {
                    unit_id,
                    unit_kind,
                    killed_by,
                } => {
                    let meta = get_unit_meta(unit_id);
                    let is_own = meta.map_or(false, |m| m.0 == team);
                    let is_vis = meta.map_or(false, |m| visible_hexes.contains(&m.2));

                    if is_own || is_vis {
                        sanitized.push(SanitizedGameEvent::UnitDied {
                            unit_id: *unit_id,
                            unit_kind: unit_kind.to_string(),
                            killed_by: *killed_by,
                        });
                    }
                }
                GameEvent::UnitWaited { unit_id } => {
                    let meta = get_unit_meta(unit_id);
                    let is_own = meta.map_or(false, |m| m.0 == team);
                    let is_vis = meta.map_or(false, |m| visible_hexes.contains(&m.2));

                    if is_own || is_vis {
                        sanitized.push(SanitizedGameEvent::UnitWaited { unit_id: *unit_id });
                    }
                }
                GameEvent::RoundEnded { round } => {
                    sanitized.push(SanitizedGameEvent::RoundEnded { round: *round });
                }
                GameEvent::MatchEnded { winner, reason } => {
                    sanitized.push(SanitizedGameEvent::MatchEnded {
                        winner: *winner,
                        reason: *reason,
                    });
                }
                GameEvent::SpellCast {
                    caster_id,
                    spell_id,
                    target,
                } => {
                    let caster_meta = get_unit_meta(caster_id);
                    let caster_vis = caster_meta
                        .map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let target_vis = match target {
                        crate::ability::SpellTarget::Unit(tid) => {
                            let tm = get_unit_meta(tid);
                            tm.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2))
                        }
                        crate::ability::SpellTarget::Hex(h) => visible_hexes.contains(h),
                        crate::ability::SpellTarget::None => false,
                    };

                    if caster_vis || target_vis {
                        let target_dto = match target {
                            crate::ability::SpellTarget::None => {
                                hexabellum_protocol::SpellTargetDto::None
                            }
                            crate::ability::SpellTarget::Hex(h) => {
                                hexabellum_protocol::SpellTargetDto::Hex {
                                    hex: HexDto::new(h.q, h.r),
                                }
                            }
                            crate::ability::SpellTarget::Unit(tid) => {
                                hexabellum_protocol::SpellTargetDto::Unit { unit_id: *tid }
                            }
                        };
                        sanitized.push(SanitizedGameEvent::SpellCast {
                            caster_id: *caster_id,
                            spell_id: spell_id.clone(),
                            target: target_dto,
                        });
                    }
                }
                GameEvent::HealApplied {
                    caster_id,
                    target_id,
                    amount,
                    target_hp_remaining,
                } => {
                    let c_meta = get_unit_meta(caster_id);
                    let t_meta = get_unit_meta(target_id);
                    let c_vis =
                        c_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let t_vis =
                        t_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if c_vis || t_vis {
                        sanitized.push(SanitizedGameEvent::HealApplied {
                            caster_id: *caster_id,
                            target_id: *target_id,
                            amount: *amount,
                            target_hp_remaining: *target_hp_remaining,
                        });
                    }
                }
                GameEvent::StructureRepaired {
                    repairer_id,
                    target_id,
                    amount,
                    target_hp_remaining,
                } => {
                    let r_meta = get_unit_meta(repairer_id);
                    let t_meta = get_unit_meta(target_id);
                    let r_vis =
                        r_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    let t_vis =
                        t_meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if r_vis || t_vis {
                        sanitized.push(SanitizedGameEvent::StructureRepaired {
                            repairer_id: *repairer_id,
                            target_id: *target_id,
                            amount: *amount,
                            target_hp_remaining: *target_hp_remaining,
                        });
                    }
                }
                GameEvent::StatusApplied {
                    unit_id,
                    status_id,
                    duration_rounds,
                } => {
                    let meta = get_unit_meta(unit_id);
                    let is_vis = meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if is_vis {
                        sanitized.push(SanitizedGameEvent::StatusApplied {
                            unit_id: *unit_id,
                            status_id: status_id.clone(),
                            duration_rounds: *duration_rounds,
                        });
                    }
                }
                GameEvent::StatusExpired { unit_id, status_id } => {
                    let meta = get_unit_meta(unit_id);
                    let is_vis = meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if is_vis {
                        sanitized.push(SanitizedGameEvent::StatusExpired {
                            unit_id: *unit_id,
                            status_id: status_id.clone(),
                        });
                    }
                }
                GameEvent::NeutralCampCleared { camp_id, killer_team } => {
                    sanitized.push(SanitizedGameEvent::NeutralCampCleared {
                        camp_id: camp_id.clone(),
                        killer_team: *killer_team,
                    });
                }
                GameEvent::TeamBuffApplied {
                    team: b_team,
                    buff_id,
                    duration_rounds,
                } => {
                    sanitized.push(SanitizedGameEvent::TeamBuffApplied {
                        team: *b_team,
                        buff_id: buff_id.clone(),
                        duration_rounds: *duration_rounds,
                    });
                }
                GameEvent::RewardGranted {
                    unit_id,
                    gold,
                    xp,
                    reason,
                } => {
                    let is_ally = get_unit_meta(unit_id).map_or(false, |m| m.0 == team);
                    if is_ally {
                        sanitized.push(SanitizedGameEvent::RewardGranted {
                            unit_id: *unit_id,
                            gold: *gold,
                            xp: *xp,
                            reason: *reason,
                        });
                    }
                }
                GameEvent::LevelUp {
                    unit_id,
                    new_level,
                    new_max_hp,
                    new_attack_damage,
                    new_max_energy,
                } => {
                    let meta = get_unit_meta(unit_id);
                    let is_vis = meta.map_or(false, |m| m.0 == team || visible_hexes.contains(&m.2));
                    if is_vis {
                        sanitized.push(SanitizedGameEvent::LevelUp {
                            unit_id: *unit_id,
                            new_level: *new_level,
                            new_max_hp: *new_max_hp,
                            new_attack_damage: *new_attack_damage,
                            new_max_energy: *new_max_energy,
                        });
                    }
                }
                GameEvent::HeroDied {
                    unit_id,
                    killed_by,
                    respawn_rounds,
                } => {
                    sanitized.push(SanitizedGameEvent::HeroDied {
                        unit_id: *unit_id,
                        killed_by: *killed_by,
                        respawn_rounds: *respawn_rounds,
                    });
                }
                GameEvent::HeroRespawned {
                    unit_id,
                    team: r_team,
                    pos: r_pos,
                } => {
                    sanitized.push(SanitizedGameEvent::HeroRespawned {
                        unit_id: *unit_id,
                        team: *r_team,
                        pos: HexDto::new(r_pos.q, r_pos.r),
                    });
                }
                GameEvent::BaseRegenerationApplied {
                    unit_id,
                    team: r_team,
                    amount,
                    new_hp,
                } => {
                    if *r_team == team {
                        sanitized.push(SanitizedGameEvent::BaseRegenerationApplied {
                            unit_id: *unit_id,
                            team: *r_team,
                            amount: *amount,
                            new_hp: *new_hp,
                        });
                    }
                }
                GameEvent::ObjectiveDestroyed {
                    objective_id,
                    destroyer_team,
                    last_attacker_id,
                    gold_awarded_per_hero,
                    xp_awarded_per_hero,
                    affected_heroes,
                } => {
                    sanitized.push(SanitizedGameEvent::ObjectiveDestroyed {
                        objective_id: *objective_id,
                        destroyer_team: *destroyer_team,
                        last_attacker_id: *last_attacker_id,
                        gold_awarded_per_hero: *gold_awarded_per_hero,
                        xp_awarded_per_hero: *xp_awarded_per_hero,
                        affected_heroes: affected_heroes.clone(),
                    });
                }
                GameEvent::CoreDestroyed {
                    core_id,
                    team: c_team,
                    destroyed_by,
                } => {
                    sanitized.push(SanitizedGameEvent::CoreDestroyed {
                        core_id: *core_id,
                        team: *c_team,
                        destroyed_by: *destroyed_by,
                    });
                }
                _ => {}
            }
        }

        sanitized
    }

    /// Build full roster entries for allied heroes and LOS-censored enemy heroes
    pub fn build_roster_entries(
        &self,
        player_team: TeamId,
        team_vision: &HashSet<HexCoord>,
    ) -> Vec<RosterEntryDto> {
        let mut roster = Vec::new();
        let mut hero_units: Vec<&Unit> = self
            .state
            .units
            .values()
            .filter(|u| u.is_hero())
            .collect();
        hero_units.sort_by_key(|u| (u.team, u.id));

        for hero in hero_units {
            let is_ally = hero.team == player_team;
            let is_visible = is_ally || team_vision.contains(&hero.pos);

            let (player_id, is_ai, display_name) = match self.controllers.get(hero.id) {
                Some(Controller::Player(pid)) => (
                    Some(pid.clone()),
                    false,
                    pid.clone(),
                ),
                Some(Controller::Ai) | None => (
                    None,
                    true,
                    format!("Bot ({})", hero.hero_id.as_deref().unwrap_or("Hero")),
                ),
                Some(Controller::Automatic) => (
                    None,
                    true,
                    "AI".to_string(),
                ),
            };

            let hero_def_id = hero
                .hero_id
                .clone()
                .unwrap_or_else(|| "hero".to_string());

            let orders_submitted = self
                .staged_orders
                .get(&hero.team)
                .map(|orders| orders.contains_key(&hero.id))
                .unwrap_or(false);

            roster.push(RosterEntryDto {
                player_id,
                display_name,
                hero_def_id,
                unit_id: hero.id,
                team: hero.team,
                connected: true,
                is_ai,
                orders_submitted,
                alive: hero.is_alive(),
                hp: if is_visible { Some(hero.hp) } else { None },
                max_hp: hero.max_hp,
                level: if is_visible { hero.level } else { 1 },
                items: if is_visible { hero.items.clone() } else { Vec::new() },
                life_state: Some(match hero.life_state {
                    LifeState::Alive => LifeStateDto::Alive,
                    LifeState::DeadAwaitingRespawn { .. } => LifeStateDto::DeadAwaitingRespawn,
                    LifeState::PermanentlyRemoved => LifeStateDto::PermanentlyRemoved,
                }),
                respawn_rounds: hero.respawn_rounds,
            });
        }

        roster
    }

    /// Generate team-sanitized snapshot strictly withholding concealed enemy positions.
    pub fn snapshot_for_team(&self, team: TeamId, deadline_unix_ms: Option<u64>) -> SnapshotDto {
        self.snapshot_for_player(team, None, deadline_unix_ms)
    }

    /// Generate snapshot customized for an individual player and their controlled units.
    pub fn snapshot_for_player(
        &self,
        team: TeamId,
        player_id: Option<&PlayerId>,
        deadline_unix_ms: Option<u64>,
    ) -> SnapshotDto {
        let visible_hexes = self.state.fog.visible_hexes(team);

        let mut sorted_unit_ids: Vec<UnitId> = self.state.units.keys().copied().collect();
        sorted_unit_ids.sort_unstable();

        let visible_units: Vec<UnitDto> = sorted_unit_ids
            .iter()
            .filter_map(|id| self.state.get_unit(*id))
            .filter(|unit| {
                if unit.is_dead_awaiting_respawn() {
                    unit.team == team
                } else if unit.is_alive() {
                    unit.team == team || visible_hexes.contains(&unit.pos)
                } else {
                    false
                }
            })
            .map(|u| {
                let is_ally = u.team == team;
                let is_sighted = is_ally || visible_hexes.contains(&u.pos);
                UnitDto {
                    id: u.id,
                    kind: u.kind.to_string(),
                    team: u.team,
                    pos: HexDto::new(u.pos.q, u.pos.r),
                    hp: u.hp,
                    max_hp: u.max_hp,
                    ap: u.ap,
                    max_ap: u.max_ap,
                    energy: u.energy,
                    max_energy: u.max_energy,
                    initiative: u.initiative,
                    attack_damage: u.attack_damage,
                    attack_range: u.attack_range,
                    vision_range: u.vision_range,
                    is_stationary: u.kind.is_stationary(),
                    cooldowns: u.cooldowns.clone(),
                    statuses: u
                        .statuses
                        .iter()
                        .map(|s| hexabellum_protocol::StatusDto {
                            id: s.def_id.clone(),
                            remaining_rounds: s.remaining_rounds,
                            attack_damage_mod: s
                                .modifiers
                                .iter()
                                .filter(|m| m.stat == crate::status::StatKind::AttackDamage)
                                .map(|m| m.value)
                                .sum(),
                        })
                        .collect(),
                    lane_id: u.lane_id.clone(),
                    hero_id: u.hero_id.clone(),
                    gold: if is_ally { Some(u.gold) } else { None },
                    xp: if is_ally { Some(u.xp) } else { None },
                    level: if is_sighted { u.level } else { 1 },
                    items: if is_sighted { u.items.clone() } else { Vec::new() },
                    life_state: match u.life_state {
                        LifeState::Alive => LifeStateDto::Alive,
                        LifeState::DeadAwaitingRespawn { .. } => LifeStateDto::DeadAwaitingRespawn,
                        LifeState::PermanentlyRemoved => LifeStateDto::PermanentlyRemoved,
                    },
                    respawn_rounds: u.respawn_rounds,
                    death_pos: u.death_pos.map(|d| HexDto::new(d.q, d.r)),
                }
            })
            .collect();

        let controlled_units: Vec<UnitId> = if let Some(pid) = player_id {
            self.controllers.get_units_for_player(pid)
        } else {
            sorted_unit_ids
                .iter()
                .filter_map(|id| self.state.get_unit(*id))
                .filter(|u| u.team == team && u.kind == UnitKind::Hero && u.is_alive())
                .map(|u| u.id)
                .collect()
        };

        let mut walkable: Vec<HexDto> = self
            .state
            .map
            .all_hexes()
            .iter()
            .filter(|h| !self.state.map.movement_blockers().contains(h))
            .map(|h| HexDto::new(h.q, h.r))
            .collect();
        walkable.sort_by_key(|h| (h.q, h.r));

        let mut obstacles: Vec<HexDto> = self
            .state
            .map
            .movement_blockers()
            .into_iter()
            .map(|h| HexDto::new(h.q, h.r))
            .collect();
        obstacles.sort_by_key(|h| (h.q, h.r));

        let mut visible_hexes_dto: Vec<HexDto> = visible_hexes
            .iter()
            .map(|h| HexDto::new(h.q, h.r))
            .collect();
        visible_hexes_dto.sort_by_key(|h| (h.q, h.r));

        let neutral_camps: Vec<hexabellum_protocol::NeutralCampDto> = self
            .state
            .neutral_camps
            .iter()
            .map(|c| {
                let is_visible = visible_hexes.contains(&c.camp_pos);
                let is_alive = self
                    .state
                    .get_unit(c.guardian_id)
                    .map(|u| u.is_alive())
                    .unwrap_or(false);
                hexabellum_protocol::NeutralCampDto {
                    id: c.id.clone(),
                    pos: HexDto::new(c.camp_pos.q, c.camp_pos.r),
                    is_alive: if is_visible { is_alive } else { true },
                    guardian_unit_id: if is_visible && is_alive {
                        Some(c.guardian_id)
                    } else {
                        None
                    },
                }
            })
            .collect();

        let roster = self.build_roster_entries(team, visible_hexes);

        let match_phase = Some(match self.state.phase {
            Phase::Planning => MatchPhaseDto::Planning,
            Phase::Resolution => MatchPhaseDto::Resolution,
            Phase::MatchEnd => MatchPhaseDto::MatchEnd,
        });

        let controlled_hero_economy = {
            let primary_unit_id = if let Some(pid) = player_id {
                self.controllers.get_units_for_player(pid).into_iter().next()
            } else {
                controlled_units.first().copied()
            };
            primary_unit_id.and_then(|uid| self.state.get_unit(uid)).and_then(|u| {
                if u.is_hero() {
                    Some(hexabellum_protocol::HeroEconomyDto {
                        unit_id: u.id,
                        hero_def_id: u.hero_id.clone().unwrap_or_else(|| "hero".to_string()),
                        gold: u.gold,
                        xp: u.xp,
                        level: u.level,
                        items: u.items.clone(),
                    })
                } else {
                    None
                }
            })
        };

        let allied_hero_economy: Vec<hexabellum_protocol::HeroEconomyDto> = sorted_unit_ids
            .iter()
            .filter_map(|id| self.state.get_unit(*id))
            .filter(|u| u.team == team && u.is_hero())
            .map(|u| hexabellum_protocol::HeroEconomyDto {
                unit_id: u.id,
                hero_def_id: u.hero_id.clone().unwrap_or_else(|| "hero".to_string()),
                gold: u.gold,
                xp: u.xp,
                level: u.level,
                items: u.items.clone(),
            })
            .collect();

        let shop_catalog: Vec<hexabellum_protocol::ItemDto> = crate::items::get_canonical_item_catalog()
            .into_iter()
            .map(|item| hexabellum_protocol::ItemDto {
                id: item.id,
                name: item.name,
                cost: item.cost,
                description: item.description,
                icon: item.icon,
                modifiers: item
                    .modifiers
                    .into_iter()
                    .map(|m| hexabellum_protocol::StatModifierDto {
                        stat: match m.stat {
                            crate::items::StatKind::AttackDamage => hexabellum_protocol::StatKind::AttackDamage,
                            crate::items::StatKind::MaxHealth => hexabellum_protocol::StatKind::MaxHealth,
                            crate::items::StatKind::VisionRange => hexabellum_protocol::StatKind::VisionRange,
                            crate::items::StatKind::EnergyRegen => hexabellum_protocol::StatKind::EnergyRegen,
                        },
                        value: m.value,
                    })
                    .collect(),
            })
            .collect();

        let (can_shop, shop_disabled_reason) = if self.state.phase != Phase::Planning {
            (false, Some(ShopDisabledReasonDto::NotPlanningPhase))
        } else {
            let primary_unit_id = if let Some(pid) = player_id {
                self.controllers.get_units_for_player(pid).into_iter().next()
            } else {
                controlled_units.first().copied()
            };
            if let Some(uid) = primary_unit_id {
                if let Some(u) = self.state.get_unit(uid) {
                    if !u.is_alive() {
                        (false, Some(ShopDisabledReasonDto::HeroDead))
                    } else if !self.base_zones.get(&u.team).map_or(false, |bz| bz.contains(u.pos)) {
                        (false, Some(ShopDisabledReasonDto::OutsideBaseZone))
                    } else if u.items.len() >= self.config.economy.max_item_slots {
                        (false, Some(ShopDisabledReasonDto::InventoryFull))
                    } else {
                        (true, None)
                    }
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        };

        let mut core_hp: HashMap<TeamId, (u32, u32)> = HashMap::new();
        for &t in &[0, 1] {
            if let Some(cid) = self.get_core_id(t) {
                if let Some(core) = self.state.get_unit(cid) {
                    let is_sighted = t == team || visible_hexes.contains(&core.pos);
                    if is_sighted {
                        core_hp.insert(t, (core.hp, core.max_hp));
                    } else {
                        core_hp.insert(t, (core.max_hp, core.max_hp));
                    }
                }
            }
        }

        let objective = self.get_vault_id().and_then(|vid| self.state.get_unit(vid)).map(|v| {
            let is_sighted = visible_hexes.contains(&v.pos);
            ObjectiveStatusDto {
                unit_id: v.id,
                pos: HexDto::new(v.pos.q, v.pos.r),
                hp: if is_sighted { v.hp } else { v.max_hp },
                max_hp: v.max_hp,
                is_alive: v.is_alive(),
            }
        });

        let mut base_zones: Vec<BaseZoneDto> = self
            .base_zones
            .values()
            .map(|bz| BaseZoneDto {
                team: bz.team,
                center: HexDto::new(bz.center.q, bz.center.r),
                radius: bz.radius,
            })
            .collect();
        base_zones.sort_by_key(|b| b.team);

        SnapshotDto {
            match_id: self.match_id.clone(),
            round: self.state.round,
            phase: format!("{:?}", self.state.phase),
            winner: self.state.winner,
            map: MapDto {
                radius: self.config.map_radius,
                walkable,
                obstacles,
            },
            units: visible_units,
            visible_hexes: visible_hexes_dto,
            controlled_units,
            deadline_unix_ms,
            state_hash: self.state_hash(),
            neutral_camps,
            player_team: team,
            roster,
            match_phase,
            controlled_hero_economy,
            allied_hero_economy,
            shop_catalog,
            can_shop,
            victory_mode: Some(format!("{:?}", self.config.phase7.victory_mode)),
            base_zones,
            shop_disabled_reason,
            core_hp,
            objective,
        }
    }

    pub fn calculate_state_hash(&self) -> String {
        self.state_hash()
    }
}

pub fn build_sanitized_snapshot(session: &BattleSession, team: u8, player_id: &str) -> SnapshotDto {
    let pid = player_id.to_string();
    session.snapshot_for_player(team, Some(&pid), None)
}

impl BattleSession {

    pub fn resolve_ai_round(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        events.extend(self.process_round_start_respawns());
        events.extend(self.process_base_regeneration());
        events.extend(self.distribute_passive_income());
        self.execute_ai_bot_shopping();
        events.extend(self.resolve_round());
        events
    }

    /// Compute cryptographic BLAKE3 state hash for audit and desync detection.
    pub fn state_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.state.round.to_le_bytes());
        hasher.update(&[self.state.phase as u8]);
        hasher.update(&[self.state.winner.unwrap_or(255)]);

        let mut unit_ids: Vec<UnitId> = self.state.units.keys().copied().collect();
        unit_ids.sort_unstable();

        for id in unit_ids {
            if let Some(unit) = self.state.get_unit(id) {
                hasher.update(&unit.id.to_le_bytes());
                hasher.update(&[unit.team]);
                hasher.update(&[unit.kind as u8]);
                hasher.update(&unit.pos.q.to_le_bytes());
                hasher.update(&unit.pos.r.to_le_bytes());
                hasher.update(&unit.hp.to_le_bytes());
                hasher.update(&unit.max_hp.to_le_bytes());
                hasher.update(&unit.ap.to_le_bytes());
                hasher.update(&unit.max_ap.to_le_bytes());
                hasher.update(&unit.energy.to_le_bytes());
                hasher.update(&unit.max_energy.to_le_bytes());
                hasher.update(&unit.attack_damage.to_le_bytes());
                hasher.update(&unit.attack_range.to_le_bytes());
                hasher.update(&unit.vision_range.to_le_bytes());
                hasher.update(&unit.initiative.to_le_bytes());

                let mut cd_keys: Vec<&String> = unit.cooldowns.keys().collect();
                cd_keys.sort();
                for k in cd_keys {
                    hasher.update(k.as_bytes());
                    hasher.update(&unit.cooldowns[k].to_le_bytes());
                }

                for s in &unit.statuses {
                    hasher.update(s.def_id.as_bytes());
                    hasher.update(&s.remaining_rounds.to_le_bytes());
                }

                hasher.update(&unit.gold.to_le_bytes());
                hasher.update(&unit.xp.to_le_bytes());
                hasher.update(&unit.level.to_le_bytes());
                for it in &unit.items {
                    hasher.update(it.as_bytes());
                }

                hasher.update(&[match unit.life_state {
                    LifeState::Alive => 0,
                    LifeState::DeadAwaitingRespawn { .. } => 1,
                    LifeState::PermanentlyRemoved => 2,
                }]);
                hasher.update(&unit.respawn_rounds.unwrap_or(0).to_le_bytes());
                if let Some(dp) = unit.death_pos {
                    hasher.update(&dp.q.to_le_bytes());
                    hasher.update(&dp.r.to_le_bytes());
                } else {
                    hasher.update(&[0u8; 8]);
                }
            }
        }

        hasher.finalize().to_hex().to_string()
    }

    pub fn get_unit(&self, id: UnitId) -> Option<&Unit> {
        self.state.get_unit(id)
    }

    pub fn get_unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.state.get_unit_mut(id)
    }

    pub fn get_hero(&self, id: UnitId) -> &Unit {
        self.state.get_unit(id).expect("Hero not found")
    }

    pub fn get_living_heroes_for_team(&self, team: TeamId) -> Vec<&Unit> {
        let mut heroes: Vec<&Unit> = self
            .state
            .units
            .values()
            .filter(|u| u.team == team && u.is_hero() && u.is_alive())
            .collect();
        heroes.sort_by_key(|u| u.id);
        heroes
    }

    pub fn distribute_passive_income(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let amount = self.config.economy.passive_income_per_round;
        let mut sorted_hero_ids: Vec<UnitId> = self
            .state
            .units
            .values()
            .filter(|u| u.is_hero() && u.is_alive())
            .map(|u| u.id)
            .collect();
        sorted_hero_ids.sort_unstable();

        for id in sorted_hero_ids {
            if let Some(hero) = self.state.units.get_mut(&id) {
                hero.gold += amount;
                events.push(GameEvent::RewardGranted {
                    unit_id: id,
                    gold: amount,
                    xp: 0,
                    reason: hexabellum_protocol::RewardReason::PassiveIncome,
                });
            }
        }
        events
    }

    pub fn distribute_passive_gold(&mut self) -> Vec<GameEvent> {
        self.distribute_passive_income()
    }

    pub fn execute_ai_bot_shopping(&mut self) {
        let catalog = crate::items::get_canonical_item_catalog();
        let max_slots = self.config.economy.max_item_slots;
        let allow_duplicates = self.config.economy.allow_duplicate_items;

        let mut bot_hero_ids: Vec<UnitId> = self
            .state
            .units
            .values()
            .filter(|u| {
                u.is_hero()
                    && u.is_alive()
                    && matches!(
                        self.controllers.get(u.id),
                        Some(Controller::Ai) | Some(Controller::Automatic)
                    )
            })
            .map(|u| u.id)
            .collect();
        bot_hero_ids.sort_unstable();

        for unit_id in bot_hero_ids {
            // AI heroes execute greedy item purchases strictly when positioned inside their allied base zone.
            let in_base = if let Some(unit) = self.state.get_unit(unit_id) {
                self.base_zones
                    .get(&unit.team)
                    .map(|bz| bz.contains(unit.pos))
                    .unwrap_or(true)
            } else {
                false
            };
            if !in_base {
                continue;
            }

            loop {
                let Some(unit) = self.state.units.get_mut(&unit_id) else {
                    break;
                };
                if unit.items.len() >= max_slots {
                    break;
                }
                let item_to_buy = if unit.gold >= 120 && !unit.items.contains(&"plate_armor".to_string()) {
                    catalog.iter().find(|i| i.id == "plate_armor")
                } else if unit.gold >= 100 && !unit.items.contains(&"longblade".to_string()) {
                    catalog.iter().find(|i| i.id == "longblade")
                } else if unit.gold >= 100 && !unit.items.contains(&"focus_charm".to_string()) {
                    catalog.iter().find(|i| i.id == "focus_charm")
                } else if unit.gold >= 80 && !unit.items.contains(&"scout_lens".to_string()) {
                    catalog.iter().find(|i| i.id == "scout_lens")
                } else {
                    None
                };

                if let Some(item) = item_to_buy {
                    if crate::shop::execute_purchase(unit, item, max_slots, allow_duplicates).is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    pub fn apply_damage(&mut self, target_id: UnitId, damage: u32, attacker_id: UnitId) {
        if let Some(target) = self.state.get_unit_mut(target_id) {
            target.hp = target.hp.saturating_sub(damage);
            target.last_attacker = Some(attacker_id);
        }
    }

    pub fn resolve_fatalities(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let dead_units: Vec<(UnitId, UnitKind, TeamId, Option<UnitId>)> = self
            .state
            .units
            .values()
            .filter(|u| !u.is_alive())
            .map(|u| (u.id, u.kind, u.team, u.last_attacker))
            .collect();

        for (victim_id, victim_kind, victim_team, last_attacker) in dead_units {
            self.state.units.remove(&victim_id);
            let killed_by = last_attacker.unwrap_or(0);
            events.push(GameEvent::UnitDied {
                unit_id: victim_id,
                unit_kind: victim_kind,
                killed_by,
            });

            crate::turn::handle_kill_rewards_in_state(
                &mut self.state,
                victim_kind,
                victim_team,
                killed_by,
                &mut events,
            );
        }

        events
    }

    pub fn destroy_structure(&mut self, structure_id: UnitId) -> Vec<GameEvent> {
        if let Some(s) = self.state.get_unit_mut(structure_id) {
            s.hp = 0;
        }
        self.resolve_fatalities()
    }

    pub fn new_test_session_5v5() -> Self {
        let mut config = BattleConfig::default();
        config.heroes_per_team = 5;
        config.players_per_team = 5;
        config.skip_draft = true;
        config.enable_ai_team_1 = true;
        let mut session = Self::new("test_session_5v5".to_string(), config.clone());

        session.state.units.clear();
        session.controllers = ControllerMap::default();
        session.hero_assignments.clear();
        session.unit_registry.clear();
        session.is_match_over = false;
        session.winning_team = None;

        let bz0 = BaseZone::new(0, HexCoord::new(-7, 0), config.phase7.base_zone_radius);
        let bz1 = BaseZone::new(1, HexCoord::new(7, 0), config.phase7.base_zone_radius);
        session.base_zones.insert(0, bz0);
        session.base_zones.insert(1, bz1);

        // Team 0 Core: 500 at (-7, 0)
        let t0_core = Unit::new_core(
            500,
            0,
            HexCoord::new(-7, 0),
            config.phase7.core_hp,
            config.phase7.core_vision_range,
        );
        session.controllers.assign(500, Controller::Automatic);
        session.state.add_unit(t0_core);

        // Team 1 Core: 501 at (7, 0)
        let t1_core = Unit::new_core(
            501,
            1,
            HexCoord::new(7, 0),
            config.phase7.core_hp,
            config.phase7.core_vision_range,
        );
        session.controllers.assign(501, Controller::Automatic);
        session.state.add_unit(t1_core);

        // Neutral Vault: 300 at (0, 0)
        let vault = Unit::new_vault(300, HexCoord::new(0, 0), config.phase7.objective_hp);
        session.controllers.assign(300, Controller::Automatic);
        session.state.add_unit(vault);

        // Team 0 Spawners & Towers
        let sp0_n = Unit::new_spawner(110, 0, HexCoord::new(-6, -1), config.spawn_interval);
        let sp0_s = Unit::new_spawner(112, 0, HexCoord::new(-6, 1), config.spawn_interval);
        let tw0_n = Unit::new_tower(120, 0, HexCoord::new(-4, -1));
        let tw0_s = Unit::new_tower(122, 0, HexCoord::new(-4, 1));
        session.controllers.assign(110, Controller::Automatic);
        session.controllers.assign(112, Controller::Automatic);
        session.controllers.assign(120, Controller::Automatic);
        session.controllers.assign(122, Controller::Automatic);
        session.state.add_unit(sp0_n);
        session.state.add_unit(sp0_s);
        session.state.add_unit(tw0_n);
        session.state.add_unit(tw0_s);

        // Team 1 Spawners & Towers
        let sp1_n = Unit::new_spawner(210, 1, HexCoord::new(6, -1), config.spawn_interval);
        let sp1_s = Unit::new_spawner(212, 1, HexCoord::new(6, 1), config.spawn_interval);
        let tw1_n = Unit::new_tower(220, 1, HexCoord::new(4, -1));
        let tw1_s = Unit::new_tower(222, 1, HexCoord::new(4, 1));
        session.controllers.assign(210, Controller::Automatic);
        session.controllers.assign(212, Controller::Automatic);
        session.controllers.assign(220, Controller::Automatic);
        session.controllers.assign(222, Controller::Automatic);
        session.state.add_unit(sp1_n);
        session.state.add_unit(sp1_s);
        session.state.add_unit(tw1_n);
        session.state.add_unit(tw1_s);

        // Team 0 Heroes: 101..105, all inside base zone (-7, 0) r=2
        session.state.add_unit(Unit::new_vanguard(101, 0, HexCoord::new(-6, 0), 3));
        session.state.add_unit(Unit::new_ranger(102, 0, HexCoord::new(-7, -1), 4));
        session.state.add_unit(Unit::new_warden(103, 0, HexCoord::new(-8, 0), 2));
        session.state.add_unit(Unit::new_sniper(104, 0, HexCoord::new(-7, 1), 4));
        session.state.add_unit(Unit::new_berserker(105, 0, HexCoord::new(-8, 1), 3));
        for uid in 101..=105 {
            session.controllers.assign(uid, Controller::Ai);
        }

        // Team 1 Heroes: 201..205, all inside base zone (7, 0) r=2
        session.state.add_unit(Unit::new_vanguard(201, 1, HexCoord::new(6, 0), 3));
        session.state.add_unit(Unit::new_ranger(202, 1, HexCoord::new(7, -1), 4));
        session.state.add_unit(Unit::new_warden(203, 1, HexCoord::new(8, 0), 2));
        session.state.add_unit(Unit::new_sniper(204, 1, HexCoord::new(7, 1), 4));
        session.state.add_unit(Unit::new_berserker(205, 1, HexCoord::new(8, 1), 3));
        for uid in 201..=205 {
            session.controllers.assign(uid, Controller::Ai);
        }

        session.state.next_unit_id = 700;
        session.state.update_fog();
        session
    }

    pub fn get_core_id(&self, team: TeamId) -> Option<UnitId> {
        self.state
            .units
            .values()
            .find(|u| u.team == team && u.kind == UnitKind::Core)
            .map(|u| u.id)
    }

    pub fn get_vault_id(&self) -> Option<UnitId> {
        self.state
            .units
            .values()
            .find(|u| u.kind == UnitKind::Objective)
            .map(|u| u.id)
    }

    pub fn get_base_zone(&self, team: TeamId) -> Option<&BaseZone> {
        self.base_zones.get(&team)
    }

    pub fn is_hex_occupied(&self, pos: HexCoord) -> bool {
        self.state.units.values().any(|u| u.is_alive() && u.pos == pos)
    }

    pub fn inflict_damage(&mut self, target_id: UnitId, damage: u32, attacker_id: UnitId) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let attacker_team = self.state.get_unit(attacker_id).map(|u| u.team);
        let Some(target) = self.state.get_unit_mut(target_id) else {
            return events;
        };

        target.hp = target.hp.saturating_sub(damage);
        target.last_attacker = Some(attacker_id);

        if target.hp == 0 {
            let target_kind = target.kind;
            let target_team = target.team;
            let target_pos = target.pos;
            let respawn_delay = self.config.phase7.respawn_delay_rounds;

            match target_kind {
                UnitKind::Hero => {
                    target.life_state = LifeState::DeadAwaitingRespawn {
                        rounds_left: respawn_delay,
                        death_pos: target_pos,
                    };
                    target.respawn_rounds = Some(respawn_delay);
                    target.death_pos = Some(target_pos);
                    target.statuses.clear();

                    events.push(GameEvent::HeroDied {
                        unit_id: target_id,
                        killed_by: attacker_id,
                        respawn_rounds: respawn_delay,
                    });
                    events.push(GameEvent::UnitDied {
                        unit_id: target_id,
                        unit_kind: target_kind,
                        killed_by: attacker_id,
                    });

                    crate::turn::handle_kill_rewards_in_state(
                        &mut self.state,
                        target_kind,
                        target_team,
                        attacker_id,
                        &mut events,
                    );
                }
                UnitKind::Objective => {
                    target.life_state = LifeState::PermanentlyRemoved;
                    target.hp = 0;
                    if let Some(a_team) = attacker_team {
                        let bounty_gold = self.config.phase7.objective_gold_reward_per_hero;
                        let bounty_xp = self.config.phase7.objective_xp_reward_per_hero;
                        let buff_duration = self.config.phase7.objective_buff_duration_rounds;
                        let buff_bonus = self.config.phase7.objective_buff_damage_bonus;

                        let buff_def = crate::status::vault_damage_buff_def(buff_duration, buff_bonus);
                        let mut affected = Vec::new();
                        for hero in self.state.units.values_mut() {
                            if hero.team == a_team && hero.is_hero() && hero.is_alive() {
                                hero.gold += bounty_gold;
                                hero.xp += bounty_xp;
                                hero.statuses.retain(|s| s.def_id != buff_def.id);
                                hero.statuses.push(crate::status::StatusInstance::from_def(&buff_def));
                                affected.push(hero.id);
                            }
                        }

                        events.push(GameEvent::ObjectiveDestroyed {
                            objective_id: target_id,
                            destroyer_team: a_team,
                            last_attacker_id: attacker_id,
                            gold_awarded_per_hero: bounty_gold,
                            xp_awarded_per_hero: bounty_xp,
                            affected_heroes: affected,
                        });
                    }
                }
                UnitKind::Core => {
                    target.life_state = LifeState::PermanentlyRemoved;
                    target.hp = 0;
                    let winning = if target_team == 0 { 1 } else { 0 };
                    self.is_match_over = true;
                    self.winning_team = Some(winning);
                    self.state.winner = Some(winning);
                    self.state.phase = Phase::MatchEnd;

                    events.push(GameEvent::CoreDestroyed {
                        core_id: target_id,
                        team: target_team,
                        destroyed_by: attacker_id,
                    });
                    events.push(GameEvent::UnitDied {
                        unit_id: target_id,
                        unit_kind: target_kind,
                        killed_by: attacker_id,
                    });
                    events.push(GameEvent::MatchEnded {
                        winner: Some(winning),
                        reason: Some(hexabellum_protocol::VictoryReasonDto::CoreDestroyed {
                            destroyed_core_id: target_id,
                            destroyed_team: target_team,
                            destroyer_team: winning,
                        }),
                    });
                }
                _ => {
                    self.state.units.remove(&target_id);
                    events.push(GameEvent::UnitDied {
                        unit_id: target_id,
                        unit_kind: target_kind,
                        killed_by: attacker_id,
                    });
                    crate::turn::handle_kill_rewards_in_state(
                        &mut self.state,
                        target_kind,
                        target_team,
                        attacker_id,
                        &mut events,
                    );
                }
            }
        }

        self.check_core_victory();
        events
    }

    pub fn kill_hero_for_test(&mut self, hero_id: UnitId) {
        if let Some(hero) = self.state.get_unit_mut(hero_id) {
            hero.hp = 0;
            let death_pos = hero.pos;
            let respawn_delay = self.config.phase7.respawn_delay_rounds;
            hero.life_state = LifeState::DeadAwaitingRespawn {
                rounds_left: respawn_delay,
                death_pos,
            };
            hero.respawn_rounds = Some(respawn_delay);
            hero.death_pos = Some(death_pos);
            hero.statuses.clear();
        }
    }

    pub fn fast_forward_respawn(&mut self, hero_id: UnitId) {
        if let Some(hero) = self.state.get_unit_mut(hero_id) {
            hero.life_state = LifeState::DeadAwaitingRespawn {
                rounds_left: 0,
                death_pos: hero.death_pos.unwrap_or(hero.pos),
            };
            hero.respawn_rounds = Some(0);
        }
        self.execute_hero_respawn(hero_id);
    }

    pub fn set_unit_pos(&mut self, unit_id: UnitId, pos: HexCoord) {
        if let Some(unit) = self.state.get_unit_mut(unit_id) {
            unit.pos = pos;
        }
    }

    pub fn set_unit_hp(&mut self, unit_id: UnitId, hp: u32) {
        if let Some(unit) = self.state.get_unit_mut(unit_id) {
            unit.hp = hp.min(unit.max_hp);
        }
    }

    pub fn destroy_vault(&mut self, attacker_id: UnitId) -> Vec<GameEvent> {
        let vault_id = self.get_vault_id().expect("Vault not found");
        self.inflict_damage(vault_id, 250, attacker_id)
    }

    pub fn has_active_buff(&self, unit_id: UnitId, buff_id: &str) -> bool {
        if let Some(unit) = self.state.get_unit(unit_id) {
            unit.statuses.iter().any(|s| {
                s.def_id == buff_id
                    || (buff_id == "attack_damage_buff"
                        && (s.def_id == "attack_damage_buff" || s.def_id == "vault_buff"))
                    || (buff_id == "vault_buff"
                        && (s.def_id == "attack_damage_buff" || s.def_id == "vault_buff"))
            })
        } else {
            false
        }
    }

    pub fn can_hero_shop(&self, hero_id: UnitId) -> bool {
        if self.state.phase != Phase::Planning {
            return false;
        }
        let Some(hero) = self.state.get_unit(hero_id) else {
            return false;
        };
        if !hero.is_alive() {
            return false;
        }
        let Some(base_zone) = self.base_zones.get(&hero.team) else {
            return false;
        };
        base_zone.contains(hero.pos)
    }

    pub fn process_round_start_respawns(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut dead_hero_ids: Vec<UnitId> = self
            .state
            .units
            .values()
            .filter(|u| u.is_hero() && u.is_dead_awaiting_respawn())
            .map(|u| u.id)
            .collect();
        dead_hero_ids.sort_unstable();

        for id in dead_hero_ids {
            let mut ready_to_respawn = false;
            if let Some(hero) = self.state.units.get_mut(&id) {
                if let LifeState::DeadAwaitingRespawn {
                    rounds_left,
                    death_pos,
                } = hero.life_state
                {
                    let new_rounds = rounds_left.saturating_sub(1);
                    if new_rounds == 0 {
                        ready_to_respawn = true;
                    } else {
                        hero.life_state = LifeState::DeadAwaitingRespawn {
                            rounds_left: new_rounds,
                            death_pos,
                        };
                        hero.respawn_rounds = Some(new_rounds);
                    }
                }
            }

            if ready_to_respawn {
                if let Some(respawn_event) = self.execute_hero_respawn(id) {
                    events.push(respawn_event);
                }
            }
        }

        events
    }

    pub fn execute_hero_respawn(&mut self, hero_id: UnitId) -> Option<GameEvent> {
        let team = {
            let hero = self.state.get_unit(hero_id)?;
            hero.team
        };

        let base_zone = self.base_zones.get(&team)?.clone();
        let candidates = base_zone.candidate_spawn_hexes();

        let respawn_hex = candidates.into_iter().find(|hex| {
            !self.state.map.movement_blockers().contains(hex) && !self.is_hex_occupied(*hex)
        });

        if let Some(hex) = respawn_hex {
            if let Some(hero) = self.state.units.get_mut(&hero_id) {
                hero.pos = hex;
                hero.hp = hero.max_hp;
                hero.ap = hero.max_ap;
                hero.energy = hero.max_energy;
                hero.cooldowns.clear();
                hero.statuses.clear();
                hero.life_state = LifeState::Alive;
                hero.respawn_rounds = None;
                hero.death_pos = None;

                return Some(GameEvent::HeroRespawned {
                    unit_id: hero_id,
                    team,
                    pos: hex,
                });
            }
        } else {
            if let Some(hero) = self.state.units.get_mut(&hero_id) {
                let death_pos = hero.death_pos.unwrap_or(hero.pos);
                hero.life_state = LifeState::DeadAwaitingRespawn {
                    rounds_left: 1,
                    death_pos,
                };
                hero.respawn_rounds = Some(1);
            }
        }
        None
    }

    pub fn process_base_regeneration(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let regen_amount = self.config.phase7.base_regen_per_round;

        let mut candidate_heroes: Vec<(UnitId, TeamId, HexCoord)> = self
            .state
            .units
            .values()
            .filter(|u| u.is_hero() && u.is_alive())
            .map(|u| (u.id, u.team, u.pos))
            .collect();
        candidate_heroes.sort_by_key(|(id, ..)| *id);

        for (id, team, pos) in candidate_heroes {
            if let Some(base_zone) = self.base_zones.get(&team) {
                if base_zone.contains(pos) {
                    if let Some(hero) = self.state.units.get_mut(&id) {
                        if hero.hp < hero.max_hp {
                            let old_hp = hero.hp;
                            hero.hp = (hero.hp + regen_amount).min(hero.max_hp);
                            let restored = hero.hp - old_hp;
                            events.push(GameEvent::BaseRegenerationApplied {
                                unit_id: id,
                                team,
                                amount: restored,
                                new_hp: hero.hp,
                            });
                        }
                    }
                }
            }
        }
        events
    }

    pub fn check_core_victory(&mut self) -> Option<GameEvent> {
        if self.is_match_over {
            return self.winning_team.map(|winner| {
                let reason = self.state.check_winner_with_reason().map(|(_, r)| r);
                GameEvent::MatchEnded {
                    winner: Some(winner),
                    reason,
                }
            });
        }
        for team in [0, 1] {
            if let Some(core_id) = self.get_core_id(team) {
                if let Some(core) = self.state.get_unit(core_id) {
                    if core.hp == 0 || !core.is_alive() {
                        let winning = if team == 0 { 1 } else { 0 };
                        self.is_match_over = true;
                        self.winning_team = Some(winning);
                        self.state.winner = Some(winning);
                        self.state.phase = Phase::MatchEnd;
                        return Some(GameEvent::MatchEnded {
                            winner: Some(winning),
                            reason: Some(hexabellum_protocol::VictoryReasonDto::CoreDestroyed {
                                destroyed_core_id: core_id,
                                destroyed_team: team,
                                destroyer_team: winning,
                            }),
                        });
                    }
                }
            }
        }
        None
    }

    pub fn spawn_test_minion(&mut self, team: TeamId, pos: HexCoord) -> UnitId {
        let id = self.state.next_unit_id;
        self.state.next_unit_id += 1;
        let mut minion = Unit::new_minion(id, team, pos);
        minion.attack_range = 1;
        self.state.add_unit(minion);
        self.controllers.assign(id, Controller::Automatic);
        id
    }

    pub fn get_valid_attack_targets(&self, unit_id: UnitId) -> Vec<UnitId> {
        let Some(unit) = self.state.get_unit(unit_id) else {
            return Vec::new();
        };

        self.state
            .units
            .values()
            .filter(|target| {
                if !target.is_alive() || target.team == unit.team {
                    return false;
                }
                if unit.kind == UnitKind::Minion && target.kind == UnitKind::Objective {
                    return false;
                }
                if unit.kind == UnitKind::Tower && target.kind == UnitKind::Objective {
                    return false;
                }
                true
            })
            .map(|target| target.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_snapshot_censors_hidden_enemy_units() {
        let mut session = BattleSession::new("test_match".into(), BattleConfig::default());

        // Add concealed enemy scout in distant fog hex (0, 5)
        let scout = Unit::new_hero(99, 1, HexCoord::new(0, 5), 1);
        session.state.add_unit(scout);
        session.state.update_fog();

        // Team 0 Snapshot
        let snap_t0 = session.snapshot_for_team(0, None);
        assert!(
            !snap_t0.units.iter().any(|u| u.id == 99),
            "Concealed enemy leaked to Team 0 snapshot!"
        );

        // Team 1 Snapshot
        let snap_t1 = session.snapshot_for_team(1, None);
        assert!(
            snap_t1.units.iter().any(|u| u.id == 99),
            "Friendly unit missing from Team 1 snapshot!"
        );
    }

    #[test]
    fn test_fog_event_sanitization_suppresses_hidden_movements() {
        let mut session = BattleSession::new("test_match".into(), BattleConfig::default());
        session.state.update_fog();

        let hidden_move = GameEvent::UnitMoved {
            unit_id: 6, // Team 1 Hero
            from: HexCoord::new(6, 0),
            to: HexCoord::new(7, -1),
            path: vec![HexCoord::new(6, 0), HexCoord::new(7, -1)],
            ap_spent: 1,
        };

        let sanitized = session.sanitize_events_for_team(0, &[hidden_move]);
        assert!(
            sanitized.is_empty(),
            "Concealed movement leaked across fog of war in event stream!"
        );
    }

    #[test]
    fn test_blake3_state_hash_determinism() {
        let session1 = BattleSession::new("m1".into(), BattleConfig::default());
        let session2 = BattleSession::new("m2".into(), BattleConfig::default());

        assert_eq!(
            session1.state_hash(),
            session2.state_hash(),
            "Identical match setups produced divergent BLAKE3 state hashes!"
        );

        let mut session3 = BattleSession::new("m3".into(), BattleConfig::default());
        if let Some(unit) = session3.state.get_unit_mut(1) {
            unit.attack_damage += 2;
        }
        assert_ne!(
            session1.state_hash(),
            session3.state_hash(),
            "Mutating unit attack damage must change BLAKE3 state hash!"
        );
    }

    #[test]
    fn test_headless_ai_vs_ai_server_simulation_terminates() {
        let mut session = BattleSession::new("sim".into(), BattleConfig::default());

        for round in 1..=50 {
            if session.state.winner.is_some() {
                break;
            }
            let events = session.resolve_round();
            assert!(
                !events.is_empty(),
                "Round resolution produced zero events in round {}",
                round
            );
        }

        assert!(session.state.round > 1, "Simulation failed to advance rounds");
    }

    #[test]
    fn test_submit_orders_validation() {
        let mut session = BattleSession::new("order_test".into(), BattleConfig::default());
        let player_p1 = "player-1".to_string();
        session.assign_team_player(0, player_p1.clone());

        // 1. Submit orders for hero 1 with valid move
        let valid_order = OrderDto {
            unit_id: 1,
            move_target: Some(HexDto::new(-3, -1)),
            action: ActionDto::Wait,
        };
        let res = session.submit_player_orders(&player_p1, 0, 0, vec![valid_order]);
        assert!(res.is_ok());

        // 2. Reject stale round
        let res_stale = session.submit_player_orders(&player_p1, 0, 99, vec![]);
        assert_eq!(res_stale.unwrap_err().code, ProtocolErrorCode::StaleRound);

        // 3. Reject unit not owned (Hero 6 is Team 1)
        let unowned_order = OrderDto {
            unit_id: 6,
            move_target: None,
            action: ActionDto::Wait,
        };
        let err_unowned = session
            .submit_player_orders(&player_p1, 0, 0, vec![unowned_order])
            .unwrap_err();
        assert_eq!(err_unowned.code, ProtocolErrorCode::NotYourUnit);
        assert_eq!(err_unowned.unit_id, Some(6));
        assert!(err_unowned.reason.contains("Unit #6"));

        // 4. Reject invalid target (cannot attack own team)
        let friendly_attack = OrderDto {
            unit_id: 1,
            move_target: None,
            action: ActionDto::Attack { target_id: 2 },
        };
        let err_friendly = session
            .submit_player_orders(&player_p1, 0, 0, vec![friendly_attack])
            .unwrap_err();
        assert_eq!(err_friendly.code, ProtocolErrorCode::InvalidTarget);
        assert_eq!(err_friendly.unit_id, Some(1));

        // 5. Valid Cleave Cast order for Vanguard (1 AP, 3 Energy)
        let cleave_order = OrderDto {
            unit_id: 1,
            move_target: None,
            action: ActionDto::Cast {
                spell_id: "cleave".to_string(),
                target: hexabellum_protocol::SpellTargetDto::None,
            },
        };
        let res_cleave = session.submit_player_orders(&player_p1, 0, 0, vec![cleave_order]);
        assert!(res_cleave.is_ok());

        // 6. Reject Cast if insufficient Energy
        let mut session_no_energy = BattleSession::new("energy_test".into(), BattleConfig::default());
        session_no_energy.assign_team_player(0, player_p1.clone());
        session_no_energy.state.get_unit_mut(1).unwrap().energy = 0;
        let err_energy = session_no_energy
            .submit_player_orders(
                &player_p1,
                0,
                0,
                vec![OrderDto {
                    unit_id: 1,
                    move_target: None,
                    action: ActionDto::Cast {
                        spell_id: "cleave".to_string(),
                        target: hexabellum_protocol::SpellTargetDto::None,
                    },
                }],
            )
            .unwrap_err();
        assert_eq!(err_energy.code, ProtocolErrorCode::InsufficientResources);

        // 7. Reject Cast if on cooldown
        let mut session_cd = BattleSession::new("cd_test".into(), BattleConfig::default());
        session_cd.assign_team_player(0, player_p1.clone());
        session_cd.state.get_unit_mut(1).unwrap().cooldowns.insert("cleave".to_string(), 2);
        let err_cd = session_cd
            .submit_player_orders(
                &player_p1,
                0,
                0,
                vec![OrderDto {
                    unit_id: 1,
                    move_target: None,
                    action: ActionDto::Cast {
                        spell_id: "cleave".to_string(),
                        target: hexabellum_protocol::SpellTargetDto::None,
                    },
                }],
            )
            .unwrap_err();
        assert_eq!(err_cd.code, ProtocolErrorCode::CooldownActive);

        // 8. Reject Repair if structure is already at full health
        let err_full_repair = session
            .submit_player_orders(
                &player_p1,
                0,
                0,
                vec![OrderDto {
                    unit_id: 1,
                    move_target: Some(HexDto::new(-4, 0)),
                    action: ActionDto::Repair { target_id: 101 },
                }],
            )
            .unwrap_err();
        assert_eq!(err_full_repair.code, ProtocolErrorCode::InvalidTarget);
        assert!(err_full_repair.reason.contains("full health"));

        // 9. Accept Repair if structure is damaged and adjacent
        session.state.get_unit_mut(101).unwrap().hp = 50;
        let valid_repair = session.submit_player_orders(
            &player_p1,
            0,
            0,
            vec![OrderDto {
                unit_id: 1,
                move_target: Some(HexDto::new(-4, 0)),
                action: ActionDto::Repair { target_id: 101 },
            }],
        );
        assert!(valid_repair.is_ok());
    }
}
