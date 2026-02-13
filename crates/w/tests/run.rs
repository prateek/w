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
fn w_run_executes_in_worktree() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let output = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args([
            "run",
            "feature",
            "--",
            "git",
            "rev-parse",
            "--show-toplevel",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w run failed: {output:?}");

    let top_level = parse_path(&output.stdout)
        .canonicalize()
        .expect("worktree path should exist");
    let expected = tmp
        .path()
        .join(".worktrees/feature")
        .canonicalize()
        .expect("worktree path should exist");
    assert_eq!(top_level, expected);
}
