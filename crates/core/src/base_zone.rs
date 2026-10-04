use crate::hex::HexCoord;
use crate::unit::TeamId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseZone {
    pub team: TeamId,
    pub center: HexCoord,
    pub radius: u32,
}

impl BaseZone {
    pub fn new(team: TeamId, center: HexCoord, radius: u32) -> Self {
        Self { team, center, radius }
    }

    /// Verifies whether a given coordinate falls within this sovereign base territory.
    pub fn contains(&self, coord: HexCoord) -> bool {
        self.center.distance(&coord) <= self.radius
    }

    /// Generates all valid candidate hexes in this base zone sorted deterministically
    /// for hero respawn placement:
    /// 1. Distance from Core ascending
    /// 2. Axial q ascending
    /// 3. Axial r ascending
    pub fn candidate_spawn_hexes(&self) -> Vec<HexCoord> {
        let mut candidates = Vec::new();
        let r = self.radius as i32;

        for q in -r..=r {
            let r1 = (-r).max(-q - r);
            let r2 = r.min(-q + r);
            for r_coord in r1..=r2 {
                candidates.push(HexCoord::new(self.center.q + q, self.center.r + r_coord));
            }
        }

        let center = self.center;
        candidates.sort_by(|a, b| {
            let dist_a = center.distance(a);
            let dist_b = center.distance(b);
            dist_a
                .cmp(&dist_b)
                .then_with(|| a.q.cmp(&b.q))
                .then_with(|| a.r.cmp(&b.r))
        });

        candidates
    }
}
