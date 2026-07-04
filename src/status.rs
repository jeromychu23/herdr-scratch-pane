use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::herdr::PaneInfo;
use crate::scope::{session_name, Scope};
use crate::toggle::is_scratch;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScratchState {
    pub scope: Scope,
    pub workspace_id: Option<String>,
    pub host_pane_id: String,
    pub scratch_pane_id: Option<String>,
}

pub fn choose_marker_target(
    state: Option<&ScratchState>,
    panes: &[PaneInfo],
    current: &PaneInfo,
) -> Option<String> {
    if let Some(state) = state {
        if panes.iter().any(|pane| pane.pane_id == state.host_pane_id) {
            return Some(state.host_pane_id.clone());
        }
    }

    let workspace = state
        .and_then(|state| state.workspace_id.as_deref())
        .or(current.workspace_id.as_deref());

    panes
        .iter()
        .find(|pane| {
            !is_scratch(pane)
                && match workspace {
                    Some(workspace) => pane.workspace_id.as_deref() == Some(workspace),
                    None => true,
                }
        })
        .map(|pane| pane.pane_id.clone())
        .or_else(|| (!is_scratch(current)).then(|| current.pane_id.clone()))
}

pub fn state_path(
    state_dir: &Path,
    scope: Scope,
    workspace_id: Option<&str>,
    server_id: Option<&str>,
) -> PathBuf {
    state_dir.join(format!(
        "{}.json",
        session_name(scope, workspace_id, server_id)
    ))
}

pub fn dtach_socket_path(
    state_dir: &Path,
    scope: Scope,
    workspace_id: Option<&str>,
    server_id: Option<&str>,
) -> PathBuf {
    state_dir.join(format!(
        "{}.dtach",
        session_name(scope, workspace_id, server_id)
    ))
}

pub fn default_state_dir() -> PathBuf {
    if let Ok(dir) = env::var("HERDR_PLUGIN_STATE_DIR") {
        return PathBuf::from(dir);
    }
    env::temp_dir().join("herdr-scratch-pane")
}

pub fn read_state(path: &Path) -> Result<Option<ScratchState>> {
    if !path.exists() {
        return Ok(None);
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&text)
        .map(Some)
        .with_context(|| format!("failed to parse {}", path.display()))
}

pub fn write_state(path: &Path, state: &ScratchState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(state)?;
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}

pub fn remove_state(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to remove {}", path.display())),
    }
}
