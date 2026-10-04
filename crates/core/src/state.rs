use crate::fog::FogState;
use crate::hex::{HexCoord, HexMap};
use crate::unit::{TeamId, Unit, UnitId, UnitKind};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Planning,
    Resolution,
    MatchEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub round: u32,
    pub phase: Phase,
    pub map: HexMap,
    pub units: HashMap<UnitId, Unit>,
    pub fog: FogState,
    pub winner: Option<TeamId>,
    pub next_unit_id: UnitId,
    pub spawners_initialized: bool,
    #[serde(default)]
    pub neutral_camps: Vec<crate::neutral::NeutralCamp>,
}

impl GameState {
    pub fn new(map: HexMap) -> Self {
        Self {
            round: 0,
            phase: Phase::Planning,
            map,
            units: HashMap::new(),
            fog: FogState::new(2),
            winner: None,
            next_unit_id: 1,
            spawners_initialized: false,
            neutral_camps: Vec::new(),
        }
    }

    pub fn add_unit(&mut self, unit: Unit) {
        if unit.kind == UnitKind::Spawner {
            self.spawners_initialized = true;
        }
        self.next_unit_id = self.next_unit_id.max(unit.id + 1);
        self.units.insert(unit.id, unit);
    }

    #[inline]
    pub fn get_unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(&id)
    }

    #[inline]
    pub fn get_unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.get_mut(&id)
    }

    pub fn get_unit_at(&self, coord: &HexCoord) -> Option<&Unit> {
        self.units
            .values()
            .find(|u| u.pos == *coord && u.is_alive())
    }

    /// Check if a hex is occupied by any alive unit.
    pub fn is_occupied(&self, coord: &HexCoord) -> bool {
        self.get_unit_at(coord).is_some()
    }

    pub fn occupied_hexes(&self) -> HashSet<HexCoord> {
        self.units
            .values()
            .filter(|u| u.is_alive())
            .map(|u| u.pos)
            .collect()
    }

    pub fn alive_units(&self) -> Vec<&Unit> {
        let mut list: Vec<&Unit> = self.units.values().filter(|u| u.is_alive()).collect();
        list.sort_by_key(|u| u.id);
        list
    }

    pub fn team_units(&self, team: TeamId) -> Vec<&Unit> {
        let mut list: Vec<&Unit> = self
            .units
            .values()
            .filter(|u| u.team == team && u.is_alive())
            .collect();
        list.sort_by_key(|u| u.id);
        list
    }

    pub fn enemy_units(&self, team: TeamId) -> Vec<&Unit> {
        let mut list: Vec<&Unit> = self
            .units
            .values()
            .filter(|u| u.team != team && u.is_alive())
            .collect();
        list.sort_by_key(|u| u.id);
        list
    }

    pub fn visible_enemy_units(&self, team: TeamId) -> Vec<&Unit> {
        self.enemy_units(team)
            .into_iter()
            .filter(|u| self.fog.is_visible(team, &u.pos))
            .collect()
    }

    pub fn team_has_heroes(&self, team: TeamId) -> bool {
        self.units.values().any(|u| {
            u.team == team && u.is_alive() && u.kind == UnitKind::Hero
        })
    }

    pub fn team_has_spawner(&self, team: TeamId) -> bool {
        self.units.values().any(|u| {
            u.team == team && u.is_alive() && u.kind == UnitKind::Spawner
        })
    }

    /// Evaluates match winner. If Cores exist (Phase 7), Core destruction governs victory.
    /// Otherwise, falls back to legacy dual check (All heroes dead OR Spawner Tower destroyed).
    pub fn check_winner(&self) -> Option<TeamId> {
        let cores: Vec<&Unit> = self.units.values().filter(|u| u.kind == UnitKind::Core).collect();
        if !cores.is_empty() {
            let team0_core_alive = cores.iter().any(|u| u.team == 0 && u.is_alive());
            let team1_core_alive = cores.iter().any(|u| u.team == 1 && u.is_alive());

            if !team0_core_alive && !team1_core_alive {
                return None; // Draw
            } else if !team0_core_alive {
                return Some(1);
            } else if !team1_core_alive {
                return Some(0);
            } else {
                return None;
            }
        }

        let team0_heroes = self.team_has_heroes(0);
        let team1_heroes = self.team_has_heroes(1);
        let team0_spawner = self.team_has_spawner(0);
        let team1_spawner = self.team_has_spawner(1);

        let team0_lost = !team0_heroes || (self.spawners_initialized && !team0_spawner);
        let team1_lost = !team1_heroes || (self.spawners_initialized && !team1_spawner);

        if team0_lost && team1_lost {
            None // Draw
        } else if team0_lost {
            Some(1)
        } else if team1_lost {
            Some(0)
        } else {
            None
        }
    }

    pub fn reset_all_ap(&mut self) {
        for unit in self.units.values_mut() {
            if unit.is_alive() {
                unit.reset_ap();
            }
        }
    }

    pub fn update_fog(&mut self) {
        self.fog.update(&self.map, &self.units);
    }

    pub fn alloc_unit_id(&mut self) -> UnitId {
        let id = self.next_unit_id;
        self.next_unit_id += 1;
        id
    }
}
