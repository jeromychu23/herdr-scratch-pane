use herdr_scratch_pane::commands::{
    open_pane_args, open_popup_args, pane_run_command, run_pane_args, workspace_get_args,
    workspace_rename_args, OpenPaneRequest, PopupOpenRequest,
};
use herdr_scratch_pane::decisions::{minimize_decision, open_target_for_current, MinimizeDecision};
use herdr_scratch_pane::herdr::{parse_opened_pane_id, PaneInfo};
use herdr_scratch_pane::keybindings::{
    install_keybindings_text, install_popup_keybindings_text, tmux_prefix_from_config,
    DEFAULT_KEYBINDINGS_MARKER,
};
use herdr_scratch_pane::scope::Scope;
use herdr_scratch_pane::state::ScratchState;
use herdr_scratch_pane::workspace_marker::{
    legacy_marker_cleanup_target, marked_workspace_label, restore_workspace_label,
};

#[cfg(unix)]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
struct TestDir {
    path: PathBuf,
}

#[cfg(unix)]
impl TestDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "herdr-scratch-pane-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("test directory should be created");
        Self { path }
    }
}

#[cfg(unix)]
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
fn fake_herdr_path(dir: &Path) -> PathBuf {
    let path = dir.join("fake-herdr");
    fs::write(
        &path,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_HERDR_LOG"

case "$1 $2" in
  "pane current")
    printf '%s\n' '{"result":{"pane":{"pane_id":"host","workspace_id":"w1","cwd":"/tmp","label":"main","focused":true}}}'
    exit 0
    ;;
  "pane list")
    printf '%s\n' '{"result":{"panes":[]}}'
    exit 0
    ;;
  "pane split")
    printf '%s\n' '{"result":{"pane":{"pane_id":"scratch"}}}'
    exit 0
    ;;
  "plugin pane")
    if [ "${FAKE_HERDR_FAIL_POPUP:-0}" = "1" ]; then
      printf '%s\n' 'simulated popup open failure' >&2
      exit 42
    fi
    printf '%s\n' '{}'
    exit 0
    ;;
esac

if [ "$1 $2" = "pane run" ] && [ "${FAKE_HERDR_FAIL_RUN:-0}" = "1" ]; then
  printf '%s\n' 'simulated pane run failure' >&2
  exit 42
fi

if [ "$1 $2 $3 $4" = "pane zoom scratch --off" ] && [ "${FAKE_HERDR_FAIL_RUN:-0}" = "1" ]; then
  printf '%s\n' 'simulated rollback failure' >&2
  exit 43
fi

printf '%s\n' '{}'
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

#[cfg(unix)]
fn run_toggle_with_fake_herdr(name: &str, fail_popup: bool) -> (Output, Vec<String>, bool) {
    let test_dir = TestDir::new(name);
    let fake_herdr = fake_herdr_path(&test_dir.path);
    let log_path = test_dir.path.join("commands.log");
    let config_dir = test_dir.path.join("config");
    fs::create_dir_all(&config_dir).expect("config directory should be created");
    fs::write(
        config_dir.join("config.toml"),
        "[keys]\nprefix = \"ctrl+b\"\n",
    )
    .expect("test config should be written");
    let state_dir = test_dir.path.join("state");
    let output = Command::new(env!("CARGO_BIN_EXE_herdr-scratch-pane"))
        .args(["toggle", "--scope", "workspace"])
        .env("HERDR_BIN_PATH", fake_herdr)
        .env("HERDR_PLUGIN_STATE_DIR", &state_dir)
        .env("HERDR_CONFIG_DIR", &config_dir)
        .env("FAKE_HERDR_LOG", &log_path)
        .env("FAKE_HERDR_FAIL_POPUP", if fail_popup { "1" } else { "0" })
        .env_remove("HERDR_SERVER_ID")
        .env_remove("HERDR_SOCKET_PATH")
        .output()
        .expect("toggle command should run");
    let commands = fs::read_to_string(log_path)
        .expect("fake Herdr command log should be readable")
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    let state_exists = fs::read_dir(state_dir)
        .map(|entries| {
            entries.filter_map(Result::ok).any(|entry| {
                entry.path().extension().and_then(|value| value.to_str()) == Some("json")
            })
        })
        .unwrap_or(false);
    (output, commands, state_exists)
}

#[cfg(unix)]
#[test]
fn opening_scratch_uses_native_popup_without_mutating_layout() {
    let (output, commands, state_exists) = run_toggle_with_fake_herdr("native-popup", false);
    assert!(
        output.status.success(),
        "toggle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        state_exists,
        "successful popup open should persist scratch state"
    );
    assert!(commands.iter().any(|command| {
        command.starts_with(
            "plugin pane open --plugin herdr-scratch-pane --entrypoint workspace-scratch \
             --placement popup --width 85% --height 80%",
        )
    }));
    assert!(!commands.iter().any(|command| {
        command.starts_with("pane split")
            || command.starts_with("pane zoom")
            || command.starts_with("pane run")
    }));
}

#[cfg(unix)]
#[test]
fn popup_open_failure_removes_new_state_and_preserves_original_error() {
    let (output, commands, state_exists) =
        run_toggle_with_fake_herdr("popup-failure-rollback", true);
    assert!(
        !output.status.success(),
        "toggle should report popup open failure"
    );
    assert!(
        !state_exists,
        "failed first open must not leave stale state"
    );
    assert!(commands
        .iter()
        .any(|command| command.starts_with("plugin pane open")));
    assert!(!commands
        .iter()
        .any(|command| command.starts_with("pane zoom")));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("simulated popup open failure"),
        "original popup error should be preserved: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn popup_open_args_use_percentage_geometry_and_omit_layout_targets() {
    let args = open_popup_args(PopupOpenRequest {
        scope: Scope::Workspace,
        workspace_id: Some("w1".into()),
        session_id: "default".into(),
        state_dir: "/tmp/state".into(),
        cwd: Some("/tmp/proj".into()),
        tmux_prefix: "C-b".into(),
    });

    assert_eq!(
        args,
        vec![
            "plugin",
            "pane",
            "open",
            "--plugin",
            "herdr-scratch-pane",
            "--entrypoint",
            "workspace-scratch",
            "--placement",
            "popup",
            "--width",
            "85%",
            "--height",
            "80%",
            "--cwd",
            "/tmp/proj",
            "--env",
            "HERDR_SCRATCH_PANE_SCOPE=workspace",
            "--env",
            "HERDR_WORKSPACE_ID=w1",
            "--env",
            "HERDR_SCRATCH_PANE_SESSION_ID=default",
            "--env",
            "HERDR_SCRATCH_PANE_STATE_DIR=/tmp/state",
            "--env",
            "HERDR_SCRATCH_PANE_CWD=/tmp/proj",
            "--env",
            "HERDR_SCRATCH_PANE_PREFIX=C-b",
            "--focus",
        ]
    );
    assert!(!args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--pane" | "--current" | "--direction")));
}

#[test]
fn herdr_prefix_is_normalized_for_tmux() {
    assert_eq!(
        tmux_prefix_from_config("[keys]\nprefix = \"ctrl+b\"\n").unwrap(),
        "C-b"
    );
    assert_eq!(tmux_prefix_from_config("").unwrap(), "C-b");
    assert_eq!(
        tmux_prefix_from_config("[keys]\nprefix = \"f12\"\n").unwrap(),
        "F12"
    );
    assert!(tmux_prefix_from_config("[keys]\nprefix = \"cmd+b\"\n").is_err());
}

#[cfg(unix)]
#[test]
fn herdr_config_path_takes_precedence_for_popup_prefix() {
    let test_dir = TestDir::new("config-path-prefix");
    let fake_herdr = fake_herdr_path(&test_dir.path);
    let log_path = test_dir.path.join("commands.log");
    let config_dir = test_dir.path.join("config-dir");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        "[keys]\nprefix = \"ctrl+b\"\n",
    )
    .unwrap();
    let explicit_config = test_dir.path.join("explicit.toml");
    fs::write(&explicit_config, "[keys]\nprefix = \"f12\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_herdr-scratch-pane"))
        .args(["toggle", "--scope", "workspace"])
        .env("HERDR_BIN_PATH", fake_herdr)
        .env("HERDR_PLUGIN_STATE_DIR", test_dir.path.join("state"))
        .env("HERDR_CONFIG_DIR", config_dir)
        .env("HERDR_CONFIG_PATH", explicit_config)
        .env("FAKE_HERDR_LOG", &log_path)
        .env("FAKE_HERDR_FAIL_POPUP", "0")
        .env_remove("HERDR_SERVER_ID")
        .env_remove("HERDR_SOCKET_PATH")
        .output()
        .expect("toggle command should run");
    assert!(
        output.status.success(),
        "toggle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log = fs::read_to_string(log_path).unwrap();
    assert!(log.contains("--env HERDR_SCRATCH_PANE_PREFIX=F12"));
}

#[test]
fn manifest_declares_native_percentage_popups_and_required_herdr_version() {
    let manifest = include_str!("../herdr-plugin.toml");
    assert!(manifest.contains("min_herdr_version = \"0.7.4\""));
    assert_eq!(manifest.matches("placement = \"popup\"").count(), 2);
    assert_eq!(manifest.matches("width = \"85%\"").count(), 2);
    assert_eq!(manifest.matches("height = \"80%\"").count(), 2);
}

#[test]
fn open_pane_args_use_native_split_zoom_host_and_pass_scope_cwd_env() {
    let args = open_pane_args(OpenPaneRequest {
        scope: Scope::Workspace,
        target_pane_id: Some("p1".into()),
        cwd: Some("/tmp/proj".into()),
    });

    assert_eq!(
        args,
        vec![
            "pane",
            "split",
            "--pane",
            "p1",
            "--direction",
            "right",
            "--cwd",
            "/tmp/proj",
            "--env",
            "HERDR_SCRATCH_PANE_SCOPE=workspace",
            "--env",
            "HERDR_SCRATCH_PANE_CWD=/tmp/proj",
            "--focus",
        ]
    );
}

#[test]
fn pane_run_command_execs_current_binary_with_scope() {
    assert_eq!(
        pane_run_command(
            "/Applications/Test App/herdr-scratch-pane",
            Scope::Workspace
        ),
        "exec '/Applications/Test App/herdr-scratch-pane' run-pane --scope workspace"
    );
    assert_eq!(
        run_pane_args(
            "p1",
            &pane_run_command("/tmp/herdr-scratch-pane", Scope::Session)
        ),
        vec![
            "pane",
            "run",
            "p1",
            "exec /tmp/herdr-scratch-pane run-pane --scope session"
        ]
    );
}

#[test]
fn pane_run_command_quotes_single_quotes_in_binary_path() {
    assert_eq!(
        pane_run_command("/tmp/it's/herdr-scratch-pane", Scope::Workspace),
        "exec '/tmp/it'\\''s/herdr-scratch-pane' run-pane --scope workspace"
    );
}

#[test]
fn open_pane_args_omit_target_when_absent() {
    let args = open_pane_args(OpenPaneRequest {
        scope: Scope::Session,
        target_pane_id: None,
        cwd: None,
    });

    assert_eq!(
        args,
        vec![
            "pane",
            "split",
            "--current",
            "--direction",
            "right",
            "--env",
            "HERDR_SCRATCH_PANE_SCOPE=session",
            "--focus",
        ]
    );
}

#[test]
fn open_pane_args_target_existing_pane_when_present() {
    let args = open_pane_args(OpenPaneRequest {
        scope: Scope::Workspace,
        target_pane_id: Some("p1".into()),
        cwd: None,
    });

    assert!(args.contains(&"p1".to_string()));
    assert!(args.contains(&"--pane".to_string()));
    assert!(!args.contains(&"--current".to_string()));
}

#[test]
fn keybinding_install_writes_shell_commands_for_named_sessions() {
    let initial = "[keys]\nprefix = \"ctrl+b\"\nsplit_vertical = \"prefix+|\"\n";
    let first = install_popup_keybindings_text(
        initial,
        "prefix+f",
        "prefix+shift+f",
        "/Applications/Test App/herdr-scratch-pane",
    )
    .unwrap();

    assert!(first.contains(DEFAULT_KEYBINDINGS_MARKER));
    assert!(first.contains("type = \"shell\""));
    assert!(first.contains(
        "command = \"'/Applications/Test App/herdr-scratch-pane' toggle --scope workspace\""
    ));
    assert!(first.contains(
        "command = \"'/Applications/Test App/herdr-scratch-pane' toggle --scope session\""
    ));
    assert!(!first.contains("herdr-scratch-pane' minimize"));
    assert!(first.contains("split_vertical = \"prefix+|\""));

    let second = install_popup_keybindings_text(
        &first,
        "prefix+g",
        "prefix+shift+g",
        "/Applications/Test App/herdr-scratch-pane",
    )
    .unwrap();
    assert_eq!(first, second);
}

#[test]
fn keybinding_install_removes_managed_minimize_binding() {
    let initial = r#"[keys]

[[keys.command]]
key = "prefix+cmd+z"
type = "shell"
command = "/tmp/herdr-scratch-pane minimize"
description = "Minimize current scratch pane"
"#;

    let updated = install_popup_keybindings_text(
        initial,
        "prefix+f",
        "prefix+shift+f",
        "/tmp/herdr-scratch-pane",
    )
    .unwrap();

    assert!(!updated.contains("prefix+cmd+z"));
    assert!(!updated.contains(" minimize"));
}

#[test]
fn keybinding_install_migrates_legacy_plugin_actions_and_restores_split_keys() {
    let initial = r#"[keys]
split_vertical = ""
split_horizontal = ""

[[keys.command]]
key = "prefix+|"
type = "plugin_action"
command = "herdr-scratch-pane.safe-split-right"
description = "Split right unless scratch pane is active"

[[keys.command]]
key = "prefix+minus"
type = "plugin_action"
command = "herdr-scratch-pane.safe-split-down"
description = "Split down unless scratch pane is active"

[[keys.command]]
key = "prefix+f"
type = "plugin_action"
command = "herdr-scratch-pane.toggle-workspace"
description = "Toggle workspace scratch pane"
"#;

    let updated = install_keybindings_text(
        initial,
        "prefix+f",
        "prefix+shift+f",
        "prefix+cmd+z",
        "/tmp/herdr-scratch-pane",
    )
    .unwrap();

    assert!(updated.contains("split_vertical = \"prefix+v\""));
    assert!(updated.contains("split_horizontal = \"prefix+minus\""));
    assert!(!updated.contains("safe-split"));
    assert!(!updated.contains("type = \"plugin_action\"\ncommand = \"herdr-scratch-pane"));
    assert!(updated.contains("key = \"prefix+f\""));
    assert!(updated.contains("command = \"/tmp/herdr-scratch-pane toggle --scope workspace\""));
}

#[test]
fn keybinding_install_preserves_custom_plugin_actions() {
    let initial = r#"[keys]

[[keys.command]]
key = "prefix+x"
type = "plugin_action"
command = "herdr-scratch-pane.custom-action"
description = "Custom scratch action"
"#;

    let updated = install_keybindings_text(
        initial,
        "prefix+f",
        "prefix+shift+f",
        "prefix+cmd+z",
        "/tmp/herdr-scratch-pane",
    )
    .unwrap();

    assert!(updated.contains("command = \"herdr-scratch-pane.custom-action\""));
    assert!(updated.contains("description = \"Custom scratch action\""));
    assert!(updated.contains("key = \"prefix+x\""));
}

#[test]
fn workspace_marker_args_use_workspace_get_and_rename() {
    assert_eq!(workspace_get_args("w1"), vec!["workspace", "get", "w1"]);
    assert_eq!(
        workspace_rename_args("w1", "floating-pane [scratch-on]"),
        vec!["workspace", "rename", "w1", "floating-pane [scratch-on]"]
    );
}

#[test]
fn workspace_marker_appends_suffix_once() {
    assert_eq!(
        marked_workspace_label("floating-pane"),
        "floating-pane [scratch-on]"
    );
    assert_eq!(
        marked_workspace_label("floating-pane [scratch-on]"),
        "floating-pane [scratch-on]"
    );
}

#[test]
fn workspace_marker_restores_only_plugin_written_label() {
    let state = ScratchState {
        scope: Scope::Workspace,
        workspace_id: Some("w1".into()),
        host_pane_id: "host".into(),
        scratch_pane_id: Some("scratch".into()),
        original_workspace_label: Some("floating-pane".into()),
        marked_workspace_label: Some("floating-pane [scratch-on]".into()),
    };

    assert_eq!(
        restore_workspace_label(&state, "floating-pane [scratch-on]"),
        Some("floating-pane".into())
    );
    assert_eq!(restore_workspace_label(&state, "renamed-by-user"), None);

    let stale_state = ScratchState {
        scope: Scope::Workspace,
        workspace_id: Some("w1".into()),
        host_pane_id: "host".into(),
        scratch_pane_id: Some("scratch".into()),
        original_workspace_label: None,
        marked_workspace_label: None,
    };
    assert_eq!(
        restore_workspace_label(&stale_state, "floating-pane [scratch-on]"),
        Some("floating-pane".into())
    );
}

#[test]
fn marker_target_uses_recorded_host_then_workspace_fallback() {
    let state = ScratchState {
        scope: Scope::Workspace,
        workspace_id: Some("w1".into()),
        host_pane_id: "host".into(),
        scratch_pane_id: Some("scratch".into()),
        original_workspace_label: None,
        marked_workspace_label: None,
    };
    let host = PaneInfo {
        pane_id: "host".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("main".into()),
        focused: false,
    };
    let scratch = PaneInfo {
        pane_id: "scratch".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ scratch workspace".into()),
        focused: true,
    };
    assert_eq!(
        legacy_marker_cleanup_target(Some(&state), &[scratch.clone(), host.clone()], &scratch),
        Some("host".into())
    );

    let fallback = PaneInfo {
        pane_id: "fallback".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("main".into()),
        focused: false,
    };
    assert_eq!(
        legacy_marker_cleanup_target(Some(&state), &[scratch], &fallback),
        Some("fallback".into())
    );
}

#[test]
fn minimize_closes_current_or_focused_scratch_pane_else_notifies() {
    let current_scratch = PaneInfo {
        pane_id: "fw".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ scratch workspace".into()),
        focused: true,
    };
    assert_eq!(
        minimize_decision(&current_scratch, &[]),
        MinimizeDecision::Close {
            pane_id: "fw".into()
        }
    );

    let current_normal = PaneInfo {
        pane_id: "p1".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("main".into()),
        focused: true,
    };
    let focused_session = PaneInfo {
        pane_id: "fs".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ scratch session".into()),
        focused: true,
    };
    assert_eq!(
        minimize_decision(&current_normal, &[focused_session]),
        MinimizeDecision::Close {
            pane_id: "fs".into()
        }
    );
    assert_eq!(
        minimize_decision(&current_normal, &[]),
        MinimizeDecision::NotifyNoVisiblePane
    );
}

#[test]
fn open_target_skips_current_pane_when_current_is_scratch_and_about_to_close() {
    let normal = PaneInfo {
        pane_id: "p1".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("main".into()),
        focused: true,
    };
    assert_eq!(open_target_for_current(&normal), Some("p1".into()));

    let scratch = PaneInfo {
        pane_id: "fw".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ scratch workspace".into()),
        focused: true,
    };
    assert_eq!(open_target_for_current(&scratch), None);
}

#[test]
fn opened_pane_id_parses_known_herdr_plugin_shapes() {
    assert_eq!(
        parse_opened_pane_id(r#"{"result":{"plugin_pane":{"pane":{"pane_id":"new-pane"}}}}"#),
        Some("new-pane".into())
    );
    assert_eq!(
        parse_opened_pane_id(r#"{"result":{"pane":{"pane_id":"new-pane-2"}}}"#),
        Some("new-pane-2".into())
    );
    assert_eq!(parse_opened_pane_id(r#"{"result":{}}"#), None);
}
