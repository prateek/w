mod git;

use clap::{Parser, Subcommand};
use git::{GitError, list_worktrees};

#[derive(Parser)]
#[command(name = "arbor")]
#[command(about = "Git worktree management tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all worktrees
    List,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::List => list_command(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn list_command() -> Result<(), GitError> {
    let worktrees = list_worktrees()?;

    for wt in worktrees {
        println!("{}", wt.path.display());
        println!("  HEAD: {}", &wt.head[..8]);

        if let Some(branch) = wt.branch {
            println!("  branch: {}", branch);
        }

        if wt.detached {
            println!("  (detached)");
        }

        if wt.bare {
            println!("  (bare)");
        }

        if let Some(reason) = wt.locked {
            if reason.is_empty() {
                println!("  (locked)");
            } else {
                println!("  (locked: {})", reason);
            }
        }

        if let Some(reason) = wt.prunable {
            if reason.is_empty() {
                println!("  (prunable)");
            } else {
                println!("  (prunable: {})", reason);
            }
        }

        println!();
    }

    Ok(())
}
