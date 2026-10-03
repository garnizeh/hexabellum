use crate::hex::HexCoord;
use crate::status::StatusDef;
use crate::unit::UnitId;
use serde::{Deserialize, Serialize};

pub type SpellId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetingMode {
    SelfOnly,
    EnemyUnit,
    AllyUnit,
    UnitAny,
    Hex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    Damage,
    Heal,
    ApplyStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectDef {
    pub kind: EffectKind,
    pub amount: u32,
    pub radius: Option<u32>,
    pub status: Option<StatusDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellDef {
    pub id: SpellId,
    pub name: String,
    pub ap_cost: u32,
    pub energy_cost: u32,
    pub cooldown: u32,
    pub range: u32,
    pub min_range: u32,
    pub targeting: TargetingMode,
    pub requires_line_of_sight: bool,
    pub effects: Vec<EffectDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellTarget {
    None,
    Unit(UnitId),
    Hex(HexCoord),
}

pub struct SpellCatalog;

impl SpellCatalog {
    pub fn get(id: &str) -> Option<SpellDef> {
        match id {
            "cleave" => Some(SpellDef {
                id: "cleave".to_string(),
                name: "Cleave".to_string(),
                ap_cost: 1,
                energy_cost: 3,
                cooldown: 3,
                range: 0,
                min_range: 0,
                targeting: TargetingMode::SelfOnly,
                requires_line_of_sight: false,
                effects: vec![EffectDef {
                    kind: EffectKind::Damage,
                    amount: 15,
                    radius: Some(1),
                    status: None,
                }],
            }),
            "bolt" => Some(SpellDef {
                id: "bolt".to_string(),
                name: "Bolt".to_string(),
                ap_cost: 1,
                energy_cost: 2,
                cooldown: 2,
                range: 3,
                min_range: 1,
                targeting: TargetingMode::EnemyUnit,
                requires_line_of_sight: true,
                effects: vec![EffectDef {
                    kind: EffectKind::Damage,
                    amount: 25,
                    radius: None,
                    status: None,
                }],
            }),
            "mend" => Some(SpellDef {
                id: "mend".to_string(),
                name: "Mend".to_string(),
                ap_cost: 1,
                energy_cost: 2,
                cooldown: 2,
                range: 2,
                min_range: 1,
                targeting: TargetingMode::AllyUnit,
                requires_line_of_sight: true,
                effects: vec![EffectDef {
                    kind: EffectKind::Heal,
                    amount: 20,
                    radius: None,
                    status: None,
                }],
            }),
            "longshot" => Some(SpellDef {
                id: "longshot".to_string(),
                name: "Longshot".to_string(),
                ap_cost: 1,
                energy_cost: 3,
                cooldown: 3,
                range: 4,
                min_range: 2,
                targeting: TargetingMode::EnemyUnit,
                requires_line_of_sight: true,
                effects: vec![EffectDef {
                    kind: EffectKind::Damage,
                    amount: 30,
                    radius: None,
                    status: None,
                }],
            }),
            "fury" => Some(SpellDef {
                id: "fury".to_string(),
                name: "Fury".to_string(),
                ap_cost: 1,
                energy_cost: 2,
                cooldown: 3,
                range: 0,
                min_range: 0,
                targeting: TargetingMode::SelfOnly,
                requires_line_of_sight: false,
                effects: vec![EffectDef {
                    kind: EffectKind::ApplyStatus,
                    amount: 0,
                    radius: None,
                    status: Some(StatusDef {
                        id: "fury_buff".to_string(),
                        duration_rounds: 2,
                        modifiers: vec![crate::status::StatModifier {
                            stat: crate::status::StatKind::AttackDamage,
                            value: 8,
                        }],
                    }),
                }],
            }),
            _ => None,
        }
    }
}
