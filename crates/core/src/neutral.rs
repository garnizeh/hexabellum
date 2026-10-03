use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::status::neutral_camp_damage_buff_def;
use crate::unit::{TeamId, Unit, UnitId};
use serde::{Deserialize, Serialize};

pub const NEUTRAL_LEASH_RADIUS: u32 = 3;
pub const NEUTRAL_AGGRO_RADIUS: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeutralCampState {
    Active,
    Cleared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutralCamp {
    pub id: String,
    pub camp_pos: HexCoord,
    pub guardian_id: UnitId,
    pub state: NeutralCampState,
}

impl NeutralCamp {
    pub fn new(id: String, camp_pos: HexCoord, guardian_id: UnitId) -> Self {
        Self {
            id,
            camp_pos,
            guardian_id,
            state: NeutralCampState::Active,
        }
    }

    /// Check leash constraints. If breached, guardian resets to camp origin with 100% HP.
    pub fn check_leash_and_reset(
        &self,
        guardian: &mut Unit,
        target: Option<&Unit>,
    ) -> bool {
        if self.state == NeutralCampState::Cleared || !guardian.is_alive() {
            return false;
        }

        let guardian_dist_from_camp = guardian.pos.distance(&self.camp_pos);
        let target_dist_from_camp = target
            .map(|t| t.pos.distance(&self.camp_pos))
            .unwrap_or(0);

        let breached = guardian_dist_from_camp > NEUTRAL_LEASH_RADIUS
            || (target.is_some() && target_dist_from_camp > NEUTRAL_LEASH_RADIUS);

        if breached {
            // Leash broken: drop target, return to origin, full heal
            guardian.pos = self.camp_pos;
            guardian.hp = guardian.max_hp;
            guardian.last_attacker = None;
            return true;
        }

        false
    }

    /// Process guardian death, reward killer team with +5 damage buff for 5 rounds
    pub fn handle_guardian_death(
        &mut self,
        killer_team: TeamId,
        all_units: &mut [Unit],
        events: &mut Vec<GameEvent>,
    ) {
        if self.state == NeutralCampState::Cleared {
            return;
        }

        self.state = NeutralCampState::Cleared;

        events.push(GameEvent::NeutralCampCleared {
            camp_id: self.id.clone(),
            killer_team,
        });

        let buff_def = neutral_camp_damage_buff_def();

        // Apply buff to all currently living heroes on the killer team
        for unit in all_units.iter_mut() {
            if unit.team == killer_team && unit.is_hero() && unit.is_alive() {
                unit.statuses.retain(|s| s.def_id != buff_def.id);
                unit.statuses.push(crate::status::StatusInstance::from_def(&buff_def));

                events.push(GameEvent::StatusApplied {
                    unit_id: unit.id,
                    status_id: buff_def.id.clone(),
                    duration_rounds: buff_def.duration_rounds,
                });
            }
        }

        events.push(GameEvent::TeamBuffApplied {
            team: killer_team,
            buff_id: buff_def.id,
            duration_rounds: buff_def.duration_rounds,
        });
    }
}
