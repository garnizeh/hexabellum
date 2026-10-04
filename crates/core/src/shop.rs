use crate::items::{ItemDef, StatKind};
use crate::unit::Unit;
use hexabellum_protocol::ProtocolErrorCode;

pub fn execute_purchase(
    unit: &mut Unit,
    item: &ItemDef,
    max_slots: usize,
    allow_duplicates: bool,
) -> Result<(), ProtocolErrorCode> {
    if !unit.is_alive() {
        return Err(ProtocolErrorCode::UnitDead);
    }
    if !unit.is_hero() {
        return Err(ProtocolErrorCode::NotAHero);
    }
    if unit.items.len() >= max_slots {
        return Err(ProtocolErrorCode::InventoryFull);
    }
    if !allow_duplicates && unit.items.contains(&item.id) {
        return Err(ProtocolErrorCode::ItemAlreadyOwned);
    }
    if unit.gold < item.cost {
        return Err(ProtocolErrorCode::InsufficientGold);
    }

    // Deduct currency
    unit.gold -= item.cost;
    unit.items.push(item.id.clone());

    // Apply permanent stat bonuses
    for modifier in &item.modifiers {
        match modifier.stat {
            StatKind::AttackDamage => {
                unit.attack_damage += modifier.value;
            }
            StatKind::MaxHealth => {
                unit.max_hp += modifier.value;
                // Immediate healing bonus
                unit.hp = (unit.hp + modifier.value).min(unit.max_hp);
            }
            StatKind::VisionRange => {
                unit.vision_range += modifier.value;
            }
            StatKind::EnergyRegen => {
                unit.energy_regen += modifier.value;
            }
        }
    }

    Ok(())
}
