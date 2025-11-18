# Concurrent Test Failures: SIGKILL During Cargo Lock Contention

## Executive Summary

When running integration tests concurrently (8-10 parallel `cargo test` processes), approximately 29% of test runs fail with processes being killed by SIGKILL (signal 9). The failures occur during `cargo run -p dev-detach` invocations that are blocking on cargo's package cache lock. We implemented a workaround that eliminates the problem by avoiding concurrent cargo invocations, but **we don't understand what is sending SIGKILL or why**.

## Environment

- **OS**: macOS (Darwin 25.0.0)
- **Hardware**: Apple Silicon (aarch64-apple-darwin)
- **Cargo**: 1.90.0 (840b83a10 2025-07-30)
- **Rustc**: 1.90.0 (1159e78c4 2025-09-14)
- **Test Framework**: libtest (standard `cargo test`, NOT nextest)
- **Process Limits**:
  - `kern.maxproc: 12000`
  - `kern.maxprocperuid: 8000`
  - `ulimit -u: 8000`
  - `ulimit -n: unlimited` (file descriptors)

## Goals

1. **Primary**: Understand what is sending SIGKILL to cargo processes waiting on locks
2. **Secondary**: Determine if this is expected behavior or a bug
3. **Tertiary**: Understand if there's a better solution than avoiding concurrent cargo invocations

## System Architecture

### Test Infrastructure

Our integration tests need to execute shell scripts in isolated environments. We use a helper binary called `dev-detach` to isolate processes from the controlling terminal (prevents PTY-related hangs).

**Original implementation** (`tests/common/shell.rs`):

```rust
fn build_shell_command(repo: &TestRepo, shell: &str, script: &str) -> Command {
    // Get absolute path to workspace Cargo.toml (tests run from workspace root)
    let manifest = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("Cargo.toml");

    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--manifest-path"])
        .arg(&manifest)
        .args(["-p", "dev-detach", "--"]);

    // ... configure shell and arguments ...

    cmd
}
```

This function is called **hundreds of times** during test execution (each shell test invokes it multiple times).

### The dev-detach Binary

`tests/helpers/dev-detach/src/main.rs`:

```rust
#[cfg(unix)]
use nix::unistd::{execvp, setsid};
use std::ffi::CString;

fn main() {
    #[cfg(unix)]
    {
        // Become a new session leader with no controlling terminal
        if let Err(e) = setsid() {
            eprintln!("dev-detach: setsid failed: {}", e);
            process::exit(1);
        }

        // Get command and arguments from our argv
        let args: Vec<String> = env::args().skip(1).collect();
        let prog = CString::new(args[0].clone()).unwrap();
        let cargs: Vec<CString> = args
            .iter()
            .map(|a| CString::new(a.as_str()).unwrap())
            .collect();

        // Replace this process with the target command
        let _ = execvp(prog.as_c_str(), &cargs);
        eprintln!("dev-detach: execvp failed");
        process::exit(127);
    }
}
```

**Key point**: This is a simple wrapper that calls `setsid()` then `execvp()`. It doesn't interact with cargo locks.

## Observed Behavior

### Test Execution Pattern

When running `cargo test` with high concurrency:

```bash
# 8 parallel test processes, 3 iterations = 24 total runs
PARALLEL_RUNS=8
ITERATIONS=3

for iter in 1..3; do
    for i in 1..8; do
        cargo test integration_tests::e2e_shell -- --test-threads=8 &
    done
    wait
done
```

### Failure Rate

**Without fix**: 7/24 failures (29%)
**With fix**: 0/50 failures (0%) across 10 parallel × 5 iterations

### Actual Failure Examples

#### Example 1: SIGKILL During Lock Wait

```
thread 'integration_tests::e2e_shell_post_start::test_bash_post_create_blocks' panicked at tests/common/shell.rs:112:9:
Shell script failed (killed by signal 9 (SIGKILL)):
Command: cargo run --manifest-path <workspace>/Cargo.toml -p dev-detach -- bash [shell-flags...] -c <script>
stdout:
stderr:     Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.52s
     Running `/Users/maximilian/workspace/worktrunk.concurrent-tests/target/debug/dev-detach bash --noprofile --norc -c '
        export PATH="/Users/maximilian/workspace/worktrunk.concurrent-tests/target/debug:$PATH"
```

**Critical observation**: The stderr shows:
1. Multiple "Blocking waiting for file lock" messages
2. Successfully completes: "Finished `dev` profile"
3. Starts executing: "Running `/path/to/dev-detach ...`"
4. Then gets SIGKILL

#### Example 2: SIGKILL Before Build Completion

```
thread 'integration_tests::e2e_shell::test_e2e_switch_and_remove_roundtrip::case_2' panicked at tests/common/shell.rs:112:9:
Shell script failed (killed by signal 9 (SIGKILL)):
Command: cargo run --manifest-path <workspace>/Cargo.toml -p dev-detach -- fish [shell-flags...] -c <script>
stdout:
stderr:     Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
```

**Critical observation**: No "Finished" message - the process was killed **while waiting for the lock**.

### CPU and Resource Usage

During concurrent execution (without fix):

```bash
# From test run observations:
- CPU usage: 909% (9+ cores saturated)
- Active cargo processes: ~20-30 concurrent
- Shell processes: ~50-80 concurrent (including dev-detach, bash, fish, zsh)
```

## What We Tried

### Attempt 1: Using get_cargo_bin() with Once Guard

**Hypothesis**: `insta_cmd::get_cargo_bin()` might invoke cargo every time. Let's call it once and cache the result.

```rust
use std::sync::Once;
static BUILD_DEV_DETACH: Once = Once::new();

fn ensure_dev_detach_built() {
    BUILD_DEV_DETACH.call_once(|| {
        let status = Command::new("cargo")
            .args(["build", "-p", "dev-detach"])
            .status()
            .expect("Failed to run cargo build for dev-detach");

        if !status.success() {
            panic!("Failed to build dev-detach binary");
        }
    });
}

fn build_shell_command(repo: &TestRepo, shell: &str, script: &str) -> Command {
    ensure_dev_detach_built();
    let mut cmd = Command::new(get_cargo_bin("dev-detach"));
    // ...
}
```

**Result**: Failures reduced from 29% to 12.5% (7/24 → 3/24), but still failing.

**Analysis**: Each test process has its own `Once` instance, so we still had 8 concurrent builds at startup.

### Attempt 2: Using OnceLock to Cache Binary Path

**Hypothesis**: `get_cargo_bin()` itself might be invoking cargo. Let's cache the PathBuf.

```rust
use std::sync::OnceLock;
static DEV_DETACH_BIN: OnceLock<PathBuf> = OnceLock::new();

fn get_dev_detach_bin() -> &'static PathBuf {
    DEV_DETACH_BIN.get_or_init(|| {
        let status = Command::new("cargo")
            .args(["build", "-p", "dev-detach"])
            .status()
            .expect("Failed to run cargo build");

        if !status.success() {
            panic!("Failed to build dev-detach binary");
        }

        get_cargo_bin("dev-detach")
    })
}
```

**Result**: Still ~12-29% failure rate.

**Analysis**: Each test process still has independent static storage, so 8 concurrent builds still occur.

### Attempt 3: Direct Path Construction + Pre-build (THE FIX)

**Hypothesis**: Avoid ALL cargo invocations during test execution by:
1. Building dev-detach once before running tests
2. Using direct path construction instead of calling any cargo-related functions

```rust
/// Get path to dev-detach binary.
/// Assumes the binary has been built (e.g., by `cargo build -p dev-detach` before tests).
fn get_dev_detach_bin() -> PathBuf {
    // Construct path manually: target/debug/dev-detach
    let manifest_dir = std::env::current_dir().expect("Failed to get current directory");
    manifest_dir.join("target/debug/dev-detach")
}

fn build_shell_command(repo: &TestRepo, shell: &str, script: &str) -> Command {
    // Use pre-built dev-detach binary (no cargo invocation)
    let mut cmd = Command::new(get_dev_detach_bin());
    // ...
}
```

**Pre-build in test script**:
```bash
# Build dev-detach once before running tests
cargo build -p dev-detach --quiet

# Now run concurrent tests
for iter in 1..5; do
    for i in 1..10; do
        cargo test integration_tests::e2e_shell -- --test-threads=8 &
    done
    wait
done
```

**Result**: ✅ **100% success rate** - 50/50 runs passed (10 parallel × 5 iterations)

**Verification**:
```bash
$ cargo build -p dev-detach --quiet && cargo test --test integration -- --test-threads=12
...
test result: ok. 321 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.81s
```

## Key Findings

### 1. SIGKILL Timing is Inconsistent

From our failure logs, SIGKILL happens at two different points:

**Point A**: While waiting for lock (no "Finished" message)
```
    Blocking waiting for file lock on build directory
[Process killed here - no "Finished" message]
```

**Point B**: After successful build, during execution
```
    Blocking waiting for file lock on build directory
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.52s
     Running `/path/to/dev-detach bash ...`
[Process killed here - mid-execution]
```

### 2. No Evidence of Timeout Configuration

We checked:
- ❌ No `RUST_TEST_TIME_*` environment variables set
- ❌ No `.nextest.toml` configuration (and we're not using nextest anyway)
- ❌ No timeout flags in `cargo test --help`
- ❌ libtest has `--report-time` and `--ensure-time`, but these don't kill processes
- ❌ No cargo lock timeout documented in cargo source or documentation

### 3. System Limits are Not Exhausted

```bash
$ ulimit -a
-u: processes                       8000
-n: file descriptors                unlimited

$ sysctl kern.maxproc kern.maxprocperuid
kern.maxproc: 12000
kern.maxprocperuid: 8000

# During test execution:
$ ps aux | grep -E "bash|zsh|fish|dev-detach" | wc -l
53

$ ps aux | grep -c "[c]argo"
21
```

We're using ~74 processes out of 8000 limit - well within bounds.

### 4. No OOM Killer Evidence

```bash
$ dmesg 2>&1 | grep -i "killed\|oom"
[No output - dmesg not available on macOS by default]
```

macOS doesn't have a traditional OOM killer like Linux. Process termination under memory pressure is handled differently.

## Our Assumptions (UNPROVEN)

### Assumption 1: macOS is Sending SIGKILL

**Reasoning**:
- SIGKILL (signal 9) cannot be caught or blocked by the process
- It's typically sent by the kernel or system management
- User processes rarely send SIGKILL (they use SIGTERM first)

**Unproven**: We haven't traced which process/kernel subsystem sends the signal.

**How to verify**:
- Use `sudo dtruss -p <cargo_pid>` to trace system calls
- Monitor Console.app for system messages during failures
- Check if cargo itself has watchdog timers in source code

### Assumption 2: High Lock Contention Triggers System Intervention

**Reasoning**:
- Failures correlate with "Blocking waiting for file lock" messages
- Eliminating concurrent cargo invocations eliminates SIGKILL
- macOS might detect processes in prolonged wait states as "stuck"

**Unproven**: We don't know macOS's specific heuristics for killing processes.

**How to verify**:
- Research macOS process management policies
- Check if macOS has configurable thresholds for process wait states
- Look for Apple documentation on system resource management

### Assumption 3: Cargo Lock Implementation Doesn't Have Timeouts

**Reasoning**:
- No timeout configuration found in cargo documentation
- `cargo build --help` has no timeout flags
- Cargo source code (github.com/rust-lang/cargo) would show timeout logic

**Unproven**: We haven't audited cargo's source code for lock timeout mechanisms.

**How to verify**:
- Search cargo source for lock timeout implementations
- Check cargo issue tracker for lock-related SIGKILL reports
- Review cargo's `util/config` module for hidden timeout settings

### Assumption 4: The Problem is Lock-Specific, Not Load-Specific

**Reasoning**:
- The fix (pre-building) eliminates concurrent cargo invocations
- Pre-building doesn't reduce total system load (tests still run concurrently)
- SIGKILL stops happening despite similar CPU/memory usage

**Counter-evidence**:
- We observed 909% CPU during original runs (high load)
- With fix, CPU usage is still high during test execution
- Maybe the difference is **duration** of high load, not peak?

**Unproven**: We haven't compared system metrics (CPU, I/O, memory pressure) between failing and passing runs.

**How to verify**:
- Monitor system pressure metrics during both scenarios
- Use `powermetrics` or Activity Monitor to compare resource usage
- Check if macOS's App Nap or other power management features are involved

## Open Questions

### Question 1: What Process/Subsystem Sends SIGKILL?

**What we need to know**:
- Is it the kernel?
- Is it cargo itself (internal watchdog)?
- Is it macOS's process management daemon (e.g., `kernel_task`, `launchd`)?
- Is it a resource governor (memory pressure, thermal throttling)?

**Research directions**:
1. Search for "macOS SIGKILL blocked processes" or "macOS kill waiting processes"
2. Look for cargo source code related to subprocess management and timeouts
3. Check Apple documentation on process lifecycle management
4. Search for similar reports in Rust/cargo communities
5. Look for macOS system calls that can trigger SIGKILL (`kill(2)`, `killpg(2)`)

### Question 2: Is This Expected macOS Behavior?

**What we need to know**:
- Does macOS have documented policies for killing processes in wait states?
- Are there system-wide timeout thresholds we're exceeding?
- Is this a feature or a bug?

**Research directions**:
1. Apple Developer Documentation on process management
2. Darwin kernel source code (open source) for process scheduler
3. XNU (kernel) documentation on process states and lifecycle
4. Search for "macOS kill blocking process" or "macOS I/O wait timeout"

### Question 3: Why is the Timing Inconsistent?

Some processes get killed while waiting for lock, others after build completion. Why?

**Hypotheses**:
- Different cargo subprocesses have different wait times
- macOS uses cumulative metrics (total time in wait state)
- Race condition in detection logic
- Different code paths have different watchdogs

**Research directions**:
1. Map cargo's subprocess tree during `cargo run`
2. Understand which process gets SIGKILL (parent cargo or child rustc?)
3. Check if cargo uses process groups and how SIGKILL propagates

### Question 4: Are There Hidden Cargo Configuration Options?

**What we need to know**:
- Does cargo have undocumented timeout settings?
- Are there environment variables we're not aware of?
- Does cargo have different behavior in "test mode"?

**Research directions**:
1. Review `cargo --list` for all subcommands and options
2. Search cargo source for `CARGO_*` environment variables
3. Check if `cargo test` sets special environment for child processes
4. Look for `.cargo/config.toml` options related to timeouts or locks

### Question 5: Why Does Lock Contention Occur at All?

**What we need to know**:
- What is cargo locking when building dev-detach?
- Why does building a simple helper binary require multiple locks?
- Can we configure cargo to use different lock granularity?

**From our observations**:
```
Blocking waiting for file lock on package cache
Blocking waiting for file lock on build directory
```

This suggests at least two separate locks. Why multiple locks for a simple build?

**Research directions**:
1. Understand cargo's lock hierarchy (package cache → dependency resolution → build directory)
2. Check if `cargo build -p` (workspace member build) has different locking than `cargo build`
3. Research cargo's concurrent build architecture
4. Look for cargo RFC/documentation on lock implementation

### Question 6: Could This Be Related to `insta_cmd`?

**What we need to know**:
- What does `insta_cmd::get_cargo_bin()` actually do?
- Does it invoke cargo, or just locate the binary?
- Could it be interacting with cargo in unexpected ways?

**Research directions**:
1. Review insta_cmd source code for `get_cargo_bin()` implementation
2. Check if it caches binary locations
3. See if it has any lock files or state that could conflict

## Detailed Code Context

### Complete Test Execution Flow

1. **Test harness** runs `cargo test integration_tests::e2e_shell`
2. **Test function** (e.g., `test_bash_e2e_switch_existing_worktree`) calls:
   ```rust
   let output = execute_shell_script(&repo, "bash", &script);
   ```
3. **`execute_shell_script`** (in `tests/common/shell.rs:59`) calls:
   ```rust
   let mut cmd = build_shell_command(repo, shell, script);
   let output = cmd.current_dir(repo.root_path()).output().unwrap();
   ```
4. **`build_shell_command`** (original version) constructs:
   ```rust
   Command::new("cargo")
       .args(["run", "--manifest-path", &manifest, "-p", "dev-detach", "--"])
       .arg(shell)
       .args([shell_flags...])
       .arg("-c")
       .arg(script)
   ```
5. **Cargo** receives this and must:
   - Acquire package cache lock
   - Check if dev-detach needs rebuilding
   - Acquire build directory lock
   - Build dev-detach (if needed)
   - Execute dev-detach with remaining arguments

When 8-10 test processes do this concurrently, step 5 becomes a bottleneck.

### Signal Handling Code

We added signal number reporting to diagnose the issue:

```rust
/// Convert signal number to human-readable name
#[cfg(unix)]
fn signal_name(sig: i32) -> &'static str {
    match sig {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        6 => "SIGABRT",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        15 => "SIGTERM",
        _ => "UNKNOWN",
    }
}

// In execute_shell_script:
if !output.status.success() {
    let exit_info = match output.status.code() {
        Some(code) => format!("exit code {}", code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                match output.status.signal() {
                    Some(sig) => format!("killed by signal {} ({})", sig, signal_name(sig)),
                    None => "killed by signal (unknown)".to_string(),
                }
            }
            #[cfg(not(unix))]
            {
                "killed by signal".to_string()
            }
        }
    };
    panic!("Shell script failed ({}):\n...", exit_info);
}
```

This confirmed all failures were **signal 9 (SIGKILL)**, not other signals like SIGTERM or SIGPIPE.

## What Success Looks Like

The workaround successfully eliminates the problem:

```bash
$ cargo build -p dev-detach --quiet
$ cargo test --test integration -- --test-threads=12
...
test result: ok. 321 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.81s

# Stress test: 10 parallel × 5 iterations = 50 runs
$ for iter in 1..5; do
    for i in 1..10; do
        cargo test integration_tests::e2e_shell -- --test-threads=8 &
    done
    wait
done

Result: ✅ All 50 runs passed (0 failures)
```

## Priority Research Questions

**Highest Priority (Critical to Understanding)**:

1. **Who sends SIGKILL?** - Trace the actual source of the signal
2. **Is there a cargo lock timeout?** - Audit cargo source code or find documentation
3. **Does macOS have process wait state policies?** - Find Apple documentation on process lifecycle

**Medium Priority (Nice to Know)**:

4. Why is timing inconsistent (kill during wait vs. after build)?
5. What exactly triggers the decision to send SIGKILL?
6. Are there configuration options to adjust thresholds?

**Lower Priority (Optimization)**:

7. Could cargo's locking be more granular for workspace members?
8. Is there a way to pre-warm cargo's cache to avoid lock contention?
9. Should we file a bug report with cargo or macOS?

## Potential Alternative Solutions

If we understood the root cause, we might be able to:

1. **Configure a timeout** (if one exists) to be longer
2. **Adjust system policies** (if macOS has configurable thresholds)
3. **Use cargo's jobserver** to coordinate builds (if that's relevant)
4. **File a bug report** if this is unintended behavior
5. **Document the workaround** as best practice for test infrastructure

## Current Workaround (Implemented)

```rust
// In tests/common/shell.rs:
fn get_dev_detach_bin() -> PathBuf {
    std::env::current_dir()
        .expect("Failed to get current directory")
        .join("target/debug/dev-detach")
}

fn build_shell_command(repo: &TestRepo, shell: &str, script: &str) -> Command {
    let mut cmd = Command::new(get_dev_detach_bin());
    // ... rest of setup
}
```

**Requirements**:
- Must run `cargo build -p dev-detach` before running tests
- Binary must exist at `target/debug/dev-detach`

**Advantages**:
- ✅ 100% reliable (no failures in 50+ runs)
- ✅ Faster test execution (no cargo overhead)
- ✅ Simple implementation

**Disadvantages**:
- ❌ Requires explicit pre-build step
- ❌ Won't automatically rebuild if source changes
- ❌ Doesn't solve the underlying mystery

## Summary

We have a **reliable workaround** but **no understanding of root cause**. The key mystery is: **What sends SIGKILL to cargo processes waiting on locks, and why?**

The evidence strongly suggests it's not:
- Process/FD limits (we're well within limits)
- OOM killer (no memory pressure observed)
- Test framework timeouts (libtest doesn't kill processes)
- Cargo configuration (no timeout options found)

The most likely explanation is **macOS system-level process management** responding to prolonged wait states under high load, but we have no concrete evidence or documentation to support this.

**We need research into**:
1. macOS process lifecycle policies
2. Cargo's subprocess and lock management
3. Darwin/XNU kernel behavior for I/O-blocked processes
4. Similar reports in the Rust/cargo community

The goal is to determine if this is:
- **Expected behavior** we should work around (as we have)
- **A bug** in cargo's lock management
- **A system limitation** we can configure
- **Something else** we haven't considered
