# Yazelix Zellij Popup

Yazelix Zellij Popup is a small Zellij plugin for toggling configured floating TUI popups. The Zellij plugin alias and wasm artifact are `yzpp`.

It gives plain Zellij users the popup behavior that Yazelix uses for tools like git UIs: press one key to open the floating pane, press it again from another pane to focus it, press it while the popup is focused to close it.

## Install

Download `yzpp.wasm` and `yzpp.wasm.sha256` from the
[latest release](https://github.com/Yazelix/zellij-popup/releases/latest), place
them in the same directory, and verify the download:

```bash
# Linux
sha256sum --check yzpp.wasm.sha256

# macOS
shasum --algorithm 256 --check yzpp.wasm.sha256
```

To build without Nix, install the WASI target and compile the plugin:

```bash
rustup target add wasm32-wasip1
cargo build --locked --target wasm32-wasip1 --profile release
```

The artifact is `target/wasm32-wasip1/release/yzpp.wasm`.

To build the Nix package:

```bash
nix build .#yazelix-zellij-popup
```

`.#yzpp` is also provided as a short package alias for the plugin artifact.

The package installs:

- `share/yazelix_zellij_popup/yzpp.wasm`
- `share/yazelix_zellij_popup/examples/gitui.kdl`
- `share/yazelix_zellij_popup/examples/gitui.template.kdl`

## Configure

Add the plugin and a popup spec to your Zellij config:

```kdl
plugins {
    yzpp location="file:/path/to/yzpp.wasm" {
        popup_defaults {
            side_margin 1
            vertical_margin 0
        }

        popup {
            command "gitui"
            pane_title "gitui_popup"
            command_marker "gitui"
            cwd "."
        }
    }
}

load_plugins {
    yzpp
}

keybinds {
    normal {
        bind "Alt g" {
            MessagePlugin "yzpp" {
                name "toggle"
            }
        }
    }
}
```

The message `name` is the action. Supported actions are `toggle`, `open`,
`focus`, `replace`, `hide`, and `close`. When there is only one configured popup
or a `default` popup, no payload is needed. A caller that owns runtime launch
values can send `{"id":"agent","cwd":"/repo","args":["resume"]}` instead of a
plain popup id; the configured spec remains the command and layout owner while
the request arguments are appended for that launch. `focus` never hides a live
popup and replaces it when its remembered launch cwd differs from an explicit
request cwd. `replace` closes any matching live pane, runs its configured
`on_close` hook, and then opens a fresh one.

## Popup Specs

The default popup uses a nested `popup` block:

```kdl
popup {
    command "gitui"
    arg_1 "--watch"
    pane_title "gitui_popup"
    command_marker "gitui"
    cwd "."
}
```

Required:

- `command`

Optional:

- `arg_1`, `arg_2`, and so on for argv arguments
- `pane_title`, defaulting to `default_popup`
- `preserve_terminal_title`, defaulting to `false`; when `true`, the launch title is cleared, pane identity uses the command marker, and the application may publish its terminal title
- `command_marker`, defaulting to the command path
- `cwd`, defaulting to the focused terminal pane cwd; relative values resolve against that focused cwd
- `on_close`, an optional command hook run when `yzpp` closes the popup through `toggle` or `close`
- `on_hide`, an optional command hook run when `yzpp` hides the popup through focused `toggle` or popup displacement
- `toggle_close_behavior`, either `close` or `hide`, defaulting to `close`
- `side_margin`, the exact terminal-cell inset from each horizontal edge, defaulting to `popup_defaults.side_margin` or `0`
- `vertical_margin`, the exact terminal-cell inset from each vertical edge, defaulting to `popup_defaults.vertical_margin` or `0`

In `yzpp`, a margin is empty space outside the popup pane, between the viewport
edge and the popup border. It is not padding between the popup border and the
application content. Margins define the popup size from the current viewport:
a side margin of `2` uses two terminal cells on both the left and right, while a
vertical margin of `1` uses one row above and below. Zero margins fill the
viewport. Oversized margins are clamped so at least one row and column remain
visible. Commands are argv, not shell strings.

Use `popup_defaults` to share margins and lifecycle hooks across configured popups:

```kdl
popup_defaults {
    side_margin 1
    vertical_margin 0

    on_close {
        command "hook"
        arg_1 "close"
    }

    on_hide {
        command "hook"
        arg_1 "hide"
    }
}

popups {
    gitui {
        command "gitui"
    }

    lazygit {
        command "lazygit"
        side_margin 3
        vertical_margin 2
    }
}
```

Per-popup `side_margin`, `vertical_margin`, `on_close`, and `on_hide` override the shared defaults.

Hooks are also argv, not shell strings:

```kdl
popup {
    command "lazygit"
    pane_title "lazygit_popup"
    on_close {
        command "hook"
        arg_1 "close"
    }
    on_hide {
        command "hook"
        arg_1 "hide"
    }
}
```

`on_close` runs only when `yzpp` closes the pane in response to `toggle` or `close`. It does not run when the child process exits on its own.

`on_hide` runs only when `yzpp` hides the pane through `hide`, a focused
hide-mode toggle, or displacement by another configured popup. It does not run
when the pane is opened, shown, focused, explicitly closed, or when the child
process exits on its own.

Use `toggle_close_behavior "hide"` for monitor TUIs that should keep process state between toggles:

```kdl
popup {
    command "btm"
    pane_title "btm_popup"
    toggle_close_behavior "hide"
}
```

With `hide`, pressing the toggle key while the popup is focused and the floating layer is visible, replacing it with another configured popup, or sending the explicit `hide` action hides that pane without killing the popup process and runs `on_hide`. Pressing the toggle key again shows and focuses the existing pane, including when Zellij hid the floating layer while moving focus to a tiled pane, provided its remembered launch cwd still matches the effective request cwd. Applications may change their own process cwd while running without becoming stale. If the requested launch cwd changes, toggle closes the stale pane (and runs `on_close` if set) and opens a fresh popup there. The explicit `close` action still closes the pane and runs `on_close`.

Toggling a focused popup off, explicitly hiding it, or explicitly closing it also hides the current
tab's floating layer. This returns focus to the tiled workspace instead of
exposing an unrelated floating pane that was underneath the popup. Other
floating panes keep running and reappear when the user explicitly shows the
floating layer again.

For multiple popups in the same plugin config, use a nested `popups` block and send the popup id as the payload:

```kdl
popups {
    gitui {
        command "gitui"
        pane_title "gitui_popup"
        side_margin 2
        vertical_margin 1
    }

    lazygit {
        command "lazygit"
        pane_title "lazygit_popup"
        side_margin 1
        vertical_margin 0
    }
}
```

```kdl
MessagePlugin "yzpp" {
    name "toggle"
    payload "lazygit"
}
```

## Raw Pipe API

Generated integrations may still send the raw JSON request shape through `name "transient_popup"`:

```kdl
MessagePlugin "yzpp" {
    name "transient_popup"
    payload "{\"action\":\"toggle\",\"spec\":{\"id\":\"gitui\",\"pane_title\":\"gitui_popup\",\"command_marker\":\"gitui\",\"command\":[\"gitui\"],\"cwd\":\".\",\"side_margin\":1,\"vertical_margin\":0},\"args\":[]}"
}
```

That raw path exists for generated callers. Human-authored Zellij config should prefer configured popup specs plus `name "toggle"`.

Raw JSON specs support the same optional `on_close` and `on_hide` hook objects as configured popup specs.

## Permissions

Zellij prompts for plugin permissions when the plugin first loads. `yzpp` requests:

- `ReadApplicationState`
- `ChangeApplicationState`
- `OpenTerminalsOrPlugins`
- `RunCommands`
- `ReadCliPipes`

These permissions cover pane discovery, opening command panes, focusing and closing the managed pane, and receiving `MessagePlugin` pipe requests.

## Verify

```bash
cargo test
nix build
```
