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
fn w_switch_filter_selects_across_repos() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo_a = root.join("repo_a");
    let repo_b = root.join("repo_b");
    std::fs::create_dir_all(&repo_a).unwrap();
    std::fs::create_dir_all(&repo_b).unwrap();
    init_repo(&repo_a);
    init_repo(&repo_b);

    let wt_a = tmp.path().join("worktree_feature_a");
    let wt_b = tmp.path().join("worktree_feature_b");

    git(
        &repo_a,
        &["worktree", "add", "-b", "feature-a", wt_a.to_str().unwrap()],
    );
    git(
        &repo_b,
        &["worktree", "add", "-b", "feature-b", wt_b.to_str().unwrap()],
    );

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "switch",
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--filter",
            "feature-b",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w switch failed: {output:?}");

    let selected = parse_path(&output.stdout);
    assert_eq!(selected, std::fs::canonicalize(&wt_b).unwrap());
}

#[test]
fn w_switch_with_c_scopes_to_repo() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let wt_a = tmp.path().join("worktree_feature_a");
    let wt_b = tmp.path().join("worktree_feature_b");

    git(
        tmp.path(),
        &["worktree", "add", "-b", "feature-a", wt_a.to_str().unwrap()],
    );
    git(
        tmp.path(),
        &["worktree", "add", "-b", "feature-b", wt_b.to_str().unwrap()],
    );

    let output = cargo_bin_cmd!("w")
        .args([
            "-C",
            tmp.path().to_str().unwrap(),
            "switch",
            "--filter",
            "feature-b",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w switch failed: {output:?}");

    let selected = parse_path(&output.stdout);
    assert_eq!(selected, std::fs::canonicalize(&wt_b).unwrap());
}

#[test]
fn w_switch_without_filter_requires_tty() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_repo(&repo);

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "switch",
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    #[cfg(windows)]
    assert!(stderr.contains("interactive picker is not supported on Windows"));
    #[cfg(not(windows))]
    assert!(stderr.contains("interactive picker requires a TTY"));
}
