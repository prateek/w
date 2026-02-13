use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use worktrunk::{
    config::UserConfig,
    git::Repository,
    integration::v1::{SwitchRequest, switch as worktrunk_switch},
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

fn current_repo_and_config() -> anyhow::Result<(Repository, UserConfig)> {
    let repo = Repository::current().context("failed to discover git repo")?;
    let config = UserConfig::load().context("failed to load Worktrunk config")?;
    Ok((repo, config))
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
}
