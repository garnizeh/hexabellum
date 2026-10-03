use crate::hex::HexCoord;
use crate::unit::UnitId;
use serde::{Deserialize, Serialize};

/// Action a unit can take after (or instead of) moving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Wait,
    Attack { target_id: UnitId },
}

/// A single unit's orders for the round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitOrder {
    pub unit_id: UnitId,
    pub move_target: Option<HexCoord>,
    pub action: Action,
}

/// All orders collected during planning phase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnOrders {
    pub orders: Vec<UnitOrder>,
}

impl TurnOrders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_order(&mut self, order: UnitOrder) {
        // Replace existing order for same unit
        self.orders.retain(|o| o.unit_id != order.unit_id);
        self.orders.push(order);
    }

    pub fn get_order(&self, unit_id: UnitId) -> Option<&UnitOrder> {
        self.orders.iter().find(|o| o.unit_id == unit_id)
    }

    pub fn clear(&mut self) {
        self.orders.clear();
    }
}
