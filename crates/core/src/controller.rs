use crate::unit::UnitId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type PlayerId = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Controller {
    /// Commanded by an authenticated human player.
    Player(PlayerId),
    /// Commanded by server-side AI (bot or disconnected human backfill).
    Ai,
    /// Automated game entity (minions, defensive towers, spawners, neutrals).
    Automatic,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControllerMap {
    controllers: HashMap<UnitId, Controller>,
}

impl ControllerMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assign(&mut self, unit_id: UnitId, controller: Controller) {
        self.controllers.insert(unit_id, controller);
    }

    pub fn get(&self, unit_id: UnitId) -> Option<&Controller> {
        self.controllers.get(&unit_id)
    }

    pub fn is_controlled_by_player(&self, unit_id: UnitId, player_id: &PlayerId) -> bool {
        matches!(self.controllers.get(&unit_id), Some(Controller::Player(id)) if id == player_id)
    }

    pub fn get_units_for_player(&self, player_id: &PlayerId) -> Vec<UnitId> {
        let mut units: Vec<UnitId> = self
            .controllers
            .iter()
            .filter_map(|(&uid, ctrl)| match ctrl {
                Controller::Player(id) if id == player_id => Some(uid),
                _ => None,
            })
            .collect();
        units.sort_unstable();
        units
    }

    pub fn transfer_to_ai(&mut self, unit_id: UnitId) {
        if let Some(ctrl) = self.controllers.get_mut(&unit_id) {
            *ctrl = Controller::Ai;
        } else {
            self.controllers.insert(unit_id, Controller::Ai);
        }
    }

    pub fn transfer_to_player(&mut self, unit_id: UnitId, player_id: PlayerId) {
        self.controllers.insert(unit_id, Controller::Player(player_id));
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, UnitId, Controller> {
        self.controllers.iter()
    }

    pub fn len(&self) -> usize {
        self.controllers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.controllers.is_empty()
    }

    pub fn remove(&mut self, unit_id: UnitId) -> Option<Controller> {
        self.controllers.remove(&unit_id)
    }
}
