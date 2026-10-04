use crate::ability::SpellId;
use crate::hex::HexCoord;
use crate::status::{StatKind, StatusInstance};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type UnitId = u64;
pub type TeamId = u8;
pub type LaneId = String;

pub const TEAM_0: TeamId = 0;
pub const TEAM_1: TeamId = 1;
pub const TEAM_NEUTRAL: TeamId = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitKind {
    Hero,
    Minion,
    Tower,
    #[serde(alias = "SpawnerTower")]
    Spawner,
    #[serde(alias = "Neutral")]
    NeutralGuardian,
    Core,
    Objective,
}

impl UnitKind {
    #[inline]
    pub fn is_stationary(&self) -> bool {
        self.is_structure()
    }

    #[inline]
    pub fn is_structure(&self) -> bool {
        matches!(
            self,
            UnitKind::Tower | UnitKind::Spawner | UnitKind::Core | UnitKind::Objective
        )
    }

    #[inline]
    pub fn is_repairable(&self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::Spawner)
    }
}

impl std::fmt::Display for UnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitKind::Hero => write!(f, "Hero"),
            UnitKind::Minion => write!(f, "Minion"),
            UnitKind::Tower => write!(f, "Tower"),
            UnitKind::Spawner => write!(f, "Spawner"),
            UnitKind::NeutralGuardian => write!(f, "NeutralGuardian"),
            UnitKind::Core => write!(f, "Core"),
            UnitKind::Objective => write!(f, "Objective"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifeState {
    Alive,
    DeadAwaitingRespawn {
        rounds_left: u32,
        death_pos: HexCoord,
    },
    PermanentlyRemoved,
}

impl Default for LifeState {
    fn default() -> Self {
        LifeState::Alive
    }
}

fn default_life_state() -> LifeState {
    LifeState::Alive
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneDirection {
    None,
    TowardEnemy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub team: TeamId,
    pub pos: HexCoord,
    pub hp: u32,
    pub max_hp: u32,

    // AP Resource
    pub ap: u32,
    pub max_ap: u32,
    pub initiative: u32,

    // Base Combat Stats
    pub attack_damage: u32,
    pub attack_range: u32,
    pub vision_range: u32,

    // Phase 4 Additions: Energy & Cooldowns
    #[serde(default)]
    pub energy: u32,
    #[serde(default)]
    pub max_energy: u32,
    #[serde(default)]
    pub energy_regen: u32,
    #[serde(default)]
    pub cooldowns: HashMap<SpellId, u32>,

    // Status Modifiers
    #[serde(default)]
    pub statuses: Vec<StatusInstance>,

    // Hero Archetype Identifier (e.g. "vanguard", "ranger", "warden", "sniper", "berserker")
    #[serde(default)]
    pub hero_id: Option<String>,

    // Lane Waypoint Navigation
    #[serde(default)]
    pub lane_id: Option<LaneId>,
    #[serde(default)]
    pub waypoint_index: Option<usize>,

    // Combat AI Memory
    #[serde(default)]
    pub aggro_range: u32,
    #[serde(default)]
    pub last_attacker: Option<UnitId>,

    // Phase 2 MOBA Legacy Support
    #[serde(default)]
    pub spawn_interval: Option<u32>,
    #[serde(default)]
    pub spawn_counter: u32,
    #[serde(default = "default_lane_direction")]
    pub lane_direction: LaneDirection,

    // Phase 6 Economy & Progression fields
    #[serde(default)]
    pub gold: u32,
    #[serde(default)]
    pub xp: u32,
    #[serde(default = "default_unit_level")]
    pub level: u32,
    #[serde(default)]
    pub items: Vec<String>,

    // Phase 7 Macro Lifecycle fields
    #[serde(default = "default_life_state")]
    pub life_state: LifeState,
    #[serde(default)]
    pub respawn_rounds: Option<u32>,
    #[serde(default)]
    pub death_pos: Option<HexCoord>,
}

fn default_unit_level() -> u32 {
    1
}

fn default_lane_direction() -> LaneDirection {
    LaneDirection::None
}

pub const ATTACK_AP_COST: u32 = 1;

impl Unit {
    /// Hero unit: mobile fighter with high AP budget and tactical initiative.
    pub fn new_hero(id: UnitId, team: TeamId, pos: HexCoord, initiative: u32) -> Self {
        Self {
            id,
            kind: UnitKind::Hero,
            team,
            pos,
            hp: 100,
            max_hp: 100,
            ap: 3,
            max_ap: 3,
            initiative,
            attack_damage: 20,
            attack_range: 1,
            vision_range: 3,
            energy: 5,
            max_energy: 5,
            energy_regen: 1,
            cooldowns: HashMap::new(),
            statuses: Vec::new(),
            hero_id: None,
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 50,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Create Vanguard (Hero A - Frontline Cleaver)
    pub fn new_vanguard(id: UnitId, team: TeamId, pos: HexCoord, initiative: u32) -> Self {
        let mut cooldowns = HashMap::new();
        cooldowns.insert("cleave".to_string(), 0);

        Self {
            id,
            kind: UnitKind::Hero,
            team,
            pos,
            hp: 140,
            max_hp: 140,
            ap: 3,
            max_ap: 3,
            initiative,
            attack_damage: 18,
            attack_range: 1,
            vision_range: 3,
            energy: 5,
            max_energy: 5,
            energy_regen: 1,
            cooldowns,
            statuses: Vec::new(),
            hero_id: Some("vanguard".to_string()),
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 50,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Create Ranger (Hero B - Ranged Sniper)
    pub fn new_ranger(id: UnitId, team: TeamId, pos: HexCoord, initiative: u32) -> Self {
        let mut cooldowns = HashMap::new();
        cooldowns.insert("bolt".to_string(), 0);

        Self {
            id,
            kind: UnitKind::Hero,
            team,
            pos,
            hp: 90,
            max_hp: 90,
            ap: 3,
            max_ap: 3,
            initiative,
            attack_damage: 16,
            attack_range: 2,
            vision_range: 4,
            energy: 5,
            max_energy: 5,
            energy_regen: 1,
            cooldowns,
            statuses: Vec::new(),
            hero_id: Some("ranger".to_string()),
            lane_id: None,
            waypoint_index: None,
            aggro_range: 4,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 50,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Create Warden (Hero C - Support Healer)
    pub fn new_warden(id: UnitId, team: TeamId, pos: HexCoord, initiative: u32) -> Self {
        let mut cooldowns = HashMap::new();
        cooldowns.insert("mend".to_string(), 0);

        Self {
            id,
            kind: UnitKind::Hero,
            team,
            pos,
            hp: 100,
            max_hp: 100,
            ap: 3,
            max_ap: 3,
            initiative,
            attack_damage: 12,
            attack_range: 1,
            vision_range: 4,
            energy: 6,
            max_energy: 6,
            energy_regen: 1,
            cooldowns,
            statuses: Vec::new(),
            hero_id: Some("warden".to_string()),
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 50,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Create Sniper (Artillery Marksman)
    pub fn new_sniper(id: UnitId, team: TeamId, pos: HexCoord, initiative: u32) -> Self {
        let mut cooldowns = HashMap::new();
        cooldowns.insert("longshot".to_string(), 0);

        Self {
            id,
            kind: UnitKind::Hero,
            team,
            pos,
            hp: 80,
            max_hp: 80,
            ap: 3,
            max_ap: 3,
            initiative,
            attack_damage: 14,
            attack_range: 3,
            vision_range: 5,
            energy: 5,
            max_energy: 5,
            energy_regen: 1,
            cooldowns,
            statuses: Vec::new(),
            hero_id: Some("sniper".to_string()),
            lane_id: None,
            waypoint_index: None,
            aggro_range: 4,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 50,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Create Berserker (Bruiser Diver)
    pub fn new_berserker(id: UnitId, team: TeamId, pos: HexCoord, initiative: u32) -> Self {
        let mut cooldowns = HashMap::new();
        cooldowns.insert("fury".to_string(), 0);

        Self {
            id,
            kind: UnitKind::Hero,
            team,
            pos,
            hp: 120,
            max_hp: 120,
            ap: 3,
            max_ap: 3,
            initiative,
            attack_damage: 20,
            attack_range: 1,
            vision_range: 3,
            energy: 5,
            max_energy: 5,
            energy_regen: 1,
            cooldowns,
            statuses: Vec::new(),
            hero_id: Some("berserker".to_string()),
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 50,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Create Neutral Guardian
    pub fn new_neutral_guardian(id: UnitId, pos: HexCoord) -> Self {
        Self {
            id,
            kind: UnitKind::NeutralGuardian,
            team: TEAM_NEUTRAL,
            pos,
            hp: 80,
            max_hp: 80,
            ap: 2,
            max_ap: 2,
            initiative: 2,
            attack_damage: 15,
            attack_range: 1,
            vision_range: 3,
            energy: 0,
            max_energy: 0,
            energy_regen: 0,
            cooldowns: HashMap::new(),
            statuses: Vec::new(),
            hero_id: None,
            lane_id: None,
            waypoint_index: None,
            aggro_range: 2,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 0,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Minion unit: cheap infantry pushing the lane.
    pub fn new_minion(id: UnitId, team: TeamId, pos: HexCoord) -> Self {
        Self {
            id,
            kind: UnitKind::Minion,
            team,
            pos,
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
            hero_id: None,
            lane_id: Some("mid".to_string()),
            waypoint_index: Some(if team == TEAM_0 { 0 } else { 4 }),
            aggro_range: 2,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::TowardEnemy,
            gold: 0,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Defensive Tower: immovable fortress providing long-range perimeter defense.
    pub fn new_tower(id: UnitId, team: TeamId, pos: HexCoord) -> Self {
        Self {
            id,
            kind: UnitKind::Tower,
            team,
            pos,
            hp: 150,
            max_hp: 150,
            ap: 1,
            max_ap: 1,
            initiative: 5,
            attack_damage: 25,
            attack_range: 3,
            vision_range: 4,
            energy: 0,
            max_energy: 0,
            energy_regen: 0,
            cooldowns: HashMap::new(),
            statuses: Vec::new(),
            hero_id: None,
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 0,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Spawner Tower: key base structure producing minion waves.
    pub fn new_spawner(id: UnitId, team: TeamId, pos: HexCoord, spawn_interval: u32) -> Self {
        Self {
            id,
            kind: UnitKind::Spawner,
            team,
            pos,
            hp: 150,
            max_hp: 150,
            ap: 0,
            max_ap: 0,
            initiative: 0,
            attack_damage: 0,
            attack_range: 0,
            vision_range: 2,
            energy: 0,
            max_energy: 0,
            energy_regen: 0,
            cooldowns: HashMap::new(),
            statuses: Vec::new(),
            hero_id: None,
            lane_id: None,
            waypoint_index: None,
            aggro_range: 0,
            last_attacker: None,
            spawn_interval: Some(spawn_interval),
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 0,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Factory method for the team Core structure.
    pub fn new_core(id: UnitId, team: TeamId, pos: HexCoord, hp: u32, vision: u32) -> Self {
        Self {
            id,
            kind: UnitKind::Core,
            team,
            pos,
            hp,
            max_hp: hp,
            ap: 0,
            max_ap: 0,
            initiative: 0,
            attack_damage: 0,
            attack_range: 0,
            vision_range: vision,
            energy: 0,
            max_energy: 0,
            energy_regen: 0,
            cooldowns: HashMap::new(),
            statuses: Vec::new(),
            hero_id: None,
            lane_id: None,
            waypoint_index: None,
            aggro_range: 0,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 0,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    /// Factory method for the neutral Objective Vault.
    pub fn new_vault(id: UnitId, pos: HexCoord, hp: u32) -> Self {
        Self {
            id,
            kind: UnitKind::Objective,
            team: TEAM_NEUTRAL,
            pos,
            hp,
            max_hp: hp,
            ap: 0,
            max_ap: 0,
            initiative: 0,
            attack_damage: 0,
            attack_range: 0,
            vision_range: 2,
            energy: 0,
            max_energy: 0,
            energy_regen: 0,
            cooldowns: HashMap::new(),
            statuses: Vec::new(),
            hero_id: None,
            lane_id: None,
            waypoint_index: None,
            aggro_range: 0,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
            gold: 0,
            xp: 0,
            level: 1,
            items: Vec::new(),
            life_state: LifeState::Alive,
            respawn_rounds: None,
            death_pos: None,
        }
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        matches!(self.life_state, LifeState::Alive) && self.hp > 0
    }

    #[inline]
    pub fn is_dead_awaiting_respawn(&self) -> bool {
        matches!(self.life_state, LifeState::DeadAwaitingRespawn { .. })
    }

    #[inline]
    pub fn is_structure(&self) -> bool {
        self.kind.is_structure()
    }

    #[inline]
    pub fn is_hero(&self) -> bool {
        matches!(self.kind, UnitKind::Hero)
    }

    #[inline]
    pub fn is_stationary(&self) -> bool {
        self.kind.is_stationary()
    }

    #[inline]
    pub fn reset_ap(&mut self) {
        self.ap = self.max_ap;
    }

    #[inline]
    pub fn regenerate_energy(&mut self) {
        self.energy = (self.energy + self.energy_regen).min(self.max_energy);
    }

    #[inline]
    pub fn decrement_cooldowns(&mut self) {
        for cooldown in self.cooldowns.values_mut() {
            *cooldown = cooldown.saturating_sub(1);
        }
    }

    #[inline]
    pub fn is_spell_ready(&self, spell_id: &str) -> bool {
        self.cooldowns.get(spell_id).copied().unwrap_or(u32::MAX) == 0
    }

    #[inline]
    pub fn trigger_cooldown(&mut self, spell_id: &str, duration: u32) {
        self.cooldowns.insert(spell_id.to_string(), duration);
    }

    #[inline]
    pub fn can_afford(&self, cost: u32) -> bool {
        self.ap >= cost
    }

    #[inline]
    pub fn spend_ap(&mut self, cost: u32) -> bool {
        if self.ap >= cost {
            self.ap -= cost;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn spend_energy(&mut self, cost: u32) -> bool {
        if self.energy >= cost {
            self.energy -= cost;
            true
        } else {
            false
        }
    }

    // Dynamic Effective Stats
    pub fn effective_attack_damage(&self) -> u32 {
        let mut damage = self.attack_damage as i32;
        for status in &self.statuses {
            for modifier in &status.modifiers {
                if modifier.stat == StatKind::AttackDamage {
                    damage += modifier.value;
                }
            }
        }
        damage.max(0) as u32
    }

    pub fn effective_vision_range(&self) -> u32 {
        let mut range = self.vision_range as i32;
        for status in &self.statuses {
            for modifier in &status.modifiers {
                if modifier.stat == StatKind::VisionRange {
                    range += modifier.value;
                }
            }
        }
        range.max(0) as u32
    }

    pub fn effective_attack_range(&self) -> u32 {
        let mut range = self.attack_range as i32;
        for status in &self.statuses {
            for modifier in &status.modifiers {
                if modifier.stat == StatKind::AttackRange {
                    range += modifier.value;
                }
            }
        }
        range.max(0) as u32
    }

    /// Check if spawner is ready to release a wave.
    pub fn should_spawn(&self) -> bool {
        if let Some(interval) = self.spawn_interval {
            self.spawn_counter >= interval
        } else {
            false
        }
    }

    /// Advance spawn counter; does not reset automatically to protect against wave drops.
    pub fn tick_counter(&mut self) {
        if self.spawn_interval.is_some() {
            self.spawn_counter += 1;
        }
    }

    /// Reset counter after a minion has successfully been placed on the board.
    pub fn mark_spawned(&mut self) {
        self.spawn_counter = 0;
    }
}
