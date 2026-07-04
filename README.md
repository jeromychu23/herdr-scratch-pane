# herdr-scratch-pane

A Rust Herdr plugin for native zoomed scratch panes.

This is a fast toggleable shell pane, not a transparent floating overlay. It
uses Herdr's native pane rendering so full-screen terminal apps such as `yazi`
and `btop` keep their normal layout, mouse behavior, and terminal responses.

## Keybindings

- `prefix+f`: toggle workspace scratch pane
- `prefix+shift+f`: toggle session scratch pane
- `prefix+cmd+z`: minimize the currently visible scratch pane
- existing Herdr split keys: proxied through `herdr-scratch-pane.safe-split-*`
  after `install-keybindings`

Only one scratch host pane is visible at a time. Workspace and session shells
use separate `dtach` sessions, so they do not share cwd or running processes.

## How It Works

When you toggle a scratch pane, the plugin asks Herdr to open a normal split
pane and immediately zooms it with `herdr pane zoom --on`. The process inside
that pane is `dtach`, attached to a scope-specific session.

When you minimize, the Herdr host pane is closed. The shell and any running
processes remain alive inside `dtach`. Toggling again opens a fresh host pane
and reattaches to the same session.

Because the host pane is closed while the `dtach` session keeps running, the
plugin reports display-only Herdr pane metadata on the original host pane:
`scratch running`. Revealing the scratch pane clears that marker.

The keybinding installer also rewires Herdr's split keys through plugin actions.
Outside scratch panes, the proxy delegates to `herdr pane split --current`.
Inside scratch panes, it shows a notification instead of letting Herdr unzoom
and split the underlying layout target.

This plugin intentionally does not use `ratatui`, `vt100`, embedded PTYs, or
terminal mouse capture. Herdr remains responsible for terminal rendering,
selection, mouse delivery, resize mode, and TUI compatibility.

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
./target/release/herdr-scratch-pane install-keybindings
```

Then reload Herdr config or restart Herdr.

The installer updates `~/.config/herdr/config.toml` idempotently and writes a
timestamped backup next to it before changing an existing file. By default it
preserves your current split keys, disables Herdr's native split bindings, and
adds plugin proxy actions for those same keys. Use `--no-split-proxy` if you do
not want the split guard.

Manual equivalent:

```toml
# herdr-scratch-pane:keybindings
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

## Trust And Security

This plugin runs a local Rust binary built by `cargo build --release` during
Herdr plugin installation or linking. It does not make network requests at
runtime.

It performs these local actions:

- calls the `herdr` CLI to open, zoom, close, and inspect panes
- calls `herdr pane report-metadata` for the background scratch marker
- starts `dtach` plus your login shell directly in a Herdr pane
- writes state under Herdr's plugin state directory for `dtach` sockets
- writes `~/.config/herdr/config.toml` only when you explicitly run
  `install-keybindings`

## Limitations

- This is a native zoomed scratch pane, not a transparent overlay floating
  above another visible pane.
- It does not provide draggable floating-box resize or a mouse `[-]` button.
  Use Herdr's native pane/zoom behavior instead.
- `prefix+cmd+z` depends on your terminal forwarding `cmd/super` chords to
  Herdr. If it does not, bind `herdr-scratch-pane.minimize-current` to another
  key.

## Development

```sh
cargo test
cargo build --release
```

## Attribution

This plugin was originally explored from a tmux-floax-style idea and compared
against [Tyru5/herdr-floax](https://github.com/Tyru5/herdr-floax). The current
implementation uses Herdr native pane rendering rather than an app-drawn
floating box.
