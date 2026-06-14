use egui::{Align2, Color32, Id, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use crate::api::opentdb::TriviaQuestion;
use crate::game::scoring::trivia_score;
use crate::screens::kit::{self, SummaryAction};
use crate::theme::{
    self, BLUE_BG, BLUE_MID, CORRECT_BG, GOLD, GREEN_CORRECT, INK_DIM, LINE, RED_WRONG, WHITE,
    WRONG_BG,
};

pub enum TriviaAction {
    Next { score_delta: i32 },
    Restart,
    Quit,
}

pub struct TriviaScreen {
    pub questions: Vec<TriviaQuestion>,
    pub current_idx: usize,
    pub score: i32,
    pub streak: u32,
    pub best_streak: u32,
    pub correct_count: u32,
    pub selected: Option<usize>,
    pub timer: f32,
    pub timer_max: f32,
    pub loading: bool,
    pub error: Option<String>,
    // animation bookkeeping
    anim_idx: i64,
    enter_start: f64,
    select_start: f64,
    bump_start: f64,
}

impl TriviaScreen {
    pub fn new(questions: Vec<TriviaQuestion>) -> Self {
        Self {
            questions,
            current_idx: 0,
            score: 0,
            streak: 0,
            best_streak: 0,
            correct_count: 0,
            selected: None,
            timer: 20.0,
            timer_max: 20.0,
            loading: false,
            error: None,
            anim_idx: -1,
            enter_start: 0.0,
            select_start: 0.0,
            bump_start: -10.0,
        }
    }

    pub fn loading() -> Self {
        let mut s = Self::new(vec![]);
        s.loading = true;
        s
    }

    pub fn with_error(msg: String) -> Self {
        let mut s = Self::new(vec![]);
        s.error = Some(msg);
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, dt: f32) -> Option<TriviaAction> {
        kit::ambient(ui.ctx());

        // -- Loading ----------------------------------------------------------
        if self.loading {
            kit::loader(ui, "Pulling fresh questions…");
            return None;
        }

        // -- Error ------------------------------------------------------------
        if let Some(err) = self.error.clone() {
            let mut action = None;
            let avail = ui.available_size();
            ui.add_space((avail.y - 120.0).max(16.0) / 2.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Could not fetch questions")
                        .font(theme::spectral(26.0))
                        .color(RED_WRONG),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(&err).font(theme::plex(14.0)).color(INK_DIM));
                ui.add_space(18.0);
                if kit::primary_button(ui, "Back to menu").clicked() {
                    action = Some(TriviaAction::Quit);
                }
            });
            return action;
        }

        let now = ui.input(|i| i.time);

        // Reset the enter animation whenever the visible "view" changes.
        if self.anim_idx != self.current_idx as i64 {
            self.anim_idx = self.current_idx as i64;
            self.enter_start = now;
        }

        // -- Round complete (Summary) -----------------------------------------
        if self.current_idx >= self.questions.len() && !self.questions.is_empty() {
            if kit::quit_button(ui) {
                return Some(TriviaAction::Quit);
            }
            let misses = self.questions.len() as u32 - self.correct_count;
            let stats = [
                kit::Stat { value: self.correct_count.to_string(), label: "correct" },
                kit::Stat { value: misses.to_string(), label: "misses" },
                kit::Stat { value: format!("×{}", self.best_streak), label: "best streak" },
            ];
            return match kit::summary(
                ui,
                "ROUND COMPLETE",
                &self.score.to_string(),
                "FINAL SCORE",
                &stats,
                self.enter_start,
            ) {
                SummaryAction::Restart => Some(TriviaAction::Restart),
                SummaryAction::Quit => Some(TriviaAction::Quit),
                SummaryAction::None => None,
            };
        }

        if self.questions.is_empty() {
            return None;
        }

        if kit::quit_button(ui) {
            return Some(TriviaAction::Quit);
        }

        // Tick the countdown.
        if self.selected.is_none() {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.timer = 0.0;
                self.streak = 0;
                self.selected = Some(usize::MAX);
                self.select_start = now;
            }
        }

        let q = &self.questions[self.current_idx];
        let answers = q.shuffled_answers();

        // -- Geometry: a centered content column, .it-trivia padding 0 44 ------
        let screen = ui.ctx().screen_rect();
        let avail_w = ui.available_width();
        let content_w = (avail_w - 88.0).min(900.0).max(420.0);
        let left = (avail_w - content_w) / 2.0;
        let right = left + content_w;
        let ey = kit::enter_offset(ui, self.enter_start);
        let painter = ui.painter().clone();

        // -- Top bar (.it-trivia-bar) -----------------------------------------
        let mut y = 30.0 + ey;
        let bar_h = 30.0;
        painter.text(
            Pos2::new(left, y + bar_h / 2.0),
            Align2::LEFT_CENTER,
            format!("Q {}/{}", self.current_idx + 1, self.questions.len()),
            theme::mono(12.0),
            INK_DIM,
        );
        let qn = ui
            .painter()
            .layout_no_wrap(
                format!("Q {}/{}", self.current_idx + 1, self.questions.len()),
                theme::mono(12.0),
                INK_DIM,
            )
            .size()
            .x;
        painter.text(
            Pos2::new(left + qn + 8.0, y + bar_h / 2.0),
            Align2::LEFT_CENTER,
            "•",
            theme::mono(12.0),
            INK_DIM.gamma_multiply(0.5),
        );
        painter.text(
            Pos2::new(left + qn + 22.0, y + bar_h / 2.0),
            Align2::LEFT_CENTER,
            &q.category,
            theme::mono(12.0),
            INK_DIM,
        );

        // Score chip (right), with bump
        let bump = {
            let e = (now - self.bump_start) as f32;
            if e < 0.5 { (e / 0.5 * std::f32::consts::PI).sin() } else { 0.0 }
        };
        kit::score_chip(
            ui,
            Pos2::new(right, y + bar_h / 2.0),
            "SCORE",
            &self.score.to_string(),
            bump,
        );

        // Streak chip (left of score chip) when streak > 1
        if self.streak > 1 {
            let label = format!("STREAK ×{}", self.streak);
            let sz = ui
                .painter()
                .layout_no_wrap(label.clone(), theme::mono(10.0), GOLD)
                .size();
            let chip_w = sz.x + 20.0;
            // estimate score chip width to offset
            let score_w =
                ui.painter()
                    .layout_no_wrap(self.score.to_string(), theme::mono_semibold(theme::TYPE_SCORE_VALUE), WHITE)
                    .size()
                    .x
                    + 70.0;
            let cx_right = right - score_w - 10.0;
            let chip = Rect::from_min_size(
                Pos2::new(cx_right - chip_w, y + bar_h / 2.0 - 13.0),
                Vec2::new(chip_w, 26.0),
            );
            painter.rect_filled(chip, Rounding::same(999.0), theme::ACCENT_SOFT);
            painter.rect_stroke(chip, Rounding::same(999.0), Stroke::new(1.0, theme::ACCENT_SOFT));
            painter.text(chip.center(), Align2::CENTER_CENTER, &label, theme::mono(10.0), GOLD);
        }

        y += bar_h + 10.0;

        // -- Timer bar (.it-timer) --------------------------------------------
        let ratio = (self.timer / self.timer_max).clamp(0.0, 1.0);
        let bar_color = if ratio > 0.5 {
            GREEN_CORRECT
        } else if ratio > 0.25 {
            GOLD
        } else {
            RED_WRONG
        };
        let track = Rect::from_min_size(Pos2::new(left, y), Vec2::new(content_w, 8.0));
        painter.rect_filled(track, Rounding::same(999.0), BLUE_MID);
        if ratio > 0.0 {
            let mut fill = track;
            fill.set_right(track.left() + track.width() * ratio);
            painter.rect_filled(fill, Rounding::same(999.0), bar_color);
        }
        y += 8.0 + 20.0;

        // -- Question (.it-trivia-q) ------------------------------------------
        let q_galley = painter.layout(
            q.question.clone(),
            theme::spectral_medium(27.0),
            WHITE,
            content_w,
        );
        painter.galley(Pos2::new(left, y), q_galley.clone(), WHITE);
        y += q_galley.size().y + 20.0;

        // -- Options (.it-options, gap 11) ------------------------------------
        let opt_h = 54.0;
        let opt_gap = 11.0;
        let mut clicked_idx = None;
        let locked = self.selected.is_some();

        for (i, (ans_text, is_correct)) in answers.iter().enumerate() {
            let base = Rect::from_min_size(Pos2::new(left, y), Vec2::new(content_w, opt_h));
            let resp = ui.interact(
                base,
                Id::new(("trivia_opt", self.current_idx, i)),
                if locked { Sense::hover() } else { Sense::click() },
            );

            // hover translateX(4px)
            let hov = if locked {
                0.0
            } else {
                ui.ctx().animate_bool(Id::new(("trivia_opt_h", self.current_idx, i)), resp.hovered())
            };
            let rect = base.translate(Vec2::new(4.0 * hov, 0.0));

            // resolve state colors
            let mut dimmed = false;
            let (border, key_bg, key_fg, text_col, tint, ring) = if let Some(sel) = self.selected {
                if *is_correct {
                    (GREEN_CORRECT, GREEN_CORRECT, Color32::from_rgb(12, 26, 16), GREEN_CORRECT, Some(CORRECT_BG), Some(GREEN_CORRECT))
                } else if sel == i {
                    (RED_WRONG, RED_WRONG, Color32::from_rgb(42, 12, 17), RED_WRONG, Some(WRONG_BG), Some(RED_WRONG))
                } else {
                    dimmed = true;
                    (LINE, BLUE_BG, INK_DIM, WHITE, None, None)
                }
            } else if resp.hovered() {
                (GOLD, BLUE_BG, INK_DIM, WHITE, None, None)
            } else {
                (LINE, BLUE_BG, INK_DIM, WHITE, None, None)
            };
            let a = if dimmed { 0.42 } else { 1.0 };

            // base fill + optional tint
            painter.rect_filled(rect, Rounding::same(12.0), BLUE_MID.gamma_multiply(a));
            if let Some(t) = tint {
                painter.rect_filled(rect, Rounding::same(12.0), t);
            }
            painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, border.gamma_multiply(a)));
            if let Some(rc) = ring {
                painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, rc));
            }

            // key badge
            let badge = Rect::from_min_size(
                Pos2::new(rect.left() + 18.0, rect.center().y - 13.0),
                Vec2::splat(26.0),
            );
            painter.rect_filled(badge, Rounding::same(6.0), key_bg.gamma_multiply(a));
            if self.selected.is_none() {
                painter.rect_stroke(badge, Rounding::same(6.0), Stroke::new(1.0, LINE));
            }
            painter.text(
                badge.center(),
                Align2::CENTER_CENTER,
                (b'A' + i as u8) as char,
                theme::mono(12.0),
                key_fg.gamma_multiply(a),
            );

            // answer text
            painter.text(
                Pos2::new(badge.right() + 14.0, rect.center().y),
                Align2::LEFT_CENTER,
                ans_text,
                theme::plex(15.0),
                text_col.gamma_multiply(a),
            );

            // correct ✓ mark
            if locked && *is_correct {
                painter.text(
                    Pos2::new(rect.right() - 18.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    "✓",
                    theme::plex_semibold(16.0),
                    GREEN_CORRECT,
                );
            }

            if resp.clicked() && !locked {
                clicked_idx = Some(i);
            }
            y += opt_h + opt_gap;
        }

        // apply selection
        if let Some(i) = clicked_idx {
            let is_correct = answers[i].1;
            self.selected = Some(i);
            self.select_start = now;
            self.bump_start = now;
            let delta = trivia_score(is_correct, self.streak);
            self.score += delta;
            if is_correct {
                self.streak += 1;
                self.correct_count += 1;
                self.best_streak = self.best_streak.max(self.streak);
            } else {
                self.streak = 0;
            }
        }

        // -- Footer pinned to the bottom (.it-trivia-foot) --------------------
        let mut action = None;
        if let Some(sel) = self.selected {
            let foot_y = screen.bottom() - 28.0 - 38.0 + ey;
            let foot_cy = foot_y + 19.0;

            // gain text with it-popup slide
            let pe = ((now - self.select_start) / 0.5).clamp(0.0, 1.0) as f32;
            let pe = 1.0 - (1.0 - pe).powi(3);
            let gain_dy = (1.0 - pe) * 18.0;
            let timed = sel == usize::MAX;
            let (gain, gcol) = if timed {
                ("time!".to_string(), RED_WRONG)
            } else if answers[sel].1 {
                (format!("+{}", trivia_score(true, self.streak.saturating_sub(1))), GREEN_CORRECT)
            } else {
                ("+0".to_string(), RED_WRONG)
            };
            painter.text(
                Pos2::new(left, foot_cy + gain_dy),
                Align2::LEFT_CENTER,
                &gain,
                theme::mono_semibold(18.0),
                gcol.gamma_multiply(pe),
            );

            // Next / Finish primary button, right-aligned
            let label = if self.current_idx + 1 < self.questions.len() {
                "Next →"
            } else {
                "Finish →"
            };
            let bsz = ui
                .painter()
                .layout_no_wrap(label.to_string(), theme::plex_semibold(14.0), WHITE)
                .size()
                + Vec2::new(44.0, 24.0);
            let btn_rect = Rect::from_min_size(Pos2::new(right - bsz.x, foot_y), bsz);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
            if kit::primary_button(&mut child, label).clicked() {
                self.current_idx += 1;
                self.selected = None;
                self.timer = self.timer_max;
                action = Some(TriviaAction::Next { score_delta: 0 });
            }
        }

        action
    }
}
