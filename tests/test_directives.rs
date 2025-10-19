mod common;

use common::TestRepo;
use insta::Settings;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

/// Test the directive protocol for switch command
#[test]
fn test_switch_internal_directive() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    // Normalize the directive path output
    settings.add_filter(r"__ARBOR_CD__[^\n]+", "__ARBOR_CD__[PATH]");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("arbor"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("switch")
            .arg("my-feature")
            .arg("--internal")
            .current_dir(repo.root_path());

        assert_cmd_snapshot!("switch_internal_directive", cmd);
    });
}

/// Test switch without internal flag (should show help message)
#[test]
fn test_switch_without_internal() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("arbor"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("switch")
            .arg("my-feature")
            .current_dir(repo.root_path());

        assert_cmd_snapshot!("switch_without_internal", cmd);
    });
}

/// Test finish command with internal flag
#[test]
fn test_finish_internal_directive() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    // Normalize the directive path output
    settings.add_filter(r"__ARBOR_CD__[^\n]+", "__ARBOR_CD__[PATH]");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("arbor"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("finish")
            .arg("--internal")
            .current_dir(repo.root_path());

        assert_cmd_snapshot!("finish_internal_directive", cmd);
    });
}

/// Test finish without internal flag
#[test]
fn test_finish_without_internal() {
    let repo = TestRepo::new();
    repo.commit("Initial commit");

    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("arbor"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("finish").current_dir(repo.root_path());

        assert_cmd_snapshot!("finish_without_internal", cmd);
    });
}
