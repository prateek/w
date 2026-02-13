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

#[test]
fn w_cd_switches_existing_branch() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    git(tmp.path(), &["branch", "feature"]);

    let output1 = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["cd", "feature"])
        .output()
        .unwrap();
    assert!(output1.status.success(), "w cd failed: {output1:?}");
    let path1 = parse_path(&output1.stdout);
    assert!(path1.is_absolute());
    assert!(path1.exists());

    let output2 = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["cd", "feature"])
        .output()
        .unwrap();
    assert!(output2.status.success(), "w cd failed: {output2:?}");
    let path2 = parse_path(&output2.stdout);
    assert_eq!(path2, path1);
}

#[test]
fn w_cd_fails_for_missing_branch() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let output = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["cd", "nope"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}
