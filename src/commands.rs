use crate::scope::{scratch_label, Scope};

pub const PLUGIN_ID: &str = "herdr-scratch-pane";
pub const POPUP_WIDTH: &str = "85%";
pub const POPUP_HEIGHT: &str = "80%";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPaneRequest {
    pub scope: Scope,
    pub target_pane_id: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupOpenRequest {
    pub scope: Scope,
    pub workspace_id: Option<String>,
    pub session_id: String,
    pub state_dir: String,
    pub cwd: Option<String>,
    pub tmux_prefix: String,
}

pub fn open_popup_args(request: PopupOpenRequest) -> Vec<String> {
    let entrypoint = match request.scope {
        Scope::Workspace => "workspace-scratch",
        Scope::Session => "session-scratch",
    };
    let mut args = vec![
        "plugin".into(),
        "pane".into(),
        "open".into(),
        "--plugin".into(),
        PLUGIN_ID.into(),
        "--entrypoint".into(),
        entrypoint.into(),
        "--placement".into(),
        "popup".into(),
        "--width".into(),
        POPUP_WIDTH.into(),
        "--height".into(),
        POPUP_HEIGHT.into(),
    ];

    args.push("--env".into());
    args.push(format!(
        "HERDR_SCRATCH_PANE_SCOPE={}",
        request.scope.as_str()
    ));

    if let Some(workspace_id) = request.workspace_id {
        args.push("--env".into());
        args.push(format!("HERDR_WORKSPACE_ID={workspace_id}"));
    }

    args.push("--env".into());
    args.push(format!(
        "HERDR_SCRATCH_PANE_SESSION_ID={}",
        request.session_id
    ));
    args.push("--env".into());
    args.push(format!(
        "HERDR_SCRATCH_PANE_STATE_DIR={}",
        request.state_dir
    ));

    if let Some(cwd) = request.cwd {
        args.push("--env".into());
        args.push(format!("HERDR_SCRATCH_PANE_CWD={cwd}"));
    }

    args.push("--env".into());
    args.push(format!("HERDR_SCRATCH_PANE_PREFIX={}", request.tmux_prefix));
    args.push("--focus".into());
    args
}

pub fn open_pane_args(request: OpenPaneRequest) -> Vec<String> {
    let mut args = vec!["pane".into(), "split".into()];

    if let Some(target) = request.target_pane_id {
        args.push("--pane".into());
        args.push(target);
    } else {
        args.push("--current".into());
    }

    args.push("--direction".into());
    args.push("right".into());

    if let Some(cwd) = request.cwd.as_deref() {
        args.push("--cwd".into());
        args.push(cwd.into());
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

    args.push("--focus".into());
    args
}

pub fn pane_run_command(binary_path: &str, scope: Scope) -> String {
    format!(
        "exec {} run-pane --scope {}",
        shell_quote(binary_path),
        scope.as_str()
    )
}

pub fn run_pane_args(pane_id: &str, command: &str) -> Vec<String> {
    vec!["pane".into(), "run".into(), pane_id.into(), command.into()]
}

pub fn rename_scratch_pane_args(pane_id: &str, scope: Scope) -> Vec<String> {
    vec![
        "pane".into(),
        "rename".into(),
        pane_id.into(),
        scratch_label(scope).into(),
    ]
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn close_pane_args(pane_id: &str) -> Vec<String> {
    vec!["pane".into(), "close".into(), pane_id.into()]
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
