//! Versioned integration surface for wrappers.
//!
//! This is intentionally narrow, data-oriented, and avoids any CLI rendering or
//! shell-directive assumptions. Callers are expected to provide their own UX.

use std::path::{Path, PathBuf};

use anyhow::Context;
use dunce::canonicalize;
use normalize_path::NormalizePath;

use crate::config::UserConfig;
use crate::git::{GitError, Repository, check_integration, compute_integration_lazy};
use crate::path::format_path_for_display;

/// How to handle branch deletion after removing a worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchDeletionMode {
    /// Keep the branch regardless of merge/integration status.
    Keep,
    /// Delete the branch only if it's integrated into the target branch.
    SafeDelete,
    /// Delete the branch even if it's not integrated into the target branch.
    ForceDelete,
}

impl BranchDeletionMode {
    pub fn should_keep(self) -> bool {
        matches!(self, Self::Keep)
    }

    pub fn is_force(self) -> bool {
        matches!(self, Self::ForceDelete)
    }
}

/// Request to switch to (or create) a worktree.
#[derive(Debug, Clone)]
pub struct SwitchRequest {
    /// Branch name (supports worktrunk symbols like "@", "-", "^" via git resolution).
    pub branch: String,
    /// When true, create a new branch (and worktree) from `base` / default branch.
    pub create: bool,
    /// Base ref when creating a branch. If `None`, uses the repository's default branch.
    pub base: Option<String>,
    /// When true, move a pre-existing directory at the computed path aside.
    pub clobber: bool,
}

/// Result of a switch operation.
#[derive(Debug, Clone)]
pub struct SwitchOutcome {
    pub branch: String,
    pub path: PathBuf,
    pub created: bool,
    pub created_branch: bool,
    pub base_branch: Option<String>,
}

/// Request to remove a worktree (and optionally delete its branch).
#[derive(Debug, Clone)]
pub struct RemoveRequest {
    /// Branch name whose worktree should be removed.
    pub branch: String,
    pub deletion_mode: BranchDeletionMode,
    /// When true, allow git to remove a worktree even with untracked files.
    pub force_worktree: bool,
    /// Target branch to use for "safe delete" integration checks.
    /// If `None`, uses the repository's default branch.
    pub target_branch: Option<String>,
}

/// Outcome of a remove operation.
#[derive(Debug, Clone)]
pub struct RemoveOutcome {
    pub branch: String,
    pub removed_worktree_path: Option<PathBuf>,
    pub branch_deleted: bool,
    pub deletion_mode: BranchDeletionMode,
}

/// Compute the expected worktree path for a branch name.
///
/// - For the default branch, returns the repo root (main worktree location).
/// - For other branches, applies `worktree-path` template from config.
///
/// Note: bare repos have no main worktree, so all branches use templated paths.
pub fn compute_worktree_path(
    repo: &Repository,
    branch: &str,
    config: &UserConfig,
) -> anyhow::Result<PathBuf> {
    let repo_root = repo.repo_path();
    let default_branch = repo.default_branch().unwrap_or_default();
    let is_bare = repo.is_bare();

    if !is_bare && branch == default_branch {
        return Ok(repo_root.to_path_buf());
    }

    let repo_name = repo_root
        .file_name()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Repository path has no filename: {}",
                format_path_for_display(repo_root)
            )
        })?
        .to_str()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Repository path contains invalid UTF-8: {}",
                format_path_for_display(repo_root)
            )
        })?;

    let project = repo.project_identifier().ok();
    let expanded_path = config
        .format_path(repo_name, branch, repo, project.as_deref())
        .map_err(|e| anyhow::anyhow!("Failed to format worktree path: {e}"))?;

    Ok(repo_root.join(expanded_path).normalize())
}

/// Switch to a worktree if it exists, otherwise create a new worktree.
///
/// This is a pure operation API:
/// - no output
/// - no hooks
/// - no shell directive file writes
pub fn switch(
    repo: &Repository,
    config: &UserConfig,
    request: SwitchRequest,
) -> anyhow::Result<SwitchOutcome> {
    let branch = repo
        .resolve_worktree_name(&request.branch)
        .context("Failed to resolve branch name")?;

    if let Some(existing_path) = repo.worktree_for_branch(&branch)? {
        if !existing_path.exists() {
            return Err(GitError::WorktreeMissing { branch }.into());
        }
        let path = canonicalize(&existing_path).unwrap_or(existing_path);
        return Ok(SwitchOutcome {
            branch,
            path,
            created: false,
            created_branch: false,
            base_branch: None,
        });
    }

    let expected_path = compute_worktree_path(repo, &branch, config)?;

    // Reject path collisions with other worktrees (or missing worktree dirs).
    if let Some((existing_path, occupant)) = repo.worktree_at_path(&expected_path)? {
        if !existing_path.exists() {
            let occupant_branch = occupant.unwrap_or_else(|| branch.clone());
            return Err(GitError::WorktreeMissing {
                branch: occupant_branch,
            }
            .into());
        }
        return Err(GitError::WorktreePathOccupied {
            branch: branch.clone(),
            path: expected_path.clone(),
            occupant,
        }
        .into());
    }

    // Handle stale directories at the computed path.
    if let Some(backup_path) =
        compute_clobber_backup(&expected_path, &branch, request.clobber, request.create)?
    {
        std::fs::rename(&expected_path, &backup_path).with_context(|| {
            format!(
                "Failed to move {} to {}",
                format_path_for_display(&expected_path),
                format_path_for_display(&backup_path)
            )
        })?;
    }

    let mut base_branch = if request.create {
        if repo.branch(&branch).exists_locally()? {
            return Err(GitError::BranchAlreadyExists { branch }.into());
        }

        let resolved_base = match request.base.as_deref() {
            Some(b) => {
                let resolved = repo.resolve_worktree_name(b)?;
                if !repo.ref_exists(&resolved)? {
                    return Err(GitError::ReferenceNotFound {
                        reference: resolved,
                    }
                    .into());
                }
                resolved
            }
            None => repo.resolve_target_branch(None)?,
        };
        Some(resolved_base)
    } else {
        // For non-create switches, ensure the branch exists somewhere.
        if !repo.branch(&branch).exists()? {
            return Err(GitError::BranchNotFound {
                branch,
                show_create_hint: true,
            }
            .into());
        }
        None
    };

    let mut created_branch = request.create;

    if let Some(parent) = expected_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create directory {}",
                format_path_for_display(parent)
            )
        })?;
    }

    // Create the worktree.
    // Use `--` to prevent paths or branch names starting with '-' being interpreted as flags.
    let worktree_path_str = expected_path.to_string_lossy();
    if request.create {
        let mut args = vec![
            "worktree",
            "add",
            "-b",
            branch.as_str(),
            "--",
            worktree_path_str.as_ref(),
        ];
        if let Some(base) = base_branch.as_deref() {
            args.push(base);
        }
        let _ = repo
            .run_command(&args)
            .map_err(|e| GitError::WorktreeCreationFailed {
                branch: branch.clone(),
                base_branch: base_branch.clone(),
                error: e.to_string(),
            })?;
    } else {
        let branch_handle = repo.branch(&branch);
        let local_branch_exists = branch_handle.exists_locally().unwrap_or(false);

        // If the branch doesn't exist locally but has exactly one remote tracking ref,
        // explicitly create the local branch from that tracking ref. This avoids git's
        // DWIM dependence on fetch refspecs (single-branch clones, bare repos, etc.).
        let remote_start_point = if !local_branch_exists {
            let remotes = branch_handle.remotes().unwrap_or_default();
            (remotes.len() == 1)
                .then(|| format!("{}/{}", remotes[0], branch))
                .filter(|r| repo.ref_exists(r).unwrap_or(false))
        } else {
            None
        };

        if let Some(start_point) = &remote_start_point {
            base_branch = Some(start_point.clone());
            created_branch = true;
        }

        let args = if let Some(start_point) = remote_start_point.as_deref() {
            vec![
                "worktree",
                "add",
                "-b",
                branch.as_str(),
                "--",
                worktree_path_str.as_ref(),
                start_point,
            ]
        } else {
            vec![
                "worktree",
                "add",
                "--",
                worktree_path_str.as_ref(),
                branch.as_str(),
            ]
        };
        let _ = repo
            .run_command(&args)
            .map_err(|e| GitError::WorktreeCreationFailed {
                branch: branch.clone(),
                base_branch: base_branch.clone(),
                error: e.to_string(),
            })?;
    }

    let path = canonicalize(&expected_path).unwrap_or(expected_path);
    Ok(SwitchOutcome {
        branch,
        path,
        created: true,
        created_branch,
        base_branch,
    })
}

/// Remove a worktree and optionally delete the branch.
pub fn remove(
    repo: &Repository,
    _config: &UserConfig,
    request: RemoveRequest,
) -> anyhow::Result<RemoveOutcome> {
    let branch = repo
        .resolve_worktree_name(&request.branch)
        .context("Failed to resolve branch name")?;

    let removed_worktree_path = match repo.worktree_for_branch(&branch)? {
        Some(path) if path.exists() => {
            let output_path = canonicalize(&path).unwrap_or_else(|_| path.clone());

            // Reject locked worktrees to avoid silent data loss.
            if let Some(wt) = repo
                .list_worktrees()?
                .into_iter()
                .find(|wt| wt.branch.as_deref() == Some(branch.as_str()))
                && wt.locked.is_some()
            {
                return Err(GitError::WorktreeLocked {
                    branch: branch.clone(),
                    path: path.clone(),
                    reason: wt.locked.clone(),
                }
                .into());
            }

            let wt = repo.worktree_at(&path);
            if !wt.is_linked()? {
                return Err(GitError::CannotRemoveMainWorktree.into());
            }

            if !request.force_worktree {
                wt.ensure_clean("remove worktree", Some(&branch), true)?;
            }

            repo.remove_worktree(&path, request.force_worktree)
                .map_err(|e| GitError::WorktreeRemovalFailed {
                    branch: branch.clone(),
                    path: path.clone(),
                    error: e.to_string(),
                })?;
            Some(output_path)
        }
        Some(_) => {
            // Directory missing - prune and treat as branch-only removal.
            let _ = repo.prune_worktrees();
            None
        }
        None => None,
    };

    let branch_deleted = delete_branch(
        repo,
        &branch,
        request.deletion_mode,
        request.target_branch.as_deref(),
    )?;

    Ok(RemoveOutcome {
        branch,
        removed_worktree_path,
        branch_deleted,
        deletion_mode: request.deletion_mode,
    })
}

fn delete_branch(
    repo: &Repository,
    branch: &str,
    deletion_mode: BranchDeletionMode,
    target_branch: Option<&str>,
) -> anyhow::Result<bool> {
    if deletion_mode.should_keep() {
        return Ok(false);
    }

    // Conservative: only delete local branches.
    if !repo.branch(branch).exists_locally()? {
        return Ok(false);
    }

    let should_delete = match deletion_mode {
        BranchDeletionMode::Keep => false,
        BranchDeletionMode::ForceDelete => true,
        BranchDeletionMode::SafeDelete => {
            let target = match target_branch {
                Some(t) => repo.resolve_worktree_name(t)?,
                None => match repo.default_branch() {
                    Some(db) => db,
                    None => return Ok(false),
                },
            };

            if target == branch {
                return Ok(false);
            }

            let signals = compute_integration_lazy(repo, branch, &target)?;
            check_integration(&signals).is_some()
        }
    };

    if should_delete {
        // Use -D because we've already decided whether this is safe.
        repo.run_command(&["branch", "-D", branch])?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn compute_clobber_backup(
    path: &Path,
    branch: &str,
    clobber: bool,
    create: bool,
) -> anyhow::Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    if clobber {
        let timestamp = crate::utils::get_now() as i64;
        let datetime =
            chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_else(chrono::Utc::now);
        let suffix = datetime.format("%Y%m%d-%H%M%S").to_string();
        let backup_path = generate_backup_path(path, &suffix)?;

        if backup_path.exists() {
            anyhow::bail!(
                "Backup path already exists: {}",
                format_path_for_display(&backup_path)
            );
        }
        Ok(Some(backup_path))
    } else {
        Err(GitError::WorktreePathExists {
            branch: branch.to_string(),
            path: path.to_path_buf(),
            create,
        }
        .into())
    }
}

fn generate_backup_path(path: &Path, suffix: &str) -> anyhow::Result<PathBuf> {
    let file_name = path.file_name().ok_or_else(|| {
        anyhow::anyhow!(
            "Cannot generate backup path for {}",
            format_path_for_display(path)
        )
    })?;

    if path.extension().is_none() {
        Ok(path.with_file_name(format!("{}.bak.{suffix}", file_name.to_string_lossy())))
    } else {
        Ok(path.with_extension(format!(
            "{}.bak.{suffix}",
            path.extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRepo {
        _dir: tempfile::TempDir,
        repo: Repository,
    }

    impl TestRepo {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();

            // Explicit default branch for determinism.
            let init = std::process::Command::new("git")
                .args(["init", "-b", "main"])
                .current_dir(dir.path())
                .output()
                .unwrap();
            assert!(
                init.status.success(),
                "git init failed: {}",
                String::from_utf8_lossy(&init.stderr)
            );

            let repo = Repository::at(dir.path()).unwrap();

            // Minimal config so commits work.
            repo.run_command(&["config", "user.name", "Test User"])
                .unwrap();
            repo.run_command(&["config", "user.email", "test@example.com"])
                .unwrap();

            // Initial commit.
            std::fs::write(dir.path().join("README.md"), "hello\n").unwrap();
            repo.run_command(&["add", "README.md"]).unwrap();
            repo.run_command(&["commit", "-m", "initial"]).unwrap();

            Self { _dir: dir, repo }
        }
    }

    #[test]
    fn switch_create_then_existing_returns_paths() {
        let test_repo = TestRepo::new();
        let repo = &test_repo.repo;
        let mut config = UserConfig::default();
        config.configs.worktree_path = Some(".worktrees/{{ branch | sanitize }}".to_string());

        let created = switch(
            repo,
            &config,
            SwitchRequest {
                branch: "feature".to_string(),
                create: true,
                base: None,
                clobber: false,
            },
        )
        .unwrap();
        assert!(created.created);
        assert!(created.created_branch);

        let expected = compute_worktree_path(repo, "feature", &config).unwrap();
        assert_eq!(created.path, expected);
        assert!(created.path.exists());

        let existing = switch(
            repo,
            &config,
            SwitchRequest {
                branch: "feature".to_string(),
                create: false,
                base: None,
                clobber: false,
            },
        )
        .unwrap();
        assert!(!existing.created);
        assert_eq!(existing.path, created.path);
    }

    #[test]
    fn remove_safe_delete_removes_worktree_and_deletes_branch() {
        let test_repo = TestRepo::new();
        let repo = &test_repo.repo;
        let mut config = UserConfig::default();
        config.configs.worktree_path = Some(".worktrees/{{ branch | sanitize }}".to_string());

        let created = switch(
            repo,
            &config,
            SwitchRequest {
                branch: "feature".to_string(),
                create: true,
                base: None,
                clobber: false,
            },
        )
        .unwrap();

        let removed = remove(
            repo,
            &config,
            RemoveRequest {
                branch: "feature".to_string(),
                deletion_mode: BranchDeletionMode::SafeDelete,
                force_worktree: false,
                target_branch: None,
            },
        )
        .unwrap();

        assert_eq!(removed.branch, "feature");
        assert_eq!(removed.removed_worktree_path, Some(created.path.clone()));
        assert!(!created.path.exists());
        assert!(removed.branch_deleted);
        assert!(!repo.branch("feature").exists_locally().unwrap());
    }
}
