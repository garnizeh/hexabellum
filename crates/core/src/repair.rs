use crate::event::GameEvent;
use crate::unit::Unit;

pub const REPAIR_AP_COST: u32 = 1;
pub const REPAIR_AMOUNT: u32 = 20;
pub const REPAIR_RANGE: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairValidationError {
    HeroDead,
    NotAHero,
    TargetNotFound,
    TargetDead,
    TargetNotStructure,
    TargetNotRepairable,
    TargetNotAllied,
    TargetFullHealth,
    OutOfRange,
    InsufficientAP,
}

pub fn validate_repair(
    repairer: &Unit,
    target: &Unit,
) -> Result<(), RepairValidationError> {
    if !repairer.is_alive() {
        return Err(RepairValidationError::HeroDead);
    }
    if !repairer.is_hero() {
        return Err(RepairValidationError::NotAHero);
    }
    if !target.is_alive() {
        return Err(RepairValidationError::TargetDead);
    }
    if !target.is_structure() {
        return Err(RepairValidationError::TargetNotStructure);
    }
    if !target.kind.is_repairable() {
        return Err(RepairValidationError::TargetNotRepairable);
    }
    if repairer.team != target.team {
        return Err(RepairValidationError::TargetNotAllied);
    }
    if target.hp >= target.max_hp {
        return Err(RepairValidationError::TargetFullHealth);
    }
    if repairer.pos.distance(&target.pos) > REPAIR_RANGE {
        return Err(RepairValidationError::OutOfRange);
    }
    if repairer.ap < REPAIR_AP_COST {
        return Err(RepairValidationError::InsufficientAP);
    }

    Ok(())
}

pub fn execute_repair(
    repairer: &mut Unit,
    target: &mut Unit,
) -> Option<GameEvent> {
    if validate_repair(repairer, target).is_err() {
        return None;
    }

    repairer.spend_ap(REPAIR_AP_COST);
    let original_hp = target.hp;
    target.hp = (target.hp + REPAIR_AMOUNT).min(target.max_hp);
    let healed_amount = target.hp - original_hp;

    Some(GameEvent::StructureRepaired {
        repairer_id: repairer.id,
        target_id: target.id,
        amount: healed_amount,
        target_hp_remaining: target.hp,
    })
}
