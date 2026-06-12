use egui::{Align2, Color32, Id, Pos2, Rect, RichText, Rounding, Sense, TextFormat, Vec2};
use egui::text::LayoutJob;
use crate::api::qbreader::{check_answer, Tossup};
use crate::game::scoring::quizbowl_score;
use crate::screens::kit;
use crate::theme::{
    self, ACCENT_SOFT, GOLD, GREEN_CORRECT, INK_DIM, ON_ACCENT, RED_WRONG, WHITE,
};

const CHARS_PER_SECOND: f32 = 20.0;

pub enum TossupAction {
    Next,
    /// Correct tossup — advance to a bonus round before the next tossup.
    Bonus,
    Quit,
}

pub enum TossupState {
    Reading,
    FinishedReading { elapsed: f32 },
    BuzzedIn { answer_input: String },
    Result { correct: bool, score_delta: i32 },
    TimedOut,
}

pub struct TossupScreen {
    pub tossup: Tossup,
    pub score: i32,
    pub questions_done: u32,
    pub chars_revealed: usize,
    pub char_timer: f32,
    pub state: TossupState,
    pub loading: bool,
    pub error: Option<String>,
    // animation bookkeeping
    enter_start: f64,
    result_start: f64,
    bump_start: f64,
    focus_answer: bool,
}

impl TossupScreen {
    pub fn new(tossup: Tossup, score: i32, questions_done: u32) -> Self {
        Self {
            tossup,
            score,
            questions_done,
            chars_revealed: 0,
            char_timer: 0.0,
            state: TossupState::Reading,
            loading: false,
            error: None,
            enter_start: -1.0,
            result_start: -10.0,
            bump_start: -10.0,
            focus_answer: false,
        }
    }

    pub fn loading(score: i32, questions_done: u32) -> Self {
        let dummy = Tossup {
            question: String::new(),
            answer: String::new(),
            accepted_answers: vec![],
            category: String::new(),
            difficulty: String::new(),
        };
        let mut s = Self::new(dummy, score, questions_done);
        s.loading = true;
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> Option<TossupAction> {
        kit::ambient(ui.ctx());

        // -- Loading ----------------------------------------------------------
        if self.loading {
            kit::loader(ui, "Finding a tossup…");
            return None;
        }

        // -- Error ------------------------------------------------------------
        if let Some(err) = self.error.clone() {
            let mut action = None;
            let avail = ui.available_size();
            ui.add_space((avail.y - 120.0).max(16.0) / 2.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Could not fetch question")
                        .font(theme::spectral(26.0))
                        .color(RED_WRONG),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(&err).font(theme::plex(14.0)).color(INK_DIM));
                ui.add_space(18.0);
                if kit::primary_button(ui, "Back to menu").clicked() {
                    action = Some(TossupAction::Quit);
                }
            });
            return action;
        }

        let now = ui.input(|i| i.time);
        if self.enter_start < 0.0 {
            self.enter_start = now;
        }
        if kit::quit_button(ui) {
            return Some(TossupAction::Quit);
        }

        let total_chars = self.tossup.question.chars().count();

        // -- Advance typewriter reveal ----------------------------------------
        if matches!(self.state, TossupState::Reading) {
            self.char_timer += dt;
            self.chars_revealed = ((self.char_timer * CHARS_PER_SECOND) as usize).min(total_chars);
            if self.chars_revealed >= total_chars {
                self.state = TossupState::FinishedReading { elapsed: 0.0 };
            }
        }
        if let TossupState::FinishedReading { ref mut elapsed } = self.state {
            *elapsed += dt;
            if *elapsed >= 5.0 {
                self.state = TossupState::TimedOut;
            }
        }

        let revealed: String = self.tossup.question.chars().take(self.chars_revealed).collect();
        let remaining: String = self.tossup.question.chars().skip(self.chars_revealed).collect();

        // -- Geometry (.it-qb padding 24 / 48 / 28) ---------------------------
        let screen = ui.ctx().screen_rect();
        let avail_w = ui.available_width();
        let content_w = (avail_w - 96.0).min(860.0).max(420.0);
        let left = (avail_w - content_w) / 2.0;
        let right = left + content_w;
        let ey = kit::enter_offset(ui, self.enter_start);
        let painter = ui.painter().clone();

        // -- Top bar (.it-qb-bar) ---------------------------------------------
        let bar_cy = 24.0 + 15.0 + ey;
        painter.text(
            Pos2::new(left, bar_cy),
            Align2::LEFT_CENTER,
            format!(
                "QUIZBOWL   •   {}   •   {}",
                self.tossup.category.to_uppercase(),
                self.tossup.difficulty.to_uppercase()
            ),
            theme::mono(11.0),
            GOLD,
        );
        let bump = {
            let e = (now - self.bump_start) as f32;
            if e < 0.5 { (e / 0.5 * std::f32::consts::PI).sin() } else { 0.0 }
        };
        kit::score_chip(
            ui,
            Pos2::new(right, bar_cy),
            &format!("Q{}", self.questions_done + 1),
            &format!("{} pts", self.score),
            bump,
        );

        // -- Footer region (pinned to bottom) ---------------------------------
        let foot_bottom = screen.bottom() - 28.0 + ey;

        // -- Question readout (.it-qb-readout), vertically centered -----------
        let region_top = 24.0 + 30.0 + 18.0 + ey;
        let region_bottom = foot_bottom - 165.0; // reserve room for the foot
        let qfont = theme::spectral_medium(26.0);

        let mut job = LayoutJob::default();
        job.append(&revealed, 0.0, TextFormat { font_id: qfont.clone(), color: WHITE, ..Default::default() });
        let show_remainder = matches!(self.state, TossupState::Result { .. });
        if show_remainder && !remaining.is_empty() {
            job.append(&remaining, 0.0, TextFormat {
                font_id: qfont.clone(),
                color: INK_DIM.gamma_multiply(0.55),
                ..Default::default()
            });
        }
        job.wrap.max_width = content_w;
        let galley = painter.layout_job(job);
        let gh = galley.size().y;
        let q_top = (region_top + region_bottom - gh) / 2.0;
        painter.galley(Pos2::new(left, q_top), galley.clone(), WHITE);

        // blinking caret while reading (.it-caret)
        if matches!(self.state, TossupState::Reading) {
            if let Some(row) = galley.rows.last() {
                let cx = left + row.rect.right() + 3.0;
                let cy = q_top + row.rect.center().y;
                let half = qfont.size * 0.55;
                if now.fract() < 0.5 {
                    painter.rect_filled(
                        Rect::from_center_size(Pos2::new(cx + 1.5, cy), Vec2::new(3.0, half * 2.0)),
                        Rounding::ZERO,
                        GOLD,
                    );
                }
                ui.ctx().request_repaint();
            }
        }

        // -- Footer content per phase -----------------------------------------
        let mut buzz = false;
        let mut submitted: Option<String> = None;
        let mut next = false;
        let mut bonus = false;

        match self.state {
            TossupState::Reading | TossupState::FinishedReading { .. } => {
                // Buzz button (.it-buzz) left-aligned with hint, at the bottom.
                let label = "BUZZ IN";
                let lab_w = painter.layout_no_wrap(label.into(), theme::spectral(18.0), ON_ACCENT).size().x;
                let key_w = 52.0;
                let bw = 30.0 + lab_w + 14.0 + key_w + 30.0;
                let bh = 50.0;
                let pulse = {
                    let s = ((now as f32) / 2.2 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin();
                    0.5 + 0.5 * s
                };
                let lift = -2.0 * pulse;
                let brect = Rect::from_min_size(Pos2::new(left, foot_bottom - bh), Vec2::new(bw, bh))
                    .translate(Vec2::new(0.0, lift));
                let resp = ui.interact(brect, Id::new("qb_buzz"), Sense::click());
                // glow
                painter.rect_filled(
                    brect.expand(6.0 * pulse),
                    Rounding::same(14.0),
                    ACCENT_SOFT.gamma_multiply(0.5 * pulse),
                );
                painter.rect_filled(brect, Rounding::same(14.0), GOLD);
                painter.text(
                    Pos2::new(brect.left() + 30.0, brect.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    theme::spectral(18.0),
                    ON_ACCENT,
                );
                let key = Rect::from_center_size(
                    Pos2::new(brect.right() - 30.0 - key_w / 2.0, brect.center().y),
                    Vec2::new(key_w, 22.0),
                );
                painter.rect_filled(key, Rounding::same(5.0), Color32::from_rgba_premultiplied(0, 0, 0, 46));
                painter.text(key.center(), Align2::CENTER_CENTER, "SPACE", theme::mono(10.0), ON_ACCENT);

                // hint (.it-turn)
                painter.text(
                    Pos2::new(brect.right() + 16.0, foot_bottom - bh / 2.0),
                    Align2::LEFT_CENTER,
                    "read until you're sure — earlier = more points",
                    theme::mono(11.0),
                    INK_DIM,
                );
                ui.ctx().request_repaint();
                if resp.clicked() || ui.input(|i| i.key_pressed(egui::Key::Space)) {
                    buzz = true;
                }
            }

            TossupState::BuzzedIn { ref mut answer_input } => {
                // .it-prompt "Answer" + input row, then Submit
                let row_y = foot_bottom - 90.0;
                painter.text(
                    Pos2::new(left, row_y + 21.0),
                    Align2::LEFT_CENTER,
                    "Answer",
                    theme::mono(12.0),
                    GOLD,
                );
                let prompt_w = painter.layout_no_wrap("Answer".into(), theme::mono(12.0), GOLD).size().x;
                let field = Rect::from_min_size(
                    Pos2::new(left + prompt_w + 14.0, row_y),
                    Vec2::new(360.0, 42.0),
                );
                let mut child =
                    ui.new_child(egui::UiBuilder::new().max_rect(field).layout(egui::Layout::left_to_right(egui::Align::Center)));
                let resp = child.add(
                    egui::TextEdit::singleline(answer_input)
                        .desired_width(360.0)
                        .font(theme::mono(17.0))
                        .margin(egui::Margin::symmetric(14.0, 11.0))
                        .text_color(WHITE),
                );
                if self.focus_answer {
                    resp.request_focus();
                    self.focus_answer = false;
                }
                let mut go = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                let bsz = painter.layout_no_wrap("Submit".into(), theme::plex_semibold(14.0), WHITE).size()
                    + Vec2::new(44.0, 24.0);
                let brect = Rect::from_min_size(Pos2::new(left, foot_bottom - bsz.y), bsz);
                let mut bchild =
                    ui.new_child(egui::UiBuilder::new().max_rect(brect).layout(egui::Layout::left_to_right(egui::Align::Center)));
                if kit::primary_button(&mut bchild, "Submit").clicked() {
                    go = true;
                }
                if go {
                    // Allow submitting an empty answer (counts as incorrect).
                    submitted = Some(answer_input.clone());
                }
            }

            TossupState::TimedOut => {
                next = self.draw_verdict(&painter, ui, left, foot_bottom, ey, "TIME", RED_WRONG, None, "Next tossup →");
            }

            TossupState::Result { correct, score_delta } => {
                let (label, color) = if correct {
                    ("CORRECT", GREEN_CORRECT)
                } else {
                    ("INCORRECT", RED_WRONG)
                };
                // A correct tossup earns a bonus round; a miss goes to the next tossup.
                let btn = if correct { "Bonus round →" } else { "Next tossup →" };
                let clicked =
                    self.draw_verdict(&painter, ui, left, foot_bottom, ey, label, color, Some(score_delta), btn);
                if clicked {
                    if correct { bonus = true } else { next = true }
                }
            }
        }

        // -- Apply transitions -------------------------------------------------
        let mut action = None;
        if buzz {
            self.state = TossupState::BuzzedIn { answer_input: String::new() };
            self.focus_answer = true;
        } else if let Some(ans) = submitted {
            let correct = check_answer(&ans, &self.tossup.accepted_answers);
            let delta = quizbowl_score(correct, self.chars_revealed, total_chars);
            self.score += delta;
            self.chars_revealed = total_chars;
            self.state = TossupState::Result { correct, score_delta: delta };
            self.result_start = now;
            self.bump_start = now;
        } else if bonus {
            action = Some(TossupAction::Bonus);
        } else if next {
            action = Some(TossupAction::Next);
        }

        action
    }

    /// Result / timeout footer: verdict label + points pop + "Answer" reveal +
    /// Next button, left-aligned and bottom-pinned. Returns whether Next clicked.
    #[allow(clippy::too_many_arguments)]
    fn draw_verdict(
        &self,
        painter: &egui::Painter,
        ui: &mut egui::Ui,
        left: f32,
        foot_bottom: f32,
        _ey: f32,
        label: &str,
        color: Color32,
        delta: Option<i32>,
        button_label: &str,
    ) -> bool {
        let now = ui.input(|i| i.time);
        let t = (((now - self.result_start) / 0.5).clamp(0.0, 1.0)) as f32;
        let pop = 1.0 - (1.0 - t).powi(3); // ease-out

        // Layout: verdict row at top of a taller footer block so the answer
        // reveal has clear separation from the Next button below it.
        let block_top = foot_bottom - 150.0;

        // verdict label (.it-verdict-label fontSize 30) with pop scale
        let vfont = theme::spectral(30.0 * (0.5 + 0.5 * pop));
        painter.text(
            Pos2::new(left, block_top + 18.0),
            Align2::LEFT_CENTER,
            label,
            vfont,
            color,
        );
        let vw = painter
            .layout_no_wrap(label.into(), theme::spectral(30.0), color)
            .size()
            .x;

        // points pop (.it-pop) slide up + fade
        if let Some(d) = delta {
            let dy = (1.0 - pop) * 18.0;
            let txt = if d >= 0 { format!("+{}", d) } else { d.to_string() };
            painter.text(
                Pos2::new(left + vw + 16.0, block_top + 18.0 + dy),
                Align2::LEFT_CENTER,
                txt,
                theme::mono_semibold(22.0),
                color.gamma_multiply(pop),
            );
        }

        // answer reveal (.it-answer-reveal / .it-answer-pre)
        painter.text(
            Pos2::new(left, block_top + 52.0),
            Align2::LEFT_CENTER,
            "ANSWER",
            theme::mono(11.0),
            INK_DIM,
        );
        painter.text(
            Pos2::new(left, block_top + 74.0),
            Align2::LEFT_CENTER,
            &self.tossup.answer,
            theme::spectral(22.0),
            WHITE,
        );

        // Advance button, left-aligned at the bottom
        let bsz = painter
            .layout_no_wrap(button_label.into(), theme::plex_semibold(14.0), WHITE)
            .size()
            + Vec2::new(44.0, 24.0);
        let brect = Rect::from_min_size(Pos2::new(left, foot_bottom - bsz.y), bsz);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(brect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        kit::primary_button(&mut child, button_label).clicked()
    }
}
