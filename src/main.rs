use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use herdr_scratch_pane::commands::SplitDirection;
use herdr_scratch_pane::scope::Scope;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Toggle {
        #[arg(long, value_enum)]
        scope: CliScope,
    },
    Minimize,
    SafeSplit {
        #[arg(long, value_enum)]
        direction: CliSplitDirection,
    },
    InstallKeybindings {
        #[arg(long, default_value = "prefix+f")]
        workspace_key: String,
        #[arg(long, default_value = "prefix+shift+f")]
        session_key: String,
        #[arg(long, default_value = "prefix+cmd+z")]
        minimize_key: String,
        #[arg(long)]
        no_split_proxy: bool,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    RunPane {
        #[arg(long, value_enum)]
        scope: CliScope,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliScope {
    Workspace,
    Session,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliSplitDirection {
    Right,
    Down,
}

impl From<CliScope> for Scope {
    fn from(value: CliScope) -> Self {
        match value {
            CliScope::Workspace => Scope::Workspace,
            CliScope::Session => Scope::Session,
        }
    }
}

impl From<CliSplitDirection> for SplitDirection {
    fn from(value: CliSplitDirection) -> Self {
        match value {
            CliSplitDirection::Right => SplitDirection::Right,
            CliSplitDirection::Down => SplitDirection::Down,
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("herdr-scratch-pane: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Toggle { scope } => herdr_scratch_pane::app::toggle(scope.into()),
        Commands::Minimize => herdr_scratch_pane::app::minimize(),
        Commands::SafeSplit { direction } => herdr_scratch_pane::app::safe_split(direction.into()),
        Commands::InstallKeybindings {
            workspace_key,
            session_key,
            minimize_key,
            no_split_proxy,
            config,
        } => herdr_scratch_pane::app::install_keybindings(
            config,
            &workspace_key,
            &session_key,
            &minimize_key,
            !no_split_proxy,
        ),
        Commands::RunPane { scope } => herdr_scratch_pane::pane::run(scope.into()),
    }
}
