use serde::{Deserialize, Serialize};

/// Messages the host server sends to phone clients.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Lobby {
        players: Vec<PlayerInfo>,
        you: u32,
    },
    // Trivia
    TriviaQuestion {
        text: String,
        options: Vec<String>,
    },
    TriviaReveal {
        correct_idx: usize,
        scorer: Option<String>,
        delta: i32,
    },
    // Quizbowl
    QbReading {
        text: String,
        can_buzz: bool,
    },
    QbBuzzed {
        player_name: String,
        is_you: bool,
    },
    QbResult {
        correct: bool,
        answer: String,
        scorer: Option<String>,
        delta: i32,
    },
    // Jeopardy
    JeopardyBoard {
        categories: Vec<String>,
        values: Vec<Vec<i32>>,
        used: Vec<Vec<bool>>,
        scores: Vec<(String, i32)>,
        active_player: String,
        is_your_turn: bool,
    },
    JeopardyClue {
        category: String,
        value: i32,
        text: String,
        is_you_answer: bool,
    },
    JeopardyDd {
        category: String,
        value: i32,
        max_wager: i32,
        is_you_answer: bool,
        active_player: String,
    },
    JeopardyResult {
        correct: bool,
        answer: String,
        delta: i32,
    },
    Waiting {
        msg: String,
    },
    Scores {
        scores: Vec<(String, i32)>,
    },
}

/// Messages phone clients send to the host server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Join { name: String },
    AnswerIdx { idx: usize },
    AnswerText { text: String },
    Buzz,
    PickClue { col: usize, row: usize },
    Wager { amount: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: u32,
    pub name: String,
    pub score: i32,
}

#[derive(Debug, Clone)]
pub struct PhoneInput {
    pub player_id: u32,
    pub msg: ClientMsg,
}
