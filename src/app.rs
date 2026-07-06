use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::commands::{
    clear_legacy_marker_args, close_pane_args, notification_args, open_pane_args, pane_run_command,
    rename_scratch_pane_args, run_pane_args, workspace_rename_args, zoom_pane_args,
    OpenPaneRequest,
};
use crate::decisions::{
    decide_toggle, minimize_decision, open_target_for_current, MinimizeDecision, ToggleDecision,
    ToggleInputs,
};
use crate::herdr::{parse_opened_pane_id, Herdr, PaneInfo};
use crate::keybindings::install_keybindings_text;
use crate::scope::{session_identity, Scope};
use crate::state::{
    default_state_dir, dtach_socket_path, read_state, remove_state, state_path, write_state,
    ScratchState,
};
use crate::workspace_marker::{
    legacy_marker_cleanup_target, marked_workspace_label, original_workspace_label,
    restore_workspace_label,
};

pub fn toggle(scope: Scope) -> Result<()> {
    let herdr = Herdr::from_env();
    let current = herdr.current_pane()?;
    let panes = herdr.pane_list()?;
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

pub fn minimize() -> Result<()> {
    let herdr = Herdr::from_env();
    let current = herdr.current_pane()?;
    let panes = herdr.pane_list()?;
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

pub fn install_keybindings(
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
    let existing = read_config_or_empty(&path)?;
    let binary = current_binary_path()?;
    let updated =
        install_keybindings_text(&existing, workspace_key, session_key, minimize_key, &binary)?;
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

fn open_and_zoom(herdr: &Herdr, scope: Scope, current: &PaneInfo) -> Result<()> {
    let panes = herdr.pane_list()?;
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
    herdr.run(rename_scratch_pane_args(&pane_id, scope))?;
    let state = ScratchState {
        scope,
        workspace_id: current.workspace_id.clone(),
        host_pane_id: legacy_marker_cleanup_target(None, &panes, current)
            .unwrap_or_else(|| current.pane_id.clone()),
        scratch_pane_id: Some(pane_id.clone()),
        original_workspace_label: None,
        marked_workspace_label: None,
    };
    write_state(&state_file(scope, current.workspace_id.as_deref()), &state)?;
    let binary = current_binary_path()?;
    herdr.run(run_pane_args(&pane_id, &pane_run_command(&binary, scope)))?;
    herdr.run(zoom_pane_args(&pane_id)).map(|_| ())
}

fn close_scratch_and_mark(
    herdr: &Herdr,
    pane_id: &str,
    scope: Scope,
    current: &PaneInfo,
    panes: &[PaneInfo],
) -> Result<()> {
    let path = state_file(scope, current.workspace_id.as_deref());
    let mut state = read_state(&path)?.unwrap_or(ScratchState {
        scope,
        workspace_id: current.workspace_id.clone(),
        host_pane_id: current.pane_id.clone(),
        scratch_pane_id: Some(pane_id.to_string()),
        original_workspace_label: None,
        marked_workspace_label: None,
    });
    let legacy_marker_target = legacy_marker_cleanup_target(Some(&state), panes, current);

    herdr.run(close_pane_args(pane_id))?;

    if let Some(target) = legacy_marker_target {
        let _ = herdr.run(clear_legacy_marker_args(&target));
    }
    mark_workspace_label(herdr, &mut state, current.workspace_id.as_deref())?;
    write_state(&path, &state)?;
    Ok(())
}

fn clear_background_marker(
    herdr: &Herdr,
    scope: Scope,
    current: &PaneInfo,
    panes: &[PaneInfo],
) -> Result<()> {
    let path = state_file(scope, current.workspace_id.as_deref());
    if let Some(mut state) = read_state(&path)? {
        if let Some(target) = legacy_marker_cleanup_target(Some(&state), panes, current) {
            let _ = herdr.run(clear_legacy_marker_args(&target));
        }
        restore_workspace_marker(herdr, &mut state, current.workspace_id.as_deref())?;

        let state_dir = default_state_dir();
        let session_id = session_identity(
            std::env::var("HERDR_SERVER_ID").ok().as_deref(),
            std::env::var("HERDR_SOCKET_PATH").ok().as_deref(),
        );
        let socket = dtach_socket_path(
            &state_dir,
            scope,
            state
                .workspace_id
                .as_deref()
                .or(current.workspace_id.as_deref()),
            session_id.as_deref(),
        );
        if !socket.exists() {
            remove_state(&path)?;
        } else {
            write_state(&path, &state)?;
        }
    }
    Ok(())
}

fn mark_workspace_label(
    herdr: &Herdr,
    state: &mut ScratchState,
    fallback_workspace_id: Option<&str>,
) -> Result<()> {
    let Some(workspace_id) = state.workspace_id.as_deref().or(fallback_workspace_id) else {
        return Ok(());
    };

    let workspace = herdr.workspace_info(workspace_id)?;
    let current_label = workspace
        .label
        .as_deref()
        .unwrap_or(workspace.workspace_id.as_str());
    let original = state
        .original_workspace_label
        .clone()
        .unwrap_or_else(|| original_workspace_label(current_label));
    let marked = marked_workspace_label(&original);

    if current_label != marked {
        herdr.run(workspace_rename_args(workspace_id, &marked))?;
    }
    state.workspace_id = Some(workspace_id.to_string());
    state.original_workspace_label = Some(original);
    state.marked_workspace_label = Some(marked);
    Ok(())
}

fn restore_workspace_marker(
    herdr: &Herdr,
    state: &mut ScratchState,
    fallback_workspace_id: Option<&str>,
) -> Result<()> {
    let Some(workspace_id) = state.workspace_id.as_deref().or(fallback_workspace_id) else {
        state.original_workspace_label = None;
        state.marked_workspace_label = None;
        return Ok(());
    };

    let workspace = herdr.workspace_info(workspace_id)?;
    if let Some(label) = workspace.label.as_deref() {
        if let Some(original) = restore_workspace_label(state, label) {
            herdr.run(workspace_rename_args(workspace_id, &original))?;
        }
    }
    state.original_workspace_label = None;
    state.marked_workspace_label = None;
    Ok(())
}

fn state_file(scope: Scope, workspace_id: Option<&str>) -> PathBuf {
    let state_dir = default_state_dir();
    let session_id = session_identity(
        std::env::var("HERDR_SERVER_ID").ok().as_deref(),
        std::env::var("HERDR_SOCKET_PATH").ok().as_deref(),
    );
    state_path(&state_dir, scope, workspace_id, session_id.as_deref())
}

fn current_binary_path() -> Result<String> {
    std::env::current_exe()
        .context("failed to resolve current executable path")
        .map(|path| path.display().to_string())
}

fn scope_from_scratch_pane(pane: &PaneInfo) -> Option<Scope> {
    match pane.label.as_deref() {
        Some("⌂ scratch workspace") => Some(Scope::Workspace),
        Some("⌂ scratch session") => Some(Scope::Session),
        _ => None,
    }
}

fn read_config_or_empty(path: &Path) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(config) => Ok(config),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => {
            Err(error).with_context(|| format!("failed to read config {}", path.display()))
        }
    }
}

fn backup_config(path: &Path, existing: &str) -> Result<()> {
    if existing.is_empty() {
        return Ok(());
    }
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before unix epoch")?
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
