use serde::{Deserialize, Serialize};

pub type MatchId = String;
pub type PlayerId = String;
pub type ReconnectToken = String;
pub type UnitId = u64;
pub type TeamId = u8;
pub type Round = u32;

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

/// Unit data transfer representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitDto {
    pub id: UnitId,
    pub kind: String, // "Hero", "Minion", "Tower", "Spawner"
    pub team: TeamId,
    pub pos: HexDto,
    pub hp: u32,
    pub max_hp: u32,
    pub ap: u32,
    pub max_ap: u32,
    pub initiative: u32,
    pub attack_damage: u32,
    pub attack_range: u32,
    pub vision_range: u32,
    pub is_stationary: bool,
}

/// Arena terrain layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDto {
    pub radius: u32,
    pub walkable: Vec<HexDto>,
    pub obstacles: Vec<HexDto>,
}

/// Team-sanitized game state snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotDto {
    pub match_id: MatchId,
    pub round: Round,
    pub phase: String, // "Planning", "Resolution", "Ended"
    pub winner: Option<TeamId>,
    pub map: MapDto,
    pub units: Vec<UnitDto>,
    pub visible_hexes: Vec<HexDto>,
    pub controlled_units: Vec<UnitId>,
    pub deadline_unix_ms: Option<u64>,
    pub state_hash: String,
}

/// Order action type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActionDto {
    Wait,
    Attack { target_id: UnitId },
}

/// Unit turn order submitted by player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderDto {
    pub unit_id: UnitId,
    pub move_target: Option<HexDto>,
    pub action: ActionDto,
}

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
    UnitDied {
        unit_id: UnitId,
        unit_kind: String,
        killed_by: UnitId,
    },
    UnitWaited {
        unit_id: UnitId,
    },
    RoundEnded {
        round: Round,
    },
    MatchEnded {
        winner: Option<TeamId>,
    },
}

/// Upstream messages sent from browser client to server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Hello {
        player_id: PlayerId,
        reconnect_token: Option<ReconnectToken>,
    },
    JoinMatch {
        match_id: MatchId,
    },
    SubmitOrders {
        round: Round,
        orders: Vec<OrderDto>,
    },
    Ping {
        client_time_ms: u64,
    },
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
    RoundStarted {
        round: Round,
        deadline_unix_ms: u64,
        snapshot: SnapshotDto,
    },
    OrdersAccepted {
        round: Round,
    },
    OrderRejected {
        round: Round,
        error_code: ProtocolErrorCode,
        reason: String,
    },
    RoundResolved {
        /// The round number that was resolved (corresponds to the round that was planned).
        round: Round,
        events: Vec<SanitizedGameEvent>,
        snapshot: SnapshotDto,
    },
    MatchEnded {
        winner: Option<TeamId>,
        snapshot: SnapshotDto,
    },
    OpponentStatus {
        online: bool,
    },
    Pong {
        client_time_ms: u64,
        server_time_ms: u64,
    },
    Error {
        error_code: ProtocolErrorCode,
        message: String,
    },
}

/// Structured protocol error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolErrorCode {
    InvalidMessage,
    MatchNotFound,
    MatchFull,
    NotAuthorized,
    StaleRound,
    InvalidPhase,
    UnitNotOwned,
    UnitDead,
    InvalidTarget,
    TimerExpired,
    InternalError,
}

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
                initiative: 3,
                attack_damage: 20,
                attack_range: 1,
                vision_range: 3,
                is_stationary: false,
            }],
            visible_hexes: vec![HexDto::new(-4, -1)],
            controlled_units: vec![1],
            deadline_unix_ms: Some(1700000000000),
            state_hash: "abcd1234deadbeef".into(),
        };

        let msg = ServerMessage::RoundStarted {
            round: 1,
            deadline_unix_ms: 1700000030000,
            snapshot: snap.clone(),
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
}
