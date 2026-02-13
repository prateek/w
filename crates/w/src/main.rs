use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use worktrunk::{
    config::UserConfig,
    git::Repository,
    integration::v1::{
        BranchDeletionMode, RemoveRequest, SwitchRequest, compute_worktree_path,
        remove as worktrunk_remove, switch as worktrunk_switch,
    },
};

mod repo;

#[derive(Parser, Debug)]
#[command(
    name = "w",
    version,
    about,
    long_about = None,
    arg_required_else_help = true
)]
struct Cli {
    /// Operate on a repository at the given path (like `git -C`).
    #[arg(short = 'C', long = "repo", global = true, value_name = "PATH")]
    repo_dir: Option<PathBuf>,
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
    /// Multi-repo helpers (indexing and selection).
    Repo {
        #[command(subcommand)]
        command: RepoCommand,
    },
    /// Shell integration helpers.
    Shell {
        #[command(subcommand)]
        command: ShellCommand,
    },
}

#[derive(Subcommand, Debug)]
enum RepoCommand {
    /// Build/print the repository index.
    Index {
        /// Path to `w` config TOML (defaults to `~/.config/w/config.toml`).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Root directory to scan for git repositories (may be repeated).
        #[arg(long = "root", value_name = "PATH")]
        roots: Vec<PathBuf>,
        /// Maximum directory depth to search under each root.
        #[arg(long)]
        max_depth: Option<usize>,
        /// Cache path for the repo index.
        #[arg(long)]
        cache_path: Option<PathBuf>,
        /// Read from the cache only (do not scan).
        #[arg(long)]
        cached: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = RepoIndexFormat::Json)]
        format: RepoIndexFormat,
    },
    /// Select a repository and print its path.
    Pick {
        /// Path to `w` config TOML (defaults to `~/.config/w/config.toml`).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Root directory to scan for git repositories (may be repeated).
        #[arg(long = "root", value_name = "PATH")]
        roots: Vec<PathBuf>,
        /// Maximum directory depth to search under each root.
        #[arg(long)]
        max_depth: Option<usize>,
        /// Cache path for the repo index.
        #[arg(long)]
        cache_path: Option<PathBuf>,
        /// Read from the cache only (do not scan).
        #[arg(long, conflicts_with = "refresh")]
        cached: bool,
        /// Force a rescan and refresh the cache.
        #[arg(long, conflicts_with = "cached")]
        refresh: bool,
        /// Non-interactively select the first match (substring match on path or project identifier).
        #[arg(long)]
        filter: Option<String>,
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

#[derive(ValueEnum, Clone, Debug)]
enum RepoIndexFormat {
    Json,
    Tsv,
}

fn main() -> anyhow::Result<()> {
    let Cli { repo_dir, command } = Cli::parse();
    match command {
        Command::New {
            branch,
            base,
            clobber,
        } => {
            let path = cmd_new(repo_dir.as_deref(), branch, base, clobber)?;
            println!("{}", path.display());
        }
        Command::Cd { branch } => {
            let path = cmd_cd(repo_dir.as_deref(), branch)?;
            println!("{}", path.display());
        }
        Command::Run {
            branch,
            base,
            clobber,
            cmd,
        } => {
            let exit_code = cmd_run(repo_dir.as_deref(), branch, base, clobber, cmd)?;
            std::process::exit(exit_code);
        }
        Command::Rm { branch, force } => {
            let removed_path = cmd_rm(repo_dir.as_deref(), branch, force)?;
            println!("{}", removed_path.display());
        }
        Command::Prune => {
            for path in cmd_prune(repo_dir.as_deref())? {
                println!("{}", path.display());
            }
        }
        Command::Repo { command } => match command {
            RepoCommand::Index {
                config,
                roots,
                max_depth,
                cache_path,
                cached,
                format,
            } => {
                let cache_path = cache_path.unwrap_or(repo::default_cache_path()?);

                let index = if cached {
                    repo::read_repo_index_cache(&cache_path)?
                } else {
                    let (roots, max_depth) =
                        repo_roots_and_depth(config.as_deref(), roots, max_depth)?;
                    let index = repo::build_repo_index(&roots, max_depth)?;
                    repo::write_repo_index_cache(&cache_path, &index)?;
                    index
                };

                match format {
                    RepoIndexFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&index)?);
                    }
                    RepoIndexFormat::Tsv => {
                        for repo in index.repos {
                            println!("{}\t{}", repo.project_identifier, repo.path);
                        }
                    }
                }
            }
            RepoCommand::Pick {
                config,
                roots,
                max_depth,
                cache_path,
                cached,
                refresh,
                filter,
            } => {
                let cache_path = cache_path.unwrap_or(repo::default_cache_path()?);

                let index = if cached {
                    repo::read_repo_index_cache(&cache_path)?
                } else if refresh || !cache_path.exists() {
                    let (roots, max_depth) =
                        repo_roots_and_depth(config.as_deref(), roots, max_depth)?;
                    let index = repo::build_repo_index(&roots, max_depth)?;
                    repo::write_repo_index_cache(&cache_path, &index)?;
                    index
                } else {
                    repo::read_repo_index_cache(&cache_path)?
                };

                let selected = if let Some(filter) = filter {
                    repo::select_repo_by_filter(&index, &filter)
                        .ok_or_else(|| anyhow::anyhow!("no repository matched filter: {filter}"))?
                } else {
                    repo::pick_repo_interactive(&index)?.context("no repository selected")?
                };

                println!("{}", selected.display());
            }
        },
        Command::Shell {
            command: ShellCommand::Init { shell },
        } => {
            println!("{}", shell_init_snippet(shell));
        }
    }

    Ok(())
}

fn cmd_new(
    repo_dir: Option<&Path>,
    branch: String,
    base: Option<String>,
    clobber: bool,
) -> anyhow::Result<PathBuf> {
    let (repo, config) = current_repo_and_config(repo_dir)?;

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

fn cmd_cd(repo_dir: Option<&Path>, branch: String) -> anyhow::Result<PathBuf> {
    let (repo, config) = current_repo_and_config(repo_dir)?;

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
    repo_dir: Option<&Path>,
    branch: String,
    base: Option<String>,
    clobber: bool,
    cmd: Vec<String>,
) -> anyhow::Result<i32> {
    let (repo, config) = current_repo_and_config(repo_dir)?;

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

fn cmd_rm(repo_dir: Option<&Path>, branch: String, force: bool) -> anyhow::Result<PathBuf> {
    let (repo, config) = current_repo_and_config(repo_dir)?;

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

fn current_repo_and_config(repo_dir: Option<&Path>) -> anyhow::Result<(Repository, UserConfig)> {
    let repo = match repo_dir {
        Some(dir) => Repository::at(dir).context("failed to discover git repo")?,
        None => Repository::current().context("failed to discover git repo")?,
    };
    let config = UserConfig::load().context("failed to load Worktrunk config")?;
    Ok((repo, config))
}

fn cmd_prune(repo_dir: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    let (repo, config) = current_repo_and_config(repo_dir)?;

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

fn repo_roots_and_depth(
    config_path: Option<&Path>,
    roots: Vec<PathBuf>,
    max_depth: Option<usize>,
) -> anyhow::Result<(Vec<PathBuf>, usize)> {
    if !roots.is_empty() {
        let max_depth = max_depth.unwrap_or(6);
        return Ok((roots, max_depth));
    }

    let config_path = config_path
        .map(PathBuf::from)
        .unwrap_or(repo::default_config_path()?);
    let config = repo::load_config(&config_path)?;

    let roots = config.repo_roots;
    if roots.is_empty() {
        anyhow::bail!(
            "no repo roots configured (set repo_roots in {})",
            config_path.display()
        );
    }

    Ok((roots, max_depth.unwrap_or(config.max_depth)))
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
            r#"# w shell integration (zsh)
#
# Usage:
#   eval "$(w shell init zsh)"
#
# Notes:
# - Overrides the `w` shell function to allow `w cd`/`w new` to change the current directory.
# - Use `command w ...` to bypass the function (call the binary directly).

w() {
  case "$1" in
    cd|new)
      for arg in "$@"; do
        if [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
          command w "$@"
          return $?
        fi
      done

      local target
      target="$(command w "$@")" || return $?
      [[ -n "$target" ]] || return 1
      builtin cd -- "$target" || return $?
      ;;
    *)
      command w "$@"
      ;;
  esac
}"#
        }
        Shell::Bash => {
            r#"# w shell integration (bash)
#
# Usage:
#   eval "$(w shell init bash)"
#
# Notes:
# - Overrides the `w` shell function to allow `w cd`/`w new` to change the current directory.
# - Use `command w ...` to bypass the function (call the binary directly).

w() {
  case "$1" in
    cd|new)
      for arg in "$@"; do
        if [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
          command w "$@"
          return $?
        fi
      done

      local target
      target="$(command w "$@")" || return $?
      [[ -n "$target" ]] || return 1
      builtin cd -- "$target" || return $?
      ;;
    *)
      command w "$@"
      ;;
  esac
}"#
        }
        Shell::Fish => {
            r#"# w shell integration (fish)
#
# Usage:
#   w shell init fish | source
#
# Notes:
# - Overrides the `w` function to allow `w cd`/`w new` to change the current directory.
# - Use `command w ...` to bypass the function (call the binary directly).

function w --wraps w --description 'w wrapper with cd/new'
    if test (count $argv) -ge 1
        set -l sub $argv[1]
        if test "$sub" = "cd" -o "$sub" = "new"
            for arg in $argv
                if test "$arg" = "-h" -o "$arg" = "--help"
                    command w $argv
                    return $status
                end
            end

            set -l target (command w $argv | string collect)
            or return $status
            if test -z "$target"
                return 1
            end
            cd -- "$target"
            return $status
        end
    end

    command w $argv
end"#
        }
        Shell::Pwsh => {
            r#"# w shell integration (pwsh)
#
# Usage:
#   Invoke-Expression (& w shell init pwsh)
#
# Notes:
# - Defines a `w` function to allow `w cd`/`w new` to change the current directory.
# - The function shells out to the `w` application (not itself) to avoid recursion.

$script:__w_bin = (Get-Command w -CommandType Application).Source

function w {
    param(
        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$wArgs
    )

    if ($wArgs.Count -ge 1 -and ($wArgs[0] -eq 'cd' -or $wArgs[0] -eq 'new')) {
        if ($wArgs -contains '-h' -or $wArgs -contains '--help') {
            & $script:__w_bin @wArgs
            return
        }

        $target = & $script:__w_bin @wArgs
        if ($LASTEXITCODE -ne 0) { return }

        if ($target -is [System.Array]) { $target = $target[-1] }
        if ([string]::IsNullOrWhiteSpace($target)) { return }

        Set-Location -Path $target
        return
    }

    & $script:__w_bin @wArgs
}"#
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
            repo_dir: _,
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
            repo_dir: _,
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
            repo_dir: _,
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
            repo_dir: _,
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
            repo_dir: _,
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
            repo_dir: _,
            command: Command::Prune,
        } = cli
        else {
            panic!("expected w prune");
        };
    }
}
