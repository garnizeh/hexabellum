use hexabellum_core::event::GameEvent;
use hexabellum_core::hex::HexCoord;
use hexabellum_core::state::Phase;
use hexabellum_core::unit::UnitId;
use hexabellum_core::*;

#[test]
fn trace() {
    let mut engine = GameEngine::new();
    for round in 0..20 {
        let hero_ids: Vec<UnitId> = engine
            .state()
            .team_units(PLAYER_TEAM)
            .iter()
            .map(|u| u.id)
            .collect();
        for id in hero_ids {
            engine.clear_orders(id);
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
            let mut planned_attack = false;
            let neighbors = tgt_pos.neighbors();
            let mut adj: Vec<HexCoord> = neighbors
                .iter()
                .filter(|h| engine.state().map.is_walkable(h))
                .copied()
                .collect();
            adj.sort_by_key(|h| (h.distance(&pos), h.q, h.r));
            for hex in adj {
                if engine.set_move_order_with(id, hex.q, hex.r, true)
                    && engine.set_attack_order(id, target_id)
                {
                    planned_attack = true;
                    break;
                }
                engine.clear_orders(id);
            }
            if !planned_attack {
                let dq = (tgt_pos.q - pos.q).signum();
                let dr = (tgt_pos.r - pos.r).signum();
                let candidates = [
                    (pos.q + dq * 2, pos.r + dr * 2),
                    (pos.q + dq, pos.r + dr),
                    (pos.q + dq, pos.r),
                    (pos.q, pos.r + dr),
                ];
                for (q, r) in candidates {
                    if engine.set_move_order(id, q, r) {
                        break;
                    }
                }
            }
        }
        println!(
            "round {} orders: {}",
            round + 1,
            engine.get_pending_orders()
        );
        let evjson = engine.end_turn();
        let events: Vec<GameEvent> = serde_json::from_str(&evjson).unwrap();
        for e in &events {
            match e {
                GameEvent::UnitMoved {
                    unit_id,
                    from,
                    to,
                    ap_spent,
                    ..
                } => println!(
                    "  MOV {} ({},{})->({},{}) cost={}",
                    unit_id, from.q, from.r, to.q, to.r, ap_spent
                ),
                GameEvent::UnitAttacked {
                    attacker_id,
                    target_id,
                    damage,
                    target_hp_remaining,
                } => println!(
                    "  ATK {}->{} dmg={} hp={}",
                    attacker_id, target_id, damage, target_hp_remaining
                ),
                GameEvent::UnitDied { unit_id, killed_by, .. } => {
                    println!("  DIE {} by {}", unit_id, killed_by)
                }
                _ => {}
            }
        }
        let mut ids: Vec<u64> = engine.state().units.keys().copied().collect();
        ids.sort();
        let info: Vec<String> = ids
            .iter()
            .map(|i| {
                let u = &engine.state().units[i];
                format!("{}:t{}@({},{})hp{}", u.id, u.team, u.pos.q, u.pos.r, u.hp)
            })
            .collect();
        println!("board -> {}", info.join(" "));
        if engine.state().phase == Phase::MatchEnd {
            break;
        }
    }
}
