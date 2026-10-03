use hexabellum_core::ai::SimpleAI;
use hexabellum_core::hex::{HexCoord, HexMap};
use hexabellum_core::orders::TurnOrders;
use hexabellum_core::state::GameState;
use hexabellum_core::turn::TurnProcessor;
use hexabellum_core::unit::Unit;

#[test]
fn diagnose_stall() {
    let mut map = HexMap::new(5);
    map.obstacles.insert(HexCoord::new(0, 0));
    map.obstacles.insert(HexCoord::new(1, -1));
    map.obstacles.insert(HexCoord::new(-1, 1));
    map.obstacles.insert(HexCoord::new(2, -2));
    map.obstacles.insert(HexCoord::new(-2, 2));
    let mut state = GameState::new(map);
    state.add_unit(Unit::new_hero(1, 0, HexCoord::new(-1, 0), 3));
    state.add_unit(Unit::new_hero(2, 0, HexCoord::new(-2, 1), 2));
    state.add_unit(Unit::new_hero(3, 0, HexCoord::new(0, -1), 1));
    state.add_unit(Unit::new_hero(4, 1, HexCoord::new(-1, 2), 3));
    state.add_unit(Unit::new_hero(5, 1, HexCoord::new(1, 0), 2));
    state.add_unit(Unit::new_hero(6, 1, HexCoord::new(2, -1), 1));

    for round in 0..6 {
        let ai = SimpleAI::generate_orders(&state, 1);
        println!("round {} AI orders: {}", round + 1, serde_json::to_string(&ai).unwrap());
        TurnProcessor::resolve(&mut state, &ai);
        let mut ids: Vec<u64> = state.units.keys().copied().collect();
        ids.sort();
        let info: Vec<String> = ids.iter().map(|i| {
            let u = &state.units[i];
            format!("{}:t{}@({},{})hp{}", u.id, u.team, u.pos.q, u.pos.r, u.hp)
        }).collect();
        println!("  board -> {}", info.join(" "));
    }
}
