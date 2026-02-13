# PRD: `github.com/prateek/w`

Status: Draft  
Audience: OSS contributors and upstream Worktrunk maintainers (credit-forward, upstreamable)  

## 0) TL;DR

Build a downstream repository (`github.com/prateek/w`) that:

- Vendors upstream Worktrunk under `vendor/worktrunk/` using `git subtree`.
- Builds and ships two CLIs from one repo:
  - `wt`: Worktrunk CLI (kept as close to upstream as possible; changes are upstreamable).
  - `w`: a native Rust multi-repo wrapper with parity to the existing dotfiles `w` UX plus extra multi-repo features.
- Adds a *minimal* Worktrunk integration surface for wrappers (not a multi-repo mode in Worktrunk).
- Ships great docs, including GitHub Pages styled like Worktrunk’s docs, and gives explicit credit.
- Makes installation effortless (GitHub Releases + Homebrew tap hosted in this repo).

Primary value: fast, reliable multi-repo worktree workflows, while keeping Worktrunk’s single-repo focus intact.

## 1) Motivation / Problem

Worktrunk is excellent inside one repo (switch/create/remove semantics, hooks, approvals, default-branch logic, etc.). Multi-repo workflows still need a wrapper that can:

- choose a repo and operate across many repos
- provide a centralized, collision-safe worktree root layout
- list/switch/remove across many repos
- do this fast, without shell parsing/subprocess glue and without re-implementing Worktrunk semantics

Today, wrappers tend to:

- re-derive “project identity” by parsing remote URLs (brittle, duplicated logic)
- shell out to `wt` for core operations or reach into unstable internals
- accidentally saturate disk by fanning out git ops across many repos

## 2) Goals

### 2.1 Repository + Ops

- Vendor upstream Worktrunk code via `git subtree` so it’s easy to:
  - sync upstream changes forward
  - split and upstream our patches back to Worktrunk
- Set up CI from day one for macOS, Linux, Windows.
- After every commit/PR: CI validates everything (build, tests, lint, docs).

### 2.2 Worktrunk (vendored) goals

- Keep `wt` CLI behavior effectively identical to upstream (additive/backward-compatible only).
- Implement focused improvements needed for wrappers:
  - expose canonical project identity (`project_identifier`) to templates
  - publish a minimal, versioned integration API surface (`worktrunk::integration::v1`)
  - stabilize/version `wt list --format json` contract for external tooling

### 2.3 `w` wrapper goals

Parity with dotfiles `w` plus additional multi-repo capabilities:

- Behavioral source of truth: the existing dotfiles `w` implementation (treated as the parity spec; referenced from `AGENTS.md` and the docs).
- Dotfiles parity:
  - `w new`, `w cd`, `w run`
  - `w ls`
  - `w switch`
  - `w rm` and `w prune`
- Extra v1 features:
  - built-in repo discovery/index (no external `repo-index` dependency required)
  - user-configurable formatting (columns/presets) and output formats
  - cross-repo concurrency controls (don’t saturate disk)
  - leverage `worktrunk::integration::v1` (no shelling out for core operations)
  - docs site on GitHub Pages with Worktrunk-like styling and strong attribution
  - release automation that ships `w` and a pinned `wt` build together

## 3) Non-Goals

- Making Worktrunk itself a “multi-repo mode” tool.
- Stabilizing all of Worktrunk’s internal command modules.
- Perfect Windows UX parity for every interactive behavior in v1 (Windows is supported from day one, but some UX may be “good enough” initially).

## 4) Principles

- **Upstream-first**: every change under `vendor/worktrunk/` should be written as if it will be upstreamed.
- **Small diffs**: avoid refactors/renames/format-only churn in vendored code.
- **Value each milestone**: every milestone should ship something useful (even if small).
- **TDD / integration-tests-heavy**: add integration tests for behavior before (or alongside) changes.
- **Credit and openness**: obvious attribution to Worktrunk, and open to upstream adopting parts or all of `w`.
- **Sane defaults, composable parts**: curated end-to-end UX, but every component is usable standalone (JSON output, index command, integration helpers).
- **Proof of concept**: this starts as a one-developer exploration. Contributions are welcome, but we should be explicit that this repo is experimental.
- **Codex-friendly**: keep workflows, docs, and tests optimized for developers using Codex (and similar LLM coding tools) without relying on any internal/private tooling.

## 5) Repository Strategy and Logistics

### 5.1 Structure

- `vendor/worktrunk/`: upstream Worktrunk (via `git subtree`)
- `crates/w/`: native Rust `w` wrapper (our code)
- `docs/`: GitHub Pages site for `w` (Zola, borrowing style patterns from Worktrunk docs)
- Root-level docs and contribution files (`README.md`, `AGENTS.md`, `NOTICE`, etc.)

### 5.2 `git subtree` workflow (sync + upstreaming)

We want two operations to be boring:

1) **Sync upstream Worktrunk into our repo**
- `git subtree pull --prefix vendor/worktrunk <upstream-url> <ref>`

2) **Upstream our Worktrunk patches**
- Keep Worktrunk patches in clean commits that touch only `vendor/worktrunk/`.
- Create an upstreamable branch:
  - `git subtree split --prefix vendor/worktrunk -b worktrunk-upstreamable`
- Open PR from that branch against upstream Worktrunk.

Docs:

- Add `vendor/worktrunk/UPSTREAM.md` describing:
  - what commit is vendored
  - how to pull updates
  - how to split and open PRs upstream

## 6) Worktrunk Changes (Minimal Integration Surface)

Everything in this section is implemented in `vendor/worktrunk/` and designed to be upstreamed.

### 6.1 Template variables: expose canonical project identity

Expose `project_identifier` to:

- `worktree-path` templates
- hook template context (and `--execute` template expansion)

Contract:

- Derived from `Repository::project_identifier()` (same value Worktrunk already uses for approvals / per-project config lookup).
- Credential-safe (must not leak tokens/userinfo from URLs).
- Suitable for path names when combined with existing `sanitize` filter.

Notes:

- The string format is whatever Worktrunk considers canonical today (typically `host/namespace/repo`, including GitLab nested groups; otherwise fallback to canonical path). `w` will treat this as an opaque identifier and rely on template filters like `sanitize` for filesystem safety.

### 6.2 Versioned integration API: `worktrunk::integration::v1`

Add a narrow, explicit API intended for wrappers, e.g.:

- `switch(repo, cfg, req) -> SwitchOutcome { path, branch, created, ... }`
- `remove(repo, cfg, req) -> RemoveOutcome { removed_path, deleted_branch, ... }`
- `compute_worktree_path(repo, branch, cfg) -> PathBuf`
- (optional) list collector returning a model with granular task selection

Constraints:

- data-oriented return types (no printing, no shell directive file assumptions)
- avoid binding wrappers to internal CLI rendering modules
- explicitly document this as **experimental** in this downstream repo (no compatibility guarantees yet)
- if/when upstream adopts, upstream can decide what stability guarantees to make

### 6.3 JSON contract stability for `wt list --format json`

Define and document a stable contract for JSON output used by external tools:

- Add `schema_version` (or equivalent) in the JSON payload.
- Commit to backward compatibility rules (e.g., “additive fields only in v1”).
- Add test coverage to prevent accidental schema breaks.

Important note:

- This downstream repo does **not** promise long-term API stability. Versioning is primarily to make iterative development safe and explicit, and to make upstreaming easier if desired.

### 6.4 Concurrency knobs (supported, documented)

Wrapper-driven workflows can amplify IO. Make concurrency controls explicit and supported:

- Document `WORKTRUNK_MAX_CONCURRENT_COMMANDS` (command semaphore).
- Document thread-pool behavior and supported overrides (e.g., `RAYON_NUM_THREADS`).
- (Optional) add config/env support that is friendlier for integrations.

## 7) `w` Wrapper Product Design

### 7.1 Repo discovery / index (built-in)

Provide a built-in index so users do not need an external `repo-index` binary:

- Config: list of root directories to scan (cross-platform).
- Cache: persist index for speed; incremental refresh.
- Output formats: JSON/TSV for scripting and debugging.
- UX: reuse index for `w` picker and cross-repo list.

Config specifics (v1):

- `w` reads `~/.config/w/config.toml` (plus env overrides) for repo roots and defaults.
- `w` reads Worktrunk’s config (`~/.config/worktrunk/config.toml`) for single-repo semantics (hooks, worktree-path template, approvals) and does not invent parallel settings for those.

### 7.2 Centralized layout

Default centralized root: configurable (e.g., `~/code/wt`).

Expected path template uses Worktrunk identity:

- `{{ project_identifier }}` namespaces across hosts/owners and nested groups
- branch is sanitized as already supported

### 7.3 Commands

Parity set + extensions:

- `w new <branch>`: create/switch in selected repo into centralized root
- `w cd <branch>`: switch + cd
- `w run <branch> -- <cmd...>`: switch/create then execute command in worktree
- `w ls`: cross-repo list (fast by default; expensive computations opt-in)
- `w switch`: cross-repo picker (bundle `skim`, no `fzf` requirement)
- `w rm`: remove worktree safely (dirty prompts, force semantics)
- `w prune`: remove stale directories under centralized root

### 7.4 Formatting customization

`w` should own formatting, not Worktrunk:

- column sets/presets (e.g., compact/default/full)
- stable machine formats (TSV/JSON)
- deterministic sorting rules (configurable)

### 7.5 Cross-repo concurrency control

`w` must cap fan-out across repos:

- global semaphore for per-repo jobs
- per-command overrides (`--jobs`, config, env)
- sensible defaults (avoid disk saturation)

### 7.6 Shell integration (`w cd`)

Because a binary cannot change the parent shell’s directory, `w cd` requires shell integration (same underlying constraint Worktrunk already documents for `wt`).

Plan:

- `w shell init <shell>` prints an init snippet for zsh/bash/fish/pwsh that wraps the `w` binary and applies directory-change directives.
- As a fallback for scripting, support `w cd <branch> --print` (prints the resolved path) for users who do not want shell integration.

This keeps the “user-facing” behavior aligned with the dotfiles `w` UX while remaining cross-platform.
## 8) Documentation (Repo + GitHub Pages)

### 8.1 Repo docs

Required root docs:

- `README.md`:
  - what `w` is
  - what `wt` is (Worktrunk) and how it’s vendored
  - installation
  - quickstart examples
  - credit and license clarity
  - disclosure: code is predominantly generated using LLMs with human supervision
  - a clear “experimental / no API guarantees” statement
- `AGENTS.md`:
  - concise “how to work in this repo” guidance (Rust library idioms)
  - how `w` relates to Worktrunk single-repo concerns
  - pointers to the PRD and the dotfiles behavioral spec
  - Codex-first dev workflow (“how to use Codex and other LLM assistants to work on this repo responsibly”; no internal/private dependencies)
- `NOTICE` / attribution (explicit credit to Worktrunk authors)
- `CONTRIBUTING.md` (including subtree sync/upstream instructions)

### 8.2 GitHub Pages site

Use Zola (matching Worktrunk’s docs tooling) to create:

- installation docs for `w`
- command reference for `w` (with parity notes vs dotfiles and vs `wt`)
- “How it works” pages (identity, centralized layout, concurrency)
- attribution page linking upstream Worktrunk docs and repo

Deploy via GitHub Actions to GitHub Pages on `main`.

### 8.3 Demos

- Capture demos via `asciinema` (or similar) and embed in README + docs.

## 9) Testing Strategy (TDD-first, integration-heavy)

### 9.1 Worktrunk (vendored)

- Add targeted unit/integration tests for:
  - template context includes `project_identifier`
  - JSON schema version + stability checks
  - `integration::v1` contracts (smoke tests)
- Keep tests minimal and upstream-friendly.

### 9.2 `w` wrapper

Primary confidence comes from integration tests:

- Use temp directories + real git repos:
  - create a “canonical” repo with remote URL fixtures
  - create worktrees under a temp centralized root
  - validate behaviors for `new/cd/run/ls/switch/rm/prune`
- Snapshot only where stable (e.g., TSV/JSON output); avoid brittle ANSI snapshots.
- Cross-platform CI verifies the same suite runs on macOS/Linux/Windows.

## 10) CI / Automation (Day One)

GitHub Actions:

- Matrix: macOS, Ubuntu, Windows
- Jobs (all required on PR):
  - Build + test Worktrunk: `cargo test --manifest-path vendor/worktrunk/Cargo.toml`
  - Build + test `w`: `cargo test --manifest-path crates/w/Cargo.toml`
  - Lint/format:
    - `cargo fmt --check` (both)
    - `cargo clippy` (both)
  - Docs:
    - build Zola site
    - (on main) deploy to Pages
  - Release workflow:
    - build artifacts for `wt` + `w`
    - attach to GitHub release

Branch protection:

- Require CI to pass before merge (repo setting; manual setup required).

## 10.1 Packaging / Installation (Day One mindset)

We want “install and go” to be excellent. Target install methods:

- Homebrew tap hosted in this repo:
  - `brew tap prateek/w`
  - `brew install w` (and optionally `wt` if we decide to publish both as formulae)
- GitHub Releases:
  - attach prebuilt artifacts for macOS/Linux/Windows (both `w` and `wt`)
  - publish checksums
- `cargo install --git` as a developer escape hatch (not the primary story)

This is a downstream proof of concept, but packaging should still feel curated.

## 11) Milestones (Value Along The Way)

Each milestone should be a mergeable unit with:

- tests first (or in the same PR)
- docs updated
- CI green on all platforms

### M0: Repo Bootstrap (first milestone)

Deliverables:

- `github.com/prateek/w` initialized.
- Worktrunk vendored via `git subtree` into `vendor/worktrunk/`.
- Root `README.md`, `AGENTS.md`, `NOTICE`, `CONTRIBUTING.md` (credit-forward, experimental disclaimer, LLM disclosure).
- GitHub Actions set up for macOS/Linux/Windows; required checks enabled.
- Docs scaffold for GitHub Pages (Zola) deployed with placeholder content.
- Homebrew tap scaffolding in-repo (Formula directory + placeholder formula wiring).
- A basic demo (asciinema) embedded in README, even if the binary is minimal.

Acceptance criteria:

- `wt` builds from `vendor/worktrunk/` on all platforms in CI.
- `w` builds (even if minimal “help only”) on all platforms in CI.
- `main` is protected by required checks.

Progress (as of 2026-02-13):

- ✅ Bootstrapped Rust workspace with a minimal `w` CLI (`crates/w/`) and local checks (`cargo fmt/clippy/test`).
- ✅ Added GitHub Actions CI to run fmt/clippy/tests for `w` on macOS/Linux/Windows.
- ✅ Added initial root docs: `README.md` and `AGENTS.md`.
- ✅ Added root docs: `NOTICE` and `CONTRIBUTING.md`.
- ✅ Vendored upstream Worktrunk under `vendor/worktrunk/` via `git subtree` (currently `v0.23.2`) and added `vendor/worktrunk/UPSTREAM.md`.
- ✅ CI now runs fmt/clippy + unit tests for vendored Worktrunk (in addition to `w` checks).
- ⏳ Remaining for M0: scaffold docs site + Homebrew tap, and add a basic demo.

### M1: Project Identity in Templates (Worktrunk patch + tests)

Deliverables:

- `project_identifier` available in:
  - worktree-path templates
  - hook template context
- Tests in vendored Worktrunk proving:
  - variable exists
  - it’s credential-safe

User value:

- centralized layout can be configured without wrapper slug parsing.

Acceptance criteria:

- Worktrunk tests cover `project_identifier` in both worktree-path and hook contexts.
- A regression test demonstrates credential/userinfo is not present in the value.
- `w` can set a centralized `worktree-path` template that does not require repo slug parsing.

### M2: `worktrunk::integration::v1` (Switch/Remove/Path)

Deliverables:

- Minimal integration API for wrappers:
  - switch / remove / compute_worktree_path
- Tests for API behavior and backwards-compat expectations.

User value:

- enables wrappers to call per-repo semantics natively (no subprocess, no scraping).

Acceptance criteria:

- `worktrunk::integration::v1` APIs are documented as experimental (no compatibility guarantees yet).
- Smoke tests validate:
  - switch existing branch returns path
  - switch create returns created=true and correct path
  - remove returns removed path and branch deletion mode outcome
- `w` can depend on these APIs without importing Worktrunk CLI rendering modules.

### M3: `w` Core Parity (new/cd/run/rm/prune) using integration API

Deliverables:

- Implement dotfiles parity commands in native Rust:
  - `w new`, `w cd`, `w run`, `w rm`, `w prune`
- Integration tests matching dotfiles behavioral spec.

User value:

- replaces shell wrapper for core workflows with faster, more reliable native tool.

Acceptance criteria:

- Integration tests cover:
  - create-vs-switch decision logic
  - `w run` executes in the worktree
  - safe refusal on dirty rm without confirmation + success with confirmation/force
  - prune only removes stale directories
- `w shell init <shell>` works for at least zsh/bash/fish/pwsh in CI (smoke-level).

### M4: Built-in Repo Index + Picker (`skim`)

Deliverables:

- `w repo index` (or equivalent) + cached index file
- Cross-platform interactive picker using `skim`
- Tests for discovery on all platforms

User value:

- multi-repo workflows no longer depend on external scripts/binaries.

Acceptance criteria:

- `w repo index` produces deterministic JSON/TSV output.
- `w` commands can select a repo without external dependencies.
- Index cache is exercised in tests (cold + warm path).

### M5: `w ls` and `w switch` (multi-repo UX) + formatting config

Deliverables:

- `w ls`:
  - fast default output
  - opt-in expensive computations
  - stable TSV/JSON
- formatting config (columns/presets/sorts)
- cross-repo concurrency config and defaults
- integration tests for output contracts and safety

User value:

- “how are all my workstreams going” across repos, without melting disk.

Acceptance criteria:

- `w ls --format json|tsv` is stable and covered by snapshot tests.
- Defaults are “fast”: tests enforce that expensive computations are opt-in.
- Cross-repo concurrency defaults are bounded and configurable; tests cover the knob wiring.

### M6: Stable `wt list --format json` Contract + Docs

Deliverables:

- `schema_version` (or equivalent) in JSON output
- regression tests preventing schema breaks
- docs describing contract and compatibility rules

User value:

- non-Rust tooling can safely integrate too.

Acceptance criteria:

- JSON output includes `schema_version`.
- CI includes a schema regression test (golden fixture or JSON schema validation).
- Docs specify compatibility rules (additive-only in v1, version bump rules, etc.).

### M7: Release Automation + Polished Docs Site

Deliverables:

- GitHub release pipeline producing `wt` and `w` binaries for all platforms
- Homebrew formulae updated automatically on release (tap hosted in-repo)
- GitHub Pages site filled out (Worktrunk-like styling), including:
  - quickstarts
  - command reference
  - design rationale
  - attribution/credit
  - Codex-driven workflows + “how we use LLMs responsibly” page (written for any developer using Codex/OpenAI tooling; no internal assumptions)

User value:

- easy installation, clear docs, repeatable releases.

Acceptance criteria:

- A tagged release publishes binaries for macOS/Linux/Windows for both `wt` and `w`.
- Release artifacts include checksums.
- `brew tap prateek/w && brew install w` works against the latest release.
- GitHub Pages publishes `w` docs site from `main`.

## 12) Risks and Mitigations

- Risk: subtree workflow becomes painful.
  - Mitigation: document the exact commands; keep vendored-only commits clean; automate sync/split via scripts.
- Risk: exposing `project_identifier` leaks credentials.
  - Mitigation: tests that ensure redaction; restrict contract to safe format.
- Risk: multi-repo parallelism saturates disk, especially on Windows.
  - Mitigation: wrapper semaphore + documented Worktrunk command concurrency knobs; conservative defaults; perf tests.
- Risk: upstream doesn’t want integration API or template var changes.
  - Mitigation: keep changes tiny and well-tested; accept downstream-only if needed, but keep patch set narrow.
