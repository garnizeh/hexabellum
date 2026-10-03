use crate::ability::SpellTarget;
use crate::hex::HexCoord;
use crate::unit::{TeamId, UnitId, UnitKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum GameEvent {
    RoundStarted {
        round: u32,
    },

    UnitSpawned {
        unit_id: UnitId,
        unit_kind: UnitKind,
        team: TeamId,
        pos: HexCoord,
        spawner_id: UnitId,
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

    TowerAttacked {
        tower_id: UnitId,
        target_id: UnitId,
        damage: u32,
        target_hp_remaining: u32,
    },

    SpellCast {
        caster_id: UnitId,
        spell_id: String,
        target: SpellTarget,
    },

    HealApplied {
        caster_id: UnitId,
        target_id: UnitId,
        amount: u32,
        target_hp_remaining: u32,
    },

    StructureRepaired {
        repairer_id: UnitId,
        target_id: UnitId,
        amount: u32,
        target_hp_remaining: u32,
    },

    StatusApplied {
        unit_id: UnitId,
        status_id: String,
        duration_rounds: u32,
    },

    StatusExpired {
        unit_id: UnitId,
        status_id: String,
    },

    NeutralCampCleared {
        camp_id: String,
        killer_team: TeamId,
    },

    TeamBuffApplied {
        team: TeamId,
        buff_id: String,
        duration_rounds: u32,
    },

    UnitDied {
        unit_id: UnitId,
        unit_kind: UnitKind,
        killed_by: UnitId,
    },

    UnitWaited {
        unit_id: UnitId,
    },

    FogUpdated {
        team: TeamId,
        visible_hexes: Vec<HexCoord>,
    },

    RoundEnded {
        round: u32,
    },

    MatchEnded {
        winner: Option<TeamId>,
    },
}
