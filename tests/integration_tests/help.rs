//! Snapshot tests for top-level `--help` output.
//!
//! These ensure our compact help formatting stays stable across releases and
//! catches accidental regressions in wording or wrapping.

use crate::common::wt_command;
use insta::Settings;
use insta_cmd::assert_cmd_snapshot;

fn snapshot_help(test_name: &str, args: &[&str]) {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");
    settings.bind(|| {
        let mut cmd = wt_command();
        cmd.args(args);
        assert_cmd_snapshot!(test_name, cmd);
    });
}

#[test]
fn help_root() {
    snapshot_help("help_root", &["--help"]);
}

#[test]
fn help_init() {
    snapshot_help("help_init", &["init", "--help"]);
}

#[test]
fn help_config() {
    snapshot_help("help_config", &["config", "--help"]);
}

#[test]
fn help_beta() {
    snapshot_help("help_beta", &["beta", "--help"]);
}

#[test]
fn help_list() {
    snapshot_help("help_list", &["list", "--help"]);
}

#[test]
fn help_switch() {
    snapshot_help("help_switch", &["switch", "--help"]);
}

#[test]
fn help_remove() {
    snapshot_help("help_remove", &["remove", "--help"]);
}

#[test]
fn help_merge() {
    snapshot_help("help_merge", &["merge", "--help"]);
}
