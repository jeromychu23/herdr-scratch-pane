use serde::Deserialize;

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

pub fn parse_current_pane(input: &str) -> serde_json::Result<PaneInfo> {
    serde_json::from_str::<CurrentPaneResponse>(input).map(|response| response.result.pane)
}

pub fn parse_pane_list(input: &str) -> serde_json::Result<Vec<PaneInfo>> {
    serde_json::from_str::<PaneListResponse>(input).map(|response| response.result.panes)
}

pub fn parse_opened_pane_id(input: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(input).ok()?;
    value
        .pointer("/result/plugin_pane/pane/pane_id")
        .or_else(|| value.pointer("/result/pane/pane_id"))
        .and_then(|pane_id| pane_id.as_str())
        .map(ToOwned::to_owned)
}
