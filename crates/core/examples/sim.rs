use hexabellum_core::event::GameEvent;
use hexabellum_core::hex::HexCoord;
use hexabellum_core::orders::{Action, UnitOrder};
use hexabellum_core::state::Phase;
use hexabellum_core::unit::UnitId;
use hexabellum_core::*;

fn main() {
    let mut engine = GameEngine::new();
    for _round in 0..12 {
        // Use the same order-setting API a human would, but print internals.
        let hero_ids: Vec<UnitId> = engine
            .state()
            .team_units(PLAYER_TEAM)
            .iter()
            .map(|u| u.id)
            .collect();
        for id in hero_ids {
            let pos = engine.state().get_unit(id).unwrap().pos;
            let enemies: Vec<(u32, UnitId)> = engine
                .state()
                .enemy_units(PLAYER_TEAM)
                .iter()
                .map(|e| (pos.distance(&e.pos), e.id))
                .collect();
            let (_d, target_id) = *enemies.iter().min().unwrap();
            if engine.set_attack_order(id, target_id) {
                continue;
            }
            let tgt_pos = engine.state().get_unit(target_id).unwrap().pos;
            let mut planned = false;
            let neighbors = tgt_pos.neighbors();
            let mut adj: Vec<&HexCoord> = neighbors
                .iter()
                .filter(|h| engine.state().map.is_walkable(h))
                .collect();
            adj.sort_by_key(|h| (h.distance(&pos), h.q, h.r));
            for hex in adj {
                if engine.set_move_order(id, hex.q, hex.r) && engine.set_attack_order(id, target_id)
                {
                    planned = true;
                    break;
                }
            }
            if !planned {
                let dq = (tgt_pos.q - pos.q).signum();
                let dr = (tgt_pos.r - pos.r).signum();
                for (q, r) in [
                    (pos.q + dq * 2, pos.r + dr * 2),
                    (pos.q + dq, pos.r + dr),
                    (pos.q + dq, pos.r),
                    (pos.q, pos.r + dr),
                ] {
                    if engine.set_move_order(id, q, r) {
                        break;
                    }
                }
            }
        }
        println!("=== orders before resolve: {}", engine.get_pending_orders());
        let events: Vec<GameEvent> = serde_json::from_str(&engine.end_turn()).unwrap();
        println!("--- after round {} ---", engine.state().round);
        let mut units: Vec<_> = engine.state().units.values().collect();
        units.sort_by_key(|u| u.id);
        for u in units {
            println!(
                "  unit {} team{} init{} at ({},{}) hp{} ap{}",
                u.id, u.team, u.initiative, u.pos.q, u.pos.r, u.hp, u.ap
            );
        }
        for e in &events {
            println!("  ev {:?}", e);
        }
        if engine.state().phase == Phase::MatchEnd {
            println!("MATCH END winner={:?}", engine.state().winner);
            break;
        }
    }
    let _ = Action::Wait;
    let _ = UnitOrder {
        unit_id: 0,
        move_target: None,
        action: Action::Wait,
    };
}
