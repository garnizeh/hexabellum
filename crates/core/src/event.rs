use crate::hex::HexCoord;
use crate::unit::UnitId;
use serde::{Deserialize, Serialize};

/// Events emitted during resolution.
/// The client uses these to animate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    RoundStarted { round: u32 },
    UnitMoved { unit_id: UnitId, from: HexCoord, to: HexCoord },
    RoundEnded { round: u32 },
    // Future: UnitAttacked, UnitDied, etc.
}
