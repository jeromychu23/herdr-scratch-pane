use crate::scope::Scope;

pub const PLUGIN_ID: &str = "herdr-scratch-pane";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPaneRequest {
    pub scope: Scope,
    pub target_pane_id: Option<String>,
    pub cwd: Option<String>,
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

pub fn workspace_get_args(workspace_id: &str) -> Vec<String> {
    vec!["workspace".into(), "get".into(), workspace_id.into()]
}

pub fn workspace_rename_args(workspace_id: &str, label: &str) -> Vec<String> {
    vec![
        "workspace".into(),
        "rename".into(),
        workspace_id.into(),
        label.into(),
    ]
}

pub fn notification_args(message: &str) -> Vec<String> {
    vec!["notification".into(), "show".into(), message.into()]
}

/// Clears pane metadata written by early floating-pane prototypes.
///
/// New scratch state is represented by workspace labels and JSON state files;
/// this command exists only to remove stale titles/custom status from users who
/// tested older builds.
pub fn clear_legacy_marker_args(pane_id: &str) -> Vec<String> {
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

fn entrypoint(scope: Scope) -> &'static str {
    match scope {
        Scope::Workspace => "workspace-scratch",
        Scope::Session => "session-scratch",
    }
}
