use std::sync::Mutex;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriviaQuestion {
    pub category: String,
    pub question: String,
    pub correct_answer: String,
    pub incorrect_answers: Vec<String>,
    pub difficulty: String,
    /// Fixes the answer order for this question (picked once at fetch time).
    #[serde(default)]
    pub shuffle_seed: u64,
}

impl TriviaQuestion {
    /// All answers in a random order that stays the same for this question,
    /// so the host screen and every phone agree frame after frame.
    pub fn shuffled_answers(&self) -> Vec<(String, bool)> {
        let mut answers: Vec<(String, bool)> = self
            .incorrect_answers
            .iter()
            .map(|a| (a.clone(), false))
            .collect();
        answers.push((self.correct_answer.clone(), true));
        fastrand::Rng::with_seed(self.shuffle_seed).shuffle(&mut answers);
        answers
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    response_code: u8,
    #[serde(default)]
    results: Vec<ApiQuestion>,
}

#[derive(Deserialize)]
struct ApiQuestion {
    category: String,
    difficulty: String,
    question: String,
    correct_answer: String,
    incorrect_answers: Vec<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
}

/// Session token: Open Trivia DB won't repeat a question within a session.
static TOKEN: Mutex<Option<String>> = Mutex::new(None);

async fn session_token() -> Option<String> {
    if let Some(t) = TOKEN.lock().unwrap().clone() {
        return Some(t);
    }
    let resp = reqwest::get("https://opentdb.com/api_token.php?command=request").await.ok()?;
    let token = resp.json::<TokenResponse>().await.ok()?.token;
    *TOKEN.lock().unwrap() = Some(token.clone());
    Some(token)
}

/// What each non-zero `response_code` means, per the API docs.
fn describe(code: u8) -> &'static str {
    match code {
        1 => "Open Trivia DB has no questions for that difficulty right now.",
        2 => "Open Trivia DB rejected the request.",
        5 => "Open Trivia DB is rate-limiting requests. Wait a few seconds and try again.",
        _ => "Open Trivia DB returned an unexpected response.",
    }
}

pub async fn fetch_questions(amount: u8, difficulty: &str) -> Result<Vec<TriviaQuestion>, String> {
    let diff_param = if difficulty.is_empty() {
        String::new()
    } else {
        format!("&difficulty={}", difficulty)
    };
    for attempt in 0..3 {
        let token = session_token().await;
        let token_param = token.as_deref().map(|t| format!("&token={t}")).unwrap_or_default();
        let url = format!(
            "https://opentdb.com/api.php?amount={}&type=multiple&encode=url3986{}{}",
            amount, diff_param, token_param
        );
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| format!("Could not reach Open Trivia DB: {e}"))?
            .json::<ApiResponse>()
            .await
            .map_err(|e| format!("Unexpected reply from Open Trivia DB: {e}"))?;

        match resp.response_code {
            0 if !resp.results.is_empty() => {
                return Ok(resp.results.into_iter().map(convert).collect());
            }
            // Token unknown/expired (3) or every question seen (4): start a new session.
            3 | 4 => {
                *TOKEN.lock().unwrap() = None;
            }
            // Rate limited: one request per 5 seconds per IP.
            5 if attempt < 2 => tokio::time::sleep(Duration::from_secs(5)).await,
            code => return Err(describe(code).into()),
        }
    }
    Err(describe(255).into())
}

fn convert(q: ApiQuestion) -> TriviaQuestion {
    TriviaQuestion {
        category: url_decode(&q.category),
        question: url_decode(&q.question),
        correct_answer: url_decode(&q.correct_answer),
        incorrect_answers: q.incorrect_answers.into_iter().map(|a| url_decode(&a)).collect(),
        difficulty: q.difficulty,
        shuffle_seed: fastrand::u64(..),
    }
}

fn url_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}
