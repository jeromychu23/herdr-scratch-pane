# herdr-scratch-pane

A Herdr plugin for a persistent native scratch pane.

It opens a normal Herdr pane, zooms it, and keeps the shell alive with `dtach`
when the pane is hidden. This keeps terminal apps such as `yazi`, `btop`,
editors, and REPLs working like they do in a regular Herdr pane.

## What It Does

- Toggle a workspace scratch pane.
- Toggle a session scratch pane.
- Hide the visible scratch pane without killing its process.
- Keep hidden scratch sessions running in the background through `dtach`.
- Mark workspaces with a hidden scratch session using ` [scratch-on]`.
- Protect split keybindings while a scratch pane is active.

This is not a transparent draggable floating box. It uses Herdr's native pane
and zoom behavior for better terminal, mouse, resize, and TUI compatibility.

## Requirements

- macOS
- Herdr `0.7.1` or newer
- Rust toolchain with `cargo`
- `dtach`

Install `dtach`:

```sh
brew install dtach
```

## Installation

After this repository is published on GitHub:

```sh
herdr plugin install owner/repo
herdr plugin action invoke herdr-scratch-pane.install-keybindings
```

Replace `owner/repo` with the GitHub repository path.

Reload Herdr config or restart Herdr after installing keybindings.

For local development:

```sh
cargo build --release
herdr plugin link /path/to/herdr-scratch-pane
./target/release/herdr-scratch-pane install-keybindings
```

## Usage

The installer writes the recommended keybindings for you. If you want to
configure them manually, add these commands to `~/.config/herdr/config.toml`.

If your config already has a `[keys]` table, do not paste another `[keys]`
header; add these entries inside the existing keys section.

```toml
[keys]
split_vertical = ""
split_horizontal = ""

[[keys.command]]
key = "prefix+f"
type = "plugin_action"
command = "herdr-scratch-pane.toggle-workspace"
description = "Toggle workspace scratch pane"

[[keys.command]]
key = "prefix+shift+f"
type = "plugin_action"
command = "herdr-scratch-pane.toggle-session"
description = "Toggle session scratch pane"

[[keys.command]]
key = "prefix+cmd+z"
type = "plugin_action"
command = "herdr-scratch-pane.minimize-current"
description = "Minimize current scratch pane"

[[keys.command]]
key = "prefix+v"
type = "plugin_action"
command = "herdr-scratch-pane.safe-split-right"
description = "Split right unless scratch pane is active"

[[keys.command]]
key = "prefix+minus"
type = "plugin_action"
command = "herdr-scratch-pane.safe-split-down"
description = "Split down unless scratch pane is active"
```

## Limitations

- This is a native zoomed scratch pane, not a transparent floating overlay.
- It does not provide mouse resize handles or a mouse minimize button.
- `prefix+cmd+z` depends on your terminal forwarding Command/Super key chords
  to Herdr. If it does not work, bind `herdr-scratch-pane.minimize-current` to
  another key.

## Trust And Security

Herdr plugins are ordinary executables that run as your user. Review
`herdr-plugin.toml` and the source before installing third-party plugins.

This plugin does not make network requests at runtime. It calls the Herdr CLI,
starts `dtach` plus your shell, writes plugin state for the `dtach` socket, and
edits Herdr keybindings only when you explicitly run `install-keybindings`.

## License

MIT
