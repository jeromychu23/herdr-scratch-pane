use std::path::Path;

use herdr_scratch_pane::pane::dtach_command;
use herdr_scratch_pane::scope::Scope;

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
