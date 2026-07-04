use herdr_floating_pane::config::{AppConfig, Rgb};
use herdr_floating_pane::geometry::{box_inner, box_rect, Rect};
use herdr_floating_pane::herdr::{parse_current_pane, parse_pane_list, PaneInfo};
use herdr_floating_pane::mouse::{hit_test, HitZone};
use herdr_floating_pane::scope::{floating_label, session_name, Scope};
use herdr_floating_pane::toggle::{decide_toggle, ToggleDecision, ToggleInputs};

#[test]
fn config_parses_clamps_and_serializes_global_size() {
    let cfg = AppConfig::from_toml(
        r##"
width_pct = 5
height_pct = 120
backdrop = "#112233"
key_hint_workspace = "prefix+g"
"##,
    )
    .unwrap();

    assert_eq!(cfg.width_pct, 20);
    assert_eq!(cfg.height_pct, 100);
    assert_eq!(cfg.backdrop, Rgb(0x11, 0x22, 0x33));
    assert_eq!(cfg.key_hint_workspace, "prefix+g");

    let resized = cfg.with_size(87, 73);
    let rendered = resized.to_toml().unwrap();
    assert!(rendered.contains("width_pct = 87"));
    assert!(rendered.contains("height_pct = 73"));
}

#[test]
fn geometry_centers_box_and_inner_area() {
    let cfg = AppConfig {
        width_pct: 80,
        height_pct: 60,
        ..AppConfig::default()
    };
    let area = Rect::new(0, 0, 100, 50);

    assert_eq!(box_rect(area, &cfg), Rect::new(10, 10, 80, 30));
    assert_eq!(box_inner(area, &cfg), Rect::new(11, 11, 78, 28));
}

#[test]
fn mouse_hit_testing_distinguishes_controls_border_and_inner() {
    let cfg = AppConfig {
        width_pct: 80,
        height_pct: 60,
        ..AppConfig::default()
    };
    let area = Rect::new(0, 0, 100, 50);

    assert_eq!(hit_test(area, &cfg, 86, 10), HitZone::Minimize);
    assert_eq!(hit_test(area, &cfg, 89, 25), HitZone::ResizeRight);
    assert_eq!(hit_test(area, &cfg, 89, 39), HitZone::ResizeBottomRight);
    assert_eq!(hit_test(area, &cfg, 30, 20), HitZone::Inner);
    assert_eq!(hit_test(area, &cfg, 2, 2), HitZone::Backdrop);
}

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
        r#"{"result":{"panes":[{"pane_id":"p1","workspace_id":"w1","label":"main","focused":false},{"pane_id":"p2","workspace_id":"w1","label":"⌂ floating workspace","focused":true}]}}"#,
    )
    .unwrap();

    assert_eq!(panes.len(), 2);
    assert_eq!(panes[1].label.as_deref(), Some("⌂ floating workspace"));
    assert!(panes[1].focused);
}

#[test]
fn scope_names_are_stable_and_filename_safe() {
    assert_eq!(floating_label(Scope::Workspace), "⌂ floating workspace");
    assert_eq!(floating_label(Scope::Session), "⌂ floating session");
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
        label: Some("⌂ floating workspace".into()),
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
        label: Some("⌂ floating workspace".into()),
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
        label: Some("⌂ floating session".into()),
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
