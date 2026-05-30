use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Screen {
    Home,
    Jeopardy,
    Trivia,
    Quizbowl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub screen: Screen,
    pub session: super::session::Session,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            screen: Screen::Home,
            session: super::session::Session::default(),
        }
    }
}
