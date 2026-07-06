use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
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
    InstallKeybindings {
        #[arg(long, default_value = "prefix+f")]
        workspace_key: String,
        #[arg(long, default_value = "prefix+shift+f")]
        session_key: String,
        #[arg(long, default_value = "prefix+cmd+z")]
        minimize_key: String,
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

impl From<CliScope> for Scope {
    fn from(value: CliScope) -> Self {
        match value {
            CliScope::Workspace => Scope::Workspace,
            CliScope::Session => Scope::Session,
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
        Commands::InstallKeybindings {
            workspace_key,
            session_key,
            minimize_key,
            config,
        } => herdr_scratch_pane::app::install_keybindings(
            config,
            &workspace_key,
            &session_key,
            &minimize_key,
        ),
        Commands::RunPane { scope } => herdr_scratch_pane::pane::run(scope.into()),
    }
}
