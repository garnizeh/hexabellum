use crate::hex::HexCoord;
use crate::hex::HexMap;
use crate::unit::{TeamId, Unit, UnitId};
use crate::vision::compute_team_los_fog;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Fog of War visibility tracker for all participating teams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FogState {
    /// Vector of visible hex sets indexed by TeamId.
    pub visible: Vec<HashSet<HexCoord>>,
}

impl FogState {
    pub fn new(team_count: usize) -> Self {
        Self {
            visible: vec![HashSet::new(); team_count],
        }
    }

    /// Recompute visibility sets for all teams based on Line of Sight raycasting.
    pub fn update(&mut self, map: &HexMap, units: &HashMap<UnitId, Unit>) {
        let blockers = map.vision_blockers();
        let unit_list: Vec<Unit> = units.values().cloned().collect();
        for team in 0..self.visible.len() {
            self.visible[team] = compute_team_los_fog(
                &blockers,
                map.radius,
                &unit_list,
                team as TeamId,
            );
        }
    }

    pub fn set_team_visibility(&mut self, team: TeamId, hexes: HashSet<HexCoord>) {
        if (team as usize) < self.visible.len() {
            self.visible[team as usize] = hexes;
        }
    }

    /// Check if a specific hex coordinate is visible to a given team.
    #[inline]
    pub fn is_visible(&self, team: TeamId, hex: &HexCoord) -> bool {
        self.visible
            .get(team as usize)
            .map(|set| set.contains(hex))
            .unwrap_or(false)
    }

    /// Borrow visible hexes for a given team.
    #[inline]
    pub fn visible_hexes(&self, team: TeamId) -> &HashSet<HexCoord> {
        &self.visible[team as usize]
    }
}

pub struct TeamFogEngine;

impl TeamFogEngine {
    /// Computes the set of all hexes visible to the specified team.
    /// Uses cube-coordinate line-of-sight raycasting through terrain obstacles.
    pub fn compute_team_vision(state: &crate::state::GameState, team: TeamId) -> HashSet<HexCoord> {
        let blockers = state.map.vision_blockers();
        let unit_list: Vec<Unit> = state.units.values().cloned().collect();
        compute_team_los_fog(&blockers, state.map.radius, &unit_list, team)
    }
}

