pub const DEFAULT_KEYBINDINGS_MARKER: &str = "# herdr-scratch-pane:keybindings";

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
        "herdr-scratch-pane.toggle-workspace",
        "Toggle workspace scratch pane",
    ));
    next.push('\n');
    next.push_str(&binding_block(
        session_key,
        "herdr-scratch-pane.toggle-session",
        "Toggle session scratch pane",
    ));
    next.push('\n');
    next.push_str(&binding_block(
        minimize_key,
        "herdr-scratch-pane.minimize-current",
        "Minimize current scratch pane",
    ));
    next
}

fn binding_block(key: &str, command: &str, description: &str) -> String {
    format!(
        "[[keys.command]]\nkey = \"{key}\"\ntype = \"plugin_action\"\ncommand = \"{command}\"\ndescription = \"{description}\"\n"
    )
}
