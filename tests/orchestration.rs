use herdr_floating_pane::actions::{
    minimize_decision, open_pane_args, open_target_for_current, MinimizeDecision, OpenPaneRequest,
};
use herdr_floating_pane::herdr::{parse_opened_pane_id, PaneInfo};
use herdr_floating_pane::keybindings::{install_keybindings_text, DEFAULT_KEYBINDINGS_MARKER};
use herdr_floating_pane::scope::Scope;

#[test]
fn open_pane_args_use_split_zoom_host_and_pass_scope_cwd_env() {
    let args = open_pane_args(OpenPaneRequest {
        scope: Scope::Workspace,
        target_pane_id: Some("p1".into()),
        cwd: Some("/tmp/proj".into()),
    });

    assert_eq!(
        args,
        vec![
            "plugin",
            "pane",
            "open",
            "--plugin",
            "herdr-floating-pane",
            "--entrypoint",
            "workspace-floating",
            "--placement",
            "split",
            "--direction",
            "right",
            "--focus",
            "--target-pane",
            "p1",
            "--env",
            "HERDR_FLOATING_PANE_SCOPE=workspace",
            "--env",
            "HERDR_FLOATING_PANE_CWD=/tmp/proj",
        ]
    );
}

#[test]
fn session_open_pane_args_select_session_entrypoint_without_target_when_absent() {
    let args = open_pane_args(OpenPaneRequest {
        scope: Scope::Session,
        target_pane_id: None,
        cwd: None,
    });

    assert!(args.contains(&"session-floating".to_string()));
    assert!(!args.contains(&"--target-pane".to_string()));
    assert!(args.contains(&"HERDR_FLOATING_PANE_SCOPE=session".to_string()));
}

#[test]
fn keybinding_install_appends_once_and_preserves_existing_text() {
    let initial = "[keys]\nprefix = \"ctrl+b\"\n";
    let first = install_keybindings_text(initial, "prefix+f", "prefix+shift+f", "prefix+cmd+z");
    assert!(first.starts_with(initial));
    assert!(first.contains(DEFAULT_KEYBINDINGS_MARKER));
    assert!(first.contains("command = \"herdr-floating-pane.toggle-workspace\""));
    assert!(first.contains("command = \"herdr-floating-pane.toggle-session\""));
    assert!(first.contains("command = \"herdr-floating-pane.minimize-current\""));

    let second = install_keybindings_text(&first, "prefix+g", "prefix+shift+g", "prefix+alt+z");
    assert_eq!(first, second);
}

#[test]
fn minimize_closes_current_or_focused_floating_pane_else_notifies() {
    let current_floating = PaneInfo {
        pane_id: "fw".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ floating workspace".into()),
        focused: true,
    };
    assert_eq!(
        minimize_decision(&current_floating, &[]),
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
        label: Some("⌂ floating session".into()),
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
fn open_target_skips_current_pane_when_current_is_floating_and_about_to_close() {
    let normal = PaneInfo {
        pane_id: "p1".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("main".into()),
        focused: true,
    };
    assert_eq!(open_target_for_current(&normal), Some("p1".into()));

    let floating = PaneInfo {
        pane_id: "fw".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ floating workspace".into()),
        focused: true,
    };
    assert_eq!(open_target_for_current(&floating), None);
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
