use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use herdr_floating_pane::actions::{
    close_pane_args, minimize_decision, notification_args, open_pane_args, open_target_for_current,
    pane_current_args, pane_list_args, zoom_pane_args, MinimizeDecision, OpenPaneRequest,
};
use herdr_floating_pane::herdr::{
    parse_current_pane, parse_opened_pane_id, parse_pane_list, PaneInfo,
};
use herdr_floating_pane::keybindings::install_keybindings_text;
use herdr_floating_pane::scope::Scope;
use herdr_floating_pane::toggle::{decide_toggle, ToggleDecision, ToggleInputs};

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
        eprintln!("herdr-floating-pane: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Toggle { scope } => toggle(scope.into()),
        Commands::Minimize => minimize(),
        Commands::InstallKeybindings {
            workspace_key,
            session_key,
            minimize_key,
            config,
        } => install_keybindings(config, &workspace_key, &session_key, &minimize_key),
        Commands::RunPane { scope } => herdr_floating_pane::pane::run(scope.into()),
    }
}

fn toggle(scope: Scope) -> Result<()> {
    let herdr = Herdr::from_env();
    let current = current_pane(&herdr)?;
    let panes = pane_list(&herdr).unwrap_or_default();
    let server_id = std::env::var("HERDR_SERVER_ID").ok();
    let decision = decide_toggle(ToggleInputs {
        scope,
        current: current.clone(),
        panes,
        server_id,
    });

    match decision {
        ToggleDecision::Open { scope } => open_and_zoom(&herdr, scope, &current),
        ToggleDecision::Reveal { pane_id } => herdr.run(zoom_pane_args(&pane_id)).map(|_| ()),
        ToggleDecision::Close { pane_id } => herdr.run(close_pane_args(&pane_id)).map(|_| ()),
        ToggleDecision::CloseThenOpen {
            close_pane_id,
            scope,
        } => {
            let _ = herdr.run(close_pane_args(&close_pane_id));
            open_and_zoom(&herdr, scope, &current)
        }
    }
}

fn minimize() -> Result<()> {
    let herdr = Herdr::from_env();
    let current = current_pane(&herdr)?;
    let panes = pane_list(&herdr).unwrap_or_default();
    match minimize_decision(&current, &panes) {
        MinimizeDecision::Close { pane_id } => herdr.run(close_pane_args(&pane_id)).map(|_| ()),
        MinimizeDecision::NotifyNoVisiblePane => herdr
            .run(notification_args(
                "No visible herdr-floating-pane to minimize",
            ))
            .map(|_| ()),
    }
}

fn open_and_zoom(herdr: &Herdr, scope: Scope, current: &PaneInfo) -> Result<()> {
    let cwd = current.cwd.clone().or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.display().to_string())
    });
    let stdout = herdr.run(open_pane_args(OpenPaneRequest {
        scope,
        target_pane_id: open_target_for_current(current),
        cwd,
    }))?;
    let pane_id = parse_opened_pane_id(&stdout).context("Herdr did not return a pane id")?;
    herdr.run(zoom_pane_args(&pane_id)).map(|_| ())
}

fn current_pane(herdr: &Herdr) -> Result<PaneInfo> {
    let stdout = herdr.run(pane_current_args())?;
    parse_current_pane(&stdout).context("failed to parse `herdr pane current` output")
}

fn pane_list(herdr: &Herdr) -> Result<Vec<PaneInfo>> {
    let stdout = herdr.run(pane_list_args())?;
    parse_pane_list(&stdout).context("failed to parse `herdr pane list` output")
}

fn install_keybindings(
    config_path: Option<PathBuf>,
    workspace_key: &str,
    session_key: &str,
    minimize_key: &str,
) -> Result<()> {
    let path = config_path.unwrap_or_else(default_config_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let updated = install_keybindings_text(&existing, workspace_key, session_key, minimize_key);
    if updated != existing {
        fs::write(&path, updated).with_context(|| format!("failed to write {}", path.display()))?;
    }
    println!(
        "Keybindings installed in {}. Reload Herdr config to apply them.",
        path.display()
    );
    Ok(())
}

fn default_config_path() -> PathBuf {
    if let Ok(dir) = std::env::var("HERDR_CONFIG_DIR") {
        return PathBuf::from(dir).join("config.toml");
    }
    let home = std::env::var_os("HOME").unwrap_or_else(|| OsString::from("."));
    PathBuf::from(home).join(".config/herdr/config.toml")
}

struct Herdr {
    bin: OsString,
}

impl Herdr {
    fn from_env() -> Self {
        Self {
            bin: std::env::var_os("HERDR_BIN_PATH").unwrap_or_else(|| OsString::from("herdr")),
        }
    }

    fn run(&self, args: Vec<String>) -> Result<String> {
        let output = Command::new(&self.bin)
            .args(&args)
            .output()
            .with_context(|| format!("failed to run Herdr command: {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            bail!(
                "Herdr command failed: {}\nstdout: {}\nstderr: {}",
                args.join(" "),
                stdout.trim(),
                stderr.trim()
            );
        }

        String::from_utf8(output.stdout).map_err(|error| anyhow!(error))
    }
}
