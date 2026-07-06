use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::scope::{session_identity, session_name, Scope};
use crate::state::default_state_dir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtachCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

pub fn dtach_command(
    scope: Scope,
    workspace_id: Option<&str>,
    server_id: Option<&str>,
    state_dir: &Path,
    shell: &str,
    cwd: Option<&Path>,
) -> DtachCommand {
    let socket = state_dir.join(format!(
        "{}.dtach",
        session_name(scope, workspace_id, server_id)
    ));
    DtachCommand {
        program: "dtach".into(),
        args: vec![
            "-A".into(),
            socket.display().to_string(),
            "-z".into(),
            shell.into(),
            "-l".into(),
        ],
        cwd: cwd.map(Path::to_path_buf),
    }
}

pub fn run(scope: Scope) -> Result<()> {
    if !command_exists("dtach") {
        bail!(
            "dtach is required for persistent scratch panes. Install it with `brew install dtach`."
        );
    }

    let state_dir = state_dir();
    std::fs::create_dir_all(&state_dir)
        .with_context(|| format!("failed to create state dir {}", state_dir.display()))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let workspace_id = std::env::var("HERDR_WORKSPACE_ID").ok();
    let session_id = session_identity(
        std::env::var("HERDR_SERVER_ID").ok().as_deref(),
        std::env::var("HERDR_SOCKET_PATH").ok().as_deref(),
    );
    let cwd = std::env::var("HERDR_SCRATCH_PANE_CWD")
        .ok()
        .map(PathBuf::from);
    let dtach = dtach_command(
        scope,
        workspace_id.as_deref(),
        session_id.as_deref(),
        &state_dir,
        &shell,
        cwd.as_deref(),
    );

    exec_dtach(dtach)
}

#[cfg(unix)]
fn exec_dtach(dtach: DtachCommand) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let mut command = Command::new(&dtach.program);
    command.args(&dtach.args);
    command.env(
        "TERM",
        std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".into()),
    );
    if let Some(cwd) = dtach.cwd {
        if cwd.is_dir() {
            command.current_dir(cwd);
        }
    }

    let error = command.exec();
    Err(error).context("failed to exec dtach")
}

#[cfg(not(unix))]
fn exec_dtach(_dtach: DtachCommand) -> Result<()> {
    bail!("herdr-scratch-pane currently supports Unix-like systems only")
}

fn command_exists(program: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {program} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn state_dir() -> PathBuf {
    default_state_dir()
}
