use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct Worktree {
    pub path: PathBuf,
    pub head: String,
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: Option<String>,
    pub prunable: Option<String>,
}

#[derive(Debug)]
pub enum GitError {
    CommandFailed(String),
    ParseError(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::CommandFailed(msg) => write!(f, "Git command failed: {}", msg),
            GitError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for GitError {}

pub fn list_worktrees() -> Result<Vec<Worktree>, GitError> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|e| GitError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_worktree_list(&stdout)
}

fn parse_worktree_list(output: &str) -> Result<Vec<Worktree>, GitError> {
    let mut worktrees = Vec::new();
    let mut current = None;

    for line in output.lines() {
        if line.is_empty() {
            if let Some(wt) = current.take() {
                worktrees.push(wt);
            }
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let key = parts[0];
        let value = parts.get(1).copied();

        match key {
            "worktree" => {
                let path = value.ok_or_else(|| {
                    GitError::ParseError("worktree line missing path".to_string())
                })?;
                current = Some(Worktree {
                    path: PathBuf::from(path),
                    head: String::new(),
                    branch: None,
                    bare: false,
                    detached: false,
                    locked: None,
                    prunable: None,
                });
            }
            "HEAD" => {
                if let Some(ref mut wt) = current {
                    wt.head = value
                        .ok_or_else(|| GitError::ParseError("HEAD line missing SHA".to_string()))?
                        .to_string();
                }
            }
            "branch" => {
                if let Some(ref mut wt) = current {
                    // Strip refs/heads/ prefix if present
                    let branch = value
                        .ok_or_else(|| GitError::ParseError("branch line missing ref".to_string()))?
                        .strip_prefix("refs/heads/")
                        .unwrap_or(value.unwrap())
                        .to_string();
                    wt.branch = Some(branch);
                }
            }
            "bare" => {
                if let Some(ref mut wt) = current {
                    wt.bare = true;
                }
            }
            "detached" => {
                if let Some(ref mut wt) = current {
                    wt.detached = true;
                }
            }
            "locked" => {
                if let Some(ref mut wt) = current {
                    wt.locked = Some(value.unwrap_or("").to_string());
                }
            }
            "prunable" => {
                if let Some(ref mut wt) = current {
                    wt.prunable = Some(value.unwrap_or("").to_string());
                }
            }
            _ => {
                // Ignore unknown attributes for forward compatibility
            }
        }
    }

    // Push the last worktree if the output doesn't end with a blank line
    if let Some(wt) = current {
        worktrees.push(wt);
    }

    Ok(worktrees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_worktree_list() {
        let output = "worktree /path/to/main
HEAD abcd1234
branch refs/heads/main

worktree /path/to/feature
HEAD efgh5678
branch refs/heads/feature

";

        let worktrees = parse_worktree_list(output).unwrap();
        assert_eq!(worktrees.len(), 2);

        assert_eq!(worktrees[0].path, PathBuf::from("/path/to/main"));
        assert_eq!(worktrees[0].head, "abcd1234");
        assert_eq!(worktrees[0].branch, Some("main".to_string()));
        assert!(!worktrees[0].bare);
        assert!(!worktrees[0].detached);

        assert_eq!(worktrees[1].path, PathBuf::from("/path/to/feature"));
        assert_eq!(worktrees[1].head, "efgh5678");
        assert_eq!(worktrees[1].branch, Some("feature".to_string()));
    }

    #[test]
    fn test_parse_detached_worktree() {
        let output = "worktree /path/to/detached
HEAD abcd1234
detached

";

        let worktrees = parse_worktree_list(output).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].detached);
        assert_eq!(worktrees[0].branch, None);
    }

    #[test]
    fn test_parse_locked_worktree() {
        let output = "worktree /path/to/locked
HEAD abcd1234
branch refs/heads/main
locked reason for lock

";

        let worktrees = parse_worktree_list(output).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].locked, Some("reason for lock".to_string()));
    }

    #[test]
    fn test_parse_bare_worktree() {
        let output = "worktree /path/to/bare
HEAD abcd1234
bare

";

        let worktrees = parse_worktree_list(output).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].bare);
    }
}
