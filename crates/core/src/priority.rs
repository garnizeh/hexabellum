use crate::unit::{Unit, UnitId, UnitKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityClass {
    Highest = 0,
    High = 1,
    Medium = 2,
    Low = 3,
    Lowest = 4,
    Excluded = 5,
}

pub struct TargetScore {
    pub priority: PriorityClass,
    pub distance: u32,
    pub hp: u32,
    pub unit_id: UnitId,
}

impl TargetScore {
    /// Deterministic comparison tuple: PriorityClass -> Distance -> HP -> UnitId
    pub fn to_key(&self) -> (PriorityClass, u32, u32, UnitId) {
        (self.priority, self.distance, self.hp, self.unit_id)
    }
}

pub fn evaluate_minion_target_priority(target: &Unit) -> PriorityClass {
    match target.kind {
        UnitKind::Minion => PriorityClass::Highest,
        UnitKind::Hero => PriorityClass::High,
        UnitKind::Tower => PriorityClass::Medium,
        UnitKind::Spawner => PriorityClass::Low,
        UnitKind::Core => PriorityClass::Lowest,
        UnitKind::Objective | UnitKind::NeutralGuardian => PriorityClass::Excluded,
    }
}

pub fn evaluate_tower_target_priority(target: &Unit) -> PriorityClass {
    match target.kind {
        UnitKind::Minion => PriorityClass::Highest,
        UnitKind::Hero => PriorityClass::High,
        UnitKind::Tower | UnitKind::Spawner | UnitKind::Core => PriorityClass::Medium,
        UnitKind::Objective | UnitKind::NeutralGuardian => PriorityClass::Excluded,
    }
}

pub fn evaluate_hero_ai_target_priority(target: &Unit) -> PriorityClass {
    match target.kind {
        UnitKind::Hero => PriorityClass::Highest,
        UnitKind::Minion => PriorityClass::High,
        UnitKind::Tower | UnitKind::Spawner => PriorityClass::Medium,
        UnitKind::Core => PriorityClass::Low,
        UnitKind::Objective | UnitKind::NeutralGuardian => PriorityClass::Lowest,
    }
}

pub fn evaluate_neutral_target_priority(
    target: &Unit,
    last_attacker: Option<UnitId>,
) -> PriorityClass {
    if Some(target.id) == last_attacker {
        PriorityClass::Highest
    } else {
        PriorityClass::High
    }
}
