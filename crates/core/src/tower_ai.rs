use crate::orders::{Action, UnitOrder};
use crate::state::GameState;
use crate::unit::{Unit, UnitId};

pub struct TowerAI;

impl TowerAI {
    /// Towers are stationary defenses that auto-attack the highest priority enemy in range.
    pub fn generate_order(state: &GameState, unit_id: UnitId) -> UnitOrder {
        let tower = match state.get_unit(unit_id) {
            Some(u) if u.is_alive() && u.attack_range > 0 => u,
            _ => {
                return UnitOrder {
                    unit_id,
                    move_target: None,
                    action: Action::Wait,
                }
            }
        };

        // Find enemies within firing perimeter with unobstructed line of sight
        let blockers = state.map.vision_blockers();
        let enemies = state.enemy_units(tower.team);
        let in_range: Vec<&Unit> = enemies
            .into_iter()
            .filter(|e| {
                if crate::priority::evaluate_tower_target_priority(e)
                    == crate::priority::PriorityClass::Excluded
                {
                    return false;
                }
                let dist = tower.pos.distance(&e.pos);
                dist <= tower.attack_range
                    && (dist <= 1 || crate::vision::has_line_of_sight(&blockers, tower.pos, e.pos))
            })
            .collect();

        if in_range.is_empty() {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            };
        }

        // Tower aggro priority: Minion > Hero > Structure with (prio, dist, hp, unit_id) tie-breaking
        let mut sorted = in_range;
        sorted.sort_by(|a, b| {
            let prio_a = crate::priority::evaluate_tower_target_priority(a);
            let prio_b = crate::priority::evaluate_tower_target_priority(b);
            let dist_a = tower.pos.distance(&a.pos);
            let dist_b = tower.pos.distance(&b.pos);
            prio_a
                .cmp(&prio_b)
                .then_with(|| dist_a.cmp(&dist_b))
                .then_with(|| a.hp.cmp(&b.hp))
                .then_with(|| a.id.cmp(&b.id))
        });

        let target = sorted.first().copied();

        if let Some(t) = target {
            UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Attack { target_id: t.id },
            }
        } else {
            UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            }
        }
    }
}
