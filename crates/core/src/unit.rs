use crate::hex::HexCoord;
use serde::{Deserialize, Serialize};

pub type UnitId = u64;
pub type TeamId = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    Hero,
    Minion,
    Tower,
    SpawnerTower,
    Neutral,
}

impl UnitKind {
    #[inline]
    pub fn is_stationary(&self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::SpawnerTower)
    }

    #[inline]
    pub fn is_structure(&self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::SpawnerTower)
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
    pub ap: u32,
    pub max_ap: u32,
    pub initiative: u32,
    pub attack_damage: u32,
    pub attack_range: u32,

    // Phase 2 MOBA Additions
    pub vision_range: u32,
    pub spawn_interval: Option<u32>,
    pub spawn_counter: u32,
    pub lane_direction: LaneDirection,
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
            hp: 30,
            max_hp: 30,
            ap: 2,
            max_ap: 2,
            initiative: 1,
            attack_damage: 8,
            attack_range: 1,
            vision_range: 2,
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
            hp: 200,
            max_hp: 200,
            ap: 1,
            max_ap: 1,
            initiative: 5, // Towers fire early in initiative order
            attack_damage: 30,
            attack_range: 3,
            vision_range: 4,
            spawn_interval: None,
            spawn_counter: 0,
            lane_direction: LaneDirection::None,
        }
    }

    /// Spawner Tower: key base structure producing minion waves.
    pub fn new_spawner(id: UnitId, team: TeamId, pos: HexCoord, spawn_interval: u32) -> Self {
        Self {
            id,
            kind: UnitKind::SpawnerTower,
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
    pub fn is_stationary(&self) -> bool {
        self.kind.is_stationary()
    }

    #[inline]
    pub fn reset_ap(&mut self) {
        self.ap = self.max_ap;
    }

    #[inline]
    pub fn can_afford(&self, cost: u32) -> bool {
        self.ap >= cost
    }

    #[inline]
    pub fn spend_ap(&mut self, cost: u32) {
        self.ap = self.ap.saturating_sub(cost);
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
