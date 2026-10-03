use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::state::GameState;
use crate::unit::{Unit, UnitId, UnitKind};

pub struct SpawnerSystem;

impl SpawnerSystem {
    /// Advance spawners, determine wave readiness, and spawn minions on free lane-biased hexes.
    pub fn process_spawns(state: &mut GameState) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut new_units: Vec<Unit> = Vec::new();

        // Collect living spawner IDs deterministically
        let mut spawner_ids: Vec<UnitId> = state
            .units
            .values()
            .filter(|u| u.kind == UnitKind::SpawnerTower && u.is_alive())
            .map(|u| u.id)
            .collect();
        spawner_ids.sort_unstable();

        for spawner_id in spawner_ids {
            let spawner = state.get_unit_mut(spawner_id).unwrap();
            spawner.tick_counter();

            if !spawner.should_spawn() {
                continue;
            }

            let team = spawner.team;
            let spawner_pos = spawner.pos;

            // Find a free walkable hex adjacent to the spawner, sorted toward enemy side
            let spawn_pos = Self::find_spawn_hex(state, spawner_pos, team);

            if let Some(pos) = spawn_pos {
                // Confirm spawn and reset counter
                let spawner = state.get_unit_mut(spawner_id).unwrap();
                spawner.mark_spawned();

                let new_id = state.alloc_unit_id();
                let minion = Unit::new_minion(new_id, team, pos);
                new_units.push(minion);

                events.push(GameEvent::UnitSpawned {
                    unit_id: new_id,
                    unit_kind: UnitKind::Minion,
                    team,
                    pos,
                    spawner_id,
                });
            }
            // If all candidate hexes are blocked, counter is NOT reset; retries next round!
        }

        for unit in new_units {
            state.add_unit(unit);
        }

        events
    }

    /// Select optimal spawn hex adjacent to spawner with lane advancement bias.
    fn find_spawn_hex(state: &GameState, center: HexCoord, team: u8) -> Option<HexCoord> {
        let occupied = state.occupied_hexes();

        // 1. Try ring 1
        let mut ring1: Vec<HexCoord> = center
            .neighbors()
            .into_iter()
            .filter(|h| state.map.is_walkable(h) && !occupied.contains(h))
            .collect();

        if !ring1.is_empty() {
            // Sort by lane progress: Team 0 wants highest q (east); Team 1 wants lowest q (west)
            ring1.sort_by(|a, b| {
                if team == 0 {
                    b.q.cmp(&a.q).then_with(|| a.r.cmp(&b.r))
                } else {
                    a.q.cmp(&b.q).then_with(|| a.r.cmp(&b.r))
                }
            });
            return ring1.first().copied();
        }

        // 2. Fallback to ring 2 if ring 1 is completely congested
        let mut ring2: Vec<HexCoord> = center
            .ring(2)
            .into_iter()
            .filter(|h| state.map.is_walkable(h) && !occupied.contains(h))
            .collect();

        if !ring2.is_empty() {
            ring2.sort_by(|a, b| {
                if team == 0 {
                    b.q.cmp(&a.q).then_with(|| a.r.cmp(&b.r))
                } else {
                    a.q.cmp(&b.q).then_with(|| a.r.cmp(&b.r))
                }
            });
            return ring2.first().copied();
        }

        None
    }
}
