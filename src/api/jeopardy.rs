use std::collections::HashMap;
use serde::Deserialize;

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
            category: r.category.clone(),
            value: parse_value(r.value.as_deref()),
            question: r.question.trim_matches('\'').to_string(),
            answer: r.answer.clone(),
            round: r.round.clone(),
            air_date: r.air_date.clone(),
        }
    }
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

/// Indexed in-memory store: (round, category) → records.
pub struct JeopardyStore {
    index: HashMap<(String, String), Vec<JeopardyRecord>>,
}

impl JeopardyStore {
    pub fn load(path: &str) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path, e))?;
        let records: Vec<JeopardyRecord> = serde_json::from_str(&data)
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let mut index: HashMap<(String, String), Vec<JeopardyRecord>> = HashMap::new();
        for r in records {
            index
                .entry((r.round.clone(), r.category.clone()))
                .or_default()
                .push(r);
        }
        Ok(Self { index })
    }

    /// Returns 6 random categories × 5 clues for `round`, sorted by value.
    pub fn random_board(&self, round: &str) -> Option<Vec<(String, Vec<Clue>)>> {
        let mut categories: Vec<&String> = self
            .index
            .keys()
            .filter(|(r, _)| r == round)
            .map(|(_, c)| c)
            .collect();

        if categories.is_empty() {
            return None;
        }

        // Pseudo-random selection using system time as seed
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as usize;
        categories.sort();
        let start = seed % categories.len();
        let selected: Vec<String> = categories
            .into_iter()
            .cycle()
            .skip(start)
            .take(6)
            .cloned()
            .collect();

        let mut board = Vec::new();
        for cat in selected {
            if let Some(records) = self.index.get(&(round.to_string(), cat.clone())) {
                let mut clues: Vec<Clue> =
                    records.iter().take(5).map(Clue::from).collect();
                clues.sort_by_key(|c| c.value);
                // Fill to 5 if fewer records
                while clues.len() < 5 {
                    clues.push(Clue {
                        category: cat.clone(),
                        value: 0,
                        question: String::new(),
                        answer: String::new(),
                        round: round.to_string(),
                        air_date: String::new(),
                    });
                }
                board.push((cat, clues));
            }
        }
        Some(board)
    }

    pub fn random_final(&self) -> Option<Clue> {
        let finals: Vec<&JeopardyRecord> = self
            .index
            .iter()
            .filter(|((r, _), _)| r == "Final Jeopardy!")
            .flat_map(|(_, v)| v.iter())
            .collect();

        if finals.is_empty() {
            return None;
        }
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_millis() as usize;
        Some(Clue::from(finals[seed % finals.len()]))
    }
}

/// Case-insensitive fuzzy match; strips leading "what is / who is".
pub fn check_answer(given: &str, expected: &str) -> bool {
    let normalize = |s: &str| -> String {
        s.to_lowercase()
            .trim_start_matches("what is ")
            .trim_start_matches("who is ")
            .trim_start_matches("what are ")
            .trim_start_matches("who are ")
            .trim()
            .to_string()
    };
    let g = normalize(given);
    let e = normalize(expected);
    g == e || e.contains(&g) || g.contains(&e)
}
