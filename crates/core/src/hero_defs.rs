use crate::ability::{EffectDef, EffectKind, SpellDef, TargetingMode};
use crate::status::{StatKind, StatModifier, StatusDef};
use serde::{Deserialize, Serialize};

pub type HeroDefId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeroDef {
    pub id: HeroDefId,
    pub name: String,
    pub role: String,
    pub max_hp: u32,
    pub max_ap: u32,
    pub initiative: u32,
    pub attack_damage: u32,
    pub attack_range: u32,
    pub vision_range: u32,
    pub max_energy: u32,
    pub energy_regen: u32,
    pub spell: SpellDef,
}

pub fn get_vanguard_def() -> HeroDef {
    HeroDef {
        id: "vanguard".to_string(),
        name: "Vanguard".to_string(),
        role: "Frontline Tank".to_string(),
        max_hp: 140,
        max_ap: 3,
        initiative: 3,
        attack_damage: 18,
        attack_range: 1,
        vision_range: 3,
        max_energy: 5,
        energy_regen: 1,
        spell: SpellDef {
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
        },
    }
}

pub fn get_ranger_def() -> HeroDef {
    HeroDef {
        id: "ranger".to_string(),
        name: "Ranger".to_string(),
        role: "Marksman".to_string(),
        max_hp: 90,
        max_ap: 3,
        initiative: 4,
        attack_damage: 16,
        attack_range: 2,
        vision_range: 4,
        max_energy: 5,
        energy_regen: 1,
        spell: SpellDef {
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
        },
    }
}

pub fn get_warden_def() -> HeroDef {
    HeroDef {
        id: "warden".to_string(),
        name: "Warden".to_string(),
        role: "Support / Healer".to_string(),
        max_hp: 100,
        max_ap: 3,
        initiative: 2,
        attack_damage: 12,
        attack_range: 1,
        vision_range: 4,
        max_energy: 6,
        energy_regen: 1,
        spell: SpellDef {
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
        },
    }
}

pub fn get_sniper_def() -> HeroDef {
    HeroDef {
        id: "sniper".to_string(),
        name: "Sniper".to_string(),
        role: "Artillery".to_string(),
        max_hp: 80,
        max_ap: 3,
        initiative: 4,
        attack_damage: 14,
        attack_range: 3,
        vision_range: 5,
        max_energy: 5,
        energy_regen: 1,
        spell: SpellDef {
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
        },
    }
}

pub fn get_berserker_def() -> HeroDef {
    HeroDef {
        id: "berserker".to_string(),
        name: "Berserker".to_string(),
        role: "Bruiser".to_string(),
        max_hp: 120,
        max_ap: 3,
        initiative: 3,
        attack_damage: 20,
        attack_range: 1,
        vision_range: 3,
        max_energy: 5,
        energy_regen: 1,
        spell: SpellDef {
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
                    modifiers: vec![StatModifier {
                        stat: StatKind::AttackDamage,
                        value: 8,
                    }],
                }),
            }],
        },
    }
}

pub fn get_hero_def(id: &str) -> Option<HeroDef> {
    match id {
        "vanguard" => Some(get_vanguard_def()),
        "ranger" => Some(get_ranger_def()),
        "warden" => Some(get_warden_def()),
        "sniper" => Some(get_sniper_def()),
        "berserker" => Some(get_berserker_def()),
        _ => None,
    }
}

pub fn get_all_hero_defs() -> Vec<HeroDef> {
    vec![
        get_vanguard_def(),
        get_ranger_def(),
        get_warden_def(),
        get_sniper_def(),
        get_berserker_def(),
    ]
}
