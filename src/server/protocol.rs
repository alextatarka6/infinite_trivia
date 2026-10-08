use serde::{Deserialize, Serialize};

/// Messages the host server sends to phone clients.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    /// Sent once a phone's `hello`/`join` binds it to a seat.
    Joined {
        you: u32,
        name: String,
    },
    /// Sent in reply to `hello` when the phone has no seat yet.
    NeedName,
    Lobby {
        players: Vec<PlayerInfo>,
    },
    // Trivia
    TriviaQuestion {
        text: String,
        options: Vec<String>,
    },
    TriviaReveal {
        correct_idx: usize,
        scorers: Vec<String>,
        /// This phone's points for the question (streak multiplier included).
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
        active_player: String,
    },
    JeopardyDd {
        category: String,
        value: i32,
        max_wager: i32,
        is_you_answer: bool,
        active_player: String,
    },
    /// Final Jeopardy: every team with money wagers on its phone.
    JeopardyFinalWager {
        category: String,
        max_wager: i32,
        can_play: bool,
    },
    /// Final Jeopardy: the clue, answered on every playing phone.
    JeopardyFinalClue {
        category: String,
        text: String,
        can_answer: bool,
    },
    JeopardyResult {
        correct: bool,
        answer: String,
        delta: i32,
    },
    Waiting {
        msg: String,
    },
}

/// Messages phone clients send to the host server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    /// First message on every (re)connect; `token` is the phone's persisted seat key.
    Hello { token: String },
    Join { name: String, token: String },
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
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct PhoneInput {
    pub player_id: u32,
    pub msg: ClientMsg,
}
