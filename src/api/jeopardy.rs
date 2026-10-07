use std::collections::HashSet;
use std::sync::LazyLock;
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
            question: correct_question(&strip_html(r.question.trim_matches('\''))),
            answer: strip_html(&r.answer),
            round: r.round.clone(),
            air_date: r.air_date.clone(),
        }
    }
}

// Set of common lowercase English words used as the spell-correction vocabulary.
// Proper nouns (always capitalized in Jeopardy text) are intentionally excluded so
// we never mangle names, places, or show-specific terms.
static COMMON: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "as", "is", "was", "are", "were", "be", "been",
        "have", "has", "had", "do", "does", "did", "will", "not", "no",
        "this", "that", "from", "which", "who", "what", "it", "its",
        "he", "she", "we", "they", "his", "her", "our", "also", "if",
        "all", "two", "one", "so", "then", "too", "than", "into", "up",
        "out", "over", "own", "off", "now", "can", "may", "only",
        "time", "year", "way", "man", "men", "old", "new",
        "made", "came", "come", "take", "make", "like", "give", "most",
        "many", "long", "more", "part", "well", "good", "said", "get",
        "use", "day", "how", "when", "see", "him", "name", "very",
        "after", "back", "just", "know", "go", "first", "any", "some",
        "last", "other", "same", "great", "such", "even", "here",
        "much", "down", "used", "right", "large", "high", "both",
        "each", "every", "never", "still", "always", "often", "once",
        "about", "above", "below", "before", "since", "while", "there",
        "where", "these", "those", "them", "than", "been", "being",
        "would", "could", "should", "might", "shall", "must", "need",
        "kept", "put", "set", "let", "run", "add", "try", "ask",
        "met", "got", "led", "won", "ran", "sat", "cut", "hit",
    ].iter().copied().collect()
});

/// Auto-correct short, all-lowercase words that are one edit away from a common
/// English word. Only touches function words — proper nouns stay untouched because
/// Jeopardy capitalizes them.
fn correct_question(text: &str) -> String {
    text.split(' ')
        .map(|token| {
            // Guard: purely lowercase ASCII letters, 3–6 chars
            if token.len() < 3
                || token.len() > 6
                || !token.bytes().all(|b| b.is_ascii_lowercase())
            {
                return token.to_string();
            }
            if COMMON.contains(token) {
                return token.to_string();
            }
            // Find a common word one edit away
            COMMON
                .iter()
                .find(|&&w| {
                    (w.len() as isize - token.len() as isize).unsigned_abs() <= 1
                        && levenshtein(token, w) == 1
                })
                .map(|w| w.to_string())
                .unwrap_or_else(|| token.to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_html(s: &str) -> String {
    // Semantic line breaks → newline so the text wraps naturally
    let s = s
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n");
    // Strip remaining tags
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
    // Decode common entities
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
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

/// Indexed in-memory store: (round, category) -> records.
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

    /// Returns 6 random categories x 5 clues for `round`, sorted by value.
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

        // Fisher-Yates shuffle with a simple LCG so categories are drawn
        // from across the whole dataset rather than alphabetically adjacent.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let mut rng = now
            .as_secs()
            .wrapping_mul(1_000_000_007)
            .wrapping_add(now.subsec_nanos() as u64);
        let lcg = |s: &mut u64| -> usize {
            *s = s.wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (*s >> 33) as usize
        };

        categories.sort(); // deterministic base order before shuffling
        let n = categories.len();
        for i in (1..n).rev() {
            let j = lcg(&mut rng) % (i + 1);
            categories.swap(i, j);
        }
        let selected: Vec<String> = categories.into_iter().take(6).cloned().collect();

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

    /// Returns up to `count` random clues from `round` with positive values.
    pub fn random_clues(&self, round: &str, count: usize) -> Vec<Clue> {
        let mut records: Vec<&JeopardyRecord> = self
            .index
            .iter()
            .filter(|((r, _), _)| r == round)
            .flat_map(|(_, v)| v.iter())
            .filter(|r| {
                r.value
                    .as_deref()
                    .map_or(false, |v| v != "None" && !v.is_empty())
            })
            .collect();

        if records.is_empty() {
            return vec![];
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let mut rng = now
            .as_secs()
            .wrapping_mul(2_654_435_761)
            .wrapping_add(now.subsec_nanos() as u64);
        let lcg = |s: &mut u64| -> usize {
            *s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (*s >> 33) as usize
        };

        let n = records.len();
        for i in (1..n).rev() {
            let j = lcg(&mut rng) % (i + 1);
            records.swap(i, j);
        }

        records
            .into_iter()
            .map(Clue::from)
            .filter(|c| c.value > 0)
            .take(count)
            .collect()
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
/// Tolerates 1 spelling error on answers ≥5 chars, 2 errors on ≥10 chars.
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
    if g == e || e.contains(&g) || g.contains(&e) {
        return true;
    }
    // Fuzzy: allow typos proportional to answer length
    let tolerance = match e.len() {
        0..=4 => 0,
        5..=9 => 1,
        _ => 2,
    };
    tolerance > 0 && levenshtein(&g, &e) <= tolerance
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            curr[j] = if a[i - 1] == b[j - 1] {
                prev[j - 1]
            } else {
                1 + prev[j - 1].min(prev[j]).min(curr[j - 1])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}
