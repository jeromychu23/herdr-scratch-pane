use anyhow::{anyhow, bail, Result};
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table, Value};

pub const DEFAULT_KEYBINDINGS_MARKER: &str = "# herdr-scratch-pane:keybindings";

const TOGGLE_WORKSPACE: &str = "herdr-scratch-pane.toggle-workspace";
const TOGGLE_SESSION: &str = "herdr-scratch-pane.toggle-session";
const MINIMIZE_CURRENT: &str = "herdr-scratch-pane.minimize-current";
const LEGACY_SAFE_SPLIT_RIGHT: &str = "herdr-scratch-pane.safe-split-right";
const LEGACY_SAFE_SPLIT_DOWN: &str = "herdr-scratch-pane.safe-split-down";

const TOGGLE_WORKSPACE_DESCRIPTION: &str = "Toggle workspace scratch pane";
const TOGGLE_SESSION_DESCRIPTION: &str = "Toggle session scratch pane";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedCommand {
    ToggleWorkspace,
    ToggleSession,
    MinimizeCurrent,
}

pub fn install_keybindings_text(
    existing: &str,
    workspace_key: &str,
    session_key: &str,
    _minimize_key: &str,
    binary_path: &str,
) -> Result<String> {
    install_popup_keybindings_text(existing, workspace_key, session_key, binary_path)
}

pub fn install_popup_keybindings_text(
    existing: &str,
    workspace_key: &str,
    session_key: &str,
    binary_path: &str,
) -> Result<String> {
    let mut doc = existing.parse::<DocumentMut>()?;
    ensure_keys_table(&mut doc);

    let keys = doc["keys"]
        .as_table_mut()
        .ok_or_else(|| anyhow!("expected [keys] to be a TOML table"))?;

    let existing_commands = keys
        .get("command")
        .and_then(Item::as_array_of_tables)
        .cloned()
        .unwrap_or_default();

    let workspace_keys = managed_command_keys(&existing_commands, ManagedCommand::ToggleWorkspace)
        .unwrap_or_else(|| vec![workspace_key.to_string()]);
    let session_keys = managed_command_keys(&existing_commands, ManagedCommand::ToggleSession)
        .unwrap_or_else(|| vec![session_key.to_string()]);
    restore_empty_key(keys, "split_vertical", "prefix+v");
    restore_empty_key(keys, "split_horizontal", "prefix+minus");

    let mut retained = ArrayOfTables::new();
    for table in existing_commands.iter() {
        if !is_managed_command(table) {
            retained.push(table.clone());
        }
    }

    add_shell_command_bindings(
        &mut retained,
        &workspace_keys,
        &action_command(binary_path, "toggle --scope workspace"),
        TOGGLE_WORKSPACE_DESCRIPTION,
    );
    add_shell_command_bindings(
        &mut retained,
        &session_keys,
        &action_command(binary_path, "toggle --scope session"),
        TOGGLE_SESSION_DESCRIPTION,
    );
    keys["command"] = Item::ArrayOfTables(retained);

    let mut output = doc.to_string();
    if !output.contains(DEFAULT_KEYBINDINGS_MARKER) {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(DEFAULT_KEYBINDINGS_MARKER);
        output.push('\n');
    }
    Ok(output)
}

pub fn tmux_prefix_from_config(config: &str) -> Result<String> {
    let doc = config.parse::<DocumentMut>()?;
    let prefix = doc
        .get("keys")
        .and_then(Item::as_table)
        .and_then(|keys| keys.get("prefix"))
        .and_then(Item::as_str)
        .unwrap_or("ctrl+b");
    normalize_tmux_prefix(prefix)
}

pub fn normalize_tmux_prefix(prefix: &str) -> Result<String> {
    let value = prefix.trim().to_ascii_lowercase();
    if value.is_empty() {
        bail!("Herdr prefix cannot be empty");
    }

    if matches!(value.as_str(), "esc" | "escape") {
        return Ok("Escape".into());
    }
    if let Some(number) = value
        .strip_prefix('f')
        .and_then(|value| value.parse::<u8>().ok())
    {
        if (1..=24).contains(&number) {
            return Ok(format!("F{number}"));
        }
    }

    let parts = value.split('+').collect::<Vec<_>>();
    if parts.len() == 1 {
        return tmux_base_key(parts[0]);
    }

    let (modifiers, base) = parts.split_at(parts.len() - 1);
    let mut control = false;
    let mut alt = false;
    let mut shift = false;
    for modifier in modifiers {
        match *modifier {
            "ctrl" | "control" => control = true,
            "alt" | "option" | "meta" => alt = true,
            "shift" => shift = true,
            "cmd" | "command" | "super" => {
                bail!("Herdr prefix `{prefix}` uses Command/Super, which tmux cannot receive")
            }
            _ => bail!("unsupported Herdr prefix modifier `{modifier}` in `{prefix}`"),
        }
    }

    let mut base = tmux_base_key(base[0])?;
    if shift && base.len() == 1 && base.as_bytes()[0].is_ascii_alphabetic() {
        base.make_ascii_uppercase();
        shift = false;
    }

    let mut tmux_modifiers = Vec::new();
    if control {
        tmux_modifiers.push("C");
    }
    if alt {
        tmux_modifiers.push("M");
    }
    if shift {
        tmux_modifiers.push("S");
    }
    if tmux_modifiers.is_empty() {
        return Ok(base);
    }
    Ok(format!("{}-{base}", tmux_modifiers.join("-")))
}

fn tmux_base_key(key: &str) -> Result<String> {
    let named = match key {
        "space" => Some("Space"),
        "tab" => Some("Tab"),
        "enter" | "return" => Some("Enter"),
        "backspace" => Some("BSpace"),
        "delete" => Some("DC"),
        "up" => Some("Up"),
        "down" => Some("Down"),
        "left" => Some("Left"),
        "right" => Some("Right"),
        _ => None,
    };
    if let Some(named) = named {
        return Ok(named.into());
    }
    if key.chars().count() == 1 {
        return Ok(key.to_string());
    }
    bail!("unsupported Herdr prefix key `{key}`")
}

fn ensure_keys_table(doc: &mut DocumentMut) {
    if !doc.as_table().contains_key("keys") || !doc["keys"].is_table() {
        doc["keys"] = Item::Table(Table::new());
    }
}

fn restore_empty_key(keys: &mut Table, field: &str, default_key: &str) {
    if keys
        .get(field)
        .and_then(Item::as_str)
        .map(|value| value.trim().is_empty())
        .unwrap_or(false)
    {
        keys[field] = value(default_key);
    }
}

fn item_to_keys(item: &Item) -> Vec<String> {
    if let Some(key) = item.as_str() {
        return non_empty_key(key).into_iter().collect();
    }

    if let Some(array) = item.as_array() {
        return array
            .iter()
            .filter_map(Value::as_str)
            .filter_map(non_empty_key)
            .collect();
    }

    Vec::new()
}

fn non_empty_key(key: &str) -> Option<String> {
    let trimmed = key.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn managed_command_keys(commands: &ArrayOfTables, command: ManagedCommand) -> Option<Vec<String>> {
    let keys = commands
        .iter()
        .filter(|table| command_kind(table) == Some(command))
        .filter_map(|table| table.get("key"))
        .flat_map(item_to_keys)
        .collect::<Vec<_>>();

    (!keys.is_empty()).then_some(keys)
}

fn is_managed_command(table: &Table) -> bool {
    command_kind(table).is_some()
        || command_value(table)
            .map(is_legacy_safe_split_command)
            .unwrap_or(false)
}

fn command_kind(table: &Table) -> Option<ManagedCommand> {
    let command = command_value(table)?;
    match command {
        TOGGLE_WORKSPACE => Some(ManagedCommand::ToggleWorkspace),
        TOGGLE_SESSION => Some(ManagedCommand::ToggleSession),
        MINIMIZE_CURRENT => Some(ManagedCommand::MinimizeCurrent),
        _ => shell_command_kind(command),
    }
}

fn shell_command_kind(command: &str) -> Option<ManagedCommand> {
    if !command.contains("herdr-scratch-pane") {
        return None;
    }

    if command.ends_with(" toggle --scope workspace") {
        Some(ManagedCommand::ToggleWorkspace)
    } else if command.ends_with(" toggle --scope session") {
        Some(ManagedCommand::ToggleSession)
    } else if command.ends_with(" minimize") {
        Some(ManagedCommand::MinimizeCurrent)
    } else {
        None
    }
}

fn is_legacy_safe_split_command(command: &str) -> bool {
    matches!(command, LEGACY_SAFE_SPLIT_RIGHT | LEGACY_SAFE_SPLIT_DOWN)
}

fn command_value(table: &Table) -> Option<&str> {
    table.get("command").and_then(Item::as_str)
}

fn add_shell_command_bindings(
    commands: &mut ArrayOfTables,
    keys: &[String],
    command: &str,
    description: &str,
) {
    for key in keys {
        let mut table = Table::new();
        table["key"] = value(key);
        table["type"] = value("shell");
        table["command"] = value(command);
        table["description"] = value(description);
        commands.push(table);
    }
}

fn action_command(binary_path: &str, args: &str) -> String {
    format!("{} {args}", shell_quote(binary_path))
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}
