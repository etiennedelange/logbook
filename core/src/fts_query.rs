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

/// Words of an agent's resume command, as `build` splits it: `claude --resume <id>`,
/// `copilot --resume=<id>`, `claude -r <id>`.
const RESUME_WORDS: [&str; 5] = ["claude", "copilot", "--resume", "-r", "resume"];

/// A pasted resume command searches for its session id alone: when one token is long enough
/// to be an id and every other is a word of the command, only that token is kept.
pub fn strip_resume_command(tokens: Vec<String>) -> Vec<String> {
    let (ids, rest): (Vec<_>, Vec<_>) = tokens.iter().partition(|t| !RESUME_WORDS.contains(&t.as_str()));
    match ids.as_slice() {
        [id] if !rest.is_empty() && looks_like_id(id) => vec![(*id).clone()],
        _ => tokens,
    }
}

/// Long enough to find a session by its id, and not a word: ids are UUIDs or the like.
fn looks_like_id(token: &str) -> bool {
    token.chars().count() >= crate::index::ID_PREFIX_MIN && token.chars().any(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod resume_tests {
    use super::*;

    #[test]
    fn a_pasted_resume_command_searches_its_id() {
        let id = "4f5a6b7c-8d9e-4f0a-a1b2-c3d4e5f6a794";
        for pasted in [format!("claude --resume {id}"), format!("copilot --resume={id}"), format!("claude -r {id}")] {
            assert_eq!(strip_resume_command(build(&pasted)), [id]);
        }
        // Anything else is left alone.
        assert_eq!(strip_resume_command(build("resume claude sessions")), ["resume", "claude", "sessions"]);
        assert_eq!(strip_resume_command(build("claude --resume transcripts")), ["claude", "--resume", "transcripts"]);
        assert_eq!(strip_resume_command(build("claude --resume abc")), ["claude", "--resume", "abc"]);
        assert_eq!(strip_resume_command(build(id)), [id]);
    }
}
