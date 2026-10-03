use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Axial coordinates for hex grid.
/// q = column, r = row
/// Cube constraint: q + r + s = 0 (s is implicit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Distance between two hexes (cube distance).
    pub fn distance(&self, other: &HexCoord) -> u32 {
        let dq = (self.q - other.q).abs();
        let dr = (self.r - other.r).abs();
        let ds = ((-self.q - self.r) - (-other.q - other.r)).abs();
        ((dq + dr + ds) / 2) as u32
    }

    /// Get all 6 neighboring hexes.
    pub fn neighbors(&self) -> [HexCoord; 6] {
        [
            HexCoord::new(self.q + 1, self.r),
            HexCoord::new(self.q - 1, self.r),
            HexCoord::new(self.q, self.r + 1),
            HexCoord::new(self.q, self.r - 1),
            HexCoord::new(self.q + 1, self.r - 1),
            HexCoord::new(self.q - 1, self.r + 1),
        ]
    }

    /// Get all hexes on a ring at the given radius.
    pub fn ring(&self, radius: u32) -> Vec<HexCoord> {
        if radius == 0 {
            return vec![*self];
        }
        let mut results = Vec::new();
        // Start from one direction and walk around
        let directions = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, -1), (-1, 1)];
        // Walk to starting position
        let mut current = HexCoord::new(
            self.q + (directions[4].0 * radius as i32),
            self.r + (directions[4].1 * radius as i32),
        );
        for i in 0..6 {
            for _ in 0..radius {
                results.push(current);
                current = HexCoord::new(
                    current.q + directions[i].0,
                    current.r + directions[i].1,
                );
            }
        }
        results
    }

    /// Get all hexes in a filled radius (for map generation).
    pub fn spiral(&self, radius: u32) -> Vec<HexCoord> {
        let mut results = vec![*self];
        for r in 1..=radius {
            results.extend(self.ring(r));
        }
        results
    }
}

/// The hex map definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexMap {
    pub radius: u32,
    #[serde(default)]
    pub obstacles: HashSet<HexCoord>,
}

impl HexMap {
    pub fn new(radius: u32) -> Self {
        Self {
            radius,
            obstacles: HashSet::new(),
        }
    }

    /// Get all walkable hexes.
    pub fn all_hexes(&self) -> Vec<HexCoord> {
        let center = HexCoord::new(0, 0);
        center.spiral(self.radius)
    }

    /// Check if a hex is walkable.
    pub fn is_walkable(&self, coord: &HexCoord) -> bool {
        let center = HexCoord::new(0, 0);
        center.distance(coord) <= self.radius && !self.obstacles.contains(coord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance() {
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(3, 0);
        assert_eq!(a.distance(&b), 3);
        assert_eq!(a.distance(&a), 0);
        let c = HexCoord::new(-1, 2);
        assert_eq!(a.distance(&c), 2);
    }

    #[test]
    fn test_neighbors() {
        let a = HexCoord::new(0, 0);
        let n = a.neighbors();
        assert_eq!(n.len(), 6);
        for h in &n {
            assert_eq!(a.distance(h), 1);
        }
    }

    #[test]
    fn test_ring_counts() {
        let a = HexCoord::new(0, 0);
        assert_eq!(a.ring(1).len(), 6);
        assert_eq!(a.ring(2).len(), 12);
        assert_eq!(a.ring(3).len(), 18);
    }

    #[test]
    fn test_spiral_count() {
        let a = HexCoord::new(0, 0);
        // radius 4 => 61 hexes
        assert_eq!(a.spiral(4).len(), 61);
    }

    #[test]
    fn test_walkable() {
        let mut map = HexMap::new(2);
        assert!(map.is_walkable(&HexCoord::new(0, 0)));
        assert!(!map.is_walkable(&HexCoord::new(5, 5)));
        map.obstacles.insert(HexCoord::new(1, 0));
        assert!(!map.is_walkable(&HexCoord::new(1, 0)));
    }
}
