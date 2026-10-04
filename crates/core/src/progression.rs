use crate::event::GameEvent;
use crate::unit::Unit;

pub const MAX_LEVEL: u32 = 5;

pub fn xp_threshold(level: u32) -> u32 {
    match level {
        1 => 0,
        2 => 50,
        3 => 120,
        4 => 220,
        5 => 350,
        _ => u32::MAX,
    }
}

pub fn grant_xp(unit: &mut Unit, amount: u32, events: &mut Vec<GameEvent>) {
    if unit.level >= MAX_LEVEL {
        return;
    }

    unit.xp = unit.xp.saturating_add(amount);

    while unit.level < MAX_LEVEL && unit.xp >= xp_threshold(unit.level + 1) {
        unit.level += 1;
        unit.max_hp += 12;
        unit.hp = (unit.hp + 12).min(unit.max_hp);
        unit.attack_damage += 3;
        unit.max_energy += 1;

        events.push(GameEvent::LevelUp {
            unit_id: unit.id,
            new_level: unit.level,
            new_max_hp: unit.max_hp,
            new_attack_damage: unit.attack_damage,
            new_max_energy: unit.max_energy,
        });
    }
}
