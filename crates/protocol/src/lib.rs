use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod tutorial;
pub use tutorial::*;

pub type MatchId = String;
pub type PlayerId = String;
pub type ReconnectToken = String;
pub type UnitId = u64;
pub type TeamId = u8;
pub type Round = u32;
pub type SpellId = String;
pub type ItemDefId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatKind {
    AttackDamage,
    MaxHealth,
    VisionRange,
    EnergyRegen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatModifierDto {
    pub stat: StatKind,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDto {
    pub id: ItemDefId,
    pub name: String,
    pub cost: u32,
    pub description: String,
    pub icon: String,
    pub modifiers: Vec<StatModifierDto>,
}

pub type HeroDefId = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroEconomyDto {
    pub unit_id: UnitId,
    pub hero_def_id: HeroDefId,
    pub gold: u32,
    pub xp: u32,
    pub level: u32,
    pub items: Vec<ItemDefId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardReason {
    PassiveIncome,
    HeroKill,
    MinionKill,
    NeutralKill,
    TowerDestroyed,
    SpawnerDestroyed,
}

fn default_level_u32() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeStateDto {
    Alive,
    DeadAwaitingRespawn,
    PermanentlyRemoved,
}

fn default_life_state_alive() -> LifeStateDto {
    LifeStateDto::Alive
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseZoneDto {
    pub team: TeamId,
    pub center: HexDto,
    pub radius: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveStatusDto {
    pub unit_id: UnitId,
    pub pos: HexDto,
    pub hp: u32,
    pub max_hp: u32,
    pub is_alive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopDisabledReasonDto {
    NotPlanningPhase,
    HeroDead,
    OutsideBaseZone,
    InventoryFull,
    InsufficientGold,
}

/// Axial hex coordinate DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexDto {
    pub q: i32,
    pub r: i32,
}

impl HexDto {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }
}

/// Status effect DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusDto {
    pub id: String,
    pub remaining_rounds: u32,
    pub attack_damage_mod: i32,
}

/// Neutral camp DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeutralCampDto {
    pub id: String,
    pub pos: HexDto,
    pub is_alive: bool,
    pub guardian_unit_id: Option<UnitId>,
}

/// Unit data transfer representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitDto {
    pub id: UnitId,
    pub kind: String, // "Hero", "Minion", "Tower", "Spawner", "NeutralGuardian", "Core", "Objective"
    pub team: TeamId,
    pub pos: HexDto,
    pub hp: u32,
    pub max_hp: u32,
    pub ap: u32,
    pub max_ap: u32,
    #[serde(default)]
    pub energy: u32,
    #[serde(default)]
    pub max_energy: u32,
    pub initiative: u32,
    pub attack_damage: u32,
    pub attack_range: u32,
    pub vision_range: u32,
    #[serde(default)]
    pub is_stationary: bool,
    #[serde(default)]
    pub cooldowns: HashMap<SpellId, u32>,
    #[serde(default)]
    pub statuses: Vec<StatusDto>,
    #[serde(default)]
    pub lane_id: Option<String>,
    #[serde(default)]
    pub hero_id: Option<String>,
    #[serde(default)]
    pub gold: Option<u32>,
    #[serde(default)]
    pub xp: Option<u32>,
    #[serde(default = "default_level_u32")]
    pub level: u32,
    #[serde(default)]
    pub items: Vec<ItemDefId>,
    #[serde(default = "default_life_state_alive")]
    pub life_state: LifeStateDto,
    #[serde(default)]
    pub respawn_rounds: Option<u32>,
    #[serde(default)]
    pub death_pos: Option<HexDto>,
}

/// Arena terrain layout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDto {
    pub radius: u32,
    pub walkable: Vec<HexDto>,
    pub obstacles: Vec<HexDto>,
}

pub type HexCoordDto = HexDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchPhaseDto {
    Lobby,
    HeroSelect,
    Planning,
    Resolution,
    MatchEnd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerLobbyDto {
    pub player_id: String,
    pub display_name: String,
    pub team: u8,
    pub connected: bool,
    pub ready: bool,
    pub hero_def_id: Option<HeroDefId>,
    pub is_ai: bool,
    #[serde(default)]
    pub ping_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroDto {
    pub id: HeroDefId,
    pub name: String,
    pub role: String,
    pub max_hp: u32,
    pub attack_damage: u32,
    pub attack_range: u32,
    pub vision_range: u32,
    pub max_energy: u32,
    pub spell_id: String,
    pub spell_name: String,
    pub spell_desc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterEntryDto {
    pub player_id: Option<String>,
    pub display_name: String,
    pub hero_def_id: HeroDefId,
    pub unit_id: UnitId,
    pub team: u8,
    pub connected: bool,
    pub is_ai: bool,
    pub orders_submitted: bool,
    pub alive: bool,
    /// Exact HP is included for all allies; masked for enemies unless currently in LOS.
    pub hp: Option<u32>,
    pub max_hp: u32,
    #[serde(default = "default_level_u32")]
    pub level: u32,
    #[serde(default)]
    pub items: Vec<ItemDefId>,
    #[serde(default)]
    pub life_state: Option<LifeStateDto>,
    #[serde(default)]
    pub respawn_rounds: Option<u32>,
}

/// Team-sanitized game state snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotDto {
    #[serde(default)]
    pub match_id: MatchId,
    pub round: Round,
    pub phase: String, // "Planning", "Resolution", "Ended", etc.
    pub winner: Option<TeamId>,
    #[serde(default)]
    pub map: MapDto,
    pub units: Vec<UnitDto>,
    pub visible_hexes: Vec<HexDto>,
    pub controlled_units: Vec<UnitId>,
    #[serde(default)]
    pub deadline_unix_ms: Option<u64>,
    #[serde(alias = "blake3_hash")]
    pub state_hash: String,
    #[serde(default)]
    pub neutral_camps: Vec<NeutralCampDto>,
    #[serde(default)]
    pub player_team: TeamId,
    #[serde(default)]
    pub roster: Vec<RosterEntryDto>,
    #[serde(default)]
    pub match_phase: Option<MatchPhaseDto>,
    #[serde(default)]
    pub controlled_hero_economy: Option<HeroEconomyDto>,
    #[serde(default)]
    pub allied_hero_economy: Vec<HeroEconomyDto>,
    #[serde(default)]
    pub shop_catalog: Vec<ItemDto>,
    #[serde(default)]
    pub can_shop: bool,
    #[serde(default)]
    pub victory_mode: Option<String>,
    #[serde(default)]
    pub base_zones: Vec<BaseZoneDto>,
    #[serde(default)]
    pub shop_disabled_reason: Option<ShopDisabledReasonDto>,
    #[serde(default, with = "team_map_serde")]
    pub core_hp: HashMap<TeamId, (u32, u32)>,
    #[serde(default)]
    pub objective: Option<ObjectiveStatusDto>,
}

/// Spell target DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SpellTargetDto {
    None,
    Unit { unit_id: UnitId },
    Hex { hex: HexDto },
}

/// Order action type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActionDto {
    Wait,
    Attack {
        target_id: UnitId,
    },
    Cast {
        spell_id: SpellId,
        target: SpellTargetDto,
    },
    Repair {
        target_id: UnitId,
    },
}

/// Unit turn order submitted by player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderDto {
    pub unit_id: UnitId,
    pub move_target: Option<HexDto>,
    pub action: ActionDto,
}

pub type UnitOrderDto = OrderDto;

/// Victory reason declaration for match completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VictoryReasonDto {
    CoreDestroyed {
        destroyed_core_id: UnitId,
        destroyed_team: TeamId,
        destroyer_team: TeamId,
    },
    HeroElimination {
        eliminated_team: TeamId,
    },
}

pub type VictoryReason = VictoryReasonDto;

/// Fog-sanitized event emitted during round resolution.
/// Ensures coordinates and hidden unit activities in fog are masked or omitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SanitizedGameEvent {
    RoundStarted {
        round: Round,
    },
    UnitSpawned {
        unit_id: UnitId,
        unit_kind: String,
        team: TeamId,
        pos: HexDto,
        spawner_id: UnitId,
    },
    UnitMoved {
        unit_id: UnitId,
        from: HexDto,
        to: HexDto,
        path: Vec<HexDto>,
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
        spell_id: SpellId,
        target: SpellTargetDto,
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
        unit_kind: String,
        killed_by: UnitId,
    },
    UnitWaited {
        unit_id: UnitId,
    },
    HeroDied {
        unit_id: UnitId,
        killed_by: UnitId,
        respawn_rounds: u32,
    },
    HeroRespawned {
        unit_id: UnitId,
        team: TeamId,
        pos: HexDto,
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
    RewardGranted {
        unit_id: UnitId,
        gold: u32,
        xp: u32,
        reason: RewardReason,
    },
    LevelUp {
        unit_id: UnitId,
        new_level: u32,
        new_max_hp: u32,
        new_attack_damage: u32,
        new_max_energy: u32,
    },
    RoundEnded {
        round: Round,
    },
    MatchEnded {
        winner: Option<TeamId>,
        #[serde(default)]
        reason: Option<VictoryReasonDto>,
    },
}

pub type GameEventDto = SanitizedGameEvent;
pub type ServerEventDto = SanitizedGameEvent;

/// Upstream messages sent from browser client to server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Hello {
        player_id: PlayerId,
        reconnect_token: Option<ReconnectToken>,
    },
    JoinMatch {
        #[serde(default)]
        match_id: MatchId,
        #[serde(default)]
        player_id: Option<String>,
        #[serde(default)]
        display_name: Option<String>,
        #[serde(default)]
        reconnect_token: Option<String>,
        #[serde(default)]
        preferred_team: Option<u8>,
    },
    SelectHero {
        hero_def_id: HeroDefId,
    },
    SetReady {
        ready: bool,
    },
    SubmitOrders {
        round: Round,
        orders: Vec<OrderDto>,
    },
    CancelOrders {
        round: Round,
    },
    Ping {
        #[serde(alias = "timestamp_ms")]
        client_time_ms: u64,
    },
    BuyItem {
        item_id: ItemDefId,
    },
}

pub mod team_map_serde {
    use std::collections::HashMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde::de::Visitor;

    pub fn serialize<S, V>(map: &HashMap<u8, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: serde::Serialize,
    {
        map.serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<HashMap<u8, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        struct TeamMapVisitor<V>(std::marker::PhantomData<V>);
        impl<'de, V: Deserialize<'de>> Visitor<'de> for TeamMapVisitor<V> {
            type Value = HashMap<u8, V>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map with integer or string keys")
            }
            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut map = HashMap::new();
                while let Some(key_str) = access.next_key::<String>()? {
                    let team: u8 = key_str.parse().map_err(serde::de::Error::custom)?;
                    let value = access.next_value()?;
                    map.insert(team, value);
                }
                Ok(map)
            }
        }
        deserializer.deserialize_map(TeamMapVisitor(std::marker::PhantomData))
    }
}

/// Downstream messages sent from server to browser client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    HelloAck {
        player_id: PlayerId,
        reconnect_token: ReconnectToken,
    },
    MatchJoined {
        match_id: MatchId,
        player_id: PlayerId,
        team: TeamId,
        is_spectator: bool,
        snapshot: SnapshotDto,
    },
    LobbyUpdated {
        match_id: String,
        phase: MatchPhaseDto,
        players: Vec<PlayerLobbyDto>,
        #[serde(with = "team_map_serde")]
        hero_pools: HashMap<u8, Vec<HeroDto>>,
        #[serde(default)]
        countdown_ms: Option<u64>,
    },
    HeroSelected {
        player_id: String,
        team: u8,
        hero_def_id: HeroDefId,
    },
    MatchStarting {
        round: Round,
        initial_snapshot: SnapshotDto,
    },
    RoundStarted {
        round: Round,
        deadline_unix_ms: u64,
        snapshot: SnapshotDto,
        #[serde(default)]
        events: Vec<SanitizedGameEvent>,
    },
    OrdersAccepted {
        round: Round,
    },
    OrderRejected {
        round: Round,
        error_code: ProtocolErrorCode,
        reason: String,
    },
    EarlyResolutionTriggered {
        round: Round,
        resolution_unix_ms: u64,
    },
    RoundResolved {
        /// The round number that was resolved (corresponds to the round that was planned).
        round: Round,
        events: Vec<SanitizedGameEvent>,
        snapshot: SnapshotDto,
        #[serde(default)]
        state_hash: Option<String>,
    },
    PlayerConnectionUpdated {
        player_id: String,
        connected: bool,
        is_ai_controlled: bool,
    },
    MatchEnded {
        winner: Option<TeamId>,
        snapshot: SnapshotDto,
        #[serde(default)]
        state_hash: Option<String>,
        #[serde(default)]
        total_rounds: Option<Round>,
        #[serde(default)]
        reason: Option<VictoryReasonDto>,
    },
    OpponentStatus {
        online: bool,
    },
    Pong {
        client_time_ms: u64,
        server_time_ms: u64,
    },
    PurchaseResolved {
        unit_id: UnitId,
        item_id: ItemDefId,
        success: bool,
        gold_remaining: u32,
        #[serde(default)]
        error: Option<ProtocolErrorCode>,
    },
    EconomyUpdated {
        unit_id: UnitId,
        gold: u32,
        xp: u32,
        level: u32,
        items: Vec<ItemDefId>,
    },
    LevelUpOccurred {
        unit_id: UnitId,
        new_level: u32,
        new_max_hp: u32,
        new_attack_damage: u32,
        new_max_energy: u32,
    },
    Error {
        #[serde(alias = "code")]
        error_code: ProtocolErrorCode,
        message: String,
    },
}

/// Structured protocol error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolErrorCode {
    // Authentication & Controller Permissions
    NotYourUnit,
    InvalidOrderCount,
    PlayerNotAuthenticated,
    UnauthorizedAction,

    // Match & Lobby Lifecycle
    MatchFull,
    LobbyAlreadyStarted,
    NotInLobbyPhase,
    NotInHeroSelectPhase,
    NotInPlanningPhase,
    HeroAlreadySelected,
    InvalidHeroDef,
    HeroSelectLocked,
    PlayerAlreadyConnected,

    // Turn & Timing
    RoundMismatch,
    TurnDeadlineExceeded,
    OrdersAlreadySubmitted,

    // Gameplay Rules
    InsufficientAp,
    InsufficientEnergy,
    AbilityOnCooldown,
    LineOfSightBlocked,
    TargetOutOfRange,
    InvalidTarget,
    TargetDead,
    PathObstructed,

    // Legacy & General codes
    InvalidMessage,
    MatchNotFound,
    NotAuthorized,
    StaleRound,
    InvalidPhase,
    UnitNotOwned,
    UnitDead,
    TimerExpired,
    InternalError,
    InsufficientResources,
    CooldownActive,
    MissingLineOfSight,

    // Phase 6 Economic Error Codes
    InsufficientGold,
    InventoryFull,
    ItemAlreadyOwned,
    NoSuchItem,
    CannotShopInPhase,
    NotAHero,

    // Phase 7 Macro Error Codes
    CannotShopOutsideBase,
    HeroDeadAwaitingRespawn,
    CannotOrderDeadHero,
    TargetUntargetable,
    CoreCannotBeRepaired,
}

pub use ProtocolErrorCode as ErrorCode;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::SubmitOrders {
            round: 1,
            orders: vec![OrderDto {
                unit_id: 1,
                move_target: Some(HexDto::new(-3, -1)),
                action: ActionDto::Attack { target_id: 6 },
            }],
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize ClientMessage");
        assert!(json.contains("\"type\":\"SubmitOrders\""));
        assert!(json.contains("\"round\":1"));

        let deserialized: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize ClientMessage");
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_server_message_serialization() {
        let snap = SnapshotDto {
            match_id: "match-123".into(),
            round: 0,
            phase: "Planning".into(),
            winner: None,
            map: MapDto {
                radius: 6,
                walkable: vec![HexDto::new(0, 0)],
                obstacles: vec![HexDto::new(0, 2)],
            },
            units: vec![UnitDto {
                id: 1,
                kind: "Hero".into(),
                team: 0,
                pos: HexDto::new(-4, -1),
                hp: 100,
                max_hp: 100,
                ap: 3,
                max_ap: 3,
                energy: 5,
                max_energy: 5,
                initiative: 3,
                attack_damage: 20,
                attack_range: 1,
                vision_range: 3,
                is_stationary: false,
                cooldowns: HashMap::new(),
                statuses: Vec::new(),
                lane_id: None,
                hero_id: None,
                gold: Some(50),
                xp: Some(0),
                level: 1,
                items: Vec::new(),
                life_state: LifeStateDto::Alive,
                respawn_rounds: None,
                death_pos: None,
            }],
            visible_hexes: vec![HexDto::new(-4, -1)],
            controlled_units: vec![1],
            deadline_unix_ms: Some(1700000000000),
            state_hash: "abcd1234deadbeef".into(),
            neutral_camps: Vec::new(),
            player_team: 0,
            roster: Vec::new(),
            match_phase: Some(MatchPhaseDto::Planning),
            controlled_hero_economy: None,
            allied_hero_economy: Vec::new(),
            shop_catalog: Vec::new(),
            can_shop: true,
            victory_mode: None,
            base_zones: Vec::new(),
            shop_disabled_reason: None,
            core_hp: HashMap::new(),
            objective: None,
        };

        let msg = ServerMessage::RoundStarted {
            round: 1,
            deadline_unix_ms: 1700000030000,
            snapshot: snap.clone(),
            events: Vec::new(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize ServerMessage");
        assert!(json.contains("\"type\":\"RoundStarted\""));
        assert!(json.contains("\"deadline_unix_ms\":1700000030000"));

        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize ServerMessage");
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_sanitized_game_events_serialization() {
        let events = vec![
            SanitizedGameEvent::RoundStarted { round: 1 },
            SanitizedGameEvent::UnitMoved {
                unit_id: 1,
                from: HexDto::new(-4, -1),
                to: HexDto::new(-3, -1),
                path: vec![HexDto::new(-4, -1), HexDto::new(-3, -1)],
                ap_spent: 1,
            },
            SanitizedGameEvent::UnitAttacked {
                attacker_id: 1,
                target_id: 6,
                damage: 20,
                target_hp_remaining: 80,
            },
            SanitizedGameEvent::SpellCast {
                caster_id: 1,
                spell_id: "bolt".into(),
                target: SpellTargetDto::Unit { unit_id: 6 },
            },
            SanitizedGameEvent::HealApplied {
                caster_id: 3,
                target_id: 1,
                amount: 20,
                target_hp_remaining: 100,
            },
            SanitizedGameEvent::StructureRepaired {
                repairer_id: 1,
                target_id: 11,
                amount: 20,
                target_hp_remaining: 120,
            },
            SanitizedGameEvent::RoundEnded { round: 1 },
        ];

        let json = serde_json::to_string(&events).expect("Failed to serialize events");
        let decoded: Vec<SanitizedGameEvent> =
            serde_json::from_str(&json).expect("Failed to deserialize events");
        assert_eq!(events, decoded);
    }

    #[test]
    fn test_opponent_status_serialization() {
        let msg = ServerMessage::OpponentStatus { online: false };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"OpponentStatus\""));
        assert!(json.contains("\"online\":false"));
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_phase5_messages_serialization() {
        let lobby_msg = ServerMessage::LobbyUpdated {
            match_id: "m-5v5".into(),
            phase: MatchPhaseDto::Lobby,
            players: vec![PlayerLobbyDto {
                player_id: "p1".into(),
                display_name: "Commander1".into(),
                team: 0,
                connected: true,
                ready: true,
                hero_def_id: Some("sniper".into()),
                is_ai: false,
                ping_ms: Some(25),
            }],
            hero_pools: HashMap::new(),
            countdown_ms: Some(20000),
        };
        let json = serde_json::to_string(&lobby_msg).unwrap();
        assert!(json.contains("\"type\":\"LobbyUpdated\""));
        assert!(json.contains("\"phase\":\"Lobby\""));
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(lobby_msg, decoded);

        let select_hero_client = ClientMessage::SelectHero {
            hero_def_id: "berserker".into(),
        };
        let json2 = serde_json::to_string(&select_hero_client).unwrap();
        assert!(json2.contains("\"type\":\"SelectHero\""));
        assert!(json2.contains("\"hero_def_id\":\"berserker\""));
        let decoded2: ClientMessage = serde_json::from_str(&json2).unwrap();
        assert_eq!(select_hero_client, decoded2);
    }

    #[test]
    fn test_phase7_macro_dto_serialization() {
        let mut core_hp = HashMap::new();
        core_hp.insert(0, (700, 700));
        core_hp.insert(1, (650, 700));

        let base_zones = vec![
            BaseZoneDto {
                team: 0,
                center: HexDto::new(-7, 0),
                radius: 2,
            },
            BaseZoneDto {
                team: 1,
                center: HexDto::new(7, 0),
                radius: 2,
            },
        ];

        let objective = ObjectiveStatusDto {
            unit_id: 300,
            pos: HexDto::new(0, 0),
            hp: 250,
            max_hp: 250,
            is_alive: true,
        };

        let unit = UnitDto {
            id: 1,
            kind: "Hero".into(),
            team: 0,
            pos: HexDto::new(-6, -2),
            hp: 100,
            max_hp: 100,
            ap: 2,
            max_ap: 2,
            energy: 0,
            max_energy: 0,
            initiative: 3,
            attack_damage: 15,
            attack_range: 1,
            vision_range: 3,
            is_stationary: false,
            cooldowns: HashMap::new(),
            statuses: Vec::new(),
            lane_id: None,
            hero_id: Some("vanguard".into()),
            gold: Some(50),
            xp: Some(0),
            level: 1,
            items: Vec::new(),
            life_state: LifeStateDto::DeadAwaitingRespawn,
            respawn_rounds: Some(3),
            death_pos: Some(HexDto::new(-6, -2)),
        };

        let snap = SnapshotDto {
            match_id: "phase7-test".into(),
            round: 1,
            phase: "Planning".into(),
            winner: None,
            map: MapDto::default(),
            units: vec![unit],
            visible_hexes: vec![HexDto::new(0, 0)],
            controlled_units: vec![1],
            deadline_unix_ms: Some(123456789),
            state_hash: "blake3hash".into(),
            neutral_camps: Vec::new(),
            player_team: 0,
            roster: vec![RosterEntryDto {
                player_id: Some("p1".into()),
                display_name: "Player 1".into(),
                hero_def_id: "vanguard".into(),
                unit_id: 1,
                team: 0,
                connected: true,
                is_ai: false,
                orders_submitted: false,
                alive: false,
                hp: Some(0),
                max_hp: 100,
                level: 1,
                items: Vec::new(),
                life_state: Some(LifeStateDto::DeadAwaitingRespawn),
                respawn_rounds: Some(3),
            }],
            match_phase: Some(MatchPhaseDto::Planning),
            controlled_hero_economy: None,
            allied_hero_economy: Vec::new(),
            shop_catalog: Vec::new(),
            can_shop: false,
            victory_mode: Some("CoreDestruction".into()),
            base_zones,
            shop_disabled_reason: Some(ShopDisabledReasonDto::HeroDead),
            core_hp,
            objective: Some(objective),
        };

        let json = serde_json::to_string(&snap).expect("Failed to serialize Phase 7 SnapshotDto");
        assert!(json.contains("\"dead_awaiting_respawn\""));
        assert!(json.contains("\"victory_mode\":\"CoreDestruction\""));
        assert!(json.contains("\"shop_disabled_reason\":\"hero_dead\""));

        let deserialized: SnapshotDto =
            serde_json::from_str(&json).expect("Failed to deserialize Phase 7 SnapshotDto");
        assert_eq!(snap, deserialized);
    }

    #[test]
    fn test_phase7_events_and_errors_serialization() {
        let events = vec![
            SanitizedGameEvent::HeroDied {
                unit_id: 1,
                killed_by: 6,
                respawn_rounds: 3,
            },
            SanitizedGameEvent::HeroRespawned {
                unit_id: 1,
                team: 0,
                pos: HexDto::new(-7, 0),
            },
            SanitizedGameEvent::BaseRegenerationApplied {
                unit_id: 1,
                team: 0,
                amount: 15,
                new_hp: 100,
            },
            SanitizedGameEvent::ObjectiveDestroyed {
                objective_id: 300,
                destroyer_team: 0,
                last_attacker_id: 1,
                gold_awarded_per_hero: 50,
                xp_awarded_per_hero: 40,
                affected_heroes: vec![1, 2, 3],
            },
            SanitizedGameEvent::CoreDestroyed {
                core_id: 501,
                team: 1,
                destroyed_by: 1,
            },
        ];

        let json = serde_json::to_string(&events).expect("Failed to serialize Phase 7 events");
        let decoded: Vec<SanitizedGameEvent> =
            serde_json::from_str(&json).expect("Failed to deserialize Phase 7 events");
        assert_eq!(events, decoded);

        let errors = vec![
            ProtocolErrorCode::CannotShopOutsideBase,
            ProtocolErrorCode::HeroDeadAwaitingRespawn,
            ProtocolErrorCode::CannotOrderDeadHero,
            ProtocolErrorCode::TargetUntargetable,
            ProtocolErrorCode::CoreCannotBeRepaired,
        ];
        let err_json = serde_json::to_string(&errors).unwrap();
        let decoded_errs: Vec<ProtocolErrorCode> = serde_json::from_str(&err_json).unwrap();
        assert_eq!(errors, decoded_errs);
    }
}

