# herdr-scratch-pane

A Herdr plugin that provides a persistent scratch pane using Herdr's native pane
and zoom behavior.

It is built for the workflow where you want to quickly open a temporary shell,
run tools, hide it, and bring it back later without killing the process.

## Why This Exists

The original idea was a tmux-floax-style floating pane for Herdr. In practice,
an app-rendered floating terminal is fragile: mouse input, text selection,
terminal resize, and full-screen TUI apps such as `yazi` and `btop` can easily
break.

This plugin takes a more stable approach:

- open a normal Herdr plugin pane
- zoom it with Herdr's native pane API
- keep the shell alive in `dtach` when the pane is hidden

The result is not a transparent draggable overlay. It is a native zoomed
scratch pane that keeps Herdr responsible for terminal rendering, resize mode,
mouse behavior, and TUI compatibility.

## What It Does

- `prefix+f` toggles a workspace scratch pane.
- `prefix+shift+f` toggles a session scratch pane.
- `prefix+cmd+z` hides the visible scratch pane while keeping its process alive.
- Hidden scratch sessions continue running in the background through `dtach`.
- Workspaces with a hidden scratch session show a ` [scratch-on]` label marker.
- Split keybindings can be proxied so split actions do not accidentally split
  the underlying layout while a scratch pane is active.

Only one scratch host pane is visible at a time.

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

## Local Development

```sh
cargo build --release
herdr plugin link /path/to/herdr-scratch-pane
./target/release/herdr-scratch-pane install-keybindings
```

`herdr plugin link` does not run the build command, so rebuild manually after
changing Rust code.

## Usage

| Keybinding | Action |
| --- | --- |
| `prefix+f` | Toggle workspace scratch pane |
| `prefix+shift+f` | Toggle session scratch pane |
| `prefix+cmd+z` | Hide the visible scratch pane |
| existing split keys | Split normally outside scratch panes; block split while scratch is active |

## Design Notes

This plugin intentionally avoids custom terminal rendering. It does not use
`ratatui`, `vt100`, embedded PTYs, or terminal mouse capture.

The scratch process is persisted with `dtach`. When a scratch pane is hidden,
Herdr closes the host pane, but the shell session keeps running. Toggling the
scratch pane again opens a fresh Herdr pane and reattaches to the same session.

## Limitations

- This is not a transparent draggable floating box.
- It does not provide mouse resize handles or a mouse minimize button.
- `prefix+cmd+z` depends on your terminal forwarding Command/Super key chords
  to Herdr. If it does not work, bind `herdr-scratch-pane.minimize-current` to
  another key.
- The current implementation targets macOS and Unix-like behavior because it
  uses `dtach` and process `exec`.

## Trust And Security

Herdr plugins are ordinary executables that run as your user. Review
`herdr-plugin.toml` and the source before installing third-party plugins.

This plugin does not make network requests at runtime. It calls the Herdr CLI,
starts `dtach` plus your shell, writes plugin state for the `dtach` socket, and
edits Herdr keybindings only when you explicitly run `install-keybindings`.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Attribution

This project was inspired by the tmux-floax scratch pane workflow and compared
against [Tyru5/herdr-floax](https://github.com/Tyru5/herdr-floax). The current
implementation uses Herdr native panes rather than an app-rendered floating
terminal.

## License

MIT
