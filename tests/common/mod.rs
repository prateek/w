use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub struct TestRepo {
    temp_dir: TempDir, // Keep temp_dir accessible
    root: PathBuf,
    pub worktrees: HashMap<String, PathBuf>,
}

impl TestRepo {
    /// Create a new test repository with isolated git environment
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        // Create main repo as a subdirectory so worktrees can be siblings
        let root = temp_dir.path().join("main");
        std::fs::create_dir(&root).expect("Failed to create main repo directory");
        // Canonicalize to resolve symlinks (important on macOS where /var is symlink to /private/var)
        let root = root
            .canonicalize()
            .expect("Failed to canonicalize temp path");

        // Set up isolated git environment
        unsafe {
            std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
            std::env::set_var("GIT_CONFIG_SYSTEM", "/dev/null");
            std::env::set_var("GIT_AUTHOR_DATE", "2025-01-01T00:00:00Z");
            std::env::set_var("GIT_COMMITTER_DATE", "2025-01-01T00:00:00Z");
            std::env::set_var("LC_ALL", "C");
            std::env::set_var("LANG", "C");
            std::env::set_var("SOURCE_DATE_EPOCH", "1704067200");
        }

        // Initialize git repo
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(&root)
            .output()
            .expect("Failed to init git repo");

        // Configure git user
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&root)
            .output()
            .expect("Failed to set user.name");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&root)
            .output()
            .expect("Failed to set user.email");

        Self {
            temp_dir,
            root,
            worktrees: HashMap::new(),
        }
    }

    /// Get the root path of the repository
    pub fn root_path(&self) -> &Path {
        &self.root
    }

    /// Get the path to a named worktree
    pub fn worktree_path(&self, name: &str) -> &Path {
        self.worktrees
            .get(name)
            .unwrap_or_else(|| panic!("Worktree '{}' not found", name))
    }

    /// Create a commit with the given message
    pub fn commit(&self, message: &str) {
        // Create a file to ensure there's something to commit
        let file_path = self.root.join("file.txt");
        std::fs::write(&file_path, message).expect("Failed to write file");

        Command::new("git")
            .args(["add", "."])
            .current_dir(&self.root)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.root)
            .output()
            .expect("Failed to git commit");
    }

    /// Add a worktree with the given name and branch
    pub fn add_worktree(&mut self, name: &str, branch: &str) -> PathBuf {
        // Create worktree inside temp directory to ensure cleanup
        let worktree_path = self.temp_dir.path().join(name);

        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                branch,
                worktree_path.to_str().unwrap(),
            ])
            .current_dir(&self.root)
            .output()
            .expect("Failed to execute git worktree add");

        if !output.status.success() {
            panic!(
                "Failed to add worktree:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Canonicalize worktree path to match what git returns
        let canonical_path = worktree_path
            .canonicalize()
            .expect("Failed to canonicalize worktree path");
        self.worktrees
            .insert(name.to_string(), canonical_path.clone());
        canonical_path
    }

    /// Detach HEAD in the repository
    pub fn detach_head(&self) {
        // Get current commit SHA
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.root)
            .output()
            .expect("Failed to get HEAD SHA");

        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Command::new("git")
            .args(["checkout", "--detach", &sha])
            .current_dir(&self.root)
            .output()
            .expect("Failed to detach HEAD");
    }

    /// Lock a worktree with an optional reason
    pub fn lock_worktree(&self, name: &str, reason: Option<&str>) {
        let worktree_path = self.worktree_path(name);

        let mut args = vec!["worktree", "lock"];
        if let Some(r) = reason {
            args.push("--reason");
            args.push(r);
        }
        args.push(worktree_path.to_str().unwrap());

        Command::new("git")
            .args(&args)
            .current_dir(&self.root)
            .output()
            .expect("Failed to lock worktree");
    }
}
