use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub score: i32,
    /// Two-letter monogram shown on the default crest.
    pub monogram: String,
    /// Uploaded team/player photo. Not serialized: textures live on the host only.
    #[serde(skip)]
    pub photo: Option<egui::TextureHandle>,
}

// Manual Debug: TextureHandle doesn't implement it.
impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("score", &self.score)
            .field("monogram", &self.monogram)
            .field("photo", &self.photo.is_some())
            .finish()
    }
}

impl Player {
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        let name = name.into();
        Self { id, monogram: monogram(&name), name, score: 0, photo: None }
    }
}

/// Built-in default crests: the line pattern drawn on a photo plate when no
/// photo has been uploaded. Picked deterministically from the team name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crest {
    Hatch,
    Grid,
    Scan,
    Checker,
}

impl Crest {
    pub fn for_name(name: &str) -> Self {
        let h = name.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
        [Crest::Hatch, Crest::Grid, Crest::Scan, Crest::Checker][(h % 4) as usize]
    }
}

/// "Moths" -> "MO", "Riley the Great" -> "RT", "" -> "?".
pub fn monogram(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let letters: String = match words.as_slice() {
        [] => "?".into(),
        [one] => one.chars().filter(|c| c.is_alphanumeric()).take(2).collect(),
        [a, b, ..] => a.chars().take(1).chain(b.chars().take(1)).collect(),
    };
    letters.to_uppercase()
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
