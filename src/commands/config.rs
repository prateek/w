use directories::ProjectDirs;
use std::path::PathBuf;
use worktrunk::git::{GitError, Repository};
use worktrunk::styling::{AnstyleStyle, HINT, HINT_EMOJI, println};

/// Handle the config list command
pub fn handle_config_list() -> Result<(), GitError> {
    // Display global config
    display_global_config()?;
    println!();

    // Display project config if in a git repository
    display_project_config()?;

    Ok(())
}

fn display_global_config() -> Result<(), GitError> {
    let bold = AnstyleStyle::new().bold();
    let dim = AnstyleStyle::new().dimmed();

    // Get config path
    let config_path = get_global_config_path().ok_or_else(|| {
        GitError::CommandFailed("Could not determine global config path".to_string())
    })?;

    println!("Global Config: {bold}{}{bold:#}", config_path.display());

    // Check if file exists
    if !config_path.exists() {
        println!("  {HINT_EMOJI} {HINT}Not found (using defaults){HINT:#}");
        println!();
        println!("  {dim}# Default configuration:{dim:#}");
        println!("  {dim}worktree-path = \"../{{repo}}.{{branch}}\"{dim:#}");
        return Ok(());
    }

    // Read and display the file contents
    let contents = std::fs::read_to_string(&config_path)
        .map_err(|e| GitError::CommandFailed(format!("Failed to read config file: {}", e)))?;

    if contents.trim().is_empty() {
        println!("  {HINT_EMOJI} {HINT}Empty file (using defaults){HINT:#}");
        return Ok(());
    }

    // Display each line with indentation
    for line in contents.lines() {
        if !line.trim().is_empty() {
            println!("  {dim}{line}{dim:#}");
        } else {
            println!();
        }
    }

    Ok(())
}

fn display_project_config() -> Result<(), GitError> {
    let bold = AnstyleStyle::new().bold();
    let dim = AnstyleStyle::new().dimmed();

    // Try to get current repository root
    let repo = Repository::current();
    let repo_root = match repo.repo_root() {
        Ok(root) => root,
        Err(_) => {
            println!("Project Config: {dim}Not in a git repository{dim:#}");
            return Ok(());
        }
    };
    let config_path = repo_root.join(".config").join("wt.toml");

    println!("Project Config: {bold}{}{bold:#}", config_path.display());

    // Check if file exists
    if !config_path.exists() {
        println!("  {HINT_EMOJI} {HINT}Not found{HINT:#}");
        return Ok(());
    }

    // Read and display the file contents
    let contents = std::fs::read_to_string(&config_path)
        .map_err(|e| GitError::CommandFailed(format!("Failed to read config file: {}", e)))?;

    if contents.trim().is_empty() {
        println!("  {HINT_EMOJI} {HINT}Empty file{HINT:#}");
        return Ok(());
    }

    // Display each line with indentation
    for line in contents.lines() {
        if !line.trim().is_empty() {
            println!("  {dim}{line}{dim:#}");
        } else {
            println!();
        }
    }

    Ok(())
}

fn get_global_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "worktrunk").map(|dirs| dirs.config_dir().join("config.toml"))
}
