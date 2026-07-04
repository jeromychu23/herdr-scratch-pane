use crate::herdr::PaneInfo;
use crate::scope::{floating_label, Scope};
use crate::toggle::is_floating;

pub const PLUGIN_ID: &str = "herdr-floating-pane";

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
        "HERDR_FLOATING_PANE_SCOPE={}",
        request.scope.as_str()
    ));

    if let Some(cwd) = request.cwd {
        args.push("--env".into());
        args.push(format!("HERDR_FLOATING_PANE_CWD={cwd}"));
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

pub fn minimize_decision(current: &PaneInfo, panes: &[PaneInfo]) -> MinimizeDecision {
    if is_floating(current) || current.label.as_deref() == Some(floating_label(Scope::Workspace)) {
        return MinimizeDecision::Close {
            pane_id: current.pane_id.clone(),
        };
    }

    if let Some(focused) = panes.iter().find(|pane| pane.focused && is_floating(pane)) {
        return MinimizeDecision::Close {
            pane_id: focused.pane_id.clone(),
        };
    }

    MinimizeDecision::NotifyNoVisiblePane
}

pub fn open_target_for_current(current: &PaneInfo) -> Option<String> {
    (!is_floating(current)).then(|| current.pane_id.clone())
}

fn entrypoint(scope: Scope) -> &'static str {
    match scope {
        Scope::Workspace => "workspace-floating",
        Scope::Session => "session-floating",
    }
}
