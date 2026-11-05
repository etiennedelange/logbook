//! The files a session changed, from the edit records an agent leaves in its transcript, and the
//! shell commands that may have changed others. Agent-neutral: each source turns its own records
//! into `Edit`s and shell commands, and this module sums them up.

use crate::models::{FileChange, FileEdit, Hunk};
use chrono::{DateTime, Utc};

/// Most shell commands kept per session.
const MAX_COMMANDS: usize = 30;
/// Longest shell command kept, in characters.
const MAX_COMMAND_CHARS: usize = 240;
/// Most diff lines kept for one edit, so a generated file doesn't fill the detail pane.
pub const MAX_EDIT_LINES: usize = 1500;

/// One successful write by the agent's file tools.
#[derive(Debug, Clone)]
pub struct Edit {
    /// As the agent recorded it: usually absolute.
    pub path: String,
    pub created: bool,
    pub timestamp: Option<DateTime<Utc>>,
    pub hunks: Vec<Hunk>,
    pub truncated: bool,
    pub added: u32,
    pub removed: u32,
}

impl Edit {
    pub fn into_file_edit(self) -> FileEdit {
        FileEdit { timestamp: self.timestamp, created: self.created, hunks: self.hunks, truncated: self.truncated }
    }
}

/// Each file `edits` touched, once, in the order first touched.
pub fn files(edits: &[Edit], cwd: Option<&str>) -> Vec<FileChange> {
    let mut out: Vec<FileChange> = Vec::new();
    for e in edits {
        let path = relative(&e.path, cwd);
        match out.iter_mut().find(|f| f.path == path) {
            Some(f) => {
                f.edits += 1;
                f.added += e.added;
                f.removed += e.removed;
            }
            None => out.push(FileChange { path, created: e.created, edits: 1, added: e.added, removed: e.removed }),
        }
    }
    out
}

/// `path` relative to `cwd` when it's inside it, else unchanged. Compared as text, ignoring case and
/// which slash is used, so a Windows session's paths work the same on any machine.
pub fn relative(path: &str, cwd: Option<&str>) -> String {
    let Some(cwd) = cwd.map(|c| c.trim_end_matches(['/', '\\'])).filter(|c| !c.is_empty()) else {
        return path.to_owned();
    };
    let same = |a: char, b: char| a.eq_ignore_ascii_case(&b) || (matches!(a, '/' | '\\') && matches!(b, '/' | '\\'));
    let mut p = path.chars();
    if !cwd.chars().all(|c| p.next().is_some_and(|q| same(c, q))) {
        return path.to_owned();
    }
    match p.next() {
        Some('/' | '\\') if !p.as_str().is_empty() => p.as_str().to_owned(),
        _ => path.to_owned(),
    }
}

/// A diff's line counts: lines starting `+` and lines starting `-`.
pub fn counts(hunks: &[Hunk]) -> (u32, u32) {
    let lines = hunks.iter().flat_map(|h| &h.lines);
    let (mut added, mut removed) = (0, 0);
    for l in lines {
        match l.as_bytes().first() {
            Some(b'+') => added += 1,
            Some(b'-') => removed += 1,
            _ => {}
        }
    }
    (added, removed)
}

/// A new file as a diff: every line added.
pub fn created_hunk(content: &str) -> (Vec<Hunk>, bool) {
    let lines: Vec<String> = content.lines().map(|l| format!("+{l}")).collect();
    let truncated = lines.len() > MAX_EDIT_LINES;
    let lines = lines.into_iter().take(MAX_EDIT_LINES).collect::<Vec<_>>();
    if lines.is_empty() {
        return (Vec::new(), false);
    }
    (vec![Hunk { old_start: 0, new_start: 1, lines }], truncated)
}

/// Adds `command` to `list` if it looks like it changes files, isn't there already, and the list
/// isn't full. Only its first line is kept, cut to `MAX_COMMAND_CHARS`.
pub fn note_command(list: &mut Vec<String>, command: &str) {
    if list.len() >= MAX_COMMANDS || !may_change_files(command) {
        return;
    }
    let first = command.trim().lines().next().unwrap_or_default();
    let mut kept: String = first.chars().take(MAX_COMMAND_CHARS).collect();
    if kept.len() < first.len() || command.trim().lines().nth(1).is_some() {
        kept.push_str(" …");
    }
    if !list.contains(&kept) {
        list.push(kept);
    }
}

/// Whether a shell command looks like it writes files: moves, deletes, in-place edits, output
/// redirected to a file, git commands that rewrite the working tree, formatters. A guess from the
/// command's words; it's only used to say "these may have changed files".
pub fn may_change_files(command: &str) -> bool {
    shell_lines(command).any(|line| {
        line.split([';', '|', '&']).any(|part| {
            let words: Vec<&str> = part.split_whitespace().collect();
            segment_changes_files(&words) || redirects_to_file(&words)
        })
    })
}

/// The command's lines, leaving out heredoc bodies (`<<'EOF'` ... `EOF`): they're a script's
/// text, not commands.
fn shell_lines(command: &str) -> impl Iterator<Item = &str> {
    let mut until: Option<String> = None;
    command.lines().filter(move |line| {
        if let Some(end) = &until {
            if line.trim() == end {
                until = None;
            }
            return false;
        }
        until = line.split_once("<<").map(|(_, rest)| {
            let rest = rest.trim_start_matches('-').trim_start();
            rest.split_whitespace().next().unwrap_or_default().trim_matches(['\'', '"']).to_owned()
        });
        true
    })
}

fn segment_changes_files(words: &[&str]) -> bool {
    // Skip `sudo`, `env X=1` and the like to the program itself.
    let words: Vec<&str> =
        words.iter().copied().skip_while(|w| *w == "sudo" || *w == "env" || w.contains('=')).collect();
    let Some(&program) = words.first() else { return false };
    let program = program.rsplit(['/', '\\']).next().unwrap_or(program).to_ascii_lowercase();
    let has = |flag: &str| words.iter().skip(1).any(|w| *w == flag || w.starts_with(&format!("{flag}=")));
    let sub = words.get(1).map(|w| w.to_ascii_lowercase());
    match program.as_str() {
        "rm" | "mv" | "cp" | "touch" | "mkdir" | "rmdir" | "ln" | "patch" | "tee" | "truncate" | "unzip" | "del"
        | "move" | "copy" | "ren" | "rename" | "rustfmt" | "gofmt" | "black" | "remove-item" | "move-item"
        | "copy-item" | "rename-item" | "new-item" | "set-content" | "add-content" | "out-file" => true,
        "sed" | "perl" => words.iter().skip(1).any(|w| w.starts_with("-i") || *w == "--in-place"),
        "git" => matches!(
            sub.as_deref(),
            Some(
                "mv" | "rm"
                    | "checkout"
                    | "restore"
                    | "reset"
                    | "apply"
                    | "am"
                    | "stash"
                    | "merge"
                    | "rebase"
                    | "cherry-pick"
                    | "revert"
                    | "pull"
                    | "switch"
                    | "clean"
            )
        ),
        "cargo" => matches!(sub.as_deref(), Some("fmt" | "fix")) || has("--fix"),
        "ruff" => matches!(sub.as_deref(), Some("format")) || has("--fix"),
        "tar" => words.get(1).is_some_and(|w| w.trim_start_matches('-').contains('x')),
        _ => has("--write") || has("--fix") || has("-w") && program.ends_with("fmt"),
    }
}

/// `> file` or `>> file`, other than `/dev/null` and descriptor copies like `2>&1`.
fn redirects_to_file(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(i, w)| {
        let Some(at) = w.find('>') else { return false };
        // `->` and `=>` are arrows in arguments, not redirects.
        if w[..at].ends_with(['-', '=']) {
            return false;
        }
        let rest = w[at..].trim_start_matches('>');
        // `>=` compares.
        if rest.starts_with('=') {
            return false;
        }
        let target = if rest.is_empty() { words.get(i + 1).copied().unwrap_or("") } else { rest };
        !target.is_empty()
            && !target.starts_with('&')
            && !target.starts_with("/dev/")
            && !target.eq_ignore_ascii_case("nul")
            && !target.eq_ignore_ascii_case("$null")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_inside_the_folder_are_relative() {
        assert_eq!(relative("/home/me/demo/src/a.rs", Some("/home/me/demo")), "src/a.rs");
        assert_eq!(relative("/home/me/demo/src/a.rs", Some("/home/me/demo/")), "src/a.rs");
        assert_eq!(relative(r"C:\Repos\App\src\a.rs", Some(r"c:\repos\app")), r"src\a.rs");
        assert_eq!(relative("/home/me/demo2/a.rs", Some("/home/me/demo")), "/home/me/demo2/a.rs");
        assert_eq!(relative("/tmp/x", None), "/tmp/x");
        assert_eq!(relative("/home/me/demo", Some("/home/me/demo")), "/home/me/demo");
    }

    #[test]
    fn edits_add_up_per_file() {
        let edit = |path: &str, created: bool, added: u32, removed: u32| Edit {
            path: path.into(),
            created,
            timestamp: None,
            hunks: Vec::new(),
            truncated: false,
            added,
            removed,
        };
        let f = files(
            &[edit("/d/a.rs", true, 10, 0), edit("/d/b.rs", false, 2, 1), edit("/d/a.rs", false, 3, 4)],
            Some("/d"),
        );
        assert_eq!(
            f,
            [
                FileChange { path: "a.rs".into(), created: true, edits: 2, added: 13, removed: 4 },
                FileChange { path: "b.rs".into(), created: false, edits: 1, added: 2, removed: 1 },
            ]
        );
    }

    #[test]
    fn guesses_which_commands_change_files() {
        for yes in [
            "git mv src/a.rs src/b.rs",
            "cd /x && rm -rf target/tmp",
            "sed -i 's/a/b/' file.txt",
            "cargo fmt",
            "npx prettier --write .",
            "echo hi > notes.txt",
            "cat a >>b.txt",
            "git checkout -- src/",
            "Remove-Item foo.txt",
            "tar -xzf a.tgz",
            "sudo mv a b",
            "cat > notes.md <<'EOF'\n# Notes\nEOF",
        ] {
            assert!(may_change_files(yes), "{yes}");
        }
        for no in [
            "cargo test",
            "git status && git diff",
            "ls -la 2>&1",
            "grep -rn foo src > /dev/null",
            "rg 'a -> b' src",
            "pnpm ui-check",
            "git log --oneline | head",
            "sed -n 1,20p a.rs",
            "tar -tzf a.tgz",
            "python3 - <<'EOF'\nopen('a', 'w').write(x > 1)\nrm = 1\nEOF\ncargo test",
            "[ $n -ge 2 ] && echo $((a>=b))",
        ] {
            assert!(!may_change_files(no), "{no}");
        }
    }

    #[test]
    fn commands_are_kept_once_and_short() {
        let mut list = Vec::new();
        note_command(&mut list, "cargo test");
        note_command(&mut list, "cargo fmt");
        note_command(&mut list, "cargo fmt");
        note_command(&mut list, "rm a\nrm b");
        assert_eq!(list, ["cargo fmt", "rm a …"]);
    }
}
