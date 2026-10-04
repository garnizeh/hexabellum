use serde::{Deserialize, Serialize};

pub type ItemDefId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatKind {
    AttackDamage,
    MaxHealth,
    VisionRange,
    EnergyRegen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatModifier {
    pub stat: StatKind,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: ItemDefId,
    pub name: String,
    pub cost: u32,
    pub description: String,
    pub icon: String,
    pub modifiers: Vec<StatModifier>,
}

pub fn get_canonical_item_catalog() -> Vec<ItemDef> {
    vec![
        ItemDef {
            id: "longblade".to_string(),
            name: "Longblade".to_string(),
            cost: 100,
            description: "Tempered high-carbon steel blade. Increases attack damage by 6.".to_string(),
            icon: "item_longblade".to_string(),
            modifiers: vec![StatModifier {
                stat: StatKind::AttackDamage,
                value: 6,
            }],
        },
        ItemDef {
            id: "plate_armor".to_string(),
            name: "Plate Armor".to_string(),
            cost: 120,
            description: "Reinforced cuirass. Increases maximum health and current health by 35.".to_string(),
            icon: "item_plate_armor".to_string(),
            modifiers: vec![StatModifier {
                stat: StatKind::MaxHealth,
                value: 35,
            }],
        },
        ItemDef {
            id: "scout_lens".to_string(),
            name: "Scout Lens".to_string(),
            cost: 80,
            description: "Convex optical lens. Expands sight radius into the fog of war by 1 hex.".to_string(),
            icon: "item_scout_lens".to_string(),
            modifiers: vec![StatModifier {
                stat: StatKind::VisionRange,
                value: 1,
            }],
        },
        ItemDef {
            id: "focus_charm".to_string(),
            name: "Focus Charm".to_string(),
            cost: 100,
            description: "Inscribed channel talisman. Restores 1 additional energy at round start.".to_string(),
            icon: "item_focus_charm".to_string(),
            modifiers: vec![StatModifier {
                stat: StatKind::EnergyRegen,
                value: 1,
            }],
        },
    ]
}

pub fn get_item_def(id: &str) -> Option<ItemDef> {
    get_canonical_item_catalog().into_iter().find(|i| i.id == id)
}
