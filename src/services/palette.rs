use regex::Regex;
use std::sync::LazyLock;

pub const PALETTE: &[&str] = &[
    "#e30613", "#0a7cff", "#7c5cff", "#00b86b", "#ff8a00", "#e5008c", "#00a3a8", "#5c6bc0",
    "#c26a00", "#d1112b", "#2e7d32", "#00838f",
];

pub fn accent_for(seed: &str) -> String {
    let mut total: u64 = 0;
    for c in seed.chars() {
        total = (total * 31 + c as u64) % 100_000;
    }
    PALETTE[(total % PALETTE.len() as u64) as usize].to_string()
}

static WORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9\u00c0-\u00ff]+").unwrap());

pub fn make_initials(name: &str) -> String {
    let upper = name.to_uppercase();
    let words: Vec<&str> = WORD_RE.find_iter(&upper).map(|m| m.as_str()).collect();
    if words.is_empty() {
        return "TV".to_string();
    }
    if words.len() == 1 {
        return words[0].chars().take(8).collect();
    }
    let initials: String = words
        .iter()
        .take(4)
        .filter_map(|w| w.chars().next())
        .collect();
    let short: String = initials.chars().take(8).collect();
    if short.is_empty() {
        "TV".to_string()
    } else {
        short
    }
}
