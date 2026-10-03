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
}

impl UnitKind {
    #[inline]
    pub fn is_stationary(&self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::Spawner)
    }

    #[inline]
    pub fn is_structure(&self) -> bool {
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
        }
    }
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
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
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
            attack_damage: 20,
            attack_range: 1,
            vision_range: 3,
            energy: 5,
            max_energy: 5,
            energy_regen: 1,
            cooldowns,
            statuses: Vec::new(),
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
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
            lane_id: None,
            waypoint_index: None,
            aggro_range: 4,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
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
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
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
            lane_id: None,
            waypoint_index: None,
            aggro_range: 2,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
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
            lane_id: Some("mid".to_string()),
            waypoint_index: Some(if team == TEAM_0 { 0 } else { 4 }),
            aggro_range: 2,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::TowardEnemy,
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
            lane_id: None,
            waypoint_index: None,
            aggro_range: 3,
            last_attacker: None,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
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
            lane_id: None,
            waypoint_index: None,
            aggro_range: 0,
            last_attacker: None,
            spawn_interval: Some(spawn_interval),
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
        }
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    #[inline]
    pub fn is_structure(&self) -> bool {
        matches!(self.kind, UnitKind::Tower | UnitKind::Spawner)
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
