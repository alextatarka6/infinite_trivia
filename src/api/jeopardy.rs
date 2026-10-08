use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde::Deserialize;

use crate::game::scoring::jeopardy_value;

/// Mirrors the JSON record exactly.
#[derive(Debug, Clone, Deserialize)]
pub struct JeopardyRecord {
    pub category: String,
    pub air_date: String,
    pub question: String,
    pub value: Option<String>,   // "$200", "$1,000", or null/None string
    pub answer: String,
    pub round: String,
    pub show_number: String,
}

/// The clue type used by the UI.
#[derive(Debug, Clone)]
pub struct Clue {
    pub category: String,
    pub value: i32,
    pub question: String,
    pub answer: String,
    pub round: String,
    pub air_date: String,
}

impl From<&JeopardyRecord> for Clue {
    fn from(r: &JeopardyRecord) -> Self {
        Clue {
            category: strip_html(&r.category),
            value: parse_value(r.value.as_deref()),
            question: strip_html(r.question.trim_matches('\'')),
            answer: strip_html(&r.answer),
            round: r.round.clone(),
            air_date: r.air_date.clone(),
        }
    }
}

fn strip_html(s: &str) -> String {
    // Semantic line breaks → newline so the text wraps naturally
    let s = s
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n");
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    html_escape::decode_html_entities(&out).replace('\u{a0}', " ")
}

fn parse_value(s: Option<&str>) -> i32 {
    match s {
        None | Some("None") => 0,
        Some(v) => v
            .trim_start_matches('$')
            .replace(',', "")
            .parse::<i32>()
            .unwrap_or(0),
    }
}

/// Clues that lean on a picture, audio or video the app can't show.
fn needs_media(r: &JeopardyRecord) -> bool {
    let q = r.question.to_lowercase();
    q.contains("<a href")
        || ["seen here", "shown here", "heard here", "pictured here", "here is", "clue crew"]
            .iter()
            .any(|p| q.contains(p))
}

/// One real board column: a category as it aired in a single episode.
struct Column {
    category: String,
    clues: Vec<JeopardyRecord>,
}

impl Column {
    /// A full, text-only column: safe to put on a board as-is.
    fn playable(&self) -> bool {
        self.clues.len() == 5 && self.clues.iter().all(|r| !r.question.trim().is_empty() && !needs_media(r))
    }
}

/// In-memory store: round → the columns that aired in it.
pub struct JeopardyStore {
    rounds: HashMap<String, Vec<Column>>,
}

impl JeopardyStore {
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        let records: Vec<JeopardyRecord> = serde_json::from_str(&data)
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // Group by (show, round, category) so a column is one episode's category.
        let mut by_column: HashMap<(String, String, String), Vec<JeopardyRecord>> = HashMap::new();
        for r in records {
            by_column
                .entry((r.show_number.clone(), r.round.clone(), r.category.clone()))
                .or_default()
                .push(r);
        }
        let mut rounds: HashMap<String, Vec<Column>> = HashMap::new();
        for ((_, round, category), mut clues) in by_column {
            clues.sort_by_key(|r| parse_value(r.value.as_deref()));
            rounds.entry(round).or_default().push(Column { category, clues });
        }
        // HashMap order varies per run; sort so random picks are the only variation.
        for cols in rounds.values_mut() {
            cols.sort_by(|a, b| (&a.category, &a.clues[0].show_number).cmp(&(&b.category, &b.clues[0].show_number)));
        }
        Ok(Self { rounds })
    }

    /// Six random full columns for `round`, with distinct category names.
    /// Clue values are set from the row so the board, phones and scoring agree.
    pub fn random_board(&self, round: &str) -> Option<Vec<(String, Vec<Clue>)>> {
        let cols = self.rounds.get(round)?;
        let double = round == "Double Jeopardy!";
        let mut names = HashSet::new();
        let mut board = Vec::new();
        for _ in 0..10_000 {
            if board.len() == 6 {
                break;
            }
            let col = &cols[fastrand::usize(..cols.len())];
            if !col.playable() || !names.insert(col.category.clone()) {
                continue;
            }
            let clues = col
                .clues
                .iter()
                .enumerate()
                .map(|(row, r)| Clue { value: jeopardy_value(row, double), ..Clue::from(r) })
                .collect();
            board.push((Clue::from(&col.clues[0]).category, clues));
        }
        (board.len() == 6).then_some(board)
    }

    /// Up to `count` random text-only clues from `round` with positive values.
    pub fn random_clues(&self, round: &str, count: usize) -> Vec<Clue> {
        let Some(cols) = self.rounds.get(round) else { return vec![] };
        let mut out = Vec::new();
        for _ in 0..count * 200 {
            if out.len() == count {
                break;
            }
            let col = &cols[fastrand::usize(..cols.len())];
            let r = &col.clues[fastrand::usize(..col.clues.len())];
            if needs_media(r) || r.question.trim().is_empty() {
                continue;
            }
            let clue = Clue::from(r);
            if clue.value > 0 {
                out.push(clue);
            }
        }
        out
    }

    pub fn random_final(&self) -> Option<Clue> {
        let finals = self.rounds.get("Final Jeopardy!")?;
        for _ in 0..1000 {
            let col = &finals[fastrand::usize(..finals.len())];
            if let Some(r) = col.clues.first().filter(|r| !needs_media(r)) {
                return Some(Clue::from(r));
            }
        }
        None
    }
}

/// Judge a typed response against a clue's correct response.
pub fn check_answer(given: &str, expected: &str) -> bool {
    crate::game::answer::is_correct(given, &[expected])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_entities() {
        assert_eq!(strip_html("<i>Moby-Dick</i> &amp; more"), "Moby-Dick & more");
        assert_eq!(strip_html("line<br />two"), "line\ntwo");
        assert_eq!(strip_html("it&#39;s"), "it's");
    }

    #[test]
    fn parses_dollar_values() {
        assert_eq!(parse_value(Some("$1,000")), 1000);
        assert_eq!(parse_value(Some("$200")), 200);
        assert_eq!(parse_value(Some("None")), 0);
        assert_eq!(parse_value(None), 0);
    }

    #[test]
    fn leaves_clue_words_alone() {
        let r = JeopardyRecord {
            category: "COLORS".into(),
            air_date: String::new(),
            question: "'A red cat ran from the war in the sun'".into(),
            value: Some("$200".into()),
            answer: "crimson".into(),
            round: "Jeopardy!".into(),
            show_number: "1".into(),
        };
        assert_eq!(Clue::from(&r).question, "A red cat ran from the war in the sun");
    }

    /// Loads the real dataset (slow in debug): `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_data_builds_valid_boards() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/JEOPARDY_QUESTIONS1.json");
        let store = JeopardyStore::load(&path).expect("load dataset");
        for (round, double) in [("Jeopardy!", false), ("Double Jeopardy!", true)] {
            for _ in 0..50 {
                let board = store.random_board(round).expect("board");
                assert_eq!(board.len(), 6);
                let names: HashSet<_> = board.iter().map(|(n, _)| n.clone()).collect();
                assert_eq!(names.len(), 6, "distinct categories");
                for (_, clues) in &board {
                    assert_eq!(clues.len(), 5);
                    for (row, c) in clues.iter().enumerate() {
                        assert_eq!(c.value, jeopardy_value(row, double));
                        assert!(!c.question.trim().is_empty());
                        assert!(!c.question.contains('<'), "html left in {:?}", c.question);
                    }
                }
            }
        }
        assert!(store.random_final().is_some());
        assert_eq!(store.random_clues("Jeopardy!", 3).len(), 3);
        let b = store.random_board("Jeopardy!").unwrap();
        println!("sample column: {} -> {:?}", b[0].0, b[0].1.iter().map(|c| &c.question).collect::<Vec<_>>());
    }
}
