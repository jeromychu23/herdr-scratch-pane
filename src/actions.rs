use crate::herdr::PaneInfo;
use crate::scope::{scratch_label, Scope};
use crate::toggle::is_scratch;

pub const PLUGIN_ID: &str = "herdr-scratch-pane";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPaneRequest {
    pub scope: Scope,
    pub target_pane_id: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinimizeDecision {
    Close { pane_id: String },
    NotifyNoVisiblePane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Right,
    Down,
}

impl SplitDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Down => "down",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafeSplitDecision {
    Split { direction: SplitDirection },
    NotifyBlocked,
}

pub fn open_pane_args(request: OpenPaneRequest) -> Vec<String> {
    let mut args = vec![
        "plugin".into(),
        "pane".into(),
        "open".into(),
        "--plugin".into(),
        PLUGIN_ID.into(),
        "--entrypoint".into(),
        entrypoint(request.scope).into(),
        "--placement".into(),
        "split".into(),
        "--direction".into(),
        "right".into(),
        "--focus".into(),
    ];

    if let Some(target) = request.target_pane_id {
        args.push("--target-pane".into());
        args.push(target);
    }

    args.push("--env".into());
    args.push(format!(
        "HERDR_SCRATCH_PANE_SCOPE={}",
        request.scope.as_str()
    ));

    if let Some(cwd) = request.cwd {
        args.push("--env".into());
        args.push(format!("HERDR_SCRATCH_PANE_CWD={cwd}"));
    }

    args
}

pub fn close_pane_args(pane_id: &str) -> Vec<String> {
    vec![
        "plugin".into(),
        "pane".into(),
        "close".into(),
        pane_id.into(),
    ]
}

pub fn zoom_pane_args(pane_id: &str) -> Vec<String> {
    vec!["pane".into(), "zoom".into(), pane_id.into(), "--on".into()]
}

pub fn pane_list_args() -> Vec<String> {
    vec!["pane".into(), "list".into()]
}

pub fn pane_current_args() -> Vec<String> {
    vec!["pane".into(), "current".into()]
}

pub fn pane_get_args(pane_id: &str) -> Vec<String> {
    vec!["pane".into(), "get".into(), pane_id.into()]
}

pub fn notification_args(message: &str) -> Vec<String> {
    vec!["notification".into(), "show".into(), message.into()]
}

pub fn split_pane_args(direction: SplitDirection) -> Vec<String> {
    vec![
        "pane".into(),
        "split".into(),
        "--current".into(),
        "--direction".into(),
        direction.as_str().into(),
        "--focus".into(),
    ]
}

pub fn report_marker_args(pane_id: &str, scope: Scope) -> Vec<String> {
    vec![
        "pane".into(),
        "report-metadata".into(),
        pane_id.into(),
        "--source".into(),
        PLUGIN_ID.into(),
        "--title".into(),
        "scratch running".into(),
        "--custom-status".into(),
        format!("scratch {}", scope.as_str()),
    ]
}

pub fn clear_marker_args(pane_id: &str) -> Vec<String> {
    vec![
        "pane".into(),
        "report-metadata".into(),
        pane_id.into(),
        "--source".into(),
        PLUGIN_ID.into(),
        "--clear-title".into(),
        "--clear-custom-status".into(),
    ]
}

pub fn safe_split_decision(
    current: &PaneInfo,
    panes: &[PaneInfo],
    direction: SplitDirection,
) -> SafeSplitDecision {
    if is_scratch(current) || panes.iter().any(|pane| pane.focused && is_scratch(pane)) {
        SafeSplitDecision::NotifyBlocked
    } else {
        SafeSplitDecision::Split { direction }
    }
}

pub fn minimize_decision(current: &PaneInfo, panes: &[PaneInfo]) -> MinimizeDecision {
    if is_scratch(current) || current.label.as_deref() == Some(scratch_label(Scope::Workspace)) {
        return MinimizeDecision::Close {
            pane_id: current.pane_id.clone(),
        };
    }

    if let Some(focused) = panes.iter().find(|pane| pane.focused && is_scratch(pane)) {
        return MinimizeDecision::Close {
            pane_id: focused.pane_id.clone(),
        };
    }

    MinimizeDecision::NotifyNoVisiblePane
}

pub fn open_target_for_current(current: &PaneInfo) -> Option<String> {
    (!is_scratch(current)).then(|| current.pane_id.clone())
}

fn entrypoint(scope: Scope) -> &'static str {
    match scope {
        Scope::Workspace => "workspace-scratch",
        Scope::Session => "session-scratch",
    }
}
