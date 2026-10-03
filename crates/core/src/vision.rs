use crate::hex::HexCoord;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obstacle {
    pub coords: HexCoord,
    pub blocks_movement: bool,
    pub blocks_vision: bool,
}

impl Obstacle {
    pub fn wall(coords: HexCoord) -> Self {
        Self {
            coords,
            blocks_movement: true,
            blocks_vision: true,
        }
    }

    pub fn smoke(coords: HexCoord) -> Self {
        Self {
            coords,
            blocks_movement: false,
            blocks_vision: true,
        }
    }

    pub fn boulder(coords: HexCoord) -> Self {
        Self {
            coords,
            blocks_movement: true,
            blocks_vision: false,
        }
    }
}

/// Convert fractional cube coordinates back to axial integer coordinates with round-to-nearest
pub fn cube_round(q: f64, r: f64, s: f64) -> HexCoord {
    let mut rq = q.round();
    let mut rr = r.round();
    let rs = s.round();

    let q_diff = (rq - q).abs();
    let r_diff = (rr - r).abs();
    let s_diff = (rs - s).abs();

    if q_diff > r_diff && q_diff > s_diff {
        rq = -rr - rs;
    } else if r_diff > s_diff {
        rr = -rq - rs;
    }

    HexCoord::new(rq as i32, rr as i32)
}

/// Fractional cube raycasting with epsilon offset (Red Blob Games canonical)
pub fn hex_line(from: HexCoord, to: HexCoord) -> Vec<HexCoord> {
    let n = from.distance(&to);
    if n == 0 {
        return vec![from];
    }

    let mut results = Vec::with_capacity(n as usize + 1);

    // Epsilon perturbation guarantees deterministic line traversal avoiding vertex edge jitter
    const EPSILON: f64 = 1e-6;

    let fq = from.q as f64 + EPSILON;
    let fr = from.r as f64 + EPSILON;
    let fs = (-from.q - from.r) as f64 - 2.0 * EPSILON;

    let tq = to.q as f64 + EPSILON;
    let tr = to.r as f64 + EPSILON;
    let ts = (-to.q - to.r) as f64 - 2.0 * EPSILON;

    for i in 0..=n {
        let t = i as f64 / n as f64;
        let q = fq + (tq - fq) * t;
        let r = fr + (tr - fr) * t;
        let s = fs + (ts - fs) * t;
        results.push(cube_round(q, r, s));
    }

    results
}

/// Verify unobstructed line of sight between two hexes
pub fn has_line_of_sight(
    vision_blockers: &HashSet<HexCoord>,
    from: HexCoord,
    to: HexCoord,
) -> bool {
    if from == to {
        return true;
    }

    let line = hex_line(from, to);

    // Skip the source hex (index 0) and destination hex (last index)
    // Vision blockers only obstruct if they are strictly between source and target
    if line.len() <= 2 {
        return true;
    }

    for hex in &line[1..line.len() - 1] {
        if vision_blockers.contains(hex) {
            return false;
        }
    }

    true
}

/// Compute complete LOS fog of war for a team
pub fn compute_team_los_fog(
    vision_blockers: &HashSet<HexCoord>,
    map_radius: u32,
    all_units: &[crate::unit::Unit],
    team: crate::unit::TeamId,
) -> HashSet<HexCoord> {
    let mut visible_hexes = HashSet::new();

    for unit in all_units {
        if unit.team != team || !unit.is_alive() {
            continue;
        }

        // Unit's own tile is always visible
        visible_hexes.insert(unit.pos);

        let vision_range = unit.effective_vision_range();
        let candidate_hexes = unit.pos.spiral(vision_range);

        for hex in candidate_hexes {
            if hex.distance(&HexCoord::new(0, 0)) > map_radius {
                continue;
            }

            if has_line_of_sight(vision_blockers, unit.pos, hex) {
                visible_hexes.insert(hex);
            }
        }
    }

    visible_hexes
}
