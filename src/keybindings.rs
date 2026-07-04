pub const DEFAULT_KEYBINDINGS_MARKER: &str = "# herdr-floating-pane:keybindings";

pub fn install_keybindings_text(
    existing: &str,
    workspace_key: &str,
    session_key: &str,
    minimize_key: &str,
) -> String {
    if existing.contains(DEFAULT_KEYBINDINGS_MARKER) {
        return existing.to_string();
    }

    let mut next = existing.to_string();
    if !next.ends_with('\n') {
        next.push('\n');
    }
    next.push('\n');
    next.push_str(DEFAULT_KEYBINDINGS_MARKER);
    next.push('\n');
    next.push_str(&binding_block(
        workspace_key,
        "herdr-floating-pane.toggle-workspace",
        "Toggle workspace floating pane",
    ));
    next.push('\n');
    next.push_str(&binding_block(
        session_key,
        "herdr-floating-pane.toggle-session",
        "Toggle session floating pane",
    ));
    next.push('\n');
    next.push_str(&binding_block(
        minimize_key,
        "herdr-floating-pane.minimize-current",
        "Minimize current floating pane",
    ));
    next
}

fn binding_block(key: &str, command: &str, description: &str) -> String {
    format!(
        "[[keys.command]]\nkey = \"{key}\"\ntype = \"plugin_action\"\ncommand = \"{command}\"\ndescription = \"{description}\"\n"
    )
}
