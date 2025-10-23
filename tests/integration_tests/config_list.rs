use crate::common::TestRepo;
use insta::Settings;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Test `wt config list` with both global and project configs present
#[test]
fn test_config_list_with_project_config() {
    let repo = TestRepo::new();
    let temp_home = TempDir::new().unwrap();

    // Create fake global config (platform-specific path)
    let global_config_dir = if cfg!(target_os = "macos") {
        temp_home
            .path()
            .join("Library")
            .join("Application Support")
            .join("worktrunk")
    } else {
        temp_home.path().join(".config").join("worktrunk")
    };
    fs::create_dir_all(&global_config_dir).unwrap();
    fs::write(
        global_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"

[[approved-commands]]
project = "test-project"
command = "npm install"
"#,
    )
    .unwrap();

    // Create project config
    let config_dir = repo.root_path().join(".config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("wt.toml"),
        r#"post-create-command = "npm install"

[post-start-command]
server = "npm run dev"
"#,
    )
    .unwrap();

    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    // Filter out the actual config paths to use placeholders
    settings.add_filter(repo.root_path().to_str().unwrap(), "[REPO]");
    settings.add_filter(&temp_home.path().to_string_lossy(), "[TEMP_HOME]");
    // Normalize Windows paths to Unix style
    settings.add_filter(r"\\", "/");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("wt"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("config")
            .arg("list")
            .env("HOME", temp_home.path())
            .current_dir(repo.root_path());

        assert_cmd_snapshot!("config_list_with_project", cmd);
    });
}

/// Test `wt config list` when there is no project config
#[test]
fn test_config_list_no_project_config() {
    let repo = TestRepo::new();
    let temp_home = TempDir::new().unwrap();

    // Create fake global config (but no project config) - platform-specific path
    let global_config_dir = if cfg!(target_os = "macos") {
        temp_home
            .path()
            .join("Library")
            .join("Application Support")
            .join("worktrunk")
    } else {
        temp_home.path().join(".config").join("worktrunk")
    };
    fs::create_dir_all(&global_config_dir).unwrap();
    fs::write(
        global_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"
"#,
    )
    .unwrap();

    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    // Filter out the actual config paths to use placeholders
    settings.add_filter(repo.root_path().to_str().unwrap(), "[REPO]");
    settings.add_filter(&temp_home.path().to_string_lossy(), "[TEMP_HOME]");
    settings.add_filter(r"\\", "/");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("wt"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("config")
            .arg("list")
            .env("HOME", temp_home.path())
            .current_dir(repo.root_path());

        assert_cmd_snapshot!("config_list_no_project", cmd);
    });
}

/// Test `wt config list` outside a git repository
#[test]
fn test_config_list_outside_git_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_home = TempDir::new().unwrap();

    // Create fake global config - platform-specific path
    let global_config_dir = if cfg!(target_os = "macos") {
        temp_home
            .path()
            .join("Library")
            .join("Application Support")
            .join("worktrunk")
    } else {
        temp_home.path().join(".config").join("worktrunk")
    };
    fs::create_dir_all(&global_config_dir).unwrap();
    fs::write(
        global_config_dir.join("config.toml"),
        r#"worktree-path = "../{repo}.{branch}"
"#,
    )
    .unwrap();

    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    // Filter out the actual config paths
    settings.add_filter(&temp_home.path().to_string_lossy(), "[TEMP_HOME]");
    settings.add_filter(r"\\", "/");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("wt"));
        cmd.arg("config")
            .arg("list")
            .env("HOME", temp_home.path())
            .current_dir(temp_dir.path());

        assert_cmd_snapshot!("config_list_outside_repo", cmd);
    });
}
