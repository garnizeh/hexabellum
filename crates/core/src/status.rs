use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatKind {
    AttackDamage,
    VisionRange,
    AttackRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatModifier {
    pub stat: StatKind,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusDef {
    pub id: String,
    pub duration_rounds: u32,
    pub modifiers: Vec<StatModifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusInstance {
    pub def_id: String,
    pub remaining_rounds: u32,
    pub modifiers: Vec<StatModifier>,
}

impl StatusInstance {
    pub fn from_def(def: &StatusDef) -> Self {
        Self {
            def_id: def.id.clone(),
            remaining_rounds: def.duration_rounds,
            modifiers: def.modifiers.clone(),
        }
    }

    /// Decrement round duration. Returns true if expired.
    pub fn tick(&mut self) -> bool {
        self.remaining_rounds = self.remaining_rounds.saturating_sub(1);
        self.remaining_rounds == 0
    }
}

pub fn neutral_camp_damage_buff_def() -> StatusDef {
    StatusDef {
        id: "neutral_damage_buff".to_string(),
        duration_rounds: 5,
        modifiers: vec![StatModifier {
            stat: StatKind::AttackDamage,
            value: 5,
        }],
    }
}

pub fn fury_buff_def() -> StatusDef {
    StatusDef {
        id: "fury_buff".to_string(),
        duration_rounds: 2,
        modifiers: vec![StatModifier {
            stat: StatKind::AttackDamage,
            value: 8,
        }],
    }
}

pub fn vault_damage_buff_def(duration_rounds: u32, damage_bonus: u32) -> StatusDef {
    StatusDef {
        id: "attack_damage_buff".to_string(),
        duration_rounds,
        modifiers: vec![StatModifier {
            stat: StatKind::AttackDamage,
            value: damage_bonus as i32,
        }],
    }
}

