use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Axial coordinates for hex grid.
/// q = column, r = row
/// Cube constraint: q + r + s = 0 (s is implicit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

/// The 6 axial unit directions in counter-clockwise cyclic order (pointy-top orientation).
pub const DIRECTIONS: [(i32, i32); 6] = [
    (1, 0),  // 0: East
    (0, 1),  // 1: South-East
    (-1, 1), // 2: South-West
    (-1, 0), // 3: West
    (0, -1), // 4: North-West
    (1, -1), // 5: North-East
];

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
            HexCoord::new(self.q + DIRECTIONS[0].0, self.r + DIRECTIONS[0].1),
            HexCoord::new(self.q + DIRECTIONS[1].0, self.r + DIRECTIONS[1].1),
            HexCoord::new(self.q + DIRECTIONS[2].0, self.r + DIRECTIONS[2].1),
            HexCoord::new(self.q + DIRECTIONS[3].0, self.r + DIRECTIONS[3].1),
            HexCoord::new(self.q + DIRECTIONS[4].0, self.r + DIRECTIONS[4].1),
            HexCoord::new(self.q + DIRECTIONS[5].0, self.r + DIRECTIONS[5].1),
        ]
    }

    /// Get all hexes on a ring at the given radius.
    /// Traverses the 6 sides cyclically, yielding exactly 6 * radius distinct hexes.
    pub fn ring(&self, radius: u32) -> Vec<HexCoord> {
        if radius == 0 {
            return vec![*self];
        }
        let mut results = Vec::with_capacity((6 * radius) as usize);
        // Start at corner: self + DIRECTIONS[4] * radius (North-West)
        let mut current = HexCoord::new(
            self.q + (DIRECTIONS[4].0 * radius as i32),
            self.r + (DIRECTIONS[4].1 * radius as i32),
        );
        for dir in DIRECTIONS {
            for _ in 0..radius {
                results.push(current);
                current = HexCoord::new(current.q + dir.0, current.r + dir.1);
            }
        }
        results
    }

    /// Get all hexes in a filled radius (for map generation).
    pub fn spiral(&self, radius: u32) -> Vec<HexCoord> {
        let count = 1 + 3 * radius * (radius + 1);
        let mut results = Vec::with_capacity(count as usize);
        results.push(*self);
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
    #[serde(default)]
    pub obstacle_defs: Vec<crate::vision::Obstacle>,
}

impl HexMap {
    pub fn new(radius: u32) -> Self {
        Self {
            radius,
            obstacles: HashSet::new(),
            obstacle_defs: Vec::new(),
        }
    }

    /// Add an obstacle definition to the map.
    pub fn add_obstacle(&mut self, obs: crate::vision::Obstacle) {
        if obs.blocks_movement {
            self.obstacles.insert(obs.coords);
        }
        self.obstacle_defs.push(obs);
    }

    /// Set of coordinates that block vision.
    pub fn vision_blockers(&self) -> HashSet<HexCoord> {
        let mut set = HashSet::new();
        if self.obstacle_defs.is_empty() {
            set.extend(&self.obstacles);
        } else {
            for obs in &self.obstacle_defs {
                if obs.blocks_vision {
                    set.insert(obs.coords);
                }
            }
        }
        set
    }

    /// Set of coordinates that block movement.
    pub fn movement_blockers(&self) -> HashSet<HexCoord> {
        let mut set = self.obstacles.clone();
        for obs in &self.obstacle_defs {
            if obs.blocks_movement {
                set.insert(obs.coords);
            }
        }
        set
    }

    /// Get all walkable hexes.
    pub fn all_hexes(&self) -> Vec<HexCoord> {
        let center = HexCoord::new(0, 0);
        center.spiral(self.radius)
    }

    /// Check if a hex is walkable.
    pub fn is_walkable(&self, coord: &HexCoord) -> bool {
        let center = HexCoord::new(0, 0);
        coord.distance(&center) <= self.radius && !self.movement_blockers().contains(coord)
    }

    /// Find the shortest walkable path from start to goal (BFS over the
    /// graph of walkable hexes).
    ///
    /// Path length is measured on the map geometry only — unit occupancy
    /// never changes how far two hexes are apart, it only decides whether a
    /// unit may *enter* a given hex at a given moment. Occupancy is therefore
    /// not a parameter here; callers that need "path around other units"
    /// semantics must check per-step legality themselves (see
    /// `TurnProcessor`). Returning None means no walkable path exists at all
    /// (e.g. the goal is off-map or blocked by an obstacle).
    /// Returns None if no path exists.
    pub fn find_path(&self, start: HexCoord, goal: HexCoord) -> Option<Vec<HexCoord>> {
        if start == goal {
            return Some(vec![start]);
        }

        if !self.is_walkable(&start) || !self.is_walkable(&goal) {
            return None;
        }

        // Deterministic BFS with lexicographic (q, r) neighbor ordering so
        // tie-broken paths are reproducible.
        let mut came_from: HashMap<HexCoord, HexCoord> = HashMap::new();
        let mut visited: HashSet<HexCoord> = HashSet::new();
        let mut queue: std::collections::VecDeque<HexCoord> = std::collections::VecDeque::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let mut nbrs: Vec<HexCoord> = current
                .neighbors()
                .into_iter()
                .filter(|n| self.is_walkable(n) && !visited.contains(n))
                .collect();
            nbrs.sort_by_key(|h| (h.q, h.r));

            for neighbor in nbrs {
                visited.insert(neighbor);
                came_from.insert(neighbor, current);
                if neighbor == goal {
                    // Reconstruct path
                    let mut path = vec![goal];
                    let mut node = goal;
                    while let Some(&prev) = came_from.get(&node) {
                        path.push(prev);
                        node = prev;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(neighbor);
            }
        }

        None // No path found
    }

    /// Get all hexes reachable from start within given AP budget.
    /// Returns a map of hex -> AP cost to reach it.
    /// Uses Dijkstra's algorithm (all edges cost 1).
    pub fn reachable_hexes(
        &self,
        start: HexCoord,
        ap_budget: u32,
        occupied: &HashSet<HexCoord>,
    ) -> HashMap<HexCoord, u32> {
        let mut result: HashMap<HexCoord, u32> = HashMap::new();
        let mut frontier: Vec<(HexCoord, u32)> = vec![(start, 0)];
        result.insert(start, 0);

        while let Some((current, cost)) = frontier.pop() {
            if cost >= ap_budget {
                continue;
            }

            for neighbor in current.neighbors() {
                if !self.is_walkable(&neighbor) {
                    continue;
                }
                if occupied.contains(&neighbor) {
                    continue;
                }

                let new_cost = cost + 1;
                if new_cost <= ap_budget {
                    let existing = result.get(&neighbor).copied().unwrap_or(u32::MAX);
                    if new_cost < existing {
                        result.insert(neighbor, new_cost);
                        frontier.push((neighbor, new_cost));
                    }
                }
            }
        }

        result.remove(&start); // Don't include starting position
        result
    }

    /// Get hexes within attack range of a position.
    pub fn hexes_in_range(&self, center: HexCoord, range: u32) -> Vec<HexCoord> {
        let mut results = Vec::new();
        for r in 1..=range {
            results.extend(center.ring(r));
        }
        results.retain(|h| self.is_walkable(h));
        results
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

        for r in 1..=4 {
            let ring = a.ring(r);
            let unique: HashSet<_> = ring.iter().copied().collect();
            assert_eq!(unique.len(), (6 * r) as usize);
            for h in &ring {
                assert_eq!(a.distance(h), r);
            }
        }
    }

    #[test]
    fn test_spiral_count() {
        let a = HexCoord::new(0, 0);
        // radius 4 => 61 hexes
        assert_eq!(a.spiral(4).len(), 61);
        let unique4: HashSet<_> = a.spiral(4).into_iter().collect();
        assert_eq!(unique4.len(), 61);

        // radius 5 => 91 hexes
        assert_eq!(a.spiral(5).len(), 91);
        let unique5: HashSet<_> = a.spiral(5).into_iter().collect();
        assert_eq!(unique5.len(), 91);
    }

    #[test]
    fn test_walkable() {
        let mut map = HexMap::new(2);
        assert!(map.is_walkable(&HexCoord::new(0, 0)));
        assert!(!map.is_walkable(&HexCoord::new(5, 5)));
        map.obstacles.insert(HexCoord::new(1, 0));
        assert!(!map.is_walkable(&HexCoord::new(1, 0)));
    }

    #[test]
    fn test_find_path_straight() {
        let map = HexMap::new(4);
        let path = map
            .find_path(HexCoord::new(-3, 0), HexCoord::new(0, 0))
            .unwrap();
        // Shortest path length = distance + 1 (includes start)
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], HexCoord::new(-3, 0));
        assert_eq!(*path.last().unwrap(), HexCoord::new(0, 0));
        // Each step must be a neighbor of the previous
        for w in path.windows(2) {
            assert_eq!(w[0].distance(&w[1]), 1);
        }
    }

    #[test]
    fn test_find_path_same_start_goal() {
        let map = HexMap::new(4);
        let path = map
            .find_path(HexCoord::new(1, 1), HexCoord::new(1, 1))
            .unwrap();
        assert_eq!(path, vec![HexCoord::new(1, 1)]);
    }

    #[test]
    fn test_find_path_blocked_by_obstacle() {
        let mut map = HexMap::new(1);
        // Block everything except center and one hex
        for h in map.all_hexes() {
            if h != HexCoord::new(0, 0) && h != HexCoord::new(1, 0) {
                map.obstacles.insert(h);
            }
        }
        assert!(
            map.find_path(HexCoord::new(0, 0), HexCoord::new(1, 0))
                .is_some()
        );

        // Goal unreachable because every route is blocked by obstacles
        map.obstacles.insert(HexCoord::new(1, 0));
        assert!(
            map.find_path(HexCoord::new(0, 0), HexCoord::new(1, 0))
                .is_none()
        );
    }

    #[test]
    fn test_reachable_hexes_ap_budget() {
        let map = HexMap::new(4);
        let occupied = HashSet::new();
        let start = HexCoord::new(0, 0);
        let reachable = map.reachable_hexes(start, 2, &occupied);
        // Doesn't include start
        assert!(!reachable.contains_key(&start));
        // All costs within budget
        for cost in reachable.values() {
            assert!(*cost >= 1 && *cost <= 2);
        }
        // A hex at distance 3 is not reachable with AP 2
        assert!(!reachable.contains_key(&HexCoord::new(3, 0)));
        // Adjacent hex reachable with cost 1
        assert_eq!(reachable.get(&HexCoord::new(1, 0)), Some(&1));
    }

    #[test]
    fn test_reachable_respects_occupied() {
        let map = HexMap::new(4);
        let mut occupied = HashSet::new();
        occupied.insert(HexCoord::new(1, 0));
        let reachable = map.reachable_hexes(HexCoord::new(0, 0), 1, &occupied);
        assert!(!reachable.contains_key(&HexCoord::new(1, 0)));
    }

    #[test]
    fn test_hexes_in_range() {
        let map = HexMap::new(4);
        let in_range = map.hexes_in_range(HexCoord::new(0, 0), 1);
        assert_eq!(in_range.len(), 6);
    }
}
