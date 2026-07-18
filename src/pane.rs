use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::scope::{session_identity, session_name, Scope};
use crate::state::default_state_dir;

pub const TMUX_SERVER_NAME: &str = "herdr-scratch-pane";

#[deprecated(note = "tmux is now the active runtime; retained for legacy dtach recovery")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtachCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[deprecated(note = "tmux is now the active runtime; retained for legacy dtach recovery")]
#[allow(deprecated)]
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
            "-r".into(),
            "winch".into(),
            "-z".into(),
            shell.into(),
            "-l".into(),
        ],
        cwd: cwd.map(Path::to_path_buf),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxCommand {
    pub program: String,
    pub args: Vec<String>,
    pub tmux_tmpdir: PathBuf,
}

pub fn tmux_command(
    scope: Scope,
    workspace_id: Option<&str>,
    server_id: Option<&str>,
    state_dir: &Path,
    shell: &str,
    cwd: Option<&Path>,
) -> TmuxCommand {
    build_tmux_command(scope, workspace_id, server_id, state_dir, shell, cwd, None)
}

pub fn popup_tmux_command(
    scope: Scope,
    workspace_id: Option<&str>,
    server_id: Option<&str>,
    state_dir: &Path,
    shell: &str,
    cwd: Option<&Path>,
    prefix: &str,
) -> TmuxCommand {
    build_tmux_command(
        scope,
        workspace_id,
        server_id,
        state_dir,
        shell,
        cwd,
        Some(prefix),
    )
}

fn build_tmux_command(
    scope: Scope,
    workspace_id: Option<&str>,
    server_id: Option<&str>,
    state_dir: &Path,
    shell: &str,
    cwd: Option<&Path>,
    prefix: Option<&str>,
) -> TmuxCommand {
    let mut args = vec![
        "-L".to_owned(),
        TMUX_SERVER_NAME.to_owned(),
        "-f".to_owned(),
        "/dev/null".to_owned(),
        "start-server".to_owned(),
        ";".to_owned(),
        "set-option".to_owned(),
        "-g".to_owned(),
        "status".to_owned(),
        "off".to_owned(),
        ";".to_owned(),
        "set-option".to_owned(),
        "-g".to_owned(),
        "prefix".to_owned(),
        prefix.unwrap_or("None").to_owned(),
        ";".to_owned(),
        "set-option".to_owned(),
        "-g".to_owned(),
        "prefix2".to_owned(),
        "None".to_owned(),
        ";".to_owned(),
        "set-option".to_owned(),
        "-g".to_owned(),
        "mouse".to_owned(),
        "off".to_owned(),
        ";".to_owned(),
        "set-option".to_owned(),
        "-s".to_owned(),
        "escape-time".to_owned(),
        "0".to_owned(),
        ";".to_owned(),
        "set-option".to_owned(),
        "-g".to_owned(),
        "default-terminal".to_owned(),
        "tmux-256color".to_owned(),
        ";".to_owned(),
        "set-option".to_owned(),
        "-g".to_owned(),
        "remain-on-exit".to_owned(),
        "off".to_owned(),
    ];

    if let Some(prefix) = prefix {
        for binding in [
            vec!["unbind-key", "-a", "-T", "prefix"],
            vec!["bind-key", "-T", "prefix", "f", "detach-client"],
            vec!["bind-key", "-T", "prefix", "F", "detach-client"],
            vec![
                "bind-key",
                "-T",
                "prefix",
                "x",
                "confirm-before",
                "-p",
                "Kill scratch session? (y/n)",
                "kill-session",
            ],
            vec!["bind-key", "-T", "prefix", prefix, "send-prefix"],
        ] {
            args.push(";".to_owned());
            args.extend(binding.into_iter().map(ToOwned::to_owned));
        }
    }

    args.extend([
        ";".to_owned(),
        "new-session".to_owned(),
        "-A".to_owned(),
        "-s".to_owned(),
        session_name(scope, workspace_id, server_id),
    ]);

    if let Some(cwd) = cwd.filter(|path| path.is_dir()) {
        args.push("-c".to_owned());
        args.push(cwd.to_string_lossy().into_owned());
    }

    args.push(shell.to_owned());
    args.push("-l".to_owned());

    TmuxCommand {
        program: "tmux".to_owned(),
        args,
        tmux_tmpdir: state_dir.to_path_buf(),
    }
}

pub fn run(scope: Scope) -> Result<()> {
    run_client(scope).map(|_| ())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionDisposition {
    Detached,
    Ended,
}

pub(crate) fn run_client(scope: Scope) -> Result<SessionDisposition> {
    if !command_exists("tmux") {
        bail!(
            "tmux is required for persistent scratch panes. Install it with `brew install tmux`."
        );
    }

    let state_dir = state_dir();
    std::fs::create_dir_all(&state_dir)
        .with_context(|| format!("failed to create state dir {}", state_dir.display()))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let workspace_id = std::env::var("HERDR_WORKSPACE_ID").ok();
    let session_id = std::env::var("HERDR_SCRATCH_PANE_SESSION_ID")
        .ok()
        .or_else(|| {
            session_identity(
                std::env::var("HERDR_SERVER_ID").ok().as_deref(),
                std::env::var("HERDR_SOCKET_PATH").ok().as_deref(),
            )
        });
    let cwd = std::env::var("HERDR_SCRATCH_PANE_CWD")
        .ok()
        .map(PathBuf::from);
    let prefix = std::env::var("HERDR_SCRATCH_PANE_PREFIX").unwrap_or_else(|_| "C-b".into());
    let tmux = popup_tmux_command(
        scope,
        workspace_id.as_deref(),
        session_id.as_deref(),
        &state_dir,
        &shell,
        cwd.as_deref(),
        &prefix,
    );

    run_tmux_client(tmux)?;
    if tmux_session_exists(
        &state_dir,
        scope,
        workspace_id.as_deref(),
        session_id.as_deref(),
    )? {
        Ok(SessionDisposition::Detached)
    } else {
        Ok(SessionDisposition::Ended)
    }
}

#[cfg(unix)]
fn run_tmux_client(tmux: TmuxCommand) -> Result<()> {
    let mut command = Command::new(&tmux.program);
    command
        .args(&tmux.args)
        .env("TMUX_TMPDIR", &tmux.tmux_tmpdir)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE");

    let _status = command.status().context("failed to start tmux client")?;
    Ok(())
}

#[cfg(not(unix))]
fn run_tmux_client(_tmux: TmuxCommand) -> Result<()> {
    bail!("herdr-scratch-pane currently supports Unix-like systems only")
}

pub fn tmux_session_exists(
    state_dir: &Path,
    scope: Scope,
    workspace_id: Option<&str>,
    server_id: Option<&str>,
) -> Result<bool> {
    let target = format!("={}", session_name(scope, workspace_id, server_id));
    let status = Command::new("tmux")
        .args(["-L", TMUX_SERVER_NAME, "has-session", "-t", target.as_str()])
        .env("TMUX_TMPDIR", state_dir)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) => Ok(status.success()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("failed to check tmux scratch session"),
    }
}

fn command_exists(program: &str) -> bool {
    Command::new(program)
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn state_dir() -> PathBuf {
    default_state_dir()
}
