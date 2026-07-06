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

This is not a transparent draggable floating box. It uses Herdr's native pane
and zoom behavior for better terminal, mouse, resize, and TUI compatibility.

## Requirements

- macOS
- Herdr `0.7.1` or newer
- Rust toolchain with `cargo`
- `dtach`

Install `dtach`:

```sh
# Install dtach so the scratch shell can keep running after the pane is hidden.
brew install dtach
```

## Installation

Install from GitHub:

```sh
# Install the plugin from this GitHub repository.
herdr plugin install jeromychu23/herdr-scratch-pane

# Add the recommended keybindings to your Herdr config.
herdr plugin action invoke herdr-scratch-pane.install-keybindings
```

Reload Herdr config or restart Herdr after installing keybindings. If you use
named Herdr sessions, reload each running session that should receive the new
keys.

For local development:

```sh
# Build the plugin binary.
cargo build --release

# Link this local checkout into Herdr.
herdr plugin link /path/to/herdr-scratch-pane

# Install keybindings from the local binary.
./target/release/herdr-scratch-pane install-keybindings
```

## Usage

The installer writes keybindings for you. If you want to configure the core
scratch pane keys manually, add these commands to `~/.config/herdr/config.toml`.
Replace `/absolute/path/to/herdr-scratch-pane` with the built plugin binary.

If your config already has a `[keys]` table, do not paste another `[keys]`
header; add these entries inside the existing keys section.

```toml
[keys]
[[keys.command]]
key = "prefix+f"
type = "shell"
command = "/absolute/path/to/herdr-scratch-pane toggle --scope workspace"
description = "Toggle workspace scratch pane"

[[keys.command]]
key = "prefix+shift+f"
type = "shell"
command = "/absolute/path/to/herdr-scratch-pane toggle --scope session"
description = "Toggle session scratch pane"

[[keys.command]]
key = "prefix+cmd+z"
type = "shell"
command = "/absolute/path/to/herdr-scratch-pane minimize"
description = "Minimize current scratch pane"
```

The keybindings intentionally use `type = "shell"` instead of
`type = "plugin_action"`. Herdr named sessions share the same config, but a new
session may not have the plugin action registry loaded yet. Calling the binary
directly keeps `prefix+f` and `prefix+shift+f` working across sessions.

If you installed an earlier version, run `install-keybindings` again. It updates
old scratch pane keybindings and removes the previous split-key workaround.

## Limitations

- This is a native zoomed scratch pane, not a transparent floating overlay.
- It does not provide mouse resize handles or a mouse minimize button.
- `prefix+cmd+z` depends on your terminal forwarding Command/Super key chords
  to Herdr. If it does not work, bind the same `minimize` shell command to
  another key.
- When a scratch pane is open, Herdr's native split keybindings may leave the
  scratch pane and split the underlying layout.

## Trust And Security

Herdr plugins are ordinary executables that run as your user. Review
`herdr-plugin.toml` and the source before installing third-party plugins.

This plugin does not make network requests at runtime. It calls the Herdr CLI,
starts `dtach` plus your shell, writes plugin state for the `dtach` socket, and
edits Herdr keybindings only when you explicitly run `install-keybindings`.

## License

MIT
