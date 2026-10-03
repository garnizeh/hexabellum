use hexabellum_protocol::{ErrorCode, HeroDefId, PlayerId};
use std::collections::HashMap;

/// Team-scoped hero selection draft coordinator.
#[derive(Debug, Clone)]
pub struct HeroSelectDraft {
    pub hero_pool: Vec<HeroDefId>,
    pub selections: HashMap<PlayerId, HeroDefId>,
    pub registered_players: Vec<PlayerId>,
}

impl HeroSelectDraft {
    pub fn new_team_draft(pool: Vec<HeroDefId>) -> Self {
        Self {
            hero_pool: pool,
            selections: HashMap::new(),
            registered_players: Vec::new(),
        }
    }

    pub fn register_player(&mut self, player_id: &str) {
        if !self.registered_players.iter().any(|p| p == player_id) {
            self.registered_players.push(player_id.to_string());
        }
    }

    pub fn select_hero(&mut self, player_id: &str, hero_def_id: &str) -> Result<(), ErrorCode> {
        if !self.hero_pool.iter().any(|h| h == hero_def_id) {
            return Err(ErrorCode::InvalidHeroDef);
        }

        // Uniqueness within team: reject if another ally already chose this hero
        for (other_pid, selected) in &self.selections {
            if other_pid != player_id && selected == hero_def_id {
                return Err(ErrorCode::HeroAlreadySelected);
            }
        }

        self.register_player(player_id);
        self.selections.insert(player_id.to_string(), hero_def_id.to_string());
        Ok(())
    }

    pub fn get_selected_hero(&self, player_id: &str) -> Option<&str> {
        self.selections.get(player_id).map(|s| s.as_str())
    }

    pub fn handle_timeout(&mut self) {
        for pid in &self.registered_players {
            if !self.selections.contains_key(pid) {
                if let Some(available) = self.hero_pool.iter().find(|h| {
                    !self.selections.values().any(|picked| picked == *h)
                }) {
                    self.selections.insert(pid.clone(), available.clone());
                }
            }
        }
    }

    pub fn is_hero_available(&self, hero_def_id: &str) -> bool {
        self.hero_pool.iter().any(|h| h == hero_def_id)
            && !self.selections.values().any(|picked| picked == hero_def_id)
    }
}
