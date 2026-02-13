use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;

fn git(current_dir: &Path, args: &[&str]) -> Vec<u8> {
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

    output.stdout
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

fn git_common_dir(repo_dir: &Path) -> PathBuf {
    let stdout = git(repo_dir, &["rev-parse", "--git-common-dir"]);
    let s = String::from_utf8(stdout).expect("stdout should be utf-8");
    let path = PathBuf::from(s.trim());
    if path.is_absolute() {
        path
    } else {
        repo_dir.join(path)
    }
}

#[test]
fn w_prune_removes_stale_worktree_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let output1 = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["new", "feature"])
        .output()
        .unwrap();
    assert!(output1.status.success(), "w new failed: {output1:?}");
    let feature_path = parse_path(&output1.stdout);
    assert!(feature_path.exists());

    let stale_dir = tmp.path().join(".worktrees/stale");
    std::fs::create_dir_all(&stale_dir).unwrap();
    let gitdir = git_common_dir(tmp.path()).join("worktrees/stale");
    std::fs::write(
        stale_dir.join(".git"),
        format!("gitdir: {}\n", gitdir.display()),
    )
    .unwrap();

    let output2 = cargo_bin_cmd!("w")
        .current_dir(tmp.path())
        .env(
            "WORKTRUNK_WORKTREE_PATH",
            ".worktrees/{{ branch | sanitize }}",
        )
        .args(["prune"])
        .output()
        .unwrap();
    assert!(output2.status.success(), "w prune failed: {output2:?}");

    assert!(feature_path.exists(), "expected feature worktree to remain");
    assert!(!stale_dir.exists(), "expected stale dir to be removed");
}
