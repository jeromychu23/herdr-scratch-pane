# herdr-scratch-pane

A Rust Herdr plugin for native zoomed scratch panes.

`herdr-scratch-pane` gives Herdr a fast, persistent scratch shell that opens as
a normal Herdr pane and immediately zooms to the full workspace. It is designed
for full-screen terminal apps such as `yazi`, `btop`, editors, REPLs, and long
running shell jobs.

## What It Is

This plugin uses Herdr's own pane renderer. When you open the scratch pane,
Herdr creates a regular plugin pane, focuses it, and the plugin zooms that pane
with Herdr's native pane API. Because Herdr still owns rendering, selection,
mouse input, resize mode, terminal responses, and TUI layout, applications
inside the scratch pane behave like they do in a normal Herdr pane.

This plugin intentionally does not draw a transparent floating terminal UI. It
does not use `ratatui`, `vt100`, embedded PTYs, terminal mouse capture, or an
app-managed renderer.

## Requirements

- macOS
- Herdr `0.7.1` or newer
- Rust toolchain with `cargo`
- `dtach`

Install `dtach` on macOS:

```sh
brew install dtach
```

Herdr builds plugins with the commands declared in `herdr-plugin.toml`. This
plugin uses `cargo build --release`, so `cargo` must be available when the
plugin is installed from GitHub.

## Install From GitHub

After publishing this repository, install it with Herdr's GitHub plugin install
syntax:

```sh
herdr plugin install owner/repo
herdr plugin action list --plugin herdr-scratch-pane
herdr plugin action invoke herdr-scratch-pane.install-keybindings
```

Replace `owner/repo` with the published GitHub repository. To appear in Herdr's
community plugin index, the public GitHub repository should use the
`herdr-plugin` topic.

## Local Development

For local testing, build the binary yourself and link the working tree:

```sh
cargo build --release
herdr plugin link /path/to/herdr-scratch-pane
./target/release/herdr-scratch-pane install-keybindings
```

Reload Herdr config or restart Herdr after installing keybindings.

`herdr plugin link` registers the local directory but does not run the build
command. Rebuild manually after changing Rust code.

## Keybindings

The default keybindings installed by `install-keybindings` are:

- `prefix+f`: toggle workspace scratch pane
- `prefix+shift+f`: toggle session scratch pane
- `prefix+cmd+z`: minimize the currently visible scratch pane
- existing Herdr split keys: proxied through `herdr-scratch-pane.safe-split-*`

Only one scratch host pane is visible at a time. Workspace and session scratch
panes use separate `dtach` sessions, so they do not share cwd or running
processes.

## How It Works

Opening a scratch pane asks Herdr to open a normal plugin pane with split
placement, then immediately runs `herdr pane zoom --on` for that pane. The pane
command starts `dtach`, attached to a scope-specific socket.

Minimizing closes only the Herdr host pane. The shell and any running processes
remain alive inside `dtach`. Toggling again opens a fresh host pane and
reattaches to the same `dtach` session.

While the scratch session is hidden, the plugin marks the workspace label with
` [scratch-on]`. Revealing the scratch pane restores the original workspace
label. If the workspace is renamed by hand while scratch is hidden, the plugin
does not overwrite the user's new label.

The keybinding installer rewires Herdr's split keys through plugin actions.
Outside scratch panes, those actions delegate to `herdr pane split --current`.
When a scratch pane is visible, they show a notification instead of letting
Herdr unzoom and split the underlying layout target.

## Keybinding Installer

Run:

```sh
herdr plugin action invoke herdr-scratch-pane.install-keybindings
```

When developing locally, the equivalent direct binary command is:

```sh
./target/release/herdr-scratch-pane install-keybindings
```

The installer edits `~/.config/herdr/config.toml` by default. It creates a
timestamped backup next to the config before changing an existing file.

The installer is idempotent. Re-running it preserves keys that were previously
assigned to its managed commands. It only manages these commands:

- `herdr-scratch-pane.toggle-workspace`
- `herdr-scratch-pane.toggle-session`
- `herdr-scratch-pane.minimize-current`
- `herdr-scratch-pane.safe-split-right`
- `herdr-scratch-pane.safe-split-down`

Other custom plugin actions are preserved, even if their command name starts
with `herdr-scratch-pane.`.

Use `--no-split-proxy` if you do not want the installer to rewrite native split
keys through the scratch split guard.

Manual configuration can be used instead. If your config already has a `[keys]`
table, do not paste another `[keys]` header; add the fields and command tables
inside the existing keys section.

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

Herdr plugins are ordinary executables that run as your user. Herdr validates
the manifest and keeps plugin config/state in plugin-specific directories, but
it does not sandbox third-party plugin code. Review `herdr-plugin.toml` and the
Rust source before installing. See Herdr's plugin trust model:
https://herdr.dev/docs/plugins/#trust-and-security

This plugin does not make network requests at runtime.

It performs these local actions:

- runs the `herdr` CLI through `HERDR_BIN_PATH` when available
- opens, zooms, closes, lists, and inspects Herdr panes
- calls `herdr workspace get` and `herdr workspace rename` for the
  ` [scratch-on]` workspace marker
- starts `dtach` plus your login shell inside a Herdr pane
- writes state and `dtach` sockets under Herdr's plugin state directory
- edits `~/.config/herdr/config.toml` only when you explicitly run
  `install-keybindings`

The plugin root is treated as source code. Durable runtime state belongs in
Herdr's plugin state directory, not in the repository checkout.

## Limitations

- This is a native zoomed scratch pane, not a transparent draggable overlay.
- It does not provide a mouse `[-]` button or app-drawn floating-box resize.
- `prefix+cmd+z` depends on your terminal forwarding Command/Super chords to
  Herdr. If it does not, bind `herdr-scratch-pane.minimize-current` to another
  key.
- Unix-like systems are required because the pane command uses `exec` and
  `dtach`.

## Troubleshooting

If `dtach` is missing, install it and rebuild/reopen the plugin:

```sh
brew install dtach
```

If `prefix+cmd+z` does not minimize, your terminal probably does not forward
that chord. Rebind `herdr-scratch-pane.minimize-current` to a key Herdr
receives.

If the workspace label keeps ` [scratch-on]`, reveal the scratch pane once with
`prefix+f`. The plugin restores labels only when it can confirm it wrote that
marker. If you renamed the workspace manually while scratch was hidden, your
manual label is preserved.

If split keys show "Scratch pane split is disabled", a scratch pane is visible
or the plugin state says a scratch host pane is still visible. Toggle or
minimize the scratch pane before splitting the main layout.

If `yazi`, `btop`, or another TUI behaves oddly, verify that the pane is opened
by this native zoomed implementation and not an older floating renderer build.
This implementation should not intercept mouse input or terminal responses.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The test suite covers Herdr JSON parsing, command argument construction,
toggle/minimize/safe-split decisions, workspace marker restoration, keybinding
installation, and `dtach` command construction.

## Attribution

This plugin was originally explored from a tmux-floax-style idea and compared
against [Tyru5/herdr-floax](https://github.com/Tyru5/herdr-floax). The current
implementation uses Herdr native pane rendering rather than an app-drawn
floating box.
