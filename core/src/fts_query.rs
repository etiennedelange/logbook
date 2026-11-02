/// Tokenizes free-form user input into lowercase word tokens used for a simple
/// prefix-match AND search across session transcripts (e.g. "dashboard env" ->
/// ["dashboard", "env"], matched as an AND of word-prefixes — see `index::search`).
pub fn build(raw_input: &str) -> Vec<String> {
    raw_input
        .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}
