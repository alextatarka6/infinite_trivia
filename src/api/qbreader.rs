use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tossup {
    pub question: String,
    pub answer: String,
    pub category: String,
    pub difficulty: String,
}

impl Tossup {
    pub fn words(&self) -> Vec<&str> {
        self.question.split_whitespace().collect()
    }
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

pub async fn fetch_tossup() -> Result<Tossup, String> {
    let resp = reqwest::get("https://www.qbreader.org/api/random-tossup")
        .await
        .map_err(|e| e.to_string())?
        .json::<ApiResponse>()
        .await
        .map_err(|e| e.to_string())?;

    let t = resp.tossups.into_iter().next()
        .ok_or_else(|| "Empty tossups array".to_string())?;

    Ok(Tossup {
        question: t.question_sanitized,
        answer: clean_answer(&t.answer_sanitized),
        category: t.category,
        difficulty: match &t.difficulty {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        },
    })
}

fn clean_answer(raw: &str) -> String {
    // Strip [accept X] / [prompt X] annotations
    let s = raw.split('[').next().unwrap_or(raw);
    s.trim().to_string()
}

pub fn check_answer(given: &str, expected: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().trim().to_string();
    let g = norm(given);
    let e = norm(expected);
    g == e || e.contains(&g) || g.contains(&e)
}
