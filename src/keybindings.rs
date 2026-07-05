use anyhow::{anyhow, Result};
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table, Value};

pub const DEFAULT_KEYBINDINGS_MARKER: &str = "# herdr-scratch-pane:keybindings";

const TOGGLE_WORKSPACE: &str = "herdr-scratch-pane.toggle-workspace";
const TOGGLE_SESSION: &str = "herdr-scratch-pane.toggle-session";
const MINIMIZE_CURRENT: &str = "herdr-scratch-pane.minimize-current";
const SAFE_SPLIT_RIGHT: &str = "herdr-scratch-pane.safe-split-right";
const SAFE_SPLIT_DOWN: &str = "herdr-scratch-pane.safe-split-down";

pub fn install_keybindings_text(
    existing: &str,
    workspace_key: &str,
    session_key: &str,
    minimize_key: &str,
    install_split_proxy: bool,
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

    let workspace_keys = managed_command_keys(&existing_commands, TOGGLE_WORKSPACE)
        .unwrap_or_else(|| vec![workspace_key.to_string()]);
    let session_keys = managed_command_keys(&existing_commands, TOGGLE_SESSION)
        .unwrap_or_else(|| vec![session_key.to_string()]);
    let minimize_keys = managed_command_keys(&existing_commands, MINIMIZE_CURRENT)
        .unwrap_or_else(|| vec![minimize_key.to_string()]);

    let split_right_keys = managed_command_keys(&existing_commands, SAFE_SPLIT_RIGHT)
        .unwrap_or_else(|| configured_keys(keys, "split_vertical", &["prefix+v"]));
    let split_down_keys = managed_command_keys(&existing_commands, SAFE_SPLIT_DOWN)
        .unwrap_or_else(|| configured_keys(keys, "split_horizontal", &["prefix+minus"]));

    let mut retained = ArrayOfTables::new();
    for table in existing_commands.iter() {
        if !is_managed_command(table) {
            retained.push(table.clone());
        }
    }

    add_command_bindings(
        &mut retained,
        &workspace_keys,
        TOGGLE_WORKSPACE,
        "Toggle workspace scratch pane",
    );
    add_command_bindings(
        &mut retained,
        &session_keys,
        TOGGLE_SESSION,
        "Toggle session scratch pane",
    );
    add_command_bindings(
        &mut retained,
        &minimize_keys,
        MINIMIZE_CURRENT,
        "Minimize current scratch pane",
    );

    if install_split_proxy {
        keys["split_vertical"] = value("");
        keys["split_horizontal"] = value("");
        add_command_bindings(
            &mut retained,
            &split_right_keys,
            SAFE_SPLIT_RIGHT,
            "Split right unless scratch pane is active",
        );
        add_command_bindings(
            &mut retained,
            &split_down_keys,
            SAFE_SPLIT_DOWN,
            "Split down unless scratch pane is active",
        );
    }

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

fn ensure_keys_table(doc: &mut DocumentMut) {
    if !doc.as_table().contains_key("keys") || !doc["keys"].is_table() {
        doc["keys"] = Item::Table(Table::new());
    }
}

fn configured_keys(keys: &Table, field: &str, defaults: &[&str]) -> Vec<String> {
    match keys.get(field) {
        Some(item) => item_to_keys(item),
        None => defaults.iter().map(|key| (*key).to_string()).collect(),
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

fn managed_command_keys(commands: &ArrayOfTables, command: &str) -> Option<Vec<String>> {
    let keys = commands
        .iter()
        .filter(|table| command_value(table) == Some(command))
        .filter_map(|table| table.get("key"))
        .flat_map(item_to_keys)
        .collect::<Vec<_>>();

    (!keys.is_empty()).then_some(keys)
}

fn is_managed_command(table: &Table) -> bool {
    command_value(table)
        .map(is_known_managed_command)
        .unwrap_or(false)
}

fn is_known_managed_command(command: &str) -> bool {
    matches!(
        command,
        TOGGLE_WORKSPACE | TOGGLE_SESSION | MINIMIZE_CURRENT | SAFE_SPLIT_RIGHT | SAFE_SPLIT_DOWN
    )
}

fn command_value(table: &Table) -> Option<&str> {
    table.get("command").and_then(Item::as_str)
}

fn add_command_bindings(
    commands: &mut ArrayOfTables,
    keys: &[String],
    command: &str,
    description: &str,
) {
    for key in keys {
        let mut table = Table::new();
        table["key"] = value(key);
        table["type"] = value("plugin_action");
        table["command"] = value(command);
        table["description"] = value(description);
        commands.push(table);
    }
}
