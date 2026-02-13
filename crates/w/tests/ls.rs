use std::path::Path;

use assert_cmd::cargo::cargo_bin_cmd;
use serde::Deserialize;

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

fn init_root_repo_with_feature_worktree(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_repo(&repo);

    let wt = tmp.path().join("worktree_feature");
    git(
        &repo,
        &["worktree", "add", "-b", "feature", wt.to_str().unwrap()],
    );

    root
}

#[derive(Debug, Deserialize)]
struct LsOutput {
    schema_version: u32,
    worktrees: Vec<LsWorktree>,
    errors: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct LsWorktree {
    repo_path: String,
    path: String,
    branch: Option<String>,
}

#[test]
fn w_ls_json_lists_worktrees_across_repos() {
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
            "ls",
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w ls failed: {output:?}");

    let out: LsOutput = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(out.schema_version, 1);
    assert!(
        out.errors.is_empty(),
        "expected no errors, got: {:?}",
        out.errors
    );

    let repo_a = std::fs::canonicalize(&repo_a)
        .unwrap()
        .to_string_lossy()
        .to_string();
    let repo_b = std::fs::canonicalize(&repo_b)
        .unwrap()
        .to_string_lossy()
        .to_string();
    let wt_a = std::fs::canonicalize(&wt_a)
        .unwrap()
        .to_string_lossy()
        .to_string();
    let wt_b = std::fs::canonicalize(&wt_b)
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut expected = vec![
        (repo_a.clone(), repo_a.clone(), Some("main".to_string())),
        (repo_a.clone(), wt_a.clone(), Some("feature-a".to_string())),
        (repo_b.clone(), repo_b.clone(), Some("main".to_string())),
        (repo_b.clone(), wt_b.clone(), Some("feature-b".to_string())),
    ];
    expected.sort();

    let mut actual = out
        .worktrees
        .into_iter()
        .map(|wt| (wt.repo_path, wt.path, wt.branch))
        .collect::<Vec<_>>();
    actual.sort();

    assert_eq!(actual, expected);
}

#[test]
fn w_ls_tsv_is_machine_parseable() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_repo(&repo);

    let wt = tmp.path().join("worktree_feature");
    git(
        &repo,
        &["worktree", "add", "-b", "feature", wt.to_str().unwrap()],
    );

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "ls",
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "tsv",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w ls failed: {output:?}");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2, "expected 2 worktrees, got: {lines:?}");

    for line in lines {
        let cols = line.split('\t').collect::<Vec<_>>();
        assert_eq!(cols.len(), 8, "expected 8 TSV columns, got: {cols:?}");
        assert!(!cols[0].is_empty(), "project_identifier should be set");
        assert!(!cols[1].is_empty(), "repo_path should be set");
        assert!(!cols[2].is_empty(), "worktree_path should be set");
    }
}

#[test]
fn w_ls_with_c_uses_repo_root_path() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let subdir = tmp.path().join("nested/dir");
    std::fs::create_dir_all(&subdir).unwrap();

    let output = cargo_bin_cmd!("w")
        .args(["-C", subdir.to_str().unwrap(), "ls", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "w ls failed: {output:?}");

    let out: LsOutput = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(out.schema_version, 1);
    assert!(
        out.errors.is_empty(),
        "expected no errors, got: {:?}",
        out.errors
    );
    assert_eq!(out.worktrees.len(), 1);

    let expected_repo_root = std::fs::canonicalize(tmp.path())
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(out.worktrees[0].repo_path, expected_repo_root);
}

#[test]
fn w_ls_errors_on_invalid_max_concurrent_repos_env() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_repo(&repo);

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "ls",
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .env("W_MAX_CONCURRENT_REPOS", "0")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected failure, got: {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("W_MAX_CONCURRENT_REPOS"),
        "stderr did not mention W_MAX_CONCURRENT_REPOS:\n{stderr}"
    );
}

#[test]
fn w_ls_jobs_overrides_invalid_max_concurrent_repos_env() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_repo(&repo);

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "ls",
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--jobs",
            "1",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .env("W_MAX_CONCURRENT_REPOS", "0")
        .output()
        .unwrap();

    assert!(output.status.success(), "w ls failed: {output:?}");
}

#[test]
fn w_ls_errors_on_invalid_jobs() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_repo(&repo);

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "ls",
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--jobs",
            "0",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected failure, got: {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--jobs"),
        "stderr did not mention --jobs:\n{stderr}"
    );
}

#[test]
fn w_ls_honors_max_concurrent_repos_in_config_even_with_roots() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_repo(&repo);

    let config_path = tmp.path().join("w-config.toml");
    std::fs::write(
        &config_path,
        "repo_roots = []\nmax_depth = 2\nmax_concurrent_repos = 0\n",
    )
    .unwrap();

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "ls",
            "--config",
            config_path.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected failure, got: {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("max_concurrent_repos"),
        "stderr did not mention max_concurrent_repos:\n{stderr}"
    );
}

#[test]
fn w_ls_accepts_max_concurrent_repos_in_config() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_repo(&repo);

    let config_path = tmp.path().join("w-config.toml");
    std::fs::write(
        &config_path,
        format!(
            "repo_roots = [\"{}\"]\nmax_depth = 2\nmax_concurrent_repos = 2\n",
            root.display()
        ),
    )
    .unwrap();

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "ls",
            "--config",
            config_path.to_str().unwrap(),
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w ls failed: {output:?}");
}

#[test]
fn w_ls_text_preset_can_be_set_via_config() {
    let tmp = tempfile::tempdir().unwrap();

    let root = init_root_repo_with_feature_worktree(&tmp);

    let config_path = tmp.path().join("w-config.toml");
    std::fs::write(&config_path, "[ls]\npreset = \"full\"\n").unwrap();

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "ls",
            "--config",
            config_path.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "text",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w ls failed: {output:?}");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2, "expected 2 worktrees, got: {lines:?}");

    for line in lines {
        let cols = line.split('\t').collect::<Vec<_>>();
        assert_eq!(cols.len(), 5, "expected 5 columns for full preset");
    }
}

#[test]
fn w_ls_text_preset_flag_overrides_config() {
    let tmp = tempfile::tempdir().unwrap();

    let root = init_root_repo_with_feature_worktree(&tmp);

    let config_path = tmp.path().join("w-config.toml");
    std::fs::write(&config_path, "[ls]\npreset = \"full\"\n").unwrap();

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output = cargo_bin_cmd!("w")
        .args([
            "ls",
            "--config",
            config_path.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "text",
            "--preset",
            "compact",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w ls failed: {output:?}");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2, "expected 2 worktrees, got: {lines:?}");

    for line in lines {
        let cols = line.split('\t').collect::<Vec<_>>();
        assert_eq!(cols.len(), 2, "expected 2 columns for compact preset");
    }
}

#[test]
fn w_ls_sort_project_orders_by_project_identifier() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo_a = root.join("repo_a");
    let repo_b = root.join("repo_b");
    std::fs::create_dir_all(&repo_a).unwrap();
    std::fs::create_dir_all(&repo_b).unwrap();
    init_repo(&repo_a);
    init_repo(&repo_b);

    git(
        &repo_a,
        &["remote", "add", "origin", "https://github.com/z/repo"],
    );
    git(
        &repo_b,
        &["remote", "add", "origin", "https://github.com/a/repo"],
    );

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
            "ls",
            "--root",
            root.to_str().unwrap(),
            "--max-depth",
            "2",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "text",
            "--sort",
            "project",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "w ls failed: {output:?}");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 4, "expected 4 worktrees, got: {lines:?}");

    let project_ids = lines
        .iter()
        .map(|line| line.split('\t').next().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(project_ids[0], "github.com/a/repo");
    assert_eq!(project_ids[1], "github.com/a/repo");
    assert_eq!(project_ids[2], "github.com/z/repo");
    assert_eq!(project_ids[3], "github.com/z/repo");
}
