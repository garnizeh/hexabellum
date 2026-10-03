use crate::hex::HexCoord;
use crate::unit::{TeamId, UnitId};
use serde::{Deserialize, Serialize};

/// Events emitted during resolution.
/// The client uses these to animate / display the battle log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameEvent {
    RoundStarted {
        round: u32,
    },

    UnitMoved {
        unit_id: UnitId,
        from: HexCoord,
        to: HexCoord,
        path: Vec<HexCoord>,
        ap_spent: u32,
    },

    UnitAttacked {
        attacker_id: UnitId,
        target_id: UnitId,
        damage: u32,
        target_hp_remaining: u32,
    },

    UnitDied {
        unit_id: UnitId,
        killed_by: UnitId,
    },

    UnitWaited {
        unit_id: UnitId,
    },

    RoundEnded {
        round: u32,
    },

    MatchEnded {
        winner: Option<TeamId>,
    },
}
