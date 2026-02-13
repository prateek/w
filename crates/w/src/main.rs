use clap::{Parser, Subcommand, ValueEnum};

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
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
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

fn main() {
    let cli = Cli::parse();
    if let Some(Command::Shell {
        command: ShellCommand::Init { shell },
    }) = cli.command
    {
        println!("{}", shell_init_snippet(shell));
    }
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
                Some(Command::Shell {
                    command: ShellCommand::Init { shell },
                }),
        } = cli
        else {
            panic!("expected w shell init");
        };
        assert!(matches!(shell, Shell::Zsh));
        assert!(shell_init_snippet(shell).contains("zsh"));
    }
}
