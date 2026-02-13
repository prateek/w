use assert_cmd::cargo::cargo_bin_cmd;

fn shell_init(shell: &str) -> (i32, String, String) {
    let output = cargo_bin_cmd!("w")
        .args(["shell", "init", shell])
        .output()
        .unwrap_or_else(|e| panic!("failed to run `w shell init {shell}`: {e}"));

    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

#[test]
fn w_shell_init_zsh_prints_snippet() {
    let (code, stdout, stderr) = shell_init("zsh");
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(!stdout.is_empty());
    assert!(!stdout.contains("TODO"));
    assert!(stdout.contains("eval \"$(w shell init zsh)\""));
    assert!(stdout.contains("w() {"));
    assert!(stdout.contains("command w"));
    assert!(stdout.contains("--print"));
}

#[test]
fn w_shell_init_bash_prints_snippet() {
    let (code, stdout, stderr) = shell_init("bash");
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(!stdout.is_empty());
    assert!(!stdout.contains("TODO"));
    assert!(stdout.contains("eval \"$(w shell init bash)\""));
    assert!(stdout.contains("w() {"));
    assert!(stdout.contains("command w"));
    assert!(stdout.contains("--print"));
}

#[test]
fn w_shell_init_fish_prints_snippet() {
    let (code, stdout, stderr) = shell_init("fish");
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(!stdout.is_empty());
    assert!(!stdout.contains("TODO"));
    assert!(stdout.contains("w shell init fish | source"));
    assert!(stdout.contains("function w"));
    assert!(stdout.contains("command w"));
    assert!(stdout.contains("--print"));
}

#[test]
fn w_shell_init_pwsh_prints_snippet() {
    let (code, stdout, stderr) = shell_init("pwsh");
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(!stdout.is_empty());
    assert!(!stdout.contains("TODO"));
    assert!(stdout.contains("Invoke-Expression (& w shell init pwsh)"));
    assert!(stdout.contains("Get-Command w -CommandType Application"));
    assert!(stdout.contains("Set-Location"));
    assert!(stdout.contains("--print"));
}
