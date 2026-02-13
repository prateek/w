use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
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
    /// Switch to a worktree across repositories and print its path.
    Switch {
        /// Path to `w` config TOML (defaults to `~/.config/w/config.toml`).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Root directory to scan for git repositories (may be repeated).
        #[arg(long = "root", value_name = "PATH")]
        roots: Vec<PathBuf>,
        /// Maximum directory depth to search under each root.
        #[arg(long)]
        max_depth: Option<usize>,
        /// Maximum number of repositories to process concurrently (overrides config/env).
        #[arg(long, value_name = "N")]
        jobs: Option<usize>,
        /// Cache path for the repo index.
        #[arg(long)]
        cache_path: Option<PathBuf>,
        /// Read from the cache only (do not scan).
        #[arg(long, conflicts_with = "refresh")]
        cached: bool,
        /// Force a rescan and refresh the cache.
        #[arg(long, conflicts_with = "cached")]
        refresh: bool,
        /// Include prunable worktrees (directories deleted but git still tracks metadata).
        #[arg(long)]
        include_prunable: bool,
        /// Non-interactively select the first match (substring match on project identifier, repo path, branch, or worktree path).
        #[arg(long)]
        filter: Option<String>,
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
    /// List worktrees across repositories.
    Ls {
        /// Path to `w` config TOML (defaults to `~/.config/w/config.toml`).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Root directory to scan for git repositories (may be repeated).
        #[arg(long = "root", value_name = "PATH")]
        roots: Vec<PathBuf>,
        /// Maximum directory depth to search under each root.
        #[arg(long)]
        max_depth: Option<usize>,
        /// Maximum number of repositories to process concurrently (overrides config/env).
        #[arg(long, value_name = "N")]
        jobs: Option<usize>,
        /// Cache path for the repo index.
        #[arg(long)]
        cache_path: Option<PathBuf>,
        /// Read from the cache only (do not scan).
        #[arg(long, conflicts_with = "refresh")]
        cached: bool,
        /// Force a rescan and refresh the cache.
        #[arg(long, conflicts_with = "cached")]
        refresh: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = LsFormat::Text)]
        format: LsFormat,
        /// Text preset (applies to `--format text`).
        #[arg(long, value_enum)]
        preset: Option<LsTextPreset>,
        /// Sort order for output.
        #[arg(long, value_enum)]
        sort: Option<LsSort>,
        /// Include prunable worktrees (directories deleted but git still tracks metadata).
        #[arg(long)]
        include_prunable: bool,
    },
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

#[derive(ValueEnum, Clone, Debug)]
enum LsFormat {
    Text,
    Json,
    Tsv,
}

#[derive(ValueEnum, Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LsTextPreset {
    #[value(name = "default")]
    Default,
    #[value(name = "compact")]
    Compact,
    #[value(name = "full")]
    Full,
}

#[derive(ValueEnum, Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LsSort {
    #[value(name = "repo")]
    Repo,
    #[value(name = "project")]
    Project,
    #[value(name = "path")]
    Path,
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
        Command::Switch {
            config,
            roots,
            max_depth,
            jobs,
            cache_path,
            cached,
            refresh,
            include_prunable,
            filter,
        } => {
            let path = cmd_switch(
                repo_dir.as_deref(),
                SwitchPickRequest {
                    config_path: config,
                    roots,
                    max_depth,
                    jobs,
                    cache_path,
                    cached,
                    refresh,
                    include_prunable,
                    filter,
                },
            )?;
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
        Command::Ls {
            config,
            roots,
            max_depth,
            jobs,
            cache_path,
            cached,
            refresh,
            format,
            preset,
            sort,
            include_prunable,
        } => {
            if preset.is_some() && !matches!(format, LsFormat::Text) {
                anyhow::bail!("--preset is only supported with --format text");
            }

            let config_for_formatting =
                load_w_config_for_ls_formatting(repo_dir.as_deref(), config.as_deref(), &roots)?;
            let sort = sort
                .or_else(|| config_for_formatting.as_ref().and_then(|c| c.ls.sort))
                .unwrap_or(LsSort::Repo);
            let preset = preset
                .or_else(|| config_for_formatting.as_ref().and_then(|c| c.ls.preset))
                .unwrap_or(LsTextPreset::Default);

            let mut output = cmd_ls(
                repo_dir.as_deref(),
                LsRequest {
                    config_path: config,
                    roots,
                    max_depth,
                    jobs,
                    cache_path,
                    cached,
                    refresh,
                    include_prunable,
                },
            )?;

            if !output.errors.is_empty() {
                for err in &output.errors {
                    eprintln!("w ls: {}: {}", err.repo_path, err.error);
                }
            }

            sort_ls_worktrees(&mut output.worktrees, sort);

            match format {
                LsFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                LsFormat::Tsv => {
                    for wt in &output.worktrees {
                        println!(
                            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                            wt.project_identifier,
                            wt.repo_path,
                            wt.path,
                            wt.branch.as_deref().unwrap_or(""),
                            wt.head,
                            wt.detached,
                            wt.locked.as_deref().unwrap_or(""),
                            wt.prunable.as_deref().unwrap_or(""),
                        );
                    }
                }
                LsFormat::Text => {
                    for wt in &output.worktrees {
                        let branch = worktree_branch_display(wt);
                        match preset {
                            LsTextPreset::Compact => {
                                println!("{}\t{}", wt.project_identifier, branch);
                            }
                            LsTextPreset::Default => {
                                println!("{}\t{}\t{}", wt.project_identifier, branch, wt.path);
                            }
                            LsTextPreset::Full => {
                                println!(
                                    "{}\t{}\t{}\t{}\t{}",
                                    wt.project_identifier,
                                    branch,
                                    wt.path,
                                    wt.locked.as_deref().unwrap_or(""),
                                    wt.prunable.as_deref().unwrap_or(""),
                                );
                            }
                        }
                    }
                }
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

struct SwitchPickRequest {
    config_path: Option<PathBuf>,
    roots: Vec<PathBuf>,
    max_depth: Option<usize>,
    jobs: Option<usize>,
    cache_path: Option<PathBuf>,
    cached: bool,
    refresh: bool,
    include_prunable: bool,
    filter: Option<String>,
}

fn cmd_switch(repo_dir: Option<&Path>, request: SwitchPickRequest) -> anyhow::Result<PathBuf> {
    let SwitchPickRequest {
        config_path,
        roots,
        max_depth,
        jobs,
        cache_path,
        cached,
        refresh,
        include_prunable,
        filter,
    } = request;

    let output = cmd_ls(
        repo_dir,
        LsRequest {
            config_path,
            roots,
            max_depth,
            jobs,
            cache_path,
            cached,
            refresh,
            include_prunable,
        },
    )?;

    if !output.errors.is_empty() {
        for err in &output.errors {
            eprintln!("w switch: {}: {}", err.repo_path, err.error);
        }
    }

    if output.worktrees.is_empty() {
        anyhow::bail!("no worktrees found");
    }

    if let Some(filter) = filter {
        let selected = select_worktree_by_filter(&output.worktrees, &filter)
            .ok_or_else(|| anyhow::anyhow!("no worktree matched filter: {filter}"))?;
        return Ok(PathBuf::from(&selected.path));
    }

    pick_worktree_interactive(&output.worktrees)?.context("no worktree selected")
}

fn select_worktree_by_filter<'a>(
    worktrees: &'a [LsWorktree],
    filter: &str,
) -> Option<&'a LsWorktree> {
    let needle = filter.to_lowercase();
    worktrees.iter().find(|wt| {
        wt.project_identifier.to_lowercase().contains(&needle)
            || wt.repo_path.to_lowercase().contains(&needle)
            || wt.path.to_lowercase().contains(&needle)
            || wt
                .branch
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains(&needle)
    })
}

#[cfg(windows)]
fn pick_worktree_interactive(_worktrees: &[LsWorktree]) -> anyhow::Result<Option<PathBuf>> {
    anyhow::bail!(
        "interactive picker is not supported on Windows; pass --filter for non-interactive selection"
    );
}

#[cfg(not(windows))]
fn pick_worktree_interactive(worktrees: &[LsWorktree]) -> anyhow::Result<Option<PathBuf>> {
    use std::io::{Cursor, IsTerminal};

    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "interactive picker requires a TTY (stdin); pass --filter for non-interactive selection"
        );
    }

    use skim::prelude::*;

    let options = SkimOptionsBuilder::default()
        .height("50%".into())
        .multi(false)
        .prompt("worktree> ".into())
        .build()
        .context("failed to build skim options")?;

    let input = worktrees
        .iter()
        .map(|wt| {
            let branch =
                wt.branch
                    .as_deref()
                    .unwrap_or(if wt.detached { "(detached)" } else { "" });
            format!("{}\t{}\t{}", wt.project_identifier, branch, wt.path)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let items = SkimItemReader::default().of_bufread(Cursor::new(input));
    let out = Skim::run_with(&options, Some(items)).map(|out| out.selected_items);
    let Some(selected) = out.and_then(|items| items.into_iter().next()) else {
        return Ok(None);
    };

    let line = selected.output();
    let line = line.as_ref();
    let path = line.split('\t').nth(2).unwrap_or(line).trim().to_string();

    if path.is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(path)))
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

#[derive(Debug, Serialize)]
struct LsOutput {
    schema_version: u32,
    worktrees: Vec<LsWorktree>,
    errors: Vec<LsError>,
}

#[derive(Debug, Serialize)]
struct LsWorktree {
    repo_path: String,
    project_identifier: String,
    path: String,
    branch: Option<String>,
    head: String,
    detached: bool,
    locked: Option<String>,
    prunable: Option<String>,
}

#[derive(Debug, Serialize)]
struct LsError {
    repo_path: String,
    error: String,
}

struct LsRequest {
    config_path: Option<PathBuf>,
    roots: Vec<PathBuf>,
    max_depth: Option<usize>,
    jobs: Option<usize>,
    cache_path: Option<PathBuf>,
    cached: bool,
    refresh: bool,
    include_prunable: bool,
}

const W_MAX_CONCURRENT_REPOS_ENV: &str = "W_MAX_CONCURRENT_REPOS";
const MAX_CONCURRENT_REPOS_CAP: usize = 32;

fn cmd_ls(repo_dir: Option<&Path>, request: LsRequest) -> anyhow::Result<LsOutput> {
    let LsRequest {
        config_path,
        roots,
        max_depth,
        jobs,
        cache_path,
        cached,
        refresh,
        include_prunable,
    } = request;

    if let Some(repo_dir) = repo_dir {
        let repo = Repository::at(repo_dir).context("failed to discover git repo")?;
        let repo_root = canonicalize_best_effort(repo.repo_path());
        let repo_path = repo_root.to_string_lossy().to_string();
        let project_identifier = repo
            .project_identifier()
            .unwrap_or_else(|_| repo_path.clone());

        let mut repo_worktrees = repo.list_worktrees()?;
        repo_worktrees.sort_by(|a, b| a.path.cmp(&b.path));

        let worktrees = repo_worktrees
            .into_iter()
            .filter(|wt| include_prunable || !wt.is_prunable())
            .map(|wt| LsWorktree {
                repo_path: repo_path.clone(),
                project_identifier: project_identifier.clone(),
                path: canonicalize_best_effort(&wt.path)
                    .to_string_lossy()
                    .to_string(),
                branch: wt.branch,
                head: wt.head,
                detached: wt.detached,
                locked: wt.locked,
                prunable: wt.prunable,
            })
            .collect();

        return Ok(LsOutput {
            schema_version: 1,
            worktrees,
            errors: Vec::new(),
        });
    }

    let max_concurrent_repos = max_concurrent_repos(jobs, config_path.as_deref(), &roots)
        .context("failed to read concurrency config")?;

    let cache_path = cache_path.unwrap_or(repo::default_cache_path()?);
    let index = if cached {
        repo::read_repo_index_cache(&cache_path)?
    } else if refresh || !cache_path.exists() {
        let (roots, max_depth) = repo_roots_and_depth(config_path.as_deref(), roots, max_depth)?;
        let index = repo::build_repo_index(&roots, max_depth)?;
        repo::write_repo_index_cache(&cache_path, &index)?;
        index
    } else {
        repo::read_repo_index_cache(&cache_path)?
    };

    let mut repos = Vec::new();
    for entry in index.repos {
        let repo_dir = PathBuf::from(&entry.path);
        repos.push((repo_dir, entry.path, entry.project_identifier));
    }

    let mut worktrees = Vec::new();
    let mut errors = Vec::new();

    if max_concurrent_repos <= 1 || repos.len() <= 1 {
        for (repo_dir, repo_path, project_identifier) in repos {
            match list_repo_worktrees(repo_dir, repo_path, project_identifier, include_prunable) {
                Ok(mut repo_worktrees) => worktrees.append(&mut repo_worktrees),
                Err(err) => errors.push(err),
            }
        }
    } else {
        enum RepoWorktreesMessage {
            Worktrees(Vec<LsWorktree>),
            Error(LsError),
        }

        let worker_count = max_concurrent_repos.min(repos.len());
        let jobs = Arc::new(Mutex::new(VecDeque::from(repos)));
        let (tx, rx) = mpsc::channel::<RepoWorktreesMessage>();

        for _ in 0..worker_count {
            let jobs = Arc::clone(&jobs);
            let tx = tx.clone();
            std::thread::spawn(move || {
                loop {
                    let job = {
                        let mut jobs = jobs.lock().unwrap_or_else(|e| e.into_inner());
                        jobs.pop_front()
                    };
                    let Some((repo_dir, repo_path, project_identifier)) = job else {
                        break;
                    };

                    let msg = match list_repo_worktrees(
                        repo_dir,
                        repo_path,
                        project_identifier,
                        include_prunable,
                    ) {
                        Ok(worktrees) => RepoWorktreesMessage::Worktrees(worktrees),
                        Err(err) => RepoWorktreesMessage::Error(err),
                    };
                    let _ = tx.send(msg);
                }
            });
        }

        drop(tx);

        for msg in rx {
            match msg {
                RepoWorktreesMessage::Worktrees(mut repo_worktrees) => {
                    worktrees.append(&mut repo_worktrees);
                }
                RepoWorktreesMessage::Error(err) => errors.push(err),
            }
        }
    }

    worktrees.sort_by(|a, b| a.repo_path.cmp(&b.repo_path).then(a.path.cmp(&b.path)));
    errors.sort_by(|a, b| a.repo_path.cmp(&b.repo_path).then(a.error.cmp(&b.error)));

    Ok(LsOutput {
        schema_version: 1,
        worktrees,
        errors,
    })
}

fn max_concurrent_repos(
    jobs: Option<usize>,
    config_path: Option<&Path>,
    roots: &[PathBuf],
) -> anyhow::Result<usize> {
    if let Some(value) = jobs {
        return normalize_max_concurrent_repos("--jobs", value);
    }

    if let Some(value) = max_concurrent_repos_from_env()? {
        return Ok(value);
    }

    if let Some(config_path) = config_path {
        let config = repo::load_config(config_path)?;
        return normalize_max_concurrent_repos("max_concurrent_repos", config.max_concurrent_repos);
    }

    if roots.is_empty() {
        let config_path = repo::default_config_path()?;
        if config_path.exists() {
            let config = repo::load_config(&config_path)?;
            return normalize_max_concurrent_repos(
                "max_concurrent_repos",
                config.max_concurrent_repos,
            );
        }
    }

    Ok(default_max_concurrent_repos())
}

fn default_max_concurrent_repos() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().min(4))
        .unwrap_or(4)
}

fn max_concurrent_repos_from_env() -> anyhow::Result<Option<usize>> {
    let Ok(raw) = std::env::var(W_MAX_CONCURRENT_REPOS_ENV) else {
        return Ok(None);
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let value: usize = raw.parse().with_context(|| {
        format!("{W_MAX_CONCURRENT_REPOS_ENV} must be a positive integer, got: {raw:?}")
    })?;
    Some(normalize_max_concurrent_repos(
        W_MAX_CONCURRENT_REPOS_ENV,
        value,
    ))
    .transpose()
}

fn normalize_max_concurrent_repos(name: &str, value: usize) -> anyhow::Result<usize> {
    if value == 0 {
        anyhow::bail!("{name} must be a positive integer (>= 1)");
    }
    Ok(value.min(MAX_CONCURRENT_REPOS_CAP))
}

fn list_repo_worktrees(
    repo_dir: PathBuf,
    repo_path: String,
    project_identifier: String,
    include_prunable: bool,
) -> Result<Vec<LsWorktree>, LsError> {
    let repo = Repository::at(&repo_dir).map_err(|err| LsError {
        repo_path: repo_path.clone(),
        error: err.to_string(),
    })?;

    let mut repo_worktrees = repo.list_worktrees().map_err(|err| LsError {
        repo_path: repo_path.clone(),
        error: err.to_string(),
    })?;
    repo_worktrees.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(repo_worktrees
        .into_iter()
        .filter(|wt| include_prunable || !wt.is_prunable())
        .map(|wt| LsWorktree {
            repo_path: repo_path.clone(),
            project_identifier: project_identifier.clone(),
            path: canonicalize_best_effort(&wt.path)
                .to_string_lossy()
                .to_string(),
            branch: wt.branch,
            head: wt.head,
            detached: wt.detached,
            locked: wt.locked,
            prunable: wt.prunable,
        })
        .collect())
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
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn load_w_config_for_ls_formatting(
    repo_dir: Option<&Path>,
    config_path: Option<&Path>,
    roots: &[PathBuf],
) -> anyhow::Result<Option<repo::WConfig>> {
    if let Some(config_path) = config_path {
        return Ok(Some(repo::load_config(config_path)?));
    }
    if repo_dir.is_some() {
        return Ok(None);
    }
    if !roots.is_empty() {
        return Ok(None);
    }

    let config_path = repo::default_config_path()?;
    if !config_path.exists() {
        return Ok(None);
    }
    Ok(Some(repo::load_config(&config_path)?))
}

fn sort_ls_worktrees(worktrees: &mut [LsWorktree], sort: LsSort) {
    match sort {
        LsSort::Repo => {
            worktrees.sort_by(|a, b| a.repo_path.cmp(&b.repo_path).then(a.path.cmp(&b.path)));
        }
        LsSort::Project => {
            worktrees.sort_by(|a, b| {
                a.project_identifier
                    .cmp(&b.project_identifier)
                    .then(a.path.cmp(&b.path))
                    .then(a.repo_path.cmp(&b.repo_path))
            });
        }
        LsSort::Path => {
            worktrees.sort_by(|a, b| {
                a.path
                    .cmp(&b.path)
                    .then(a.project_identifier.cmp(&b.project_identifier))
                    .then(a.repo_path.cmp(&b.repo_path))
            });
        }
    }
}

fn worktree_branch_display(worktree: &LsWorktree) -> Cow<'_, str> {
    if let Some(branch) = worktree.branch.as_deref() {
        return Cow::Borrowed(branch);
    }
    if worktree.detached {
        return Cow::Borrowed("(detached)");
    }
    Cow::Borrowed("")
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

    canonicalize_best_effort(parent).join(file_name)
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
# - Overrides the `w` shell function to allow `w cd`/`w new`/`w switch` to change the current directory.
# - Use `command w ...` to bypass the function (call the binary directly).

w() {
  case "$1" in
    cd|new|switch)
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
# - Overrides the `w` shell function to allow `w cd`/`w new`/`w switch` to change the current directory.
# - Use `command w ...` to bypass the function (call the binary directly).

w() {
  case "$1" in
    cd|new|switch)
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
# - Overrides the `w` function to allow `w cd`/`w new`/`w switch` to change the current directory.
# - Use `command w ...` to bypass the function (call the binary directly).

function w --wraps w --description 'w wrapper with cd/new/switch'
    if test (count $argv) -ge 1
        set -l sub $argv[1]
        if test "$sub" = "cd" -o "$sub" = "new" -o "$sub" = "switch"
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
# - Defines a `w` function to allow `w cd`/`w new`/`w switch` to change the current directory.
# - The function shells out to the `w` application (not itself) to avoid recursion.

$script:__w_bin = (Get-Command w -CommandType Application).Source

function w {
    param(
        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$wArgs
    )

    if ($wArgs.Count -ge 1 -and ($wArgs[0] -eq 'cd' -or $wArgs[0] -eq 'new' -or $wArgs[0] -eq 'switch')) {
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
    fn switch_parses() {
        let cli = Cli::try_parse_from(["w", "switch", "--filter", "feature"]).unwrap();
        let Cli {
            repo_dir: _,
            command: Command::Switch { filter, .. },
        } = cli
        else {
            panic!("expected w switch");
        };

        assert_eq!(filter.as_deref(), Some("feature"));
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

    #[test]
    fn ls_parses() {
        let cli = Cli::try_parse_from(["w", "ls", "--format", "json"]).unwrap();
        let Cli {
            repo_dir: _,
            command,
        } = cli;
        let Command::Ls { format, .. } = command else {
            panic!("expected w ls");
        };

        assert!(matches!(format, LsFormat::Json));
    }
}
