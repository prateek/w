2026-02-12 22:56: initial setup
2026-02-12 23:29: bootstrap Rust workspace + minimal `w` CLI + CI
2026-02-13 00:01: vendor Worktrunk v0.23.2 (git subtree) + CI checks for vendored Worktrunk
2026-02-13 00:10: add `NOTICE` + `CONTRIBUTING.md`
2026-02-13 00:25: scaffold Zola docs site + CI build + GitHub Pages deploy workflow
2026-02-13 00:40: add Homebrew tap scaffolding (`Formula/w.rb`) + Homebrew install docs
2026-02-13 00:55: add minimal asciinema demo (README + docs)
2026-02-13 01:26: M1: expose credential-safe `project_identifier` to worktree-path + hook templates (vendored Worktrunk) + tests; redact creds in `remote_url` hook var
2026-02-13 02:04: M2: add `worktrunk::integration::v1` (switch/remove/compute_worktree_path) + unit tests + remote-only branch handling
2026-02-13 02:26: M3 (partial): implement `w new` via `worktrunk::integration::v1` + integration test; fix Cargo workspace exclude for vendored dependency
2026-02-13 02:39: M3 (partial): implement `w cd` (existing branch worktree switch; no branch creation) + integration tests
2026-02-13 02:58: M3 (partial): implement `w run` (switch/create then execute in the worktree) + integration test
2026-02-13 03:20: M3 (partial): implement `w rm` (safe worktree removal + `--force`) + integration tests
2026-02-13 03:37: M3 (partial): implement `w prune` (remove stale worktree directories) + integration test
2026-02-13 03:55: M3: implement `w shell init` for zsh/bash/fish/pwsh + smoke tests
2026-02-13 04:18: M4: add `w repo index` (JSON/TSV + cache) + `w repo pick` (skim/--filter) + global `-C/--repo` + integration tests
2026-02-13 04:41: M5 (partial): add `w ls` (cross-repo worktree listing) with stable JSON/TSV output + integration tests
2026-02-13 04:59: M5 (partial): add `w switch` (cross-repo picker via `skim` or `--filter`) + shell integration support + integration tests
2026-02-13 05:21: M5: add bounded cross-repo concurrency for `w ls`/`w switch` (config + `W_MAX_CONCURRENT_REPOS`) + tests
2026-02-13 05:41: M6: add `schema_version` to `wt list --format=json` output + schema regression test + docs
2026-02-13 05:54: M7 (partial): add tag-driven GitHub Actions release workflow (build/publish `w` + vendored `wt` artifacts + `.sha256` checksums)
2026-02-13 06:16: M7 (partial): auto-update Homebrew formula on tagged releases + build macOS x86_64 + arm64 release artifacts
2026-02-13 06:30: docs: add Install + Quickstart pages to the Zola site + add nav links
2026-02-13 06:47: docs: add `Commands` page (CLI reference) + add nav link
2026-02-13 06:59: docs: add `LLMs / Codex` page + link it from the docs nav and README
2026-02-13 07:19: add `--jobs` override for `w ls`/`w switch` cross-repo concurrency + docs + tests
2026-02-13 07:37: docs: add `How it works` page (identity/layout/concurrency) + link it from the docs nav
