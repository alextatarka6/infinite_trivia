use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tossup {
    pub question: String,
    pub answer: String,          // clean display string (no brackets)
    pub accepted_answers: Vec<String>, // main + all [accept ...] alternatives
    pub category: String,
    pub difficulty: String,
}

impl Tossup {
    pub fn words(&self) -> Vec<&str> {
        self.question.split_whitespace().collect()
    }
}

/// A quizbowl bonus: a leadin read aloud, then (usually three) parts, each with
/// its own answer. In solo play the controlling player answers every part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bonus {
    pub leadin: String,
    pub parts: Vec<BonusPart>,
    pub category: String,
    pub difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonusPart {
    pub text: String,
    pub answer: String,                // clean display string
    pub accepted_answers: Vec<String>, // main + [accept ...] alternatives
}

#[derive(Deserialize)]
struct ApiResponse {
    tossups: Vec<ApiTossup>,
}

#[derive(Deserialize)]
struct ApiTossup {
    question_sanitized: String,
    answer_sanitized: String,
    category: String,
    difficulty: serde_json::Value, // number or string depending on the entry
}

pub async fn fetch_tossup(level: u8, exclude_categories: Vec<String>) -> Result<Tossup, String> {
    for _ in 0..6 {
        let t = fetch_one(level).await?;
        let already_seen = exclude_categories
            .iter()
            .any(|c| c.eq_ignore_ascii_case(&t.category));
        if !already_seen {
            return Ok(t);
        }
    }
    // Exhausted retries - return whatever we get
    fetch_one(level).await
}

/// qbreader difficulty 1-10: 1-4 = easy HS/MS, 5-7 = mid HS, 8-10 = hard HS/college
fn difficulty_params(level: u8) -> String {
    let difficulties: &[u8] = match level {
        1 => &[1, 2, 3, 4],
        3 => &[8, 9, 10],
        _ => &[5, 6, 7],
    };
    difficulties
        .iter()
        .map(|d| format!("difficulties[]={}", d))
        .collect::<Vec<_>>()
        .join("&")
}

fn difficulty_string(d: &serde_json::Value) -> String {
    match d {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

async fn fetch_one(level: u8) -> Result<Tossup, String> {
    let url = format!(
        "https://www.qbreader.org/api/random-tossup?{}",
        difficulty_params(level)
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json::<ApiResponse>()
        .await
        .map_err(|e| e.to_string())?;

    let t = resp.tossups.into_iter().next()
        .ok_or_else(|| "Empty tossups array".to_string())?;

    let (answer, accepted_answers) = parse_answer(&t.answer_sanitized);
    Ok(Tossup {
        question: t.question_sanitized,
        answer,
        accepted_answers,
        category: t.category,
        difficulty: difficulty_string(&t.difficulty),
    })
}

#[derive(Deserialize)]
struct BonusResponse {
    bonuses: Vec<ApiBonus>,
}

#[derive(Deserialize)]
struct ApiBonus {
    leadin_sanitized: String,
    parts_sanitized: Vec<String>,
    answers_sanitized: Vec<String>,
    category: String,
    difficulty: serde_json::Value,
}

/// Fetch a random bonus at the given difficulty level.
pub async fn fetch_bonus(level: u8) -> Result<Bonus, String> {
    let url = format!(
        "https://www.qbreader.org/api/random-bonus?{}",
        difficulty_params(level)
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json::<BonusResponse>()
        .await
        .map_err(|e| e.to_string())?;

    let b = resp
        .bonuses
        .into_iter()
        .next()
        .ok_or_else(|| "Empty bonuses array".to_string())?;

    let parts = b
        .parts_sanitized
        .into_iter()
        .zip(b.answers_sanitized)
        .map(|(text, raw_answer)| {
            let (answer, accepted_answers) = parse_answer(&raw_answer);
            BonusPart { text, answer, accepted_answers }
        })
        .collect();

    Ok(Bonus {
        leadin: b.leadin_sanitized,
        parts,
        category: b.category,
        difficulty: difficulty_string(&b.difficulty),
    })
}

// Returns (display_answer, all_accepted) by parsing qbreader bracket annotations.
// Handles: [accept X; Y], [also accept X], [or X] - ignores [prompt ...].
fn parse_answer(raw: &str) -> (String, Vec<String>) {
    let display = raw.split('[').next().unwrap_or(raw).trim().to_string();
    let mut accepted = vec![display.clone()];

    let mut rest = raw;
    while let Some(open) = rest.find('[') {
        rest = &rest[open + 1..];
        let close = rest.find(']').unwrap_or(rest.len());
        let block = &rest[..close];
        rest = if close < rest.len() { &rest[close + 1..] } else { "" };

        let lower = block.to_lowercase();
        let content_start = if let Some(i) = lower.find("also accept") {
            Some(i + "also accept".len())
        } else if let Some(i) = lower.find("accept") {
            Some(i + "accept".len())
        } else if let Some(i) = lower.find("or ") {
            Some(i + "or ".len())
        } else {
            None
        };

        if let Some(start) = content_start {
            let content = block[start..].trim_start_matches(':').trim();
            for part in content.split(';') {
                let alt = part.split('[').next().unwrap_or(part).trim();
                if !alt.is_empty() {
                    accepted.push(alt.to_string());
                }
            }
        }
    }

    (display, accepted)
}

pub fn check_answer(given: &str, accepted: &[String]) -> bool {
    let norm = |s: &str| s.to_lowercase().trim().to_string();
    let g = norm(given);
    if g.is_empty() {
        return false;
    }
    accepted.iter().any(|a| {
        let e = norm(a);
        g == e || e.contains(&g) || g.contains(&e)
    })
}
