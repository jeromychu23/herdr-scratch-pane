# herdr-floating-pane

A Rust Herdr plugin that provides tmux-floax style floating scratch panes.

It gives you two scopes:

- `prefix+f`: workspace-level floating pane
- `prefix+shift+f`: session-level floating pane
- `prefix+cmd+z`: minimize the currently visible floating pane

Only one floating host pane is visible at a time. Workspace and session shells
are separate `dtach` sessions, so they do not share cwd or running processes.

## How It Works

Herdr's current plugin API can open `overlay` panes, but those views are
transient for keybinding-launched plugin actions. To keep restore behavior
floating-looking while preserving the shell process, this plugin opens a normal
Herdr split pane, immediately zooms it, then runs a Rust TUI inside it. The TUI
draws the floating box and embeds a real PTY shell inside that box.

Minimizing closes the Herdr host pane. The shell keeps running in `dtach`, and
the next toggle reattaches to the same session.

The backdrop around the box is drawn by the plugin. It is not a live dimmed view
of the underlying Herdr panes because plugins cannot composite over other panes.

## Requirements

- macOS
- Herdr `0.7.1` or newer
- Rust toolchain with `cargo`
- `dtach`

Install `dtach` on macOS:

```sh
brew install dtach
```

## Install For Local Testing

```sh
herdr plugin link /Users/yuchu/GitHub/herdr-plugin/floating-pane
cargo build --release
./target/release/herdr-floating-pane install-keybindings
```

Then reload Herdr config or restart Herdr.

The installer appends an idempotent block to `~/.config/herdr/config.toml`.
Manual equivalent:

```toml
# herdr-floating-pane:keybindings
[[keys.command]]
key = "prefix+f"
type = "plugin_action"
command = "herdr-floating-pane.toggle-workspace"
description = "Toggle workspace floating pane"

[[keys.command]]
key = "prefix+shift+f"
type = "plugin_action"
command = "herdr-floating-pane.toggle-session"
description = "Toggle session floating pane"

[[keys.command]]
key = "prefix+cmd+z"
type = "plugin_action"
command = "herdr-floating-pane.minimize-current"
description = "Minimize current floating pane"
```

## Configuration

Copy `floating-pane.toml.example` to:

```sh
$(herdr plugin config-dir herdr-floating-pane)/floating-pane.toml
```

The most important settings are:

```toml
width_pct = 94
height_pct = 92
backdrop = "#0d2b1d"
forward_inner_mouse = true
```

Dragging the floating box border updates the global width/height config. The
`[-]` control in the title bar minimizes the pane with the mouse.

## Trust And Security

This plugin runs a local Rust binary built by `cargo build --release` during
Herdr plugin installation or linking. It does not make network requests at
runtime.

It performs these local actions:

- calls the `herdr` CLI to open, zoom, close, and inspect panes
- starts `dtach` plus your login shell inside a PTY
- reads/writes its plugin config and state directories
- writes `~/.config/herdr/config.toml` only when you explicitly run
  `install-keybindings`
- enables terminal mouse capture while the floating pane is focused

## Limitations

- The floating surface is app-drawn inside a zoomed split host pane, not a
  native Herdr persistent overlay.
- The backdrop is a solid fill, not your live panes dimmed behind the popup.
- Inner mouse forwarding is best-effort. Border resize and `[-]` minimize are
  handled by the plugin; clicks inside the embedded shell are translated to SGR
  mouse sequences.
- `prefix+cmd+z` depends on your terminal forwarding `cmd/super` chords to
  Herdr. If it does not, bind `herdr-floating-pane.minimize-current` to another
  key.

## Development

```sh
cargo test
cargo build --release
```

## Attribution

This plugin selectively reuses MIT-licensed design ideas and implementation
patterns from [Tyru5/herdr-floax](https://github.com/Tyru5/herdr-floax), and is
inspired by [tmux-floax](https://github.com/omerxx/tmux-floax).
