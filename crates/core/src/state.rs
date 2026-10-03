use crate::hex::{HexCoord, HexMap};
use crate::unit::{TeamId, Unit, UnitId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub winner: Option<TeamId>,
}

impl GameState {
    pub fn new(map: HexMap) -> Self {
        Self {
            round: 0,
            phase: Phase::Planning,
            map,
            units: HashMap::new(),
            winner: None,
        }
    }

    pub fn add_unit(&mut self, unit: Unit) {
        self.units.insert(unit.id, unit);
    }

    pub fn get_unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(&id)
    }

    pub fn get_unit_at(&self, coord: &HexCoord) -> Option<&Unit> {
        self.units.values().find(|u| u.pos == *coord && u.is_alive())
    }

    /// Check if a hex is occupied by any alive unit.
    pub fn is_occupied(&self, coord: &HexCoord) -> bool {
        self.get_unit_at(coord).is_some()
    }

    /// Get all alive units.
    pub fn alive_units(&self) -> Vec<&Unit> {
        self.units.values().filter(|u| u.is_alive()).collect()
    }

    /// Get all alive units for a team.
    pub fn team_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units
            .values()
            .filter(|u| u.team == team && u.is_alive())
            .collect()
    }
}
