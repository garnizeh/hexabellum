use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VictoryMode {
    /// Match ends solely when an enemy Core is destroyed (Phase 7 Default).
    CoreDestruction,
    /// Match ends when all enemy heroes are eliminated (Phase 5/6 fallback).
    HeroElimination,
    /// Match ends if either Core is destroyed OR all enemy heroes are dead.
    CoreOrElimination,
}

impl Default for VictoryMode {
    fn default() -> Self {
        VictoryMode::CoreDestruction
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Phase7Config {
    /// Active victory mode governing match termination (default: CoreDestruction).
    pub victory_mode: VictoryMode,
    /// Fixed rounds a dead hero must wait before respawning (default: 3).
    pub respawn_delay_rounds: u32,
    /// Radius of the sovereign base zone around each Core (default: 2 -> 19 hexes).
    pub base_zone_radius: u32,
    /// HP restored to alive allied heroes resting in base at round start (default: 15).
    pub base_regen_per_round: u32,
    /// Initial and maximum HP for each team's Core structure (default: 700).
    pub core_hp: u32,
    /// Vision range of each Core structure in axial hex distance (default: 5).
    pub core_vision_range: u32,
    /// Initial and maximum HP for the neutral Objective Vault (default: 250).
    pub objective_hp: u32,
    /// Flat gold bounty awarded to EACH living allied hero upon Vault destruction (default: 50).
    pub objective_gold_reward_per_hero: u32,
    /// Flat XP bounty awarded to EACH living allied hero upon Vault destruction (default: 40).
    pub objective_xp_reward_per_hero: u32,
    /// Bonus attack damage granted by the Vault destruction buff (default: 5).
    pub objective_buff_damage_bonus: u32,
    /// Duration in rounds of the Vault destruction buff (default: 5).
    pub objective_buff_duration_rounds: u32,
}

impl Default for Phase7Config {
    fn default() -> Self {
        Self {
            victory_mode: VictoryMode::CoreDestruction,
            respawn_delay_rounds: 3,
            base_zone_radius: 2,
            base_regen_per_round: 15,
            core_hp: 700,
            core_vision_range: 5,
            objective_hp: 250,
            objective_gold_reward_per_hero: 50,
            objective_xp_reward_per_hero: 40,
            objective_buff_damage_bonus: 5,
            objective_buff_duration_rounds: 5,
        }
    }
}
