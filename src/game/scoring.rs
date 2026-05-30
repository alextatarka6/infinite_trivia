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

pub fn quizbowl_score(correct: bool, chars_revealed: usize, total_chars: usize) -> i32 {
    if correct { 10 } else if chars_revealed < total_chars { -5 } else { 0 }
}
