pub fn jeopardy_value(row: usize, is_double_jeopardy: bool) -> i32 {
    let base = [200, 400, 600, 800, 1000][row.min(4)];
    if is_double_jeopardy { base * 2 } else { base }
}

pub fn trivia_score(correct: bool, streak: u32) -> i32 {
    if correct {
        let multiplier = 1 + (streak / 3).min(3);
        (10 * multiplier) as i32
    } else {
        0
    }
}

pub fn quizbowl_score(correct: bool, words_revealed: usize, total_words: usize) -> i32 {
    if !correct {
        return -5;
    }
    let ratio = words_revealed as f32 / total_words.max(1) as f32;
    if ratio < 0.33 {
        20
    } else if ratio < 0.66 {
        15
    } else {
        10
    }
}
