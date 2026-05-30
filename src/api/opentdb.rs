use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriviaQuestion {
    pub category: String,
    pub question: String,
    pub correct_answer: String,
    pub incorrect_answers: Vec<String>,
    pub difficulty: String,
}

impl TriviaQuestion {
    /// Returns all 4 answers shuffled (correct answer at a random index).
    pub fn shuffled_answers(&self) -> Vec<(String, bool)> {
        let mut answers: Vec<(String, bool)> = self
            .incorrect_answers
            .iter()
            .map(|a| (a.clone(), false))
            .collect();
        answers.push((self.correct_answer.clone(), true));
        // deterministic shuffle based on question content
        let seed = self.question.len() % 4;
        answers.rotate_left(seed);
        answers
    }
}

#[derive(Deserialize)]
struct ApiResponse {
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

pub async fn fetch_questions(amount: u8, difficulty: &str) -> Result<Vec<TriviaQuestion>, String> {
    let url = format!(
        "https://opentdb.com/api.php?amount={}&type=multiple&encode=url3986&difficulty={}",
        amount, difficulty
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json::<ApiResponse>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp
        .results
        .into_iter()
        .map(|q| TriviaQuestion {
            category: url_decode(&q.category),
            question: url_decode(&q.question),
            correct_answer: url_decode(&q.correct_answer),
            incorrect_answers: q.incorrect_answers.into_iter().map(|a| url_decode(&a)).collect(),
            difficulty: q.difficulty,
        })
        .collect())
}

fn url_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}
