use crate::orders::{Action, UnitOrder};
use crate::state::GameState;
use crate::unit::{Unit, UnitId, UnitKind};

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

        // Find enemies within firing perimeter
        let enemies = state.enemy_units(tower.team);
        let in_range: Vec<&Unit> = enemies
            .into_iter()
            .filter(|e| tower.pos.distance(&e.pos) <= tower.attack_range)
            .collect();

        if in_range.is_empty() {
            return UnitOrder {
                unit_id,
                move_target: None,
                action: Action::Wait,
            };
        }

        // Tower aggro priority: Minion > Hero > Structure (minions tank tower shots!)
        let mut minions: Vec<&Unit> = in_range.iter().filter(|e| e.kind == UnitKind::Minion).copied().collect();
        let mut heroes: Vec<&Unit> = in_range.iter().filter(|e| e.kind == UnitKind::Hero).copied().collect();
        let mut structures: Vec<&Unit> = in_range.iter().filter(|e| e.kind.is_structure()).copied().collect();

        let sort_fn = |a: &&Unit, b: &&Unit| {
            let dist_a = tower.pos.distance(&a.pos);
            let dist_b = tower.pos.distance(&b.pos);
            dist_a.cmp(&dist_b).then_with(|| a.id.cmp(&b.id))
        };

        minions.sort_by(sort_fn);
        heroes.sort_by(sort_fn);
        structures.sort_by(sort_fn);

        let target = minions.first().or(heroes.first()).or(structures.first()).copied();

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
