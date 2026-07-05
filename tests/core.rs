use herdr_scratch_pane::decisions::{decide_toggle, ToggleDecision, ToggleInputs};
use herdr_scratch_pane::herdr::{
    parse_current_pane, parse_pane_list, parse_workspace_get, PaneInfo,
};
use herdr_scratch_pane::scope::{scratch_label, session_name, Scope};

#[test]
fn herdr_json_parsing_accepts_current_and_list_shapes() {
    let current = parse_current_pane(
        r#"{"result":{"pane":{"pane_id":"p1","workspace_id":"w1","cwd":"/tmp/proj","label":"main","focused":true}}}"#,
    )
    .unwrap();

    assert_eq!(current.pane_id, "p1");
    assert_eq!(current.workspace_id.as_deref(), Some("w1"));
    assert_eq!(current.cwd.as_deref(), Some("/tmp/proj"));

    let panes = parse_pane_list(
        r#"{"result":{"panes":[{"pane_id":"p1","workspace_id":"w1","label":"main","focused":false},{"pane_id":"p2","workspace_id":"w1","label":"⌂ scratch workspace","focused":true}]}}"#,
    )
    .unwrap();

    assert_eq!(panes.len(), 2);
    assert_eq!(panes[1].label.as_deref(), Some("⌂ scratch workspace"));
    assert!(panes[1].focused);
}

#[test]
fn workspace_json_parsing_accepts_get_shape() {
    let workspace = parse_workspace_get(
        r#"{"result":{"type":"workspace_info","workspace":{"workspace_id":"wB","label":"floating-pane","focused":true}}}"#,
    )
    .unwrap();

    assert_eq!(workspace.workspace_id, "wB");
    assert_eq!(workspace.label.as_deref(), Some("floating-pane"));
}

#[test]
fn scope_names_are_stable_and_filename_safe() {
    assert_eq!(scratch_label(Scope::Workspace), "⌂ scratch workspace");
    assert_eq!(scratch_label(Scope::Session), "⌂ scratch session");
    assert_eq!(
        session_name(Scope::Workspace, Some("ws/a:b"), Some("srv 1")),
        "workspace-ws-a-b"
    );
    assert_eq!(
        session_name(Scope::Session, Some("ws/a:b"), Some("srv 1")),
        "session-srv-1"
    );
}

#[test]
fn toggle_decision_opens_reveals_closes_and_enforces_mutual_exclusion() {
    let ctx = PaneInfo {
        pane_id: "current".into(),
        workspace_id: Some("w1".into()),
        cwd: Some("/tmp/proj".into()),
        label: Some("main".into()),
        focused: true,
    };

    let open = decide_toggle(ToggleInputs {
        scope: Scope::Workspace,
        current: ctx.clone(),
        panes: vec![],
        server_id: Some("server".into()),
    });
    assert_eq!(
        open,
        ToggleDecision::Open {
            scope: Scope::Workspace
        }
    );

    let target = PaneInfo {
        pane_id: "fw".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ scratch workspace".into()),
        focused: true,
    };
    let close = decide_toggle(ToggleInputs {
        scope: Scope::Workspace,
        current: target.clone(),
        panes: vec![target],
        server_id: Some("server".into()),
    });
    assert_eq!(
        close,
        ToggleDecision::Close {
            pane_id: "fw".into()
        }
    );

    let hidden_target = PaneInfo {
        pane_id: "fw".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ scratch workspace".into()),
        focused: false,
    };
    let reveal = decide_toggle(ToggleInputs {
        scope: Scope::Workspace,
        current: ctx.clone(),
        panes: vec![hidden_target],
        server_id: Some("server".into()),
    });
    assert_eq!(
        reveal,
        ToggleDecision::Reveal {
            pane_id: "fw".into()
        }
    );

    let other_visible = PaneInfo {
        pane_id: "fs".into(),
        workspace_id: Some("w1".into()),
        cwd: None,
        label: Some("⌂ scratch session".into()),
        focused: true,
    };
    let switch = decide_toggle(ToggleInputs {
        scope: Scope::Workspace,
        current: other_visible.clone(),
        panes: vec![other_visible],
        server_id: Some("server".into()),
    });
    assert_eq!(
        switch,
        ToggleDecision::CloseThenOpen {
            close_pane_id: "fs".into(),
            scope: Scope::Workspace
        }
    );
}
