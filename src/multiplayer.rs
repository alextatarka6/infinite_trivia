//! Phone sync for each mode: what to broadcast when the host's screen changes,
//! and how phone input feeds back into it. App owns one `Multiplayer` and calls
//! the matching tick every frame while the phone server is running.
//!
//! Every tick drains the input queue, even while a screen is loading, so a late
//! answer or buzz can never land on the next question.

use std::collections::HashMap;

use crate::game::scoring::trivia_score;
use crate::game::state::Screen;
use crate::screens::jeopardy::clue::{ClueScreen, ClueState};
use crate::screens::quizbowl::tossup::{TossupScreen, TossupState};
use crate::screens::trivia::question::TriviaScreen;
use crate::server::protocol::{ClientMsg, ServerMsg};
use crate::server::GameServer;

#[derive(Default)]
pub struct Multiplayer {
    /// Screen seen last frame: leaving a game parks phones on a waiting screen.
    pub last_screen: Option<Screen>,
    pub trivia: TriviaSync,
    pub quizbowl: QuizbowlSync,
    pub jeopardy: JeopardySync,
    pub mix_clue: ClueSync,
    /// Waiting message the phones are parked on, so it's sent only once.
    parked: Option<&'static str>,
}

impl Multiplayer {
    /// Forget per-game state; call when any game starts.
    pub fn new_game(&mut self, server: Option<&GameServer>) {
        let last_screen = self.last_screen.take();
        let player_ids = std::mem::take(&mut self.jeopardy.player_ids);
        *self = Self { last_screen, ..Self::default() };
        self.jeopardy.player_ids = player_ids;
        if let Some(s) = server {
            s.reset_scores();
        }
    }
}

impl Multiplayer {
    /// Show `msg` on every phone (once) and ignore input meanwhile.
    pub fn park(&mut self, server: &GameServer, msg: &'static str) {
        if self.parked != Some(msg) {
            self.parked = Some(msg);
            server.broadcast(&ServerMsg::Waiting { msg: msg.into() });
        }
        drain(server);
    }

    /// Phones have moved on from the parked message.
    pub fn unpark(&mut self) {
        self.parked = None;
    }
}

pub fn drain(server: &GameServer) {
    while server.pop_input().is_some() {}
}

// ── Trivia ────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct TriviaSync {
    /// Text of the question on the phones (identity survives restarts).
    question: Option<String>,
    /// (player id, correct, points) for the current question.
    answers: Vec<(u32, bool, i32)>,
    revealed: bool,
    summary_sent: bool,
    streaks: HashMap<u32, u32>,
}

impl TriviaSync {
    pub fn tick(&mut self, server: &GameServer, trivia: Option<&TriviaScreen>) {
        let Some(trivia) = trivia.filter(|t| !t.loading && t.error.is_none()) else {
            return drain(server);
        };
        let Some(q) = trivia.questions.get(trivia.current_idx) else {
            if !self.summary_sent && !trivia.questions.is_empty() {
                self.summary_sent = true;
                server.broadcast(&ServerMsg::Waiting { msg: "Round over — scores are on the TV".into() });
            }
            return drain(server);
        };
        let answers = q.shuffled_answers();
        let correct_idx = answers.iter().position(|(_, c)| *c).unwrap_or(0);

        if self.question.as_deref() != Some(q.question.as_str()) {
            self.question = Some(q.question.clone());
            self.answers.clear();
            self.revealed = false;
            self.summary_sent = false;
            let options = answers.iter().map(|(s, _)| s.clone()).collect();
            server.broadcast(&ServerMsg::TriviaQuestion { text: q.question.clone(), options });
        }

        // One answer per phone per question, scored with that phone's streak.
        while let Some(input) = server.pop_input() {
            let ClientMsg::AnswerIdx { idx } = input.msg else { continue };
            if self.revealed || self.answers.iter().any(|(id, ..)| *id == input.player_id) {
                continue;
            }
            let streak = self.streaks.entry(input.player_id).or_default();
            let correct = idx == correct_idx;
            let delta = trivia_score(correct, *streak);
            *streak = if correct { *streak + 1 } else { 0 };
            server.add_score(input.player_id, delta);
            self.answers.push((input.player_id, correct, delta));
        }

        // Reveal once every connected phone has answered, or the host's question resolves.
        let connected: Vec<u32> = server.players().iter().filter(|p| p.connected).map(|p| p.id).collect();
        let all_in = !connected.is_empty()
            && connected.iter().all(|id| self.answers.iter().any(|(a, ..)| a == id));
        if !self.revealed && (all_in || trivia.selected.is_some()) {
            self.revealed = true;
            // Phones that didn't answer lose their streak.
            for (id, streak) in self.streaks.iter_mut() {
                if !self.answers.iter().any(|(a, ..)| a == id) {
                    *streak = 0;
                }
            }
            let scorers: Vec<String> = self
                .answers
                .iter()
                .filter(|(_, c, _)| *c)
                .filter_map(|(id, ..)| server.player_name(*id))
                .collect();
            server.send_each(|id| ServerMsg::TriviaReveal {
                correct_idx,
                scorers: scorers.clone(),
                delta: self.answers.iter().find(|(a, ..)| *a == id).map(|(.., d)| *d).unwrap_or(0),
            });
        }
    }
}

// ── Quizbowl ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct QuizbowlSync {
    /// Text of the tossup being streamed (a change means a new tossup).
    question: Option<String>,
    last_chars: usize,
    /// Phone currently allowed to answer.
    buzzer: Option<u32>,
    /// The verdict for this tossup has gone out.
    resolved: bool,
}

impl QuizbowlSync {
    pub fn tick(&mut self, server: &GameServer, qb: Option<&mut TossupScreen>, now: f64) {
        let Some(qb) = qb.filter(|q| !q.loading && q.error.is_none()) else {
            return drain(server);
        };

        if self.question.as_deref() != Some(qb.tossup.question.as_str()) {
            *self = Self { question: Some(qb.tossup.question.clone()), ..Self::default() };
            server.broadcast(&ServerMsg::QbReading { text: String::new(), can_buzz: true });
        }

        // Stream the text every 4 characters, plus once when reading finishes.
        let total = qb.tossup.question.chars().count();
        let chars = qb.chars_revealed;
        let reading = matches!(qb.state, TossupState::Reading | TossupState::FinishedReading { .. });
        if reading && (chars >= self.last_chars + 4 || (chars == total && self.last_chars != total)) {
            self.last_chars = chars;
            let text: String = qb.tossup.question.chars().take(chars).collect();
            server.broadcast(&ServerMsg::QbReading { text, can_buzz: qb.mp_buzzed_name.is_none() });
        }

        while let Some(input) = server.pop_input() {
            match input.msg {
                ClientMsg::Buzz => {
                    let open = qb.mp_buzzed_name.is_none()
                        && matches!(qb.state, TossupState::Reading | TossupState::FinishedReading { .. });
                    if !open {
                        continue;
                    }
                    let name = server.player_name(input.player_id).unwrap_or_default();
                    qb.mp_buzzed_name = Some(name.clone());
                    qb.state = TossupState::BuzzedIn { answer_input: String::new() };
                    self.buzzer = Some(input.player_id);
                    server.send_each(|id| ServerMsg::QbBuzzed {
                        player_name: name.clone(),
                        is_you: id == input.player_id,
                    });
                }
                ClientMsg::AnswerText { text } => {
                    if self.buzzer != Some(input.player_id) || self.resolved {
                        continue;
                    }
                    let (correct, delta) = qb.phone_answer(&text, now);
                    server.add_score(input.player_id, delta);
                    let scorer = correct.then(|| server.player_name(input.player_id)).flatten();
                    server.broadcast(&ServerMsg::QbResult { correct, answer: qb.tossup.answer.clone(), scorer, delta });
                    self.buzzer = None;
                    self.resolved = true;
                }
                _ => {}
            }
        }

        // The host answered or time ran out: tell the phones too.
        if !self.resolved {
            let verdict = match qb.state {
                TossupState::Result { correct, .. } => Some(correct),
                TossupState::TimedOut => Some(false),
                _ => None,
            };
            if let Some(correct) = verdict {
                self.resolved = true;
                self.buzzer = None;
                let scorer = correct.then(|| "Host".to_string());
                server.broadcast(&ServerMsg::QbResult { correct, answer: qb.tossup.answer.clone(), scorer, delta: 0 });
            }
        }
    }
}

// ── Jeopardy (driven by App, which owns the board) ───────────────────────────

#[derive(Default)]
pub struct JeopardySync {
    /// Phone player id for each board seat (same order as `Board::players`).
    pub player_ids: Vec<u32>,
    pub board_sent: bool,
    pub clue_sent: bool,
    pub dd_clue_sent: bool,
    pub result_sent: bool,
    pub final_clue_sent: bool,
    pub over_sent: bool,
}

impl JeopardySync {
    /// Board seat for a phone player, if they have one.
    pub fn seat(&self, player_id: u32) -> Option<usize> {
        self.player_ids.iter().position(|&id| id == player_id)
    }
}

// ── Mix: open Jeopardy clues anyone can answer ────────────────────────────────

#[derive(Default)]
pub struct ClueSync {
    /// Question text of the clue on the phones.
    question: Option<String>,
    answerer: Option<u32>,
    result_sent: bool,
}

impl ClueSync {
    /// Mix's Jeopardy clues go to every phone; the first response is judged
    /// and scores that phone (no penalty for a miss, matching Mix scoring).
    pub fn tick(&mut self, server: &GameServer, clue: Option<&mut ClueScreen>, now: f64) {
        let Some(cs) = clue else { return drain(server) };
        if self.question.as_deref() != Some(cs.clue.question.as_str()) {
            *self = Self { question: Some(cs.clue.question.clone()), ..Self::default() };
            server.broadcast(&ServerMsg::JeopardyClue {
                category: cs.clue.category.clone(),
                value: cs.clue.value,
                text: cs.clue.question.clone(),
                is_you_answer: true,
                active_player: "Everyone".into(),
            });
        }
        while let Some(input) = server.pop_input() {
            if let ClientMsg::AnswerText { text } = input.msg {
                if self.answerer.is_none() && matches!(cs.state, ClueState::ShowQuestion) {
                    self.answerer = Some(input.player_id);
                    cs.set_phone_answer(&text, now);
                }
            }
        }
        if let (ClueState::Answered { correct }, false) = (&cs.state, self.result_sent) {
            self.result_sent = true;
            let correct = *correct;
            let value = cs.clue.value.max(200);
            if correct {
                if let Some(id) = self.answerer {
                    server.add_score(id, value);
                }
            }
            server.send_each(|id| ServerMsg::JeopardyResult {
                correct,
                answer: cs.clue.answer.clone(),
                delta: if correct && Some(id) == self.answerer { value } else { 0 },
            });
        }
    }
}
