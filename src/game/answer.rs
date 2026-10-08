//! Typed-answer judging shared by Jeopardy and Quizbowl.
//!
//! Answers are compared as words, not substrings: both sides are lowercased,
//! accent-folded and stripped of punctuation, a leading "what is" / "who are"
//! and a leading article, and number words become digits. Expected answers
//! also yield variants without their parenthetical parts, so "(Abraham)
//! Lincoln" accepts both "Lincoln" and "Abraham Lincoln".
//!
//! A response is accepted when it
//!   * equals an expected variant, allowing typos in proportion to length, or
//!   * contains every expected word (with at most two extra words), or
//!   * is a single word matching the expected answer's last word, at least
//!     four letters long (surnames: "Lincoln" for "Abraham Lincoln").

/// True if `given` is an acceptable response for any of `expected`.
pub fn is_correct<S: AsRef<str>>(given: &str, expected: &[S]) -> bool {
    let g = words(given);
    if g.is_empty() {
        return false;
    }
    expected
        .iter()
        .flat_map(|e| variants(e.as_ref()))
        .any(|e| matches(&g, &e))
}

fn matches(g: &[String], e: &[String]) -> bool {
    if e.is_empty() {
        return false;
    }
    let (gj, ej) = (g.join(" "), e.join(" "));
    if within(&gj, &ej) || within(&g.concat(), &e.concat()) {
        return true;
    }
    // Every expected word present, and not a scattershot list of guesses.
    if g.len() <= e.len() + 2 && e.iter().all(|ew| g.iter().any(|gw| within(gw, ew))) {
        return true;
    }
    // Surname rule.
    if g.len() == 1 && e.len() >= 2 && g[0].chars().count() >= 4 {
        if let Some(last) = e.last() {
            return within(&g[0], last);
        }
    }
    false
}

/// Equal up to a typo budget that grows with the expected length.
fn within(given: &str, expected: &str) -> bool {
    if given == expected {
        return true;
    }
    let tolerance = match expected.chars().count() {
        0..=4 => 0,
        5..=9 => 1,
        _ => 2,
    };
    tolerance > 0 && levenshtein(given, expected) <= tolerance
}

/// Word lists for an expected answer: as written (parentheses dropped but
/// their contents kept), and with parenthetical/bracketed parts removed.
fn variants(expected: &str) -> Vec<Vec<String>> {
    let kept: String = expected.chars().filter(|c| !"()[]".contains(*c)).collect();
    let mut removed = String::with_capacity(expected.len());
    let mut depth = 0i32;
    for c in expected.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => removed.push(c),
            _ => {}
        }
    }
    let mut out = vec![words(&kept)];
    let r = words(&removed);
    if !r.is_empty() && r != out[0] {
        out.push(r);
    }
    out
}

const LEADS: &[&str] = &["what", "who", "where", "when", "which"];
const VERBS: &[&str] = &["is", "are", "was", "were"];
const CONTRACTED: &[&str] = &["whats", "whos", "wheres"];
const ARTICLES: &[&str] = &["the", "a", "an"];
const NUMBERS: &[&str] = &[
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen",
    "nineteen", "twenty",
];

/// Normalize to comparable words (see the module docs).
pub fn words(s: &str) -> Vec<String> {
    let mut clean = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\'' | '’' | '`' => {} // "O'Neill" -> "oneill", "what's" -> "whats"
            '&' => clean.push_str(" and "),
            c if c.is_alphanumeric() => clean.extend(fold(c).to_lowercase()),
            _ => clean.push(' '),
        }
    }
    let mut w: Vec<String> = clean.split_whitespace().map(str::to_owned).collect();
    if w.len() > 2 && LEADS.contains(&w[0].as_str()) && VERBS.contains(&w[1].as_str()) {
        w.drain(..2);
    } else if w.len() > 1 && CONTRACTED.contains(&w[0].as_str()) {
        w.remove(0);
    }
    if w.len() > 1 && ARTICLES.contains(&w[0].as_str()) {
        w.remove(0);
    }
    for word in &mut w {
        if let Some(n) = NUMBERS.iter().position(|n| n == word) {
            *word = n.to_string();
        }
    }
    w
}

/// Fold common Latin accents so "Beyonce" matches "Beyoncé".
fn fold(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'a',
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
        'ñ' | 'Ñ' => 'n',
        'ç' | 'Ç' => 'c',
        'ý' | 'ÿ' | 'Ý' => 'y',
        _ => c,
    }
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            curr[j] = if a[i - 1] == b[j - 1] {
                prev[j - 1]
            } else {
                1 + prev[j - 1].min(prev[j]).min(curr[j - 1])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::is_correct;

    fn ok(given: &str, expected: &str) -> bool {
        is_correct(given, &[expected])
    }

    #[test]
    fn rejects_empty_and_fragments() {
        assert!(!ok("", "Abraham Lincoln"));
        assert!(!ok("   ", "Paris"));
        assert!(!ok("?!", "Paris"));
        assert!(!ok("e", "the Beatles"));
        assert!(!ok("a", "Alaska"));
        assert!(!ok("lin", "Abraham Lincoln"));
        assert!(!ok("the", "the Beatles"));
    }

    #[test]
    fn accepts_exact_and_question_form() {
        assert!(ok("Paris", "Paris"));
        assert!(ok("what is paris", "Paris"));
        assert!(ok("Who are the Beatles?", "the Beatles"));
        assert!(ok("whats paris", "Paris"));
        assert!(ok("beatles", "the Beatles"));
    }

    #[test]
    fn handles_parentheticals_and_surnames() {
        assert!(ok("Lincoln", "(Abraham) Lincoln"));
        assert!(ok("Abraham Lincoln", "(Abraham) Lincoln"));
        assert!(ok("Lincoln", "Abraham Lincoln"));
        assert!(ok("goldfish", "a goldfish (fish accepted)"));
        assert!(!ok("Abraham", "Abraham Lincoln"));
    }

    #[test]
    fn tolerates_typos_by_length() {
        assert!(ok("Missisippi", "Mississippi"));
        assert!(ok("Beethovan", "Beethoven"));
        assert!(!ok("Rone", "Rome"));
        assert!(ok("Paros", "Paris"));
        assert!(!ok("Pxrxs", "Paris"));
    }

    #[test]
    fn normalizes_punctuation_accents_numbers() {
        assert!(ok("oneill", "Eugene O'Neill"));
        assert!(ok("Beyonce", "Beyoncé"));
        assert!(ok("4", "four"));
        assert!(ok("Romeo and Juliet", "Romeo & Juliet"));
        assert!(ok("new york", "New-York"));
    }

    #[test]
    fn limits_scattershot_guesses() {
        assert!(!ok("washington adams jefferson madison lincoln", "Lincoln"));
        assert!(ok("the rolling stones band", "the Rolling Stones"));
    }

    #[test]
    fn checks_every_accepted_alternative() {
        let accepted = ["Gerald Ford".to_string(), "Jerry Ford".to_string()];
        assert!(is_correct("jerry ford", &accepted));
        assert!(is_correct("ford", &accepted));
        assert!(!is_correct("carter", &accepted));
    }
}
