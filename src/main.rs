use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use herdr_scratch_pane::actions::{
    clear_marker_args, close_pane_args, minimize_decision, notification_args, open_pane_args,
    open_target_for_current, pane_current_args, pane_list_args, report_marker_args,
    safe_split_decision, split_pane_args, zoom_pane_args, MinimizeDecision, OpenPaneRequest,
    SafeSplitDecision, SplitDirection,
};
use herdr_scratch_pane::herdr::{
    parse_current_pane, parse_opened_pane_id, parse_pane_list, PaneInfo,
};
use herdr_scratch_pane::keybindings::install_keybindings_text;
use herdr_scratch_pane::scope::Scope;
use herdr_scratch_pane::status::{
    choose_marker_target, default_state_dir, dtach_socket_path, read_state, remove_state,
    state_path, write_state, ScratchState,
};
use herdr_scratch_pane::toggle::{decide_toggle, ToggleDecision, ToggleInputs};

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
        Commands::Toggle { scope } => toggle(scope.into()),
        Commands::Minimize => minimize(),
        Commands::SafeSplit { direction } => safe_split(direction.into()),
        Commands::InstallKeybindings {
            workspace_key,
            session_key,
            minimize_key,
            no_split_proxy,
            config,
        } => install_keybindings(
            config,
            &workspace_key,
            &session_key,
            &minimize_key,
            !no_split_proxy,
        ),
        Commands::RunPane { scope } => herdr_scratch_pane::pane::run(scope.into()),
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
        panes: panes.clone(),
        server_id,
    });

    match decision {
        ToggleDecision::Open { scope } => open_and_zoom(&herdr, scope, &current),
        ToggleDecision::Reveal { pane_id } => {
            clear_background_marker(&herdr, scope, &current, &panes)?;
            herdr.run(zoom_pane_args(&pane_id)).map(|_| ())
        }
        ToggleDecision::Close { pane_id } => {
            close_scratch_and_mark(&herdr, &pane_id, scope, &current, &panes)
        }
        ToggleDecision::CloseThenOpen {
            close_pane_id,
            scope,
        } => {
            if let Some(close_scope) = panes
                .iter()
                .find(|pane| pane.pane_id == close_pane_id)
                .and_then(scope_from_scratch_pane)
            {
                let _ =
                    close_scratch_and_mark(&herdr, &close_pane_id, close_scope, &current, &panes);
            } else {
                let _ = herdr.run(close_pane_args(&close_pane_id));
            }
            open_and_zoom(&herdr, scope, &current)
        }
    }
}

fn minimize() -> Result<()> {
    let herdr = Herdr::from_env();
    let current = current_pane(&herdr)?;
    let panes = pane_list(&herdr).unwrap_or_default();
    match minimize_decision(&current, &panes) {
        MinimizeDecision::Close { pane_id } => {
            let scope = panes
                .iter()
                .find(|pane| pane.pane_id == pane_id)
                .or_else(|| (current.pane_id == pane_id).then_some(&current))
                .and_then(scope_from_scratch_pane)
                .unwrap_or(Scope::Workspace);
            close_scratch_and_mark(&herdr, &pane_id, scope, &current, &panes)
        }
        MinimizeDecision::NotifyNoVisiblePane => herdr
            .run(notification_args(
                "No visible herdr-scratch-pane to minimize",
            ))
            .map(|_| ()),
    }
}

fn safe_split(direction: SplitDirection) -> Result<()> {
    let herdr = Herdr::from_env();
    let current = current_pane(&herdr)?;
    let panes = pane_list(&herdr).unwrap_or_default();
    match safe_split_decision(&current, &panes, direction) {
        SafeSplitDecision::Split { direction } => herdr.run(split_pane_args(direction)).map(|_| ()),
        SafeSplitDecision::NotifyBlocked => herdr
            .run(notification_args("Scratch pane split is disabled"))
            .map(|_| ()),
    }
}

fn open_and_zoom(herdr: &Herdr, scope: Scope, current: &PaneInfo) -> Result<()> {
    let panes = pane_list(herdr).unwrap_or_default();
    clear_background_marker(herdr, scope, current, &panes)?;

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
    let state = ScratchState {
        scope,
        workspace_id: current.workspace_id.clone(),
        host_pane_id: choose_marker_target(None, &panes, current)
            .unwrap_or_else(|| current.pane_id.clone()),
        scratch_pane_id: Some(pane_id.clone()),
    };
    write_state(&state_file(scope, current.workspace_id.as_deref()), &state)?;
    herdr.run(zoom_pane_args(&pane_id)).map(|_| ())
}

fn close_scratch_and_mark(
    herdr: &Herdr,
    pane_id: &str,
    scope: Scope,
    current: &PaneInfo,
    panes: &[PaneInfo],
) -> Result<()> {
    let state_path = state_file(scope, current.workspace_id.as_deref());
    let state = read_state(&state_path).unwrap_or_default();
    let marker_target = choose_marker_target(state.as_ref(), panes, current);

    herdr.run(close_pane_args(pane_id))?;

    if let Some(target) = marker_target {
        let _ = herdr.run(report_marker_args(&target, scope));
    }
    Ok(())
}

fn clear_background_marker(
    herdr: &Herdr,
    scope: Scope,
    current: &PaneInfo,
    panes: &[PaneInfo],
) -> Result<()> {
    let path = state_file(scope, current.workspace_id.as_deref());
    let state = read_state(&path).unwrap_or_default();
    if let Some(state) = state.as_ref() {
        if let Some(target) = choose_marker_target(Some(state), panes, current) {
            let _ = herdr.run(clear_marker_args(&target));
        }

        let state_dir = default_state_dir();
        let socket = dtach_socket_path(
            &state_dir,
            scope,
            state
                .workspace_id
                .as_deref()
                .or(current.workspace_id.as_deref()),
            std::env::var("HERDR_SERVER_ID").ok().as_deref(),
        );
        if !socket.exists() {
            remove_state(&path)?;
        }
    }
    Ok(())
}

fn state_file(scope: Scope, workspace_id: Option<&str>) -> PathBuf {
    let state_dir = default_state_dir();
    let server_id = std::env::var("HERDR_SERVER_ID").ok();
    state_path(&state_dir, scope, workspace_id, server_id.as_deref())
}

fn scope_from_scratch_pane(pane: &PaneInfo) -> Option<Scope> {
    match pane.label.as_deref() {
        Some("⌂ scratch workspace") => Some(Scope::Workspace),
        Some("⌂ scratch session") => Some(Scope::Session),
        _ => None,
    }
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
    install_split_proxy: bool,
) -> Result<()> {
    let path = config_path.unwrap_or_else(default_config_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let updated = install_keybindings_text(
        &existing,
        workspace_key,
        session_key,
        minimize_key,
        install_split_proxy,
    )?;
    if updated != existing {
        backup_config(&path, &existing)?;
        fs::write(&path, updated).with_context(|| format!("failed to write {}", path.display()))?;
    }
    println!(
        "Keybindings installed in {}. Reload Herdr config to apply them.",
        path.display()
    );
    Ok(())
}

fn backup_config(path: &std::path::Path, existing: &str) -> Result<()> {
    if existing.is_empty() {
        return Ok(());
    }
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_path = path.with_extension(format!("toml.bak.{timestamp}"));
    fs::write(&backup_path, existing)
        .with_context(|| format!("failed to write backup {}", backup_path.display()))
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
