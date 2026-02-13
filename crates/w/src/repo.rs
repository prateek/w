use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use worktrunk::git::Repository;

#[derive(Debug, Deserialize)]
pub(crate) struct WConfig {
    #[serde(default)]
    pub(crate) repo_roots: Vec<PathBuf>,
    #[serde(default = "default_max_depth")]
    pub(crate) max_depth: usize,
    #[serde(default = "default_max_concurrent_repos")]
    pub(crate) max_concurrent_repos: usize,
    #[serde(default)]
    pub(crate) ls: LsConfig,
}

fn default_max_depth() -> usize {
    6
}

fn default_max_concurrent_repos() -> usize {
    4
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct LsConfig {
    pub(crate) preset: Option<crate::LsTextPreset>,
    pub(crate) sort: Option<crate::LsSort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepoIndex {
    pub(crate) schema_version: u32,
    pub(crate) repos: Vec<RepoEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepoEntry {
    pub(crate) path: String,
    pub(crate) project_identifier: String,
}

pub(crate) fn default_config_path() -> anyhow::Result<PathBuf> {
    Ok(xdg_config_dir()?.join("w").join("config.toml"))
}

pub(crate) fn default_cache_path() -> anyhow::Result<PathBuf> {
    Ok(xdg_cache_dir()?.join("w").join("repo-index.json"))
}

pub(crate) fn load_config(config_path: &Path) -> anyhow::Result<WConfig> {
    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config file: {}", config_path.display()))?;
    let mut config: WConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse TOML: {}", config_path.display()))?;
    config.repo_roots = config
        .repo_roots
        .into_iter()
        .map(|root| expand_tilde(&root))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(config)
}

pub(crate) fn build_repo_index(roots: &[PathBuf], max_depth: usize) -> anyhow::Result<RepoIndex> {
    let mut candidates = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        discover_repo_roots(root, 0, max_depth, &mut candidates)?;
    }

    let mut seen = HashSet::<String>::new();
    let mut repos = Vec::new();

    for path in candidates {
        let path = canonicalize_best_effort(&path);
        let path_str = path.to_string_lossy().to_string();
        if !seen.insert(path_str.clone()) {
            continue;
        }

        let repo = match Repository::at(&path) {
            Ok(repo) => repo,
            Err(_) => continue,
        };
        let project_identifier = repo
            .project_identifier()
            .unwrap_or_else(|_| path_str.clone());

        repos.push(RepoEntry {
            path: path_str,
            project_identifier,
        });
    }

    repos.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(RepoIndex {
        schema_version: 1,
        repos,
    })
}

pub(crate) fn read_repo_index_cache(cache_path: &Path) -> anyhow::Result<RepoIndex> {
    let content = std::fs::read_to_string(cache_path)
        .with_context(|| format!("failed to read cache file: {}", cache_path.display()))?;
    let index: RepoIndex = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse cache JSON: {}", cache_path.display()))?;
    Ok(index)
}

pub(crate) fn write_repo_index_cache(cache_path: &Path, index: &RepoIndex) -> anyhow::Result<()> {
    let Some(parent) = cache_path.parent() else {
        anyhow::bail!(
            "cache path has no parent directory: {}",
            cache_path.display()
        );
    };
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create cache dir: {}", parent.display()))?;

    let tmp_path = cache_path.with_extension("tmp");
    let json = serde_json::to_string_pretty(index).context("failed to serialize cache JSON")?;
    std::fs::write(&tmp_path, json)
        .with_context(|| format!("failed to write {}", tmp_path.display()))?;

    std::fs::rename(&tmp_path, cache_path).with_context(|| {
        format!(
            "failed to atomically replace cache file at {}",
            cache_path.display()
        )
    })?;

    Ok(())
}

pub(crate) fn select_repo_by_filter(index: &RepoIndex, filter: &str) -> Option<PathBuf> {
    let needle = filter.to_lowercase();
    index
        .repos
        .iter()
        .find(|repo| {
            repo.path.to_lowercase().contains(&needle)
                || repo.project_identifier.to_lowercase().contains(&needle)
        })
        .map(|repo| PathBuf::from(&repo.path))
}

#[cfg(windows)]
pub(crate) fn pick_repo_interactive(_index: &RepoIndex) -> anyhow::Result<Option<PathBuf>> {
    anyhow::bail!(
        "interactive picker is not supported on Windows; pass --filter for non-interactive selection"
    );
}

#[cfg(not(windows))]
pub(crate) fn pick_repo_interactive(index: &RepoIndex) -> anyhow::Result<Option<PathBuf>> {
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
        .prompt("repo> ".into())
        .build()
        .context("failed to build skim options")?;

    let input = index
        .repos
        .iter()
        .map(|repo| format!("{}\t{}", repo.project_identifier, repo.path))
        .collect::<Vec<_>>()
        .join("\n");

    let items = SkimItemReader::default().of_bufread(Cursor::new(input));
    let out = Skim::run_with(&options, Some(items)).map(|out| out.selected_items);
    let Some(selected) = out.and_then(|items| items.into_iter().next()) else {
        return Ok(None);
    };

    let line = selected.output();
    let line = line.as_ref();
    let path = line.split('\t').nth(1).unwrap_or(line).trim().to_string();

    if path.is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(path)))
}

fn discover_repo_roots(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    if is_git_repo_root(dir) {
        out.push(dir.to_path_buf());
        return Ok(());
    }
    if depth >= max_depth {
        return Ok(());
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => return Ok(()),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read dir: {}", dir.display()));
        }
    };

    let mut entries = read_dir.collect::<Result<Vec<_>, _>>().with_context(|| {
        format!(
            "failed while listing entries for directory: {}",
            dir.display()
        )
    })?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let file_name = entry.file_name();
        if is_ignored_dir_name(&file_name) {
            continue;
        }

        discover_repo_roots(&entry.path(), depth + 1, max_depth, out)?;
    }

    Ok(())
}

fn is_git_repo_root(dir: &Path) -> bool {
    let git_dir = dir.join(".git");
    git_dir.metadata().map(|m| m.is_dir()).unwrap_or(false)
}

fn is_ignored_dir_name(name: &OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".git" | ".worktrees" | "node_modules" | "target")
    )
}

fn canonicalize_best_effort(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn xdg_config_dir() -> anyhow::Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.trim().is_empty()
    {
        return Ok(PathBuf::from(xdg));
    }

    #[cfg(windows)]
    if let Ok(appdata) = std::env::var("APPDATA")
        && !appdata.trim().is_empty()
    {
        return Ok(PathBuf::from(appdata));
    }

    Ok(home_dir()?.join(".config"))
}

fn xdg_cache_dir() -> anyhow::Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME")
        && !xdg.trim().is_empty()
    {
        return Ok(PathBuf::from(xdg));
    }

    #[cfg(windows)]
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA")
        && !local_appdata.trim().is_empty()
    {
        return Ok(PathBuf::from(local_appdata));
    }

    Ok(home_dir()?.join(".cache"))
}

fn home_dir() -> anyhow::Result<PathBuf> {
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return Ok(PathBuf::from(home));
    }

    #[cfg(windows)]
    if let Ok(user_profile) = std::env::var("USERPROFILE")
        && !user_profile.trim().is_empty()
    {
        return Ok(PathBuf::from(user_profile));
    }

    anyhow::bail!("cannot determine home directory (set $HOME or $USERPROFILE)")
}

fn expand_tilde(path: &Path) -> anyhow::Result<PathBuf> {
    let path_str = path.to_string_lossy();
    let Some(rest) = path_str.strip_prefix('~') else {
        return Ok(path.to_path_buf());
    };

    if rest.is_empty() {
        return home_dir();
    }

    let rest = rest
        .strip_prefix('/')
        .or_else(|| rest.strip_prefix('\\'))
        .context("tilde paths must be '~' or start with '~/'")?;

    Ok(home_dir()?.join(rest))
}
