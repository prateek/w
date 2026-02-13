use std::path::{Path, PathBuf};

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

#[derive(Debug, Deserialize)]
struct IndexOutput {
    schema_version: u32,
    repos: Vec<Repo>,
}

#[derive(Debug, Deserialize)]
struct Repo {
    path: String,
}

#[test]
fn w_repo_index_cold_then_cached() {
    let tmp = tempfile::tempdir().unwrap();

    let root1 = tmp.path().join("root1");
    let root2 = tmp.path().join("root2");
    std::fs::create_dir_all(&root1).unwrap();
    std::fs::create_dir_all(&root2).unwrap();

    let repo_a = root1.join("repo_a");
    let repo_b = root2.join("nested/repo_b");
    std::fs::create_dir_all(&repo_a).unwrap();
    std::fs::create_dir_all(&repo_b).unwrap();
    init_repo(&repo_a);
    init_repo(&repo_b);

    let config_good = tmp.path().join("w-config-good.toml");
    std::fs::write(
        &config_good,
        format!(
            "repo_roots = [\"{}\", \"{}\"]\nmax_depth = 3\n",
            root1.display(),
            root2.display()
        ),
    )
    .unwrap();

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output1 = cargo_bin_cmd!("w")
        .args([
            "repo",
            "index",
            "--config",
            config_good.to_str().unwrap(),
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output1.status.success(), "w repo index failed: {output1:?}");

    let index1: IndexOutput = serde_json::from_slice(&output1.stdout).unwrap();
    assert_eq!(index1.schema_version, 1);
    assert_eq!(index1.repos.len(), 2);

    let expected_a = std::fs::canonicalize(&repo_a)
        .unwrap()
        .to_string_lossy()
        .to_string();
    let expected_b = std::fs::canonicalize(&repo_b)
        .unwrap()
        .to_string_lossy()
        .to_string();
    let mut expected_paths = vec![expected_a.clone(), expected_b.clone()];
    expected_paths.sort();

    let mut actual_paths = index1
        .repos
        .iter()
        .map(|r| r.path.clone())
        .collect::<Vec<_>>();
    actual_paths.sort();
    assert_eq!(actual_paths, expected_paths);

    let config_bad = tmp.path().join("w-config-bad.toml");
    std::fs::write(&config_bad, "repo_roots = []\n").unwrap();

    let output2 = cargo_bin_cmd!("w")
        .args([
            "repo",
            "index",
            "--cached",
            "--config",
            config_bad.to_str().unwrap(),
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        output2.status.success(),
        "w repo index --cached failed: {output2:?}"
    );

    let index2: IndexOutput = serde_json::from_slice(&output2.stdout).unwrap();
    let mut actual_paths2 = index2
        .repos
        .iter()
        .map(|r| r.path.clone())
        .collect::<Vec<_>>();
    actual_paths2.sort();
    assert_eq!(actual_paths2, expected_paths);
}

#[test]
fn w_repo_pick_filter_uses_cache() {
    let tmp = tempfile::tempdir().unwrap();

    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();

    let repo_a = root.join("repo_a");
    let repo_b = root.join("repo_b");
    std::fs::create_dir_all(&repo_a).unwrap();
    std::fs::create_dir_all(&repo_b).unwrap();
    init_repo(&repo_a);
    init_repo(&repo_b);

    let config = tmp.path().join("w-config.toml");
    std::fs::write(
        &config,
        format!("repo_roots = [\"{}\"]\nmax_depth = 2\n", root.display()),
    )
    .unwrap();

    let cache_path = tmp.path().join("repo-index-cache.json");

    let output1 = cargo_bin_cmd!("w")
        .args([
            "repo",
            "index",
            "--config",
            config.to_str().unwrap(),
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output1.status.success(), "w repo index failed: {output1:?}");

    let output2 = cargo_bin_cmd!("w")
        .args([
            "repo",
            "pick",
            "--cached",
            "--cache-path",
            cache_path.to_str().unwrap(),
            "--filter",
            "repo_b",
        ])
        .output()
        .unwrap();
    assert!(output2.status.success(), "w repo pick failed: {output2:?}");

    let selected = String::from_utf8(output2.stdout).unwrap();
    let selected = PathBuf::from(selected.trim());

    assert_eq!(selected, std::fs::canonicalize(&repo_b).unwrap());
}
