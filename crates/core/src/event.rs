use crate::ability::SpellTarget;
use crate::hex::HexCoord;
use crate::unit::{TeamId, UnitId, UnitKind};
pub use hexabellum_protocol::RewardReason;
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

    HeroDied {
        unit_id: UnitId,
        killed_by: UnitId,
        respawn_rounds: u32,
    },

    HeroRespawned {
        unit_id: UnitId,
        team: TeamId,
        pos: HexCoord,
    },

    BaseRegenerationApplied {
        unit_id: UnitId,
        team: TeamId,
        amount: u32,
        new_hp: u32,
    },

    ObjectiveDestroyed {
        objective_id: UnitId,
        destroyer_team: TeamId,
        last_attacker_id: UnitId,
        gold_awarded_per_hero: u32,
        xp_awarded_per_hero: u32,
        affected_heroes: Vec<UnitId>,
    },

    CoreDestroyed {
        core_id: UnitId,
        team: TeamId,
        destroyed_by: UnitId,
    },

    UnitWaited {
        unit_id: UnitId,
    },

    RewardGranted {
        unit_id: UnitId,
        gold: u32,
        xp: u32,
        reason: hexabellum_protocol::RewardReason,
    },

    LevelUp {
        unit_id: UnitId,
        new_level: u32,
        new_max_hp: u32,
        new_attack_damage: u32,
        new_max_energy: u32,
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
