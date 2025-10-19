mod common;

use common::TestRepo;
use insta::Settings;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

/// Helper to create snapshot for init command
fn snapshot_init(test_name: &str, shell: &str, extra_args: &[&str]) {
    let repo = TestRepo::new();
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("arbor"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("init").arg(shell);

        for arg in extra_args {
            cmd.arg(arg);
        }

        cmd.current_dir(repo.root_path());

        assert_cmd_snapshot!(test_name, cmd);
    });
}

#[test]
fn test_init_bash() {
    snapshot_init("init_bash", "bash", &[]);
}

#[test]
fn test_init_fish() {
    snapshot_init("init_fish", "fish", &[]);
}

#[test]
fn test_init_zsh() {
    snapshot_init("init_zsh", "zsh", &[]);
}

#[test]
fn test_init_bash_custom_prefix() {
    snapshot_init("init_bash_custom_prefix", "bash", &["--cmd", "wt"]);
}

#[test]
fn test_init_bash_prompt_hook() {
    snapshot_init("init_bash_prompt_hook", "bash", &["--hook", "prompt"]);
}

#[test]
fn test_init_fish_prompt_hook() {
    snapshot_init("init_fish_prompt_hook", "fish", &["--hook", "prompt"]);
}

#[test]
fn test_init_bash_all_options() {
    snapshot_init(
        "init_bash_all_options",
        "bash",
        &["--cmd", "wt", "--hook", "prompt"],
    );
}

#[test]
fn test_init_invalid_shell() {
    let repo = TestRepo::new();
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("arbor"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("init")
            .arg("invalid-shell")
            .current_dir(repo.root_path());

        assert_cmd_snapshot!("init_invalid_shell", cmd);
    });
}

#[test]
fn test_init_invalid_hook() {
    let repo = TestRepo::new();
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("arbor"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("init")
            .arg("bash")
            .arg("--hook")
            .arg("invalid")
            .current_dir(repo.root_path());

        assert_cmd_snapshot!("init_invalid_hook", cmd);
    });
}
