use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub score: i32,
}

impl Player {
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self { id, name: name.into(), score: 0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    pub players: Vec<Player>,
    pub active_player_idx: usize,
}

impl Session {
    pub fn single_player(name: impl Into<String>) -> Self {
        Self {
            players: vec![Player::new(0, name)],
            active_player_idx: 0,
        }
    }

    pub fn active_player(&self) -> Option<&Player> {
        self.players.get(self.active_player_idx)
    }

    pub fn active_player_mut(&mut self) -> Option<&mut Player> {
        self.players.get_mut(self.active_player_idx)
    }

    pub fn add_score(&mut self, delta: i32) {
        if let Some(p) = self.active_player_mut() {
            p.score += delta;
        }
    }
}
