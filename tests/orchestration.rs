use herdr_scratch_pane::actions::{
    clear_marker_args, minimize_decision, open_pane_args, open_target_for_current,
    report_marker_args, safe_split_decision, split_pane_args, MinimizeDecision, OpenPaneRequest,
    SafeSplitDecision, SplitDirection,
};
use herdr_scratch_pane::herdr::{parse_opened_pane_id, PaneInfo};
use herdr_scratch_pane::keybindings::{install_keybindings_text, DEFAULT_KEYBINDINGS_MARKER};
use herdr_scratch_pane::scope::Scope;
use herdr_scratch_pane::status::{choose_marker_target, ScratchState};

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
        install_keybindings_text(initial, "prefix+f", "prefix+shift+f", "prefix+cmd+z", true)
            .unwrap();
    assert!(first.contains(DEFAULT_KEYBINDINGS_MARKER));
    assert!(first.contains("command = \"herdr-scratch-pane.toggle-workspace\""));
    assert!(first.contains("command = \"herdr-scratch-pane.toggle-session\""));
    assert!(first.contains("command = \"herdr-scratch-pane.minimize-current\""));
    assert!(first.contains("command = \"herdr-scratch-pane.safe-split-right\""));
    assert!(first.contains("command = \"herdr-scratch-pane.safe-split-down\""));
    assert!(first.contains("split_vertical = \"\""));
    assert!(first.contains("split_horizontal = \"\""));
    assert!(first.contains("key = \"prefix+|\""));
    assert!(first.contains("key = \"prefix+minus\""));

    let second =
        install_keybindings_text(&first, "prefix+g", "prefix+shift+g", "prefix+alt+z", true)
            .unwrap();
    assert_eq!(first, second);
}

#[test]
fn keybinding_install_can_skip_split_proxy() {
    let initial = "[keys]\nsplit_vertical = \"prefix+v\"\nsplit_horizontal = \"prefix+-\"\n";
    let updated =
        install_keybindings_text(initial, "prefix+f", "prefix+shift+f", "prefix+cmd+z", false)
            .unwrap();

    assert!(updated.contains("split_vertical = \"prefix+v\""));
    assert!(updated.contains("split_horizontal = \"prefix+-\""));
    assert!(!updated.contains("herdr-scratch-pane.safe-split-right"));
    assert!(!updated.contains("herdr-scratch-pane.safe-split-down"));
}

#[test]
fn safe_split_blocks_scratch_and_delegates_normal_panes() {
    let scratch = PaneInfo {
        pane_id: "fw".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ scratch workspace".into()),
        focused: true,
    };
    assert_eq!(
        safe_split_decision(&scratch, &[], SplitDirection::Right),
        SafeSplitDecision::NotifyBlocked
    );

    let normal = PaneInfo {
        pane_id: "p1".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("main".into()),
        focused: true,
    };
    assert_eq!(
        safe_split_decision(
            &normal,
            std::slice::from_ref(&scratch),
            SplitDirection::Right
        ),
        SafeSplitDecision::NotifyBlocked
    );
    assert_eq!(
        safe_split_decision(&normal, &[], SplitDirection::Down),
        SafeSplitDecision::Split {
            direction: SplitDirection::Down
        }
    );

    assert_eq!(
        split_pane_args(SplitDirection::Right),
        vec![
            "pane",
            "split",
            "--current",
            "--direction",
            "right",
            "--focus"
        ]
    );
    assert_eq!(
        split_pane_args(SplitDirection::Down),
        vec![
            "pane",
            "split",
            "--current",
            "--direction",
            "down",
            "--focus"
        ]
    );
}

#[test]
fn marker_args_use_display_only_metadata_source() {
    assert_eq!(
        report_marker_args("p1", Scope::Workspace),
        vec![
            "pane",
            "report-metadata",
            "p1",
            "--source",
            "herdr-scratch-pane",
            "--title",
            "scratch running",
            "--custom-status",
            "scratch workspace",
        ]
    );
    assert_eq!(
        clear_marker_args("p1"),
        vec![
            "pane",
            "report-metadata",
            "p1",
            "--source",
            "herdr-scratch-pane",
            "--clear-title",
            "--clear-custom-status",
        ]
    );
}

#[test]
fn marker_target_uses_recorded_host_then_workspace_fallback() {
    let state = ScratchState {
        scope: Scope::Workspace,
        workspace_id: Some("w1".into()),
        host_pane_id: "host".into(),
        scratch_pane_id: Some("scratch".into()),
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
        choose_marker_target(Some(&state), &[scratch.clone(), host.clone()], &scratch),
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
        choose_marker_target(Some(&state), &[scratch], &fallback),
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
