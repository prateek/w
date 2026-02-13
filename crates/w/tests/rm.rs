use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;

fn git(current_dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(current_dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));

    if !output.status.success() {
        panic!(
            "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

fn init_repo(repo_dir: &Path) {
    git(repo_dir, &["init", "-b", "main"]);
    git(repo_dir, &["config", "user.name", "Test User"]);
    git(repo_dir, &["config", "user.email", "test@example.com"]);

    std::fs::write(repo_dir.join("README.md"), "hello\n").unwrap();
    git(repo_dir, &["add", "README.md"]);
    git(repo_dir, &["commit", "-m", "initial"]);
}

fn parse_path(stdout: &[u8]) -> PathBuf {
    let s = String::from_utf8(stdout.to_vec()).expect("stdout should be utf-8");
    PathBuf::from(s.trim())
}

fn local_branch_exists(repo_dir: &Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .current_dir(repo_dir)
        .status()
        .unwrap()
        .success()
}

#[test]
fn w_rm_removes_clean_worktree() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let output_new = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["new", "feature"])
        .output()
        .unwrap();
    assert!(output_new.status.success(), "w new failed: {output_new:?}");

    let worktree_path = parse_path(&output_new.stdout);
    assert!(worktree_path.is_absolute());
    assert!(worktree_path.exists());
    assert!(local_branch_exists(tmp.path(), "feature"));

    let output_rm = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["rm", "feature"])
        .output()
        .unwrap();
    assert!(output_rm.status.success(), "w rm failed: {output_rm:?}");

    let removed_path = parse_path(&output_rm.stdout);
    assert_eq!(removed_path, worktree_path);
    assert!(!worktree_path.exists());
    assert!(local_branch_exists(tmp.path(), "feature"));
}

#[test]
fn w_rm_refuses_dirty_without_force_then_succeeds_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let output_new = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["new", "feature"])
        .output()
        .unwrap();
    assert!(output_new.status.success(), "w new failed: {output_new:?}");
    let worktree_path = parse_path(&output_new.stdout);

    std::fs::write(worktree_path.join("README.md"), "dirty\n").unwrap();

    let output_rm = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["rm", "feature"])
        .output()
        .unwrap();
    assert!(!output_rm.status.success(), "w rm unexpectedly succeeded");
    assert!(worktree_path.exists());

    let output_rm_force = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["rm", "feature", "--force"])
        .output()
        .unwrap();
    assert!(
        output_rm_force.status.success(),
        "w rm --force failed: {output_rm_force:?}"
    );

    let removed_path = parse_path(&output_rm_force.stdout);
    assert_eq!(removed_path, worktree_path);
    assert!(!worktree_path.exists());
    assert!(local_branch_exists(tmp.path(), "feature"));
}

#[test]
fn w_rm_fails_for_missing_worktree() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let output_rm = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["rm", "nope"])
        .output()
        .unwrap();
    assert!(!output_rm.status.success());
}
