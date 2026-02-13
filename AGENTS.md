# Working in this repo

Source of truth for the roadmap: `PRD.md`.

## Quick commands

```bash
# w
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace

# vendored worktrunk (run unit tests; integration snapshots are brittle in subtree)
cargo fmt --check --manifest-path vendor/worktrunk/Cargo.toml
cargo clippy --manifest-path vendor/worktrunk/Cargo.toml --workspace -- -D warnings
cargo test --manifest-path vendor/worktrunk/Cargo.toml --workspace --lib --bins

# docs (zola)
cd docs && zola build
cd docs && zola serve
# if `zola` isn't installed locally, use Docker:
cd docs && docker run --rm -v "$(pwd)":/app -w /app ghcr.io/getzola/zola:v0.19.2 build
cd docs && docker run --rm -p 1111:1111 -v "$(pwd)":/app -w /app ghcr.io/getzola/zola:v0.19.2 serve --interface 0.0.0.0 --port 1111
```

## Notes

- Prefer small, reviewable diffs.
- Keep vendored upstream code changes upstreamable under `vendor/worktrunk/`.
- `w ls`/`w switch` use bounded cross-repo concurrency; configure via `--jobs <n>`, `max_concurrent_repos` in `~/.config/w/config.toml`, or `W_MAX_CONCURRENT_REPOS` (cap: 32).
- `w ls` text output supports presets (`--preset` / `[ls].preset`) and configurable sorting (`--sort` / `[ls].sort`).

## Gotchas (from iteration logs)

- `cargo new` creates a nested git repo by default; prefer `cargo new --vcs none ...` for new crates.
- If `cargo fmt`/`cargo metadata` fails with "multiple workspace roots found" after adding a path dependency on `vendor/worktrunk`, add `exclude = ["vendor/worktrunk"]` to the root workspace `Cargo.toml`.
- Homebrew tap naming: `brew tap prateek/w` assumes a `prateek/homebrew-w` repo; this tap lives in `prateek/w`, so use `brew tap prateek/w https://github.com/prateek/w`.
- Homebrew may fail with `Operation not permitted` in restricted environments due to auto-update/lock writes; try `HOMEBREW_NO_AUTO_UPDATE=1 brew ...` (or avoid running `brew` during `codex review`).
- `skim` currently doesn’t build on Windows (via `skim-tuikit`/`nix`); keep interactive pickers behind `cfg(not(windows))` and require `--filter` on Windows.
- Windows-only `#[cfg(windows)]` branches can hide Clippy lints locally; CI runs `cargo clippy -- -D warnings`, so avoid patterns like nested `if`s that trigger `clippy::collapsible_if` in Windows-only code.
- When embedding Windows paths in TOML, prefer literal strings (`'C:\path'`) or escape backslashes (`C:\\path`) to avoid parse errors like `invalid unicode 8-digit hex code` from sequences such as `\U`.
- On Windows, keep canonicalization consistent (`dunce::canonicalize` vs `std::fs::canonicalize`) to avoid `\\?\`-prefix mismatches breaking path comparisons (e.g., prune skipping stale dirs).
- PAL `codereview` currently errors unless you pass `--relevant-files` (and often `--files-checked`) as comma-separated **absolute** paths.
- When passing PAL prompts via `bash .../pal ... --step "..."`, avoid backticks (`` `...` ``) in the shell string; they trigger command substitution. Prefer single quotes or escape backticks.
- PAL `codereview` via `pal-mcporter` may return JSON (even with `-o markdown`) and sometimes produces empty/low-signal findings; treat as best-effort and do a quick manual review too.
- If `codex review` tries to run `pal-mcporter`, it may fail with uv cache permission errors; run PAL reviews from your normal shell (or set a writable cache dir like `UV_CACHE_DIR`).
- PAL tool calls may time out; increase timeouts with `PAL_MCPORTER_TIMEOUT_MS` or `pal ... -t <ms>`.
- PAL `codereview` external validation may time out or model-mismatch; `--model o4-mini --thinking-mode minimal` has worked.
- PAL `continuation_id` flows generally don’t resume across separate `pal-mcporter` invocations (fresh server per call).
- `cargo clippy -- -D warnings` will fail on `clippy::too_many_arguments`; bundle CLI args into an options struct instead of adding `#[allow]` everywhere.
- `codex review` may spam opentelemetry export errors to `http://localhost:14318/v1/logs`; if it hangs, kill the spawned `codex` process and proceed with manual review.
- `codex review` runs in a read-only sandbox; if it tries to run `cargo test` and fails with `.cargo-lock`/`Operation not permitted`, ignore it and run tests from your normal shell.
- `codex` may warn that `[features].web_search_request` is deprecated; fix by setting `web_search` in `~/.codex/config.toml` (or ignore the warning).
- `codex review --uncommitted` currently errors if you pass a custom prompt; run it without a prompt (or use `--base`/`--commit`).
- `codex review --base <ref>` relies on `git diff <ref>` and won’t include untracked files; `git add` (or `git add -N`) new files before reviewing.
- `rg` treats patterns starting with `--` as flags; use `rg -- \"--refresh\" ...` when searching for CLI flags.
- To kill a hung `codex review`: `ps -eo pid,command | rg "codex .* review" | head` then `kill <pid>`.
- `codex review`: `--uncommitted` is mutually exclusive with `--base` (use one or the other).
- GitHub Actions jobs that commit+push back to the repo should use `actions/checkout` with `fetch-depth: 0` to avoid shallow clone push failures.
- GitHub Actions runner availability changes over time; if `macos-13` is unavailable, build macOS x86_64 + arm64 artifacts on `macos-latest` via explicit `--target` builds.
- Some environments block destructive shell commands (e.g. `rm -rf`); prefer adding the right ignores (e.g. `__pycache__/`) and keep diffs clean without relying on cleanup commands.
- Shell integration captures `w` stdout for `cd`-like commands; interactive `skim` pickers should not require stdout being a TTY (prefer checking stdin TTY / using `/dev/tty`) so commands like `w switch` and `w repo pick` work under command substitution.
- The docs demo cast at `docs/static/demos/w-basic.cast` is a plain asciinema v2 file; keep it in sync with `w --help` and `w shell init zsh` output (it uses `\r\n` line endings inside JSON strings).
- `docs` build via the Zola Docker image may warn about an amd64/arm64 platform mismatch; it’s safe to ignore, or add `--platform linux/amd64` to the `docker run` command.
- This environment may set `NO_COLOR=1`; Worktrunk snapshot tests expect ANSI output, so run with `NO_COLOR= CLICOLOR_FORCE=1` if you need to execute them.
- When adding `insta` snapshot tests under `vendor/worktrunk/`, generate and commit the `.snap` files (e.g., `INSTA_UPDATE=always cargo test --manifest-path vendor/worktrunk/Cargo.toml --workspace --lib --bins`). Note: `insta` is configured with `yaml` snapshots; prefer `assert_yaml_snapshot!` unless you add the `json` feature.
