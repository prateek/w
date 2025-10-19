use crate::common::TestRepo;
use insta::Settings;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

/// Helper to create snapshot with normalized paths and SHAs
fn snapshot_list(test_name: &str, repo: &TestRepo) {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    // Normalize paths - replace absolute paths with semantic names
    settings.add_filter(repo.root_path().to_str().unwrap(), "[REPO]");
    for (name, path) in &repo.worktrees {
        settings.add_filter(
            path.to_str().unwrap(),
            format!("[WORKTREE_{}]", name.to_uppercase().replace('-', "_")),
        );
    }

    // Normalize git SHAs (7-40 hex chars) to [SHA]
    settings.add_filter(r"\b[0-9a-f]{7,40}\b", "[SHA]");

    // Normalize Windows paths to Unix style
    settings.add_filter(r"\\", "/");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("wt"));
        // Clean environment to avoid interference from global git config
        repo.clean_cli_env(&mut cmd);
        cmd.arg("list").current_dir(repo.root_path());

        assert_cmd_snapshot!(test_name, cmd);
    });
}

#[test]
fn test_list_single_worktree() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    snapshot_list("single_worktree", &repo);
}

#[test]
fn test_list_multiple_worktrees() {
    let mut repo = TestRepo::new();
    repo.commit("Initial commit");

    repo.add_worktree("feature-a", "feature-a");
    repo.add_worktree("feature-b", "feature-b");

    snapshot_list("multiple_worktrees", &repo);
}

#[test]
fn test_list_detached_head() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    repo.detach_head();

    snapshot_list("detached_head", &repo);
}

#[test]
fn test_list_locked_worktree() {
    let mut repo = TestRepo::new();
    repo.commit("Initial commit");

    repo.add_worktree("locked-feature", "locked-feature");
    repo.lock_worktree("locked-feature", Some("Testing lock functionality"));

    snapshot_list("locked_worktree", &repo);
}

#[test]
fn test_list_locked_no_reason() {
    let mut repo = TestRepo::new();
    repo.commit("Initial commit");

    repo.add_worktree("locked-no-reason", "locked-no-reason");
    repo.lock_worktree("locked-no-reason", None);

    snapshot_list("locked_no_reason", &repo);
}
