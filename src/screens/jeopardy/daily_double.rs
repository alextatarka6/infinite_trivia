//! Daily Double — "04 Daily Double" in the Roster hi-fi.

use crate::api::jeopardy::{check_answer, Clue};
use crate::screens::jeopardy::wager;
use crate::screens::kit::{self, Canvas, Texture};

pub enum DailyDoubleAction {
    Done { winnings: i32 },
}

pub struct DailyDoubleScreen {
    pub clue: Clue,
    pub current_score: i32,
    pub wager_input: String,
    pub wager: Option<i32>,
    pub answer_input: String,
    pub answered: Option<bool>,
    /// Teams + the wagering one, copied from the board by App for the roster.
    pub teams: Vec<(String, i32)>,
    pub active: usize,
    enter_start: f64,
    result_start: f64,
    focus_field: bool,
}

impl DailyDoubleScreen {
    pub fn new(clue: Clue, current_score: i32) -> Self {
        Self {
            clue,
            current_score,
            wager_input: String::new(),
            wager: None,
            answer_input: String::new(),
            answered: None,
            teams: Vec::new(),
            active: 0,
            enter_start: -1.0,
            result_start: -10.0,
            focus_field: true,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<DailyDoubleAction> {
        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        let c = Canvas::new(ui.ctx());
        let enter = kit::since(ui, self.enter_start, 0.5);
        let mut action = None;
        let max_wager = self.max_wager();
        let team = self.teams.get(self.active).map(|(n, _)| n.clone()).unwrap_or_else(|| "You".into());

        match (self.wager, self.answered) {
            (None, _) => {
                kit::backdrop(ui.ctx(), None);
                wager::band(ui, &c, "DAILY DOUBLE", 1.0, enter);
                let who = if self.teams.len() > 1 { format!("{} WAGER", team.to_uppercase()) } else { "YOUR WAGER".into() };
                let heading = format!("{} · {who}", self.clue.category.to_uppercase());
                if let Some(w) = wager::wager_panel(ui, &c, &heading, &mut self.wager_input, 5, max_wager, self.focus_field, "dd") {
                    self.wager = Some(w);
                    self.focus_field = true;
                }
                self.focus_field = false;
                wager::team_line(ui, &c, &team, self.current_score, "ONLY YOU PLAY THIS ONE");
            }
            (Some(w), None) => {
                kit::backdrop(ui.ctx(), Some(Texture::Grid));
                wager::band(ui, &c, &format!("DAILY DOUBLE · {}", kit::money(w)), 0.0, 1.0);
                if wager::question(ui, &c, &self.clue.category, &self.clue.question, &mut self.answer_input, self.focus_field, "dd") {
                    self.submit(now);
                }
                self.focus_field = false;
                wager::roster(ui, &c, &self.teams, self.active);
            }
            (Some(w), Some(correct)) => {
                kit::backdrop(ui.ctx(), Some(Texture::Grid));
                wager::band(ui, &c, "DAILY DOUBLE", 0.0, 1.0);
                let delta = if correct { w } else { -w };
                let since = kit::since(ui, self.result_start, 0.4);
                if wager::result(ui, &c, since, correct, delta, &self.clue.answer, None, "Back to board", "dd") {
                    action = Some(DailyDoubleAction::Done { winnings: delta });
                }
                wager::roster(ui, &c, &self.teams, self.active);
            }
        }
        action
    }

    fn confirm_wager(&mut self, max_wager: i32) {
        if let Ok(w) = self.wager_input.trim().parse::<i32>() {
            self.wager = Some(w.clamp(5, max_wager));
            self.focus_field = true;
        }
    }

    fn submit(&mut self, now: f64) {
        let correct = check_answer(&self.answer_input, &self.clue.answer);
        self.answered = Some(correct);
        self.result_start = now;
    }

    /// The larger of the current score and the round's top clue value.
    pub fn max_wager(&self) -> i32 {
        let top = if self.clue.round == "Double Jeopardy!" { 2000 } else { 1000 };
        self.current_score.max(top)
    }

    /// Called by App when the wagering phone player locks in a wager.
    pub fn set_phone_wager(&mut self, amount: i32) {
        if self.wager.is_none() {
            self.wager_input = amount.to_string();
            self.confirm_wager(self.max_wager());
        }
    }

    /// Called by App when the wagering phone player submits a response.
    /// Judge a phone's response. Returns the verdict only for the first
    /// response, so a resent answer can't be scored twice.
    pub fn set_phone_answer(&mut self, text: &str, now: f64) -> Option<bool> {
        if self.wager.is_some() && self.answered.is_none() {
            self.answer_input = text.to_string();
            self.submit(now);
            return self.answered;
        }
        None
    }
}
