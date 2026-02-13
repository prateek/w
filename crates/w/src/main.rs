use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::path::PathBuf;
use worktrunk::{
    config::UserConfig,
    git::Repository,
    integration::v1::{
        BranchDeletionMode, RemoveRequest, SwitchRequest, compute_worktree_path,
        remove as worktrunk_remove, switch as worktrunk_switch,
    },
};

#[derive(Parser, Debug)]
#[command(
    name = "w",
    version,
    about,
    long_about = None,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create a worktree for a branch (or switch if it already exists).
    New {
        /// Branch name (or Worktrunk symbols like "@", "-", "^").
        branch: String,
        /// Base ref when creating a branch (defaults to the repo's default branch).
        #[arg(long)]
        base: Option<String>,
        /// Move aside a pre-existing directory at the computed worktree path.
        #[arg(long)]
        clobber: bool,
    },
    /// Switch to a worktree for an existing branch and print its path.
    Cd {
        /// Branch name (or Worktrunk symbols like "@", "-", "^").
        branch: String,
    },
    /// Switch/create a worktree for a branch, then run a command in it.
    Run {
        /// Branch name (or Worktrunk symbols like "@", "-", "^").
        branch: String,
        /// Base ref when creating a branch (defaults to the repo's default branch).
        #[arg(long)]
        base: Option<String>,
        /// Move aside a pre-existing directory at the computed worktree path.
        #[arg(long)]
        clobber: bool,
        /// Command to run (pass after `--`), e.g. `w run feature -- cargo test`.
        #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
        cmd: Vec<String>,
    },
    /// Remove a worktree for a branch.
    Rm {
        /// Branch name (or Worktrunk symbols like "@", "-", "^").
        branch: String,
        /// Force removal even if the worktree is dirty.
        #[arg(long, short)]
        force: bool,
    },
    /// Remove stale worktree directories under the configured worktree root.
    Prune,
    /// Shell integration helpers.
    Shell {
        #[command(subcommand)]
        command: ShellCommand,
    },
}

#[derive(Subcommand, Debug)]
enum ShellCommand {
    /// Print an init snippet for the given shell.
    Init { shell: Shell },
}

#[derive(ValueEnum, Clone, Debug)]
enum Shell {
    Zsh,
    Bash,
    Fish,
    Pwsh,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::New {
            branch,
            base,
            clobber,
        } => {
            let path = cmd_new(branch, base, clobber)?;
            println!("{}", path.display());
        }
        Command::Cd { branch } => {
            let path = cmd_cd(branch)?;
            println!("{}", path.display());
        }
        Command::Run {
            branch,
            base,
            clobber,
            cmd,
        } => {
            let exit_code = cmd_run(branch, base, clobber, cmd)?;
            std::process::exit(exit_code);
        }
        Command::Rm { branch, force } => {
            let removed_path = cmd_rm(branch, force)?;
            println!("{}", removed_path.display());
        }
        Command::Prune => {
            for path in cmd_prune()? {
                println!("{}", path.display());
            }
        }
        Command::Shell {
            command: ShellCommand::Init { shell },
        } => {
            println!("{}", shell_init_snippet(shell));
        }
    }

    Ok(())
}

fn cmd_new(branch: String, base: Option<String>, clobber: bool) -> anyhow::Result<PathBuf> {
    let (repo, config) = current_repo_and_config()?;

    let branch = repo
        .resolve_worktree_name(&branch)
        .context("failed to resolve branch name")?;
    let create = !repo
        .branch(&branch)
        .exists()
        .context("failed to check branch existence")?;

    let outcome = worktrunk_switch(
        &repo,
        &config,
        SwitchRequest {
            branch,
            create,
            base,
            clobber,
        },
    )?;

    Ok(outcome.path)
}

fn cmd_cd(branch: String) -> anyhow::Result<PathBuf> {
    let (repo, config) = current_repo_and_config()?;

    let outcome = worktrunk_switch(
        &repo,
        &config,
        SwitchRequest {
            branch,
            create: false,
            base: None,
            clobber: false,
        },
    )?;

    Ok(outcome.path)
}

fn cmd_run(
    branch: String,
    base: Option<String>,
    clobber: bool,
    cmd: Vec<String>,
) -> anyhow::Result<i32> {
    let (repo, config) = current_repo_and_config()?;

    let (program, args) = cmd.split_first().context("command must be non-empty")?;

    let branch = repo
        .resolve_worktree_name(&branch)
        .context("failed to resolve branch name")?;
    let create = !repo
        .branch(&branch)
        .exists()
        .context("failed to check branch existence")?;

    let outcome = worktrunk_switch(
        &repo,
        &config,
        SwitchRequest {
            branch,
            create,
            base,
            clobber,
        },
    )?;

    let status = std::process::Command::new(program)
        .args(args)
        .current_dir(&outcome.path)
        .status()
        .with_context(|| format!("failed to run command: {}", cmd.join(" ")))?;

    Ok(status.code().unwrap_or(1))
}

fn cmd_rm(branch: String, force: bool) -> anyhow::Result<PathBuf> {
    let (repo, config) = current_repo_and_config()?;

    let branch = repo
        .resolve_worktree_name(&branch)
        .context("failed to resolve branch name")?;
    let existing_path = repo.worktree_for_branch(&branch)?;
    let existing_path =
        existing_path.ok_or_else(|| anyhow::anyhow!("no worktree exists for branch {branch}"))?;

    let outcome = worktrunk_remove(
        &repo,
        &config,
        RemoveRequest {
            branch,
            deletion_mode: BranchDeletionMode::Keep,
            force_worktree: force,
            target_branch: None,
        },
    )?;

    Ok(outcome.removed_worktree_path.unwrap_or(existing_path))
}

fn current_repo_and_config() -> anyhow::Result<(Repository, UserConfig)> {
    let repo = Repository::current().context("failed to discover git repo")?;
    let config = UserConfig::load().context("failed to load Worktrunk config")?;
    Ok((repo, config))
}

fn cmd_prune() -> anyhow::Result<Vec<PathBuf>> {
    let (repo, config) = current_repo_and_config()?;

    let root = worktree_root_dir(&repo, &config)?;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let active_worktrees: HashSet<PathBuf> = repo
        .list_worktrees()?
        .into_iter()
        .map(|wt| canonicalize_best_effort(&wt.path))
        .collect();

    let worktrees_git_dir = canonicalize_best_effort(&repo.git_common_dir().join("worktrees"));
    let mut removed = Vec::new();

    for entry in std::fs::read_dir(&root)
        .with_context(|| format!("failed to read worktree root dir: {}", root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let candidate = entry.path();
        if active_worktrees.contains(&canonicalize_best_effort(&candidate)) {
            continue;
        }

        let git_file = candidate.join(".git");
        if !git_file.is_file() {
            continue;
        }

        let gitdir = canonicalize_gitdir_path(&parse_gitdir_file(&git_file, &candidate)?);
        if !gitdir.starts_with(&worktrees_git_dir) {
            continue;
        }
        if gitdir.exists() {
            continue;
        }

        std::fs::remove_dir_all(&candidate)
            .with_context(|| format!("failed to remove {}", candidate.display()))?;
        removed.push(candidate);
    }

    Ok(removed)
}

fn worktree_root_dir(repo: &Repository, config: &UserConfig) -> anyhow::Result<PathBuf> {
    let path_a = compute_worktree_path(repo, "__w_prune_sentinel_a__", config)?;
    let path_b = compute_worktree_path(repo, "__w_prune_sentinel_b__", config)?;

    let parent_a = path_a.parent().context("worktree path has no parent")?;
    let parent_b = path_b.parent().context("worktree path has no parent")?;
    if parent_a != parent_b {
        anyhow::bail!(
            "cannot safely prune: worktree-path template changes parent directory based on branch (e.g. {} vs {})",
            parent_a.display(),
            parent_b.display()
        );
    }

    Ok(parent_a.to_path_buf())
}

fn canonicalize_best_effort(path: &std::path::Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn canonicalize_gitdir_path(path: &std::path::Path) -> PathBuf {
    if path.exists() {
        return canonicalize_best_effort(path);
    }

    let Some(parent) = path.parent() else {
        return path.to_path_buf();
    };
    let Some(file_name) = path.file_name() else {
        return path.to_path_buf();
    };

    std::fs::canonicalize(parent)
        .map(|parent| parent.join(file_name))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn parse_gitdir_file(
    git_file: &std::path::Path,
    worktree_dir: &std::path::Path,
) -> anyhow::Result<PathBuf> {
    let content = std::fs::read_to_string(git_file)
        .with_context(|| format!("failed to read {}", git_file.display()))?;
    let gitdir = content
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:").map(str::trim))
        .context("unexpected .git file format (expected gitdir: ...)")?;

    let gitdir_path = PathBuf::from(gitdir);
    Ok(if gitdir_path.is_absolute() {
        gitdir_path
    } else {
        worktree_dir.join(gitdir_path)
    })
}

fn shell_init_snippet(shell: Shell) -> &'static str {
    match shell {
        Shell::Zsh => {
            r#"# TODO: zsh integration not implemented yet.
# For now, use: w <subcommand> --print (future) and cd manually."#
        }
        Shell::Bash => {
            r#"# TODO: bash integration not implemented yet.
# For now, use: w <subcommand> --print (future) and cd manually."#
        }
        Shell::Fish => {
            r#"# TODO: fish integration not implemented yet.
# For now, use: w <subcommand> --print (future) and cd manually."#
        }
        Shell::Pwsh => {
            r#"# TODO: pwsh integration not implemented yet.
# For now, use: w <subcommand> --print (future) and cd manually."#
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_shows_help_when_no_args() {
        let err = Cli::try_parse_from(["w"]).unwrap_err();
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn shell_init_parses() {
        let cli = Cli::try_parse_from(["w", "shell", "init", "zsh"]).unwrap();
        let Cli {
            command:
                Command::Shell {
                    command: ShellCommand::Init { shell },
                },
        } = cli
        else {
            panic!("expected w shell init");
        };
        assert!(matches!(shell, Shell::Zsh));
        assert!(shell_init_snippet(shell).contains("zsh"));
    }

    #[test]
    fn new_parses() {
        let cli = Cli::try_parse_from(["w", "new", "feature"]).unwrap();
        let Cli {
            command:
                Command::New {
                    branch,
                    base,
                    clobber,
                },
        } = cli
        else {
            panic!("expected w new");
        };

        assert_eq!(branch, "feature");
        assert!(base.is_none());
        assert!(!clobber);
    }

    #[test]
    fn cd_parses() {
        let cli = Cli::try_parse_from(["w", "cd", "feature"]).unwrap();
        let Cli {
            command: Command::Cd { branch },
        } = cli
        else {
            panic!("expected w cd");
        };

        assert_eq!(branch, "feature");
    }

    #[test]
    fn run_parses() {
        let cli = Cli::try_parse_from(["w", "run", "feature", "--", "echo", "hi"]).unwrap();
        let Cli {
            command:
                Command::Run {
                    branch,
                    base,
                    clobber,
                    cmd,
                },
        } = cli
        else {
            panic!("expected w run");
        };

        assert_eq!(branch, "feature");
        assert!(base.is_none());
        assert!(!clobber);
        assert_eq!(cmd, ["echo", "hi"]);
    }

    #[test]
    fn rm_parses() {
        let cli = Cli::try_parse_from(["w", "rm", "feature", "--force"]).unwrap();
        let Cli {
            command: Command::Rm { branch, force },
        } = cli
        else {
            panic!("expected w rm");
        };

        assert_eq!(branch, "feature");
        assert!(force);
    }

    #[test]
    fn prune_parses() {
        let cli = Cli::try_parse_from(["w", "prune"]).unwrap();
        let Cli {
            command: Command::Prune,
        } = cli
        else {
            panic!("expected w prune");
        };
    }
}
