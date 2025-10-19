# Arbor: Git Worktree Management in Rust

Port of git worktree fish functions to a Rust crate with enhanced capabilities.

## Analysis of Existing Functionality

The fish functions provide:
- **git-worktree-switch**: Create/switch to worktrees with branch management
- **git-worktree-finish**: Cleanup and return to primary worktree
- **git-worktree-push**: Fast-forward push between worktrees with conflict detection
- **git-worktree-merge**: Rebase, merge, and cleanup with optional squashing
- **git-worktree-llm**: Convenience wrapper for LLM-assisted development

## Workstreams

### 1. Foundation: Git Primitives
Core git operations that all other components depend on.

**Key capabilities:**
- Execute git commands and parse output
- Detect repository state (current branch, default branch, dirty state)
- Find git directories (git-dir, common-dir, toplevel)
- Parse worktree list (porcelain format)
- Check merge ancestry and fast-forward status
- Detect operations in progress (merge, rebase, cherry-pick)

**Dependencies:** `std::process::Command` for shelling out to git

### 2. Worktree Core
Primary worktree management operations.

**Key capabilities:**
- List existing worktrees with branch associations
- Create worktrees (with/without new branch, custom base)
- Switch to existing worktrees (cd integration)
- Remove worktrees (foreground/background)
- Validate worktree state and detect missing directories

**Dependencies:** Workstream 1 (Git Primitives)

**Challenges:** Shell integration for `cd` - may need to output shell commands for sourcing

### 3. Advanced Operations
Complex multi-step operations involving multiple worktrees.

**Key capabilities:**
- Fast-forward push between worktrees
  - Validate ancestry and fast-forward status
  - Detect and handle merge commits
  - Stash/unstash in target worktree
  - Detect file conflicts between push and working tree
  - Configure `receive.denyCurrentBranch`
- Merge and cleanup workflow
  - Auto-commit dirty state
  - Squash commits with rebase
  - Fast-forward target branch
  - Finish worktree after merge
- Branch finishing (commit, cleanup, switch)

**Dependencies:** Workstreams 1, 2, and 4 (for user confirmation)

### 4. CLI and UX
Command-line interface with rich user experience.

**Key capabilities:**
- Argument parsing (likely using `clap`)
- Colored terminal output (similar to fish functions)
- Progress indicators
- Error messages with actionable suggestions
- Subcommand structure (switch, finish, push, merge)

**Dependencies:** `clap` for argument parsing, `colored` or `owo-colors` for terminal colors

### 5. External Integrations
Interface with shell and external tools.

**Key capabilities:**
- Execute shell commands (git-commit-llm, claude, task)
- Background process management (disown equivalent)
- Shell function generation (for `cd` integration)
- Hook system for custom commands

**Dependencies:** Workstream 1

**Design considerations:**
- How to handle `cd` in a compiled binary? Options:
  - Output shell commands to eval
  - Generate shell wrapper functions
  - Use shell integration (similar to `zoxide`)

### 6. Testing and Validation
Ensure correctness with real git repositories.

**Key capabilities:**
- Integration tests with temporary git repos
- Test multiple worktree scenarios
- Validate edge cases (conflicts, missing dirs, operations in progress)
- Test shell integration

**Dependencies:** All workstreams

**Tools:** `tempfile` crate for temporary directories, possibly `insta` for snapshot testing

## Implementation Order

1. **Phase 1: Foundation**
   - Workstream 1: Git Primitives (core library)
   - Workstream 4: Basic CLI (minimal viable interface)

2. **Phase 2: Core Operations**
   - Workstream 2: Worktree Core (switch, list, create, remove)
   - Workstream 6: Basic testing

3. **Phase 3: Advanced Features**
   - Workstream 3: Advanced Operations (push, merge)
   - Workstream 5: External Integrations
   - Workstream 6: Comprehensive testing

## Open Questions

1. **Shell integration approach**: How to handle `cd` in a compiled binary?
   - Generate eval-able shell output?
   - Provide shell wrapper functions?
   - Use shell integration hooks?

2. **External command dependencies**: How to handle git-commit-llm, claude, task?
   - Configurable hooks?
   - Plugin system?
   - Just execute if available?

3. **Cross-platform support**: Focus on Unix-like systems only or support Windows?
   - Fish-specific features may not translate

4. **Git library choice**: ✅ **DECIDED: Shell out to git commands**
   - Use `std::process::Command` directly (no wrapper crates needed)
   - Parse `--porcelain` formats for stability
   - Rationale: libgit2 lags behind git on worktree features, proven approach from vergen-gitcl and PRQL
   - Git is already required for worktrees to exist, so no additional runtime dependency

## Testing Strategy

### Approach: Snapshot Testing with Insta

We'll use `insta` and `insta_cmd` for snapshot-based testing of both library functions and CLI commands.

**Rationale:**
- Snapshot tests capture actual git behavior, not mocked approximations
- Easy to review changes to output formats
- Faster to write comprehensive tests (no manual assertion writing)
- Tests serve as living documentation of expected behavior

### Test Structure

Following Rust 2018+ conventions:

```
tests/
  common/
    mod.rs              # Shared test helpers (TestRepo, etc.)
  test_list.rs          # Integration tests for `arbor list`
  test_switch.rs        # Integration tests for `arbor switch` (future)
  test_finish.rs        # Integration tests for `arbor finish` (future)
  snapshots/            # Generated by insta
    test_list__*.snap
```

### Test Helpers (tests/common/mod.rs)

**Core helpers:**
1. `TestRepo::new()` - Create temp git repo with isolated environment
2. `TestRepo::add_worktree(name, branch)` - Add worktree, return path, track by name
3. `TestRepo::commit(message)` - Make commit with fixed author/committer dates
4. `TestRepo::detach_head()` - Create detached HEAD state
5. `TestRepo::lock_worktree(name, reason)` - Lock a worktree with optional reason
6. `TestRepo::root_path()` - Get root repo path for normalization
7. `TestRepo::worktree_path(name)` - Get worktree path by semantic name

**Environment Isolation (critical for test stability):**
- Set `GIT_CONFIG_GLOBAL=/dev/null` - Ignore user's global config
- Set `GIT_CONFIG_SYSTEM=/dev/null` - Ignore system config
- Set `user.name = "Test User"` and `user.email = "test@example.com"`
- Set `GIT_AUTHOR_DATE = "2025-01-01T00:00:00Z"` - Fixed commit timestamps
- Set `GIT_COMMITTER_DATE = "2025-01-01T00:00:00Z"`
- Set `LC_ALL=C` and `LANG=C` - Force English git messages
- Set `SOURCE_DATE_EPOCH=1704067200` - Reproducible builds

### Snapshot Normalization Strategy

**Challenge:** Git output contains non-deterministic data:
- Absolute paths (different per test run, per machine)
- Git SHAs (different per test run)
- Path separators (Windows `\` vs Unix `/`)

**Solution:** Use insta's built-in filter system

```rust
use insta::Settings;
use std::process::Command;

fn snapshot_cmd(repo: &TestRepo, args: &[&str]) -> String {
    let mut settings = Settings::clone_current();

    // Normalize paths - replace absolute paths with semantic names
    settings.add_filter(repo.root_path().to_str().unwrap(), "[REPO]");
    for (name, path) in &repo.worktrees {
        settings.add_filter(path.to_str().unwrap(), &format!("[WORKTREE_{}]", name.to_uppercase()));
    }

    // Normalize git SHAs (7-40 hex chars) to [SHA]
    settings.add_regex(r"\b[0-9a-f]{7,40}\b", "[SHA]");

    // Normalize Windows paths to Unix style
    settings.add_regex(r"\\", "/");

    settings.bind(|| {
        assert_cmd_snapshot!(Command::new(get_cargo_bin("arbor")).args(args));
    });
}
```

**Key decisions:**
- ✅ **Use insta filters from the start** (not custom string replacement)
- ✅ **Track worktrees by semantic name** (not index) for stability
- ✅ **Normalize path separators** for cross-platform snapshots
- ✅ **Use regex for SHA normalization** (requires `regex` crate in filters)

### Test Scenarios

**Basic scenarios (Phase 1):**
1. Single worktree (main only)
2. Multiple worktrees on different branches
3. Detached HEAD worktree
4. Bare repository worktree
5. Locked worktree (with and without reason)
6. Prunable worktree (removed directory)

**Edge cases (Phase 2):**
7. Worktree with dirty state
8. Worktree with merge in progress
9. Worktree with rebase in progress
10. Missing worktree directory
11. Worktree at repository root
12. Nested worktree paths

**Advanced scenarios (Phase 3):**
13. Fast-forward push scenarios
14. Merge conflict scenarios
15. Squash merge scenarios
16. Multiple worktrees with same base commit

**Error cases (all phases):**
17. Running outside a git repository
18. Invalid command arguments
19. Attempting to create worktree with existing name
20. Permission denied on worktree directory
21. Corrupted git state (missing refs, invalid objects)

### Dependencies

```toml
[dev-dependencies]
insta = { version = "1.40", features = ["yaml", "redactions", "filters"] }
insta-cmd = "0.6"
assert_cmd = "2.0"     # For get_cargo_bin() helper
tempfile = "3.14"       # For temporary test directories
```

**Notes:**
- `yaml` feature for readable snapshots
- `redactions` and `filters` for path/SHA normalization with regex
- `assert_cmd` provides `get_cargo_bin()` for locating test binaries

### Workflow

**Development:**
1. Build binary: `cargo build` (required before insta-cmd tests)
2. Write test that creates git repo scenario
3. Run `cargo insta test` - creates initial snapshots
4. Review snapshots with `cargo insta review`
5. Accept/reject changes, commit accepted snapshots
6. Future changes: `cargo insta test` shows diffs, review and accept/reject

**CI:**
- Run `cargo insta test --check` - fails if snapshots need review
- Rejects PRs with pending snapshot reviews or missing snapshots
- Use `--test-threads=1` if global state isolation issues arise

### Implementation Decisions

1. ✅ **Normalization:** Use insta's built-in filters (not custom string replacement)
2. ✅ **Binary build:** Explicit `cargo build` step documented in workflow
3. ✅ **Snapshot format:** External files for all tests (easier PR review)
4. ✅ **Test structure:** Follow Rust 2018+ conventions (`tests/common/`, not `tests/integration/`)
5. ✅ **Worktree tracking:** Semantic names (not indices) for stable snapshots
6. ✅ **Git version:** Document minimum required version (2.5+) in tests

## Success Criteria

- [ ] Can create and switch between worktrees from CLI
- [ ] Can push fast-forward changes between worktrees
- [ ] Can merge and cleanup worktrees
- [ ] Provides colored, user-friendly output
- [ ] Handles edge cases gracefully (dirty state, conflicts, missing directories)
- [ ] Integrates with shell for directory changes
- [ ] Passes integration tests with real git repositories
