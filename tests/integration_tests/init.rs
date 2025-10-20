use crate::common::TestRepo;
use insta::Settings;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use rstest::rstest;
use std::process::Command;

/// Helper to create snapshot for init command
fn snapshot_init(test_name: &str, shell: &str, extra_args: &[&str]) {
    let repo = TestRepo::new();
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("../snapshots");

    settings.bind(|| {
        let mut cmd = Command::new(get_cargo_bin("wt"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("init").arg(shell);

        for arg in extra_args {
            cmd.arg(arg);
        }

        cmd.current_dir(repo.root_path());

        assert_cmd_snapshot!(test_name, cmd);
    });
}

#[rstest]
// Tier 1: Shells available in standard Ubuntu repos
#[case("bash")]
#[case("fish")]
#[case("zsh")]
// Tier 2: Shells requiring extra setup
#[cfg_attr(feature = "tier-2-integration-tests", case("elvish"))]
#[cfg_attr(feature = "tier-2-integration-tests", case("nushell"))]
#[cfg_attr(feature = "tier-2-integration-tests", case("oil"))]
#[cfg_attr(feature = "tier-2-integration-tests", case("powershell"))]
#[cfg_attr(feature = "tier-2-integration-tests", case("xonsh"))]
fn test_init(#[case] shell: &str) {
    snapshot_init(&format!("init_{}", shell), shell, &[]);
}

#[test]
fn test_init_bash_custom_prefix() {
    snapshot_init("init_bash_custom_prefix", "bash", &["--cmd", "wt"]);
}

#[rstest]
#[case("bash")]
#[case("fish")]
fn test_init_prompt_hook(#[case] shell: &str) {
    snapshot_init(
        &format!("init_{}_prompt_hook", shell),
        shell,
        &["--hook", "prompt"],
    );
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
        let mut cmd = Command::new(get_cargo_bin("wt"));
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
        let mut cmd = Command::new(get_cargo_bin("wt"));
        repo.clean_cli_env(&mut cmd);
        cmd.arg("init")
            .arg("bash")
            .arg("--hook")
            .arg("invalid")
            .current_dir(repo.root_path());

        assert_cmd_snapshot!("init_invalid_hook", cmd);
    });
}
