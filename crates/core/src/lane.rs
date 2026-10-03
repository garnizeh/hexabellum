use crate::hex::HexCoord;
use crate::unit::{TeamId, Unit, TEAM_0};

#[derive(Debug, Clone)]
pub struct LaneDef {
    pub id: String,
    pub waypoints: Vec<HexCoord>,
}

impl LaneDef {
    pub fn central_lane() -> Self {
        Self {
            id: "mid".to_string(),
            waypoints: vec![
                HexCoord::new(-5, 0), // T0 Spawner
                HexCoord::new(-3, 0), // T0 Tower
                HexCoord::new(0, 0),  // Mid Contest Point
                HexCoord::new(3, 0),  // T1 Tower
                HexCoord::new(5, 0),  // T1 Spawner
            ],
        }
    }

    pub fn start_waypoint_index_for_team(&self, team: TeamId) -> usize {
        if team == TEAM_0 {
            0
        } else {
            self.waypoints.len().saturating_sub(1)
        }
    }

    pub fn next_waypoint_index(&self, current: usize, team: TeamId) -> usize {
        if team == TEAM_0 {
            (current + 1).min(self.waypoints.len().saturating_sub(1))
        } else {
            current.saturating_sub(1)
        }
    }

    pub fn is_at_final_waypoint(&self, current: usize, team: TeamId) -> bool {
        if team == TEAM_0 {
            current >= self.waypoints.len().saturating_sub(1)
        } else {
            current == 0
        }
    }

    /// Progress waypoint if within 1 hex of current waypoint
    pub fn update_minion_waypoint(&self, minion: &mut Unit) {
        if let Some(current_idx) = minion.waypoint_index {
            let target_wp = self.waypoints[current_idx];
            if minion.pos.distance(&target_wp) <= 1 {
                if !self.is_at_final_waypoint(current_idx, minion.team) {
                    minion.waypoint_index = Some(self.next_waypoint_index(current_idx, minion.team));
                }
            }
        }
    }
}
