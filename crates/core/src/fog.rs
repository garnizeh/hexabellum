use crate::hex::{HexCoord, HexMap};
use crate::unit::{TeamId, Unit, UnitId};
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

    /// Recompute visibility sets for all teams based on map walkability and living unit positions.
    pub fn update(&mut self, map: &HexMap, units: &HashMap<UnitId, Unit>) {
        for team in 0..self.visible.len() {
            self.visible[team].clear();

            for unit in units.values() {
                if unit.team != team as TeamId || !unit.is_alive() {
                    continue;
                }

                // Add all walkable hexes within axial vision radius
                for hex in unit.pos.spiral(unit.vision_range) {
                    if map.is_walkable(&hex) {
                        self.visible[team].insert(hex);
                    }
                }
            }
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
