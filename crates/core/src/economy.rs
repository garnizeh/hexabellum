use serde::{Deserialize, Serialize};

pub type ItemDefId = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyConfig {
    /// Gold granted to each hero at match start (default: 50).
    pub starting_gold: u32,
    /// Passive gold granted to each living hero at round start (default: 6).
    pub passive_income_per_round: u32,

    /// Gold bounty awarded to killer hero on enemy hero kill (default: 30).
    pub hero_kill_gold: u32,
    /// Experience bounty awarded to killer hero on enemy hero kill (default: 30).
    pub hero_kill_xp: u32,

    /// Gold bounty awarded to killer hero on enemy minion kill (default: 10).
    pub minion_kill_gold: u32,
    /// Experience bounty awarded to killer hero on enemy minion kill (default: 10).
    pub minion_kill_xp: u32,

    /// Gold bounty awarded to killer hero on neutral guardian kill (default: 40).
    pub neutral_kill_gold: u32,
    /// Experience bounty awarded to killer hero on neutral guardian kill (default: 25).
    pub neutral_kill_xp: u32,

    /// Gold bounty awarded to EVERY living allied hero when an enemy tower is destroyed (default: 25).
    pub tower_destroy_gold_per_hero: u32,
    /// Experience bounty awarded to EVERY living allied hero when an enemy tower is destroyed (default: 20).
    pub tower_destroy_xp_per_hero: u32,

    /// Gold bounty awarded to EVERY living allied hero when an enemy spawner is destroyed (default: 30).
    pub spawner_destroy_gold_per_hero: u32,
    /// Experience bounty awarded to EVERY living allied hero when an enemy spawner is destroyed (default: 25).
    pub spawner_destroy_xp_per_hero: u32,

    /// Maximum hero level attainable in Phase 6 (default: 5).
    pub max_level: u32,
    /// Maximum item slots per hero (default: 3).
    pub max_item_slots: usize,
    /// Whether duplicate copies of the same item can be purchased (default: false).
    pub allow_duplicate_items: bool,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            starting_gold: 50,
            passive_income_per_round: 6,
            hero_kill_gold: 30,
            hero_kill_xp: 30,
            minion_kill_gold: 10,
            minion_kill_xp: 10,
            neutral_kill_gold: 40,
            neutral_kill_xp: 25,
            tower_destroy_gold_per_hero: 25,
            tower_destroy_xp_per_hero: 20,
            spawner_destroy_gold_per_hero: 30,
            spawner_destroy_xp_per_hero: 25,
            max_level: 5,
            max_item_slots: 3,
            allow_duplicate_items: false,
        }
    }
}
