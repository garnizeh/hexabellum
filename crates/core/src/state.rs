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
    pub winner: Option<TeamId>,
    pub next_unit_id: UnitId,
}

impl GameState {
    pub fn new(map: HexMap) -> Self {
        Self {
            round: 0,
            phase: Phase::Planning,
            map,
            units: HashMap::new(),
            winner: None,
            next_unit_id: 1,
        }
    }

    pub fn add_unit(&mut self, unit: Unit) {
        // Keep next_unit_id ahead of any inserted id
        self.next_unit_id = self.next_unit_id.max(unit.id + 1);
        self.units.insert(unit.id, unit);
    }

    pub fn get_unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(&id)
    }

    pub fn get_unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.get_mut(&id)
    }

    pub fn get_unit_at(&self, coord: &HexCoord) -> Option<&Unit> {
        self.units.values().find(|u| u.pos == *coord && u.is_alive())
    }

    /// Check if a hex is occupied by any alive unit.
    pub fn is_occupied(&self, coord: &HexCoord) -> bool {
        self.get_unit_at(coord).is_some()
    }

    /// Get all occupied hexes (for pathfinding).
    pub fn occupied_hexes(&self) -> HashSet<HexCoord> {
        self.units
            .values()
            .filter(|u| u.is_alive())
            .map(|u| u.pos)
            .collect()
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

    /// Get all alive enemy units for a given team.
    pub fn enemy_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units
            .values()
            .filter(|u| u.team != team && u.is_alive())
            .collect()
    }

    /// Check if a team has any alive heroes.
    pub fn team_has_heroes(&self, team: TeamId) -> bool {
        self.units.values().any(|u| {
            u.team == team && u.is_alive() && u.kind == UnitKind::Hero
        })
    }

    /// Check win condition.
    pub fn check_winner(&self) -> Option<TeamId> {
        let team0_alive = self.team_has_heroes(0);
        let team1_alive = self.team_has_heroes(1);

        if !team0_alive && !team1_alive {
            None // Draw - shouldn't happen in normal play
        } else if !team0_alive {
            Some(1) // Team 1 wins
        } else if !team1_alive {
            Some(0) // Team 0 wins
        } else {
            None
        }
    }

    /// Reset AP for all units at start of round.
    pub fn reset_all_ap(&mut self) {
        for unit in self.units.values_mut() {
            if unit.is_alive() {
                unit.reset_ap();
            }
        }
    }
}
