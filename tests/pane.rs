use std::path::Path;

use herdr_floating_pane::config::AppConfig;
use herdr_floating_pane::geometry::Rect;
use herdr_floating_pane::mouse::HitZone;
use herdr_floating_pane::pane::{dtach_command, resize_from_drag};
use herdr_floating_pane::scope::Scope;

#[test]
fn dtach_command_uses_scope_specific_socket_and_login_shell() {
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
            "-z",
            "/bin/zsh",
            "-l"
        ]
    );
    assert_eq!(command.cwd.as_deref(), Some(Path::new("/tmp/project")));
}

#[test]
fn resize_from_bottom_right_drag_updates_global_percentages() {
    let cfg = AppConfig {
        width_pct: 80,
        height_pct: 60,
        ..AppConfig::default()
    };
    let area = Rect::new(0, 0, 100, 50);

    let resized = resize_from_drag(area, &cfg, HitZone::ResizeBottomRight, 89, 39, 99, 49);

    assert_eq!(resized.width_pct, 90);
    assert_eq!(resized.height_pct, 80);
}
