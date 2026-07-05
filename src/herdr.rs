use std::ffi::OsString;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::commands::{pane_current_args, pane_list_args, workspace_get_args};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PaneInfo {
    pub pane_id: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WorkspaceInfo {
    pub workspace_id: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurrentPaneResponse {
    result: CurrentPaneResult,
}

#[derive(Debug, Deserialize)]
struct CurrentPaneResult {
    pane: PaneInfo,
}

#[derive(Debug, Deserialize)]
struct PaneListResponse {
    result: PaneListResult,
}

#[derive(Debug, Deserialize)]
struct PaneListResult {
    #[serde(default)]
    panes: Vec<PaneInfo>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceGetResponse {
    result: WorkspaceGetResult,
}

#[derive(Debug, Deserialize)]
struct WorkspaceGetResult {
    workspace: WorkspaceInfo,
}

pub fn parse_current_pane(input: &str) -> serde_json::Result<PaneInfo> {
    serde_json::from_str::<CurrentPaneResponse>(input).map(|response| response.result.pane)
}

pub fn parse_pane_list(input: &str) -> serde_json::Result<Vec<PaneInfo>> {
    serde_json::from_str::<PaneListResponse>(input).map(|response| response.result.panes)
}

pub fn parse_workspace_get(input: &str) -> serde_json::Result<WorkspaceInfo> {
    serde_json::from_str::<WorkspaceGetResponse>(input).map(|response| response.result.workspace)
}

pub fn parse_opened_pane_id(input: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(input).ok()?;
    value
        .pointer("/result/plugin_pane/pane/pane_id")
        .or_else(|| value.pointer("/result/pane/pane_id"))
        .and_then(|pane_id| pane_id.as_str())
        .map(ToOwned::to_owned)
}

pub struct Herdr {
    bin: OsString,
}

impl Herdr {
    pub fn from_env() -> Self {
        Self {
            bin: std::env::var_os("HERDR_BIN_PATH").unwrap_or_else(|| OsString::from("herdr")),
        }
    }

    pub fn run(&self, args: Vec<String>) -> Result<String> {
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

    pub fn current_pane(&self) -> Result<PaneInfo> {
        let stdout = self.run(pane_current_args())?;
        parse_current_pane(&stdout).context("failed to parse `herdr pane current` output")
    }

    pub fn pane_list(&self) -> Result<Vec<PaneInfo>> {
        let stdout = self.run(pane_list_args())?;
        parse_pane_list(&stdout).context("failed to parse `herdr pane list` output")
    }

    pub fn workspace_info(&self, workspace_id: &str) -> Result<WorkspaceInfo> {
        let stdout = self.run(workspace_get_args(workspace_id))?;
        parse_workspace_get(&stdout).context("failed to parse `herdr workspace get` output")
    }
}
