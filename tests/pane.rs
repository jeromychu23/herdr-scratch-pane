use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(deprecated)]
use herdr_scratch_pane::pane::dtach_command;
use herdr_scratch_pane::pane::{
    popup_tmux_command, tmux_command, tmux_session_exists, TMUX_SERVER_NAME,
};
use herdr_scratch_pane::scope::Scope;
use herdr_scratch_pane::state::{read_state, state_path};

#[allow(deprecated)]
#[test]
fn legacy_dtach_command_remains_available_for_recovery() {
    let command = dtach_command(
        Scope::Workspace,
        Some("workspace/a:b"),
        Some("server 1"),
        Path::new("/tmp/state"),
        "/bin/zsh",
        Some(Path::new("/tmp/project")),
    );

    assert_eq!(command.program, "dtach");
    assert_eq!(
        command.args,
        vec![
            "-A",
            "/tmp/state/workspace-workspace-a-b.dtach",
            "-r",
            "winch",
            "-z",
            "/bin/zsh",
            "-l"
        ]
    );
    assert_eq!(command.cwd.as_deref(), Some(Path::new("/tmp/project")));
}

#[test]
fn tmux_command_uses_transparent_server_scope_cwd_and_login_shell() {
    let cwd = std::env::temp_dir();
    let command = tmux_command(
        Scope::Workspace,
        Some("workspace/a:b"),
        Some("server 1"),
        Path::new("/tmp/state"),
        "/bin/zsh",
        Some(&cwd),
    );

    assert_eq!(command.program, "tmux");
    assert_eq!(command.tmux_tmpdir, Path::new("/tmp/state"));
    assert_eq!(
        command.args,
        vec![
            "-L",
            TMUX_SERVER_NAME,
            "-f",
            "/dev/null",
            "start-server",
            ";",
            "set-option",
            "-g",
            "status",
            "off",
            ";",
            "set-option",
            "-g",
            "prefix",
            "None",
            ";",
            "set-option",
            "-g",
            "prefix2",
            "None",
            ";",
            "set-option",
            "-g",
            "mouse",
            "off",
            ";",
            "set-option",
            "-s",
            "escape-time",
            "0",
            ";",
            "set-option",
            "-g",
            "default-terminal",
            "tmux-256color",
            ";",
            "set-option",
            "-g",
            "remain-on-exit",
            "off",
            ";",
            "new-session",
            "-A",
            "-s",
            "workspace-workspace-a-b",
            "-c",
            cwd.to_str().unwrap(),
            "/bin/zsh",
            "-l",
        ]
    );
}

#[test]
fn tmux_command_omits_invalid_starting_directory() {
    let command = tmux_command(
        Scope::Session,
        Some("workspace"),
        Some("server 1"),
        Path::new("/tmp/state"),
        "/bin/zsh",
        Some(Path::new("/path/that/does/not/exist")),
    );

    assert!(!command.args.iter().any(|arg| arg == "-c"));
    assert!(command
        .args
        .ends_with(&["session-server-1".into(), "/bin/zsh".into(), "-l".into(),]));
}

#[test]
fn popup_tmux_command_binds_hide_confirmed_close_and_send_prefix() {
    let command = popup_tmux_command(
        Scope::Workspace,
        Some("w1"),
        Some("server"),
        Path::new("/tmp/state"),
        "/bin/zsh",
        None,
        "C-b",
    );
    let joined = command.args.join(" ");

    assert!(joined.contains("set-option -g prefix C-b"));
    assert!(joined.contains("unbind-key -a -T prefix"));
    assert!(joined.contains("bind-key -T prefix f detach-client"));
    assert!(joined.contains("bind-key -T prefix F detach-client"));
    assert!(joined.contains(
        "bind-key -T prefix x confirm-before -p Kill scratch session? (y/n) kill-session"
    ));
    assert!(joined.contains("bind-key -T prefix C-b send-prefix"));
}

#[test]
fn popup_prefix_f_detaches_and_reattaches_the_same_viewport() {
    let test_dir = TestDir::new("popup-detach");
    start_host_pane(&test_dir);
    wait_for_scratch_session(&test_dir);

    let status = host_tmux(&test_dir)
        .args([
            "send-keys",
            "-t",
            "host:0.0",
            "printf 'POPUP_DETACH_MARKER\\n'; pwd",
            "Enter",
        ])
        .status()
        .expect("test host should accept terminal input");
    assert!(status.success());
    let before = wait_for_capture(&test_dir, "POPUP_DETACH_MARKER");

    send_popup_keys(&test_dir, &["C-b", "f"]);
    wait_for_host_to_exit(&test_dir);
    assert!(tmux_session_exists(&test_dir.path, Scope::Workspace, Some("w1"), None).unwrap());
    let state_file = state_path(&test_dir.path, Scope::Workspace, Some("w1"), None);
    let state = read_state(&state_file)
        .expect("detached state should be readable")
        .expect("detached session should keep state");
    assert_eq!(state.original_workspace_label.as_deref(), Some("test"));
    assert_eq!(
        state.marked_workspace_label.as_deref(),
        Some("test [scratch-on]")
    );

    start_host_pane(&test_dir);
    let after = wait_for_capture(&test_dir, "POPUP_DETACH_MARKER");
    let expected_cwd = test_dir.path.display().to_string();
    assert!(before.contains(&expected_cwd));
    assert!(after.contains("POPUP_DETACH_MARKER"));
    assert!(after.contains(&expected_cwd));
    stop_host_pane(&test_dir);
}

#[test]
fn popup_prefix_x_confirmation_kills_only_the_exact_session() {
    let test_dir = TestDir::new("px");
    let status = Command::new("tmux")
        .env("TMUX_TMPDIR", &test_dir.path)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .args([
            "-L",
            TMUX_SERVER_NAME,
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            "session-other",
            "/bin/sh",
        ])
        .status()
        .expect("comparison session should start");
    assert!(status.success());

    start_host_pane(&test_dir);
    wait_for_scratch_session(&test_dir);

    send_popup_keys(&test_dir, &["C-b", "x"]);
    thread::sleep(Duration::from_millis(100));
    send_popup_keys(&test_dir, &["n"]);
    thread::sleep(Duration::from_millis(100));
    assert!(tmux_session_exists(&test_dir.path, Scope::Workspace, Some("w1"), None).unwrap());
    assert!(tmux_session_exists(&test_dir.path, Scope::Session, None, Some("other")).unwrap());

    send_popup_keys(&test_dir, &["C-b", "x"]);
    thread::sleep(Duration::from_millis(100));
    send_popup_keys(&test_dir, &["y"]);
    wait_for_scratch_session_to_end(&test_dir);
    assert!(!tmux_session_exists(&test_dir.path, Scope::Workspace, Some("w1"), None).unwrap());
    assert!(tmux_session_exists(&test_dir.path, Scope::Session, None, Some("other")).unwrap());
    assert!(!state_path(&test_dir.path, Scope::Workspace, Some("w1"), None).exists());
}

#[test]
fn popup_tmux_tracks_reattached_client_geometry() {
    let test_dir = TestDir::new("resize");
    start_host_pane_with_size(&test_dir, "100", "30");
    wait_for_scratch_session(&test_dir);
    wait_for_matching_geometry(&test_dir);

    let status = host_tmux(&test_dir)
        .args([
            "send-keys",
            "-t",
            "host:0.0",
            "printf 'POPUP_RESIZE_MARKER\\n'",
            "Enter",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    wait_for_capture(&test_dir, "POPUP_RESIZE_MARKER");
    send_popup_keys(&test_dir, &["C-b", "f"]);
    wait_for_host_to_exit(&test_dir);

    start_host_pane_with_size(&test_dir, "70", "20");
    wait_for_scratch_session(&test_dir);
    wait_for_matching_geometry(&test_dir);
    let capture = wait_for_capture(&test_dir, "POPUP_RESIZE_MARKER");
    assert!(capture.contains("POPUP_RESIZE_MARKER"));
    stop_host_pane(&test_dir);
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        // Keep the tmux Unix socket below macOS's AF_UNIX path-length limit.
        let path = PathBuf::from("/tmp").join(format!(
            "herdr-scratch-pane-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("test directory should be created");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        for server in [TMUX_SERVER_NAME, "herdr-test-host"] {
            let _ = Command::new("tmux")
                .env("TMUX_TMPDIR", &self.path)
                .env_remove("TMUX")
                .env_remove("TMUX_PANE")
                .args(["-L", server, "kill-server"])
                .status();
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn tmux_liveness_checks_the_exact_scope_session() {
    let test_dir = TestDir::new("tmux-liveness");
    let status = Command::new("tmux")
        .env("TMUX_TMPDIR", &test_dir.path)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .args([
            "-L",
            TMUX_SERVER_NAME,
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            "workspace-w1",
            "/bin/sh",
        ])
        .status()
        .expect("tmux should be installed for runtime tests");
    assert!(status.success());

    assert!(
        tmux_session_exists(&test_dir.path, Scope::Workspace, Some("w1"), Some("server")).unwrap()
    );
    assert!(!tmux_session_exists(
        &test_dir.path,
        Scope::Workspace,
        Some("other"),
        Some("server")
    )
    .unwrap());

    let status = Command::new("tmux")
        .env("TMUX_TMPDIR", &test_dir.path)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .args(["-L", TMUX_SERVER_NAME, "kill-server"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        !tmux_session_exists(&test_dir.path, Scope::Workspace, Some("w1"), Some("server")).unwrap()
    );
}

#[test]
fn tmux_preserves_existing_viewport_across_ten_host_pane_reopens() {
    let test_dir = TestDir::new("tmux-viewport");
    start_host_pane(&test_dir);
    wait_for_scratch_session(&test_dir);

    let status = host_tmux(&test_dir)
        .args([
            "send-keys",
            "-t",
            "host:0.0",
            "printf 'HERDR_VIEWPORT_MARKER\\n'; pwd",
            "Enter",
        ])
        .status()
        .expect("test host should accept terminal input");
    assert!(status.success());

    let expected_cwd = test_dir.path.display().to_string();
    let initial = wait_for_capture(&test_dir, "HERDR_VIEWPORT_MARKER");
    assert!(initial.contains(&expected_cwd));
    stop_host_pane(&test_dir);

    for _ in 0..10 {
        assert!(tmux_session_exists(&test_dir.path, Scope::Workspace, Some("w1"), None,).unwrap());

        start_host_pane(&test_dir);
        let restored = wait_for_capture(&test_dir, "HERDR_VIEWPORT_MARKER");
        assert!(
            restored.contains(&expected_cwd),
            "reattached viewport should retain its previous working-directory output: {restored:?}"
        );
        stop_host_pane(&test_dir);
    }
}

fn host_tmux(test_dir: &TestDir) -> Command {
    let mut command = Command::new("tmux");
    command
        .env("TMUX_TMPDIR", &test_dir.path)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .args(["-L", "herdr-test-host"]);
    command
}

fn start_host_pane(test_dir: &TestDir) {
    start_host_pane_with_size(test_dir, "100", "30");
}

fn start_host_pane_with_size(test_dir: &TestDir, width: &str, height: &str) {
    let binary = env!("CARGO_BIN_EXE_herdr-scratch-pane");
    let fake_herdr = fake_herdr(test_dir);
    let status = host_tmux(test_dir)
        .args([
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-x",
            width,
            "-y",
            height,
            "-s",
            "host",
            "env",
        ])
        .arg(format!(
            "HERDR_PLUGIN_STATE_DIR={}",
            test_dir.path.display()
        ))
        .arg("HERDR_WORKSPACE_ID=w1")
        .arg(format!(
            "HERDR_SCRATCH_PANE_CWD={}",
            test_dir.path.display()
        ))
        .arg(format!("HERDR_BIN_PATH={}", fake_herdr.display()))
        .arg("SHELL=/bin/sh")
        .arg(binary)
        .args(["run-pane", "--scope", "workspace"])
        .status()
        .expect("test host tmux should start");
    assert!(status.success());
}

fn fake_herdr(test_dir: &TestDir) -> PathBuf {
    let path = test_dir.path.join("fake-herdr");
    if path.exists() {
        return path;
    }
    fs::write(
        &path,
        r#"#!/bin/sh
case "$1 $2" in
  "pane current")
    printf '%s\n' '{"result":{"pane":{"pane_id":"host","workspace_id":"w1","cwd":"/tmp","label":"main","focused":true}}}'
    ;;
  "workspace get")
    printf '%s\n' '{"result":{"workspace":{"workspace_id":"w1","label":"test","focused":true}}}'
    ;;
  *)
    printf '%s\n' '{}'
    ;;
esac
"#,
    )
    .expect("fake Herdr should be written");
    let mut permissions = fs::metadata(&path)
        .expect("fake Herdr metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("fake Herdr should be executable");
    path
}

fn stop_host_pane(test_dir: &TestDir) {
    let status = host_tmux(test_dir)
        .arg("kill-server")
        .status()
        .expect("test host tmux should stop");
    assert!(status.success());
}

fn wait_for_scratch_session(test_dir: &TestDir) {
    for _ in 0..100 {
        if tmux_session_exists(&test_dir.path, Scope::Workspace, Some("w1"), None).unwrap() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("scratch tmux session did not start");
}

fn wait_for_scratch_session_to_end(test_dir: &TestDir) {
    for _ in 0..100 {
        if !tmux_session_exists(&test_dir.path, Scope::Workspace, Some("w1"), None).unwrap() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("scratch tmux session did not end");
}

fn send_popup_keys(test_dir: &TestDir, keys: &[&str]) {
    let status = host_tmux(test_dir)
        .args(["send-keys", "-t", "host:0.0"])
        .args(keys)
        .status()
        .expect("popup keys should be delivered through the host PTY");
    assert!(status.success());
}

fn wait_for_host_to_exit(test_dir: &TestDir) {
    for _ in 0..100 {
        let status = host_tmux(test_dir)
            .args(["has-session", "-t", "=host"])
            .status()
            .expect("host liveness should be queryable");
        if !status.success() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("host tmux session did not exit after popup detach");
}

fn wait_for_matching_geometry(test_dir: &TestDir) {
    let mut last_host = String::new();
    let mut last_scratch = String::new();
    for _ in 0..100 {
        last_host = command_stdout(host_tmux(test_dir).args([
            "display-message",
            "-p",
            "-t",
            "host:0.0",
            "#{pane_width}x#{pane_height}",
        ]));
        let mut scratch = Command::new("tmux");
        scratch
            .env("TMUX_TMPDIR", &test_dir.path)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .args([
                "-L",
                TMUX_SERVER_NAME,
                "display-message",
                "-p",
                "-t",
                "workspace-w1:0.0",
                "#{pane_width}x#{pane_height}",
            ]);
        last_scratch = command_stdout(&mut scratch);
        if !last_host.is_empty() && last_host == last_scratch {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("popup geometry did not follow host: host={last_host:?}, scratch={last_scratch:?}");
}

fn command_stdout(command: &mut Command) -> String {
    command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
}

fn wait_for_capture(test_dir: &TestDir, expected: &str) -> String {
    let mut last_capture = String::new();
    for _ in 0..100 {
        let output = host_tmux(test_dir)
            .args(["capture-pane", "-p", "-J", "-t", "host:0.0"])
            .output()
            .expect("test host viewport should be capturable");
        if output.status.success() {
            last_capture = String::from_utf8_lossy(&output.stdout).into_owned();
            if last_capture.contains(expected) {
                return last_capture;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("viewport never contained {expected:?}; last capture: {last_capture:?}");
}
