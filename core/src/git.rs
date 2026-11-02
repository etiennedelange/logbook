//! Just enough git to name a session's repository the way the Copilot CLI does (`owner/repo`, or
//! `org/project/repo` on Azure DevOps), for agents that only record the folder they ran in.
//! Reads `.git/config` and nothing else; never runs git.

use std::path::{Path, PathBuf};

/// The repository containing `dir`: its root folder, and its `origin` remote as a name if it has one.
pub fn repository_of(dir: &Path) -> Option<(PathBuf, Option<String>)> {
    if !dir.is_dir() {
        return None;
    }
    let root = dir.ancestors().find(|d| d.join(".git").exists())?;
    let config = config_path(&root.join(".git"))?;
    let remote = std::fs::read_to_string(config).ok().and_then(|c| origin_url(&c)).and_then(|u| repo_name(&u));
    Some((root.to_path_buf(), remote))
}

/// `.git` is a folder, or in a worktree or submodule a file pointing at one (`gitdir: ...`), whose
/// shared config is in the folder named by its `commondir`.
fn config_path(dot_git: &Path) -> Option<PathBuf> {
    if dot_git.is_dir() {
        return Some(dot_git.join("config"));
    }
    let pointer = std::fs::read_to_string(dot_git).ok()?;
    let gitdir = pointer.trim().strip_prefix("gitdir:")?.trim();
    let gitdir = dot_git.parent()?.join(gitdir);
    let common = std::fs::read_to_string(gitdir.join("commondir")).ok();
    let shared = common.map_or(gitdir.clone(), |c| gitdir.join(c.trim()));
    Some(shared.join("config"))
}

/// The `url` of `[remote "origin"]`.
fn origin_url(config: &str) -> Option<String> {
    let mut in_origin = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_origin = line.replace(' ', "") == "[remote\"origin\"]";
        } else if in_origin {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "url" {
                    return Some(value.trim().to_string());
                }
            }
        }
    }
    None
}

/// `https://github.com/owner/repo.git`, `git@github.com:owner/repo.git` -> `owner/repo`;
/// `https://dev.azure.com/org/project/_git/repo`, `git@ssh.dev.azure.com:v3/org/project/repo` -> `org/project/repo`.
fn repo_name(url: &str) -> Option<String> {
    let url = url.trim_end_matches('/').trim_end_matches(".git");
    // Everything after the host: `scheme://[user@]host/path`, or scp-like `[user@]host:path`.
    let path = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?.1,
        None => url.split_once(':')?.1,
    };
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty() && *p != "_git" && *p != "v3").collect();
    let name = if url.contains("dev.azure.com") || url.contains("visualstudio.com") {
        parts.get(parts.len().checked_sub(3)?..)?.join("/")
    } else {
        parts.get(parts.len().checked_sub(2)?..)?.join("/")
    };
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_remotes_like_copilot() {
        assert_eq!(repo_name("https://github.com/me/logbook.git").as_deref(), Some("me/logbook"));
        assert_eq!(repo_name("git@github.com:me/logbook.git").as_deref(), Some("me/logbook"));
        assert_eq!(repo_name("ssh://git@github.com/me/logbook").as_deref(), Some("me/logbook"));
        assert_eq!(repo_name("https://org@dev.azure.com/org/Project/_git/Backend").as_deref(), Some("org/Project/Backend"));
        assert_eq!(repo_name("git@ssh.dev.azure.com:v3/org/Project/Backend").as_deref(), Some("org/Project/Backend"));
        assert_eq!(repo_name("https://example.com"), None);
    }

    #[test]
    fn reads_origin_and_ignores_other_remotes() {
        let config = "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = https://github.com/them/x.git\n[remote \"origin\"]\n\turl = git@github.com:me/x.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n";
        assert_eq!(origin_url(config).as_deref(), Some("git@github.com:me/x.git"));
        assert_eq!(origin_url("[core]\n"), None);
    }

    #[test]
    fn finds_the_root_from_a_subfolder_and_through_a_worktree() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let tmp = std::env::temp_dir().join(format!("logbook-git-{}-{nanos}", std::process::id()));
        let main = tmp.join("main");
        std::fs::create_dir_all(main.join(".git/worktrees/wt")).unwrap();
        std::fs::create_dir_all(main.join("src/deep")).unwrap();
        std::fs::write(main.join(".git/config"), "[remote \"origin\"]\n\turl = https://github.com/me/x\n").unwrap();
        std::fs::write(main.join(".git/worktrees/wt/commondir"), "../..\n").unwrap();
        let wt = tmp.join("wt");
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", main.join(".git/worktrees/wt").display())).unwrap();

        assert_eq!(repository_of(&main.join("src/deep")), Some((main.clone(), Some("me/x".into()))));
        assert_eq!(repository_of(&wt), Some((wt.clone(), Some("me/x".into()))));
        std::fs::remove_dir_all(&tmp).ok();
    }
}
