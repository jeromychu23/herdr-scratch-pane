# herdr-scratch-pane

A Herdr plugin for a persistent native floating scratch popup.

Herdr renders an 85% × 80% popup over the active pane, so the main pane remains
visible without changing its tiled layout. A private `tmux` session keeps the
shell, working directory, running process, and current terminal viewport alive
while the popup is hidden. Terminal apps such as `yazi`, `btop`, editors, and
REPLs therefore resume where they were left.

## v0.1.2 Highlights

- Replaced the old split-and-zoom host with Herdr 0.7.4's native popup.
- Added adaptive 85% × 80% floating geometry without changing the tiled layout.
- Preserved shell prompts, working directories, command output, and full-screen
  TUIs across repeated hide/reopen cycles with tmux.
- Added `prefix+f/F` hide and confirmed `prefix+x` session termination inside
  the modal popup.
- Migrated existing tmux-backed scratch sessions without killing their running
  processes.

## What It Does

- Toggle a workspace floating scratch popup.
- Toggle a session floating scratch popup.
- Keep the underlying main pane visible without changing its layout.
- Hide the popup without killing its process using `prefix+f` or `prefix+F`.
- Close the popup and its process using confirmed `prefix+x`.
- Keep hidden scratch sessions and their current viewport in `tmux`.
- Mark workspaces with a hidden scratch session using ` [scratch-on]`.

This uses Herdr's native session-modal popup rather than drawing a fake terminal
backdrop. The popup follows the terminal size automatically but is not
mouse-draggable.

## Requirements

- macOS
- Herdr `0.7.4` or newer
- Rust toolchain with `cargo`
- `tmux`

Install `tmux`:

```sh
# Install tmux so the scratch shell and viewport persist while hidden.
brew install tmux
```

## Installation

Install from GitHub:

```sh
# Install the plugin from this GitHub repository.
herdr plugin install jeromychu23/herdr-scratch-pane

# Add the recommended keybindings to your Herdr config.
herdr plugin action invoke herdr-scratch-pane.install-keybindings
```

Reload Herdr config after installing keybindings:

```sh
herdr server reload-config
```

If you use named Herdr sessions, reload each running session that should
receive the new keys.

For local development:

```sh
# Build the plugin binary.
cargo build --release

# Link this local checkout into Herdr.
herdr plugin link /path/to/herdr-scratch-pane

# Install keybindings from the local binary.
./target/release/herdr-scratch-pane install-keybindings
```

After the plugin is linked, rebuilding the same
`target/release/herdr-scratch-pane` path takes effect on the next toggle; the
toggle command explicitly requests native popup placement and geometry. After
upgrading from the old zoomed-pane release, run `install-keybindings` once more
and reload Herdr config so the obsolete `prefix+cmd+z` binding is removed.
Re-link the checkout when `herdr-plugin.toml` changes so direct plugin entrypoint
metadata is refreshed.

## Updating

Herdr does not have a separate `plugin update` command yet. Reinstall the
GitHub plugin to update the Herdr-managed checkout:

```sh
# Update to the latest version from GitHub.
herdr plugin install jeromychu23/herdr-scratch-pane

# Refresh the generated keybindings after updating.
herdr plugin action invoke herdr-scratch-pane.install-keybindings

# Reload Herdr config.
herdr server reload-config
```

If you use named Herdr sessions, reload each running session that should receive
the updated keybindings:

```sh
# Reload config for a named Herdr session.
herdr --session <name> server reload-config
```

For local development:

```sh
# Pull the latest source.
git pull

# Rebuild the local plugin binary.
cargo build --release
```

The existing toggle keybindings invoke this same binary path, so a local release
rebuild takes effect on the next toggle. Run `install-keybindings` and reload
config only when the generated bindings themselves change.

To pin this release, pass `--ref`:

```sh
# Install a specific release tag.
herdr plugin install jeromychu23/herdr-scratch-pane --ref v0.1.2
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
```

The keybindings intentionally use `type = "shell"` instead of
`type = "plugin_action"`. Herdr named sessions share the same config, but a new
session may not have the plugin action registry loaded yet. Calling the binary
directly keeps `prefix+f` and `prefix+shift+f` working across sessions.

If you installed an earlier version, run `install-keybindings` again. It updates
old scratch pane keybindings, removes the previous split-key workaround, and
removes the obsolete `prefix+cmd+z` minimize binding.

Herdr routes all input to an open popup, so the outer Herdr bindings open it and
the private tmux key table handles its lifecycle:

| Context | Key | Result |
| --- | --- | --- |
| Herdr | `prefix+f` | Open the workspace scratch popup |
| Herdr | `prefix+shift+f` | Open the session-scoped popup |
| Popup | `prefix+f` or `prefix+F` | Hide the popup and keep its session running |
| Popup | `prefix+x` | Confirm before ending only this scratch session |
| Popup | Press prefix twice | Send the literal prefix to the running application |

## Upgrading From v0.1.1

v0.1.2 requires Herdr 0.7.4 because native popup support is provided by Herdr.
After updating the plugin, regenerate the managed keybindings once and reload
config:

```sh
herdr plugin install jeromychu23/herdr-scratch-pane --ref v0.1.2
herdr plugin action invoke herdr-scratch-pane.install-keybindings
herdr server reload-config
```

This removes the obsolete `prefix+cmd+z` binding. Existing tmux sessions remain
available and are attached by the new popup runtime; legacy dtach sessions are
left untouched for manual recovery.

## Recovering Legacy dtach Sessions

Upgrading does not automatically kill old `dtach` processes or remove their
sockets. If an earlier plugin version still has a shell running, first locate
the legacy socket directory:

```sh
# Explicit scratch state wins, followed by Herdr's plugin state directory.
state_dir="${HERDR_SCRATCH_PANE_STATE_DIR:-${HERDR_PLUGIN_STATE_DIR:-${TMPDIR:-/tmp}/herdr-scratch-pane}}"
find "$state_dir" -name '*.dtach' -print
```

Attach to the socket you want to recover:

```sh
dtach -a <socket> -r winch -z
```

Save any needed work, then run `exit` inside that recovered shell so its
process ends normally. Only after confirming that process has stopped should
you remove the exact `.dtach` socket if it remains. The plugin deliberately
does not perform this cleanup automatically.

## Limitations

- Herdr popups are session-modal. The underlying pane stays visible but cannot
  receive keyboard or mouse input until the popup closes.
- Only one Herdr popup can be visible in a session at a time. Hidden workspace
  and session tmux sessions may still coexist independently.
- The popup is percentage-based and follows outer terminal resize, but Herdr
  0.7.4 does not expose draggable popup borders or a public runtime resize API.
- The private tmux server exposes only hide, confirmed close, and send-prefix
  bindings. Its status bar, mouse mode, and normal command table remain hidden.

## Trust And Security

Herdr plugins are ordinary executables that run as your user. Review
`herdr-plugin.toml` and the source before installing third-party plugins.

This plugin does not make network requests at runtime. It calls the Herdr CLI,
starts a private `tmux` server plus your shell, writes its local plugin state,
and edits Herdr keybindings only when you explicitly run
`install-keybindings`.

## License

MIT
