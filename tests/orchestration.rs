use herdr_scratch_pane::commands::{
    open_pane_args, workspace_get_args, workspace_rename_args, OpenPaneRequest,
};
use herdr_scratch_pane::decisions::{minimize_decision, open_target_for_current, MinimizeDecision};
use herdr_scratch_pane::herdr::{parse_opened_pane_id, PaneInfo};
use herdr_scratch_pane::keybindings::{install_keybindings_text, DEFAULT_KEYBINDINGS_MARKER};
use herdr_scratch_pane::scope::Scope;
use herdr_scratch_pane::state::ScratchState;
use herdr_scratch_pane::workspace_marker::{
    legacy_marker_cleanup_target, marked_workspace_label, restore_workspace_label,
};

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
            "plugin",
            "pane",
            "open",
            "--plugin",
            "herdr-scratch-pane",
            "--entrypoint",
            "workspace-scratch",
            "--placement",
            "split",
            "--direction",
            "right",
            "--focus",
            "--target-pane",
            "p1",
            "--env",
            "HERDR_SCRATCH_PANE_SCOPE=workspace",
            "--env",
            "HERDR_SCRATCH_PANE_CWD=/tmp/proj",
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

    assert!(args.contains(&"session-scratch".to_string()));
    assert!(!args.contains(&"--target-pane".to_string()));
    assert!(args.contains(&"HERDR_SCRATCH_PANE_SCOPE=session".to_string()));
}

#[test]
fn keybinding_install_appends_once_and_preserves_existing_text() {
    let initial = "[keys]\nprefix = \"ctrl+b\"\nsplit_vertical = \"prefix+|\"\n";
    let first =
        install_keybindings_text(initial, "prefix+f", "prefix+shift+f", "prefix+cmd+z").unwrap();
    assert!(first.contains(DEFAULT_KEYBINDINGS_MARKER));
    assert!(first.contains("command = \"herdr-scratch-pane.toggle-workspace\""));
    assert!(first.contains("command = \"herdr-scratch-pane.toggle-session\""));
    assert!(first.contains("command = \"herdr-scratch-pane.minimize-current\""));
    assert!(first.contains("split_vertical = \"prefix+|\""));

    let second =
        install_keybindings_text(&first, "prefix+g", "prefix+shift+g", "prefix+alt+z").unwrap();
    assert_eq!(first, second);
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

    let updated =
        install_keybindings_text(initial, "prefix+f", "prefix+shift+f", "prefix+cmd+z").unwrap();

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
