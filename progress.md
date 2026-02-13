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
