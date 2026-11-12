> [!IMPORTANT]
> **This is a personal fork of [glzr-io/glazewm](https://github.com/glzr-io/glazewm).** The `release` branch adds the following on top of upstream `main`:
>
> | Feature | Command / config | Default key (in my config) |
> | --- | --- | --- |
> | **Balance window sizes** – evenly resize windows on the workspace (fixes skewed sizes after resizing + moving windows). In the scrolling layout, evenly sizes the columns on screen. | `balance` | `alt+shift+b` |
> | **Scrolling layout** (like [niri](https://github.com/YaLTeR/niri) / PaperWM) – windows are columns on a horizontal strip; the screen scrolls to follow focus. Columns cut off by the screen edge peek in, unless another monitor is on that side. | `layout: { type: "scrolling" }` | – |
> | **Cycle column width** through preset widths (default 33% / 50% / 67%). | `cycle-column-width` | `alt+w` |
> | **Center the focused column.** | `center-column` | `alt+c` |
> | **Scroll animation** (experimental) – columns slide instead of jumping when the screen scrolls. | `layout: { scroll_animation_ms: 150 }` | – |
> | **Jump to first/last column**, or move the focused column there (like niri). | `focus-column-first` / `focus-column-last`, `move-column-to-first` / `move-column-to-last` | `alt+home` / `alt+end`, `alt+ctrl+home` / `alt+ctrl+end` |
> | **Fit N columns per screen** – resize all columns so N fit on screen. | `set-columns-per-screen <n>` | `alt+ctrl+1` … `alt+ctrl+0` (= 10) |
>
> Full docs: [Config: Layout](#config-layout). Minimal config to enable it:
>
> ```yaml
> layout:
>   type: "scrolling"
>   default_column_width: "50%"
>   preset_column_widths: ["33%", "50%", "67%"]
>   center_focused_column: "never" # or "always" / "on_overflow"
> ```
>
> **Install or update** (on top of the [official installer](https://github.com/glzr-io/glazewm/releases), which needs to be installed first). Run in PowerShell; it asks for admin once:
>
> ```powershell
> irm https://raw.githubusercontent.com/vladkosarev/glazewm/release/resources/scripts/update-fork.ps1 | iex
> ```
>
> Every push to the `release` branch is built by GitHub Actions and published to the [`fork-latest`](https://github.com/vladkosarev/glazewm/releases/tag/fork-latest) release. The script downloads it, replaces the GlazeWM executables (the official ones are kept as `*.orig`), and restarts GlazeWM. Re-run it to update.
>
> Builds have UIAccess enabled, so they can manage admin (elevated) windows. Windows only allows this for executables signed by a trusted certificate, so builds are signed with a self-signed code signing certificate ([`resources/fork-signing.cer`](resources/fork-signing.cer)), which the script adds to the machine's Trusted Root store on first install. Programs signed with this certificate will be trusted on that machine. To remove it, run `certlm.msc` as admin and delete "GlazeWM fork (vladkosarev)" under Trusted Root Certification Authorities → Certificates. The certificate is created by [`resources/scripts/new-fork-signing-cert.ps1`](resources/scripts/new-fork-signing-cert.ps1).
>
> **Building locally** (cross-compile from WSL/Linux, no Windows toolchain needed):
>
> ```sh
> sudo apt install mingw-w64
> rustup target add x86_64-pc-windows-gnu
> cargo build --release -p wm -p wm-cli --target x86_64-pc-windows-gnu
> cargo test -p wm --target x86_64-pc-windows-gnu # runs via WSL interop
> ```
>
> Then quit GlazeWM and replace `C:\Program Files\glzr.io\GlazeWM\glazewm.exe` with `target/x86_64-pc-windows-gnu/release/glazewm.exe`, and `...\GlazeWM\cli\glazewm.exe` with `glazewm-cli.exe` (needs admin; back up the originals first).

<div align="center">

> V3 is finally out - check out the changelog [here](https://github.com/glzr-io/GlazeWM/releases) 🔥

  <br>
  <img src="./resources/assets/logo.svg" width="230" alt="GlazeWM logo" />
  <br>

# GlazeWM

**A tiling window manager for Windows inspired by i3wm.**

[![Discord invite][discord-badge]][discord-link]
[![Downloads][downloads-badge]][downloads-link]
[![Good first issues][issues-badge]][issues-link]

GlazeWM lets you easily organize windows and adjust their layout on the fly by using keyboard-driven commands.

[Installation](#installation) •
[Default keybindings](#default-keybindings) •
[Config documentation](#config-documentation) •
[FAQ](#faq) •
[Contributing ↗](https://github.com/glzr-io/glazewm/blob/main/CONTRIBUTING.md)

![Demo video][demo-video]

</div>

### 🌟 Key features

- Simple YAML configuration
- Multi-monitor support
- Optional scrollable tiling layout (similar to niri or PaperWM)
- Customizable rules for specific windows
- Easy one-click installation
- Integration with [Zebar](https://github.com/glzr-io/zebar) as a status bar

## Installation

**The latest version of GlazeWM is downloadable via [releases](https://github.com/glzr-io/GlazeWM/releases).** Zebar can optionally be installed as well via a checkbox during installation.

GlazeWM is also available through several package managers:

**Winget**

```sh
winget install GlazeWM
```

**Chocolatey**

```sh
choco install glazewm
```

**Scoop**

```sh
scoop bucket add extras
scoop install extras/glazewm
```

## Contributing

Help fix something that annoys you, or add a feature you've been wanting for a long time! Contributions are very welcome.

Local development and guidelines are available in the [contributing guide](https://github.com/glzr-io/glazewm/blob/main/CONTRIBUTING.md).

## Default keybindings

On the first launch of GlazeWM, a default configuration can optionally be generated.

Below is a cheat sheet of all available commands and their default keybindings.

![Infographic](/resources/assets/cheatsheet.png)

## Config documentation

The [default config](https://github.com/glzr-io/glazewm/blob/main/resources/assets/sample-config.yaml) file is generated at `%userprofile%\.glzr\glazewm\config.yaml`.

To use a different config file location, you can launch the GlazeWM executable with the CLI argument `--config="..."`, like so:

```sh
./glazewm.exe start --config="C:\<PATH_TO_CONFIG>\config.yaml"
```

Or pass a value for the `GLAZEWM_CONFIG_PATH` environment variable:

```sh
setx GLAZEWM_CONFIG_PATH "C:\<PATH_TO_CONFIG>\config.yaml"
```

With the benefit of using a custom path being that you can choose a different name for the config file, such as `glazewm.yaml`.

### Config: General

```yaml
general:
  # Commands to run when the WM has started (e.g. to run a script or launch
  # another application).
  startup_commands: []

  # Commands to run just before the WM is shutdown.
  shutdown_commands: []

  # Commands to run after the WM config has reloaded.
  config_reload_commands: []

  # Whether to automatically focus windows underneath the cursor.
  focus_follows_cursor: false

  # Whether to switch back and forth between the previously focused
  # workspace when focusing the current workspace.
  toggle_workspace_on_refocus: false

  cursor_jump:
    # Whether to automatically move the cursor on the specified trigger.
    enabled: true

    # Trigger for cursor jump:
    # - 'monitor_focus': Jump when focus changes between monitors.
    # - 'window_focus': Jump when focus changes between windows.
    trigger: "monitor_focus"
```

### Config: Keybindings

The available keyboard shortcuts can be customized via the `keybindings` option. A keybinding consists of one or more key combinations and one or more commands to run when pressed.

It's recommended to use the alt key for keybindings. The Windows key is unfortunately a pain to remap, since the OS reserves certain keybindings (e.g. `lwin+l`).

```yaml
keybindings:
  # Command(s) to run.
  - commands: ["focus --workspace 1"]

    # Key combination(s) to trigger the keybinding.
    bindings: ["alt+1"]

  # Multiple commands can be run in a sequence (e.g. to move a window to a
  # workspace + focus workspace).
  - commands: ["move --workspace 1", "focus --workspace 1"]
    bindings: ["alt+shift+1"]
```

**Full list of keys that can be used for keybindings:**

<details>
<summary>Keys list</summary>

| Key                   | Description                                                               |
| --------------------- | ------------------------------------------------------------------------- |
| `a` - `z`             | Alphabetical letter keys                                                  |
| `0` - `9`             | Number keys                                                               |
| `numpad0` - `numpad9` | Numerical keypad keys                                                     |
| `f1` - `f24`          | Function keys                                                             |
| `shift`               | Either left or right SHIFT key                                            |
| `lshift`              | The left SHIFT key                                                        |
| `rshift`              | The right SHIFT key                                                       |
| `control`             | Either left or right CTRL key                                             |
| `lctrl`               | The left CTRL key                                                         |
| `rctrl`               | The right CTRL key                                                        |
| `alt`                 | Either left or right ALT key                                              |
| `lalt`                | The left ALT key                                                          |
| `ralt`                | The right ALT key                                                         |
| `lwin`                | The left ⊞ Windows logo key                                               |
| `rwin`                | The right ⊞ Windows logo key                                              |
| `space`               | The spacebar key                                                          |
| `escape`              | The ESCAPE key                                                            |
| `back`                | The BACKSPACE key                                                         |
| `tab`                 | The TAB key                                                               |
| `enter`               | The ENTER key                                                             |
| `left`                | The ← arrow key                                                           |
| `right`               | The → arrow key                                                           |
| `up`                  | The ↑ arrow key                                                           |
| `down`                | The ↓ arrow key                                                           |
| `num_lock`            | The NUM LOCK key                                                          |
| `scroll_lock`         | The SCROLL LOCK key                                                       |
| `caps_lock`           | The CAPS LOCK key                                                         |
| `page_up`             | The PAGE UP key                                                           |
| `page_down`           | The PAGE DOWN key                                                         |
| `insert`              | The INSERT key                                                            |
| `delete`              | The DELETE key                                                            |
| `end`                 | The END key                                                               |
| `home`                | The HOME key                                                              |
| `print_screen`        | The PRINT SCREEN key                                                      |
| `multiply`            | The `*` key (only on numpad)                                              |
| `add`                 | The `+` key (only on numpad)                                              |
| `subtract`            | The `-` key (only on numpad)                                              |
| `decimal`             | The DEL key (only on numpad)                                              |
| `divide`              | The `/` key (only on numpad)                                              |
| `volume_up`           | The volume up key                                                         |
| `volume_down`         | The volume down key                                                       |
| `volume_mute`         | The volume mute key                                                       |
| `media_next_track`    | The media next track key                                                  |
| `media_prev_track`    | The media prev track key                                                  |
| `media_stop`          | The media stop key                                                        |
| `media_play_pause`    | The media play/pause key                                                  |
| `oem_semicolon`       | The `;`/`:` key on a US standard keyboard (varies by keyboard)            |
| `oem_question`        | The `/`/`?` key on a US standard keyboard (varies by keyboard)            |
| `oem_tilde`           | The `` ` ``/`~` key on a US standard keyboard (varies by keyboard)        |
| `oem_open_brackets`   | The `[`/`{` key on a US standard keyboard (varies by keyboard)            |
| `oem_pipe`            | The `\`/`\|` key on a US standard keyboard (varies by keyboard)           |
| `oem_close_brackets`  | The `]`/`}` key on a US standard keyboard (varies by keyboard)            |
| `oem_quotes`          | The `'`/`"` key on a US standard keyboard (varies by keyboard)            |
| `oem_8`               | The `` ` ``/`¬` key on a UK keyboard (varies by keyboard)                 |
| `oem_102`             | The `\`/`\|` key next to left Shift on ISO keyboards (varies by keyboard) |
| `oem_plus`            | The `=`/`+` key on a US standard keyboard (varies by keyboard)            |
| `oem_comma`           | The `,`/`<` key on a US standard keyboard (varies by keyboard)            |
| `oem_minus`           | The `-`/`_` key on a US standard keyboard (varies by keyboard)            |
| `oem_period`          | The `.`/`>` key on a US standard keyboard (varies by keyboard)            |
| `muhenkan`            | The 無変換 (non-convert) key for Japanese keyboard layouts                |
| `henkan`              | The 変換 (convert) key for Japanese keyboard layouts                      |

</details>

If a key is not in the list above, it is likely still supported if you use its character in a keybinding (e.g. `alt+å` for the Norwegian Å character).

> German and US international keyboards treat the right-side alt key differently. For these keyboard layouts, use `ralt+ctrl` instead of `ralt` to bind the right-side alt key.

### Config: Gaps

The gaps between windows can be changed via the `gaps` property in the config file. Inner and outer gaps are set separately.

```yaml
gaps:
  # Gap between adjacent windows.
  inner_gap: "20px"

  # Gap between windows and the screen edge.
  outer_gap:
    top: "20px"
    right: "20px"
    bottom: "20px"
    left: "20px"
```

### Config: Layout

Tiling windows can either be tiled to fill the workspace (default), or arranged as columns on a horizontally scrollable strip, similar to [niri](https://github.com/YaLTeR/niri) or PaperWM. The layout is set via the `layout` property in the config file and applies to all workspaces.

```yaml
layout:
  # How tiling windows are arranged within a workspace.
  # - "tiling": Windows are tiled to fill the workspace.
  # - "scrolling": Windows are arranged as columns on a scrollable strip.
  type: "scrolling"

  # (Scrolling layout) Width of newly tiled columns. Percentages are
  # relative to the workspace width.
  default_column_width: "50%"

  # (Scrolling layout) Widths to cycle between via `cycle-column-width`.
  preset_column_widths: ["33%", "50%", "67%"]

  # (Scrolling layout) When to center the focused column.
  # - "never": Scroll the minimum amount needed to show the column.
  # - "always": Always center the focused column.
  # - "on_overflow": Center the column if it doesn't fit on screen
  #   together with the previously focused column.
  center_focused_column: "never"

  # (Scrolling layout) Duration of the scroll animation in milliseconds.
  # Set to 0 to disable. Experimental: some applications may not redraw
  # smoothly while being animated.
  scroll_animation_ms: 0
```

In the scrolling layout, the viewport follows the focused column. Columns cut off by the screen edge are partially shown, unless another monitor is adjacent on that side, in which case they are hidden. Columns entirely outside of the screen are hidden.

Existing commands behave as follows in the scrolling layout:

- `focus --direction left|right` moves focus between columns.
- `move --direction left|right` swaps the column with its neighbor; `up|down` moves the window to the monitor in that direction.
- `resize --width` and `size --width` change the column width; height changes are ignored.
- `toggle-tiling-direction` and `set-tiling-direction` show an error, since columns can't be split into stacks of windows.
- `balance` evenly sizes the columns that are on screen to fill the screen. In the tiling layout, it instead evenly sizes all windows on the workspace.

Additional commands for the scrolling layout:

```yaml
keybindings:
  # Cycle the focused column's width through `preset_column_widths`.
  - commands: ["cycle-column-width"]
    bindings: ["alt+w"]

  # Center the focused column on screen.
  - commands: ["center-column"]
    bindings: ["alt+c"]

  # Focus the first/last column of the workspace.
  - commands: ["focus-column-first"]
    bindings: ["alt+home"]
  - commands: ["focus-column-last"]
    bindings: ["alt+end"]

  # Move the focused column to the start/end of the workspace.
  - commands: ["move-column-to-first"]
    bindings: ["alt+ctrl+home"]
  - commands: ["move-column-to-last"]
    bindings: ["alt+ctrl+end"]

  # Resize all columns such that the given number fit on screen.
  - commands: ["set-columns-per-screen 2"]
    bindings: ["alt+ctrl+2"]
  - commands: ["set-columns-per-screen 3"]
    bindings: ["alt+ctrl+3"]
```

### Config: Workspaces

Workspaces need to be predefined via the `workspaces` property in the config file. A workspace is automatically assigned to each monitor on startup.

```yaml
workspaces:
  # This is the unique ID for the workspace. It's used in keybinding
  # commands, and is also the label shown in 3rd-party apps (e.g. Zebar) if
  # `display_name` is not provided.
  - name: "1"

    # Optional override for the workspace label used in 3rd-party apps.
    # Does not need to be unique.
    display_name: "Work"

    # Optionally force the workspace on a specific monitor if it exists.
    # 0 is your leftmost screen, 1 is the next one to the right, and so on.
    bind_to_monitor: 0

    # Optionally prevent workspace from being deactivated when empty.
    keep_alive: false
```

### Config: Window rules

Commands can be run when a window is first launched. This is useful for adding window-specific behaviors like always starting a window as fullscreen or assigning to a specific workspace.

Windows can be targeted by their process, class, and title. Multiple matching criteria can be used together to target a window more precisely.

```yaml
window_rules:
  - commands: ["move --workspace 1"]
    match:
      # Move browsers to workspace 1.
      - window_process: { regex: "msedge|brave|chrome" }

  - commands: ["ignore"]
    match:
      # Ignores any Zebar windows.
      - window_process: { equals: "zebar" }

      # Ignores picture-in-picture windows for browsers.
      # Note that *both* the title and class must match for the rule to run.
      - window_title: { regex: "[Pp]icture.in.[Pp]icture" }
        window_class: { regex: "Chrome_WidgetWin_1|MozillaDialogClass" }
```

### Config: Window effects

Visual effects can be applied to windows via the `window_effects` option. Currently, colored borders are the only effect available with more to come in the future.

> Note: Window effects are exclusive to Windows 11.

```yaml
window_effects:
  # Visual effects to apply to the focused window.
  focused_window:
    # Highlight the window with a colored border.
    border:
      enabled: true
      color: "#0000ff"

  # Visual effects to apply to non-focused windows.
  other_windows:
    border:
      enabled: false
      color: "#d3d3d3"
```

### Config: Window behavior

The `window_behavior` config option exists to customize the states that a window can be in (`tiling`, `floating`, `minimized`, and `fullscreen`).

```yaml
window_behavior:
  # New windows are created in this state whenever possible.
  # Allowed values: 'tiling', 'floating'.
  initial_state: "tiling"

  # Sets the default options for when a new window is created. This also
  # changes the defaults for when the state change commands, like
  # `set-floating`, are used without any flags.
  state_defaults:
    floating:
      # Whether to center floating windows by default.
      centered: true

      # Whether to show floating windows as always on top.
      shown_on_top: false

    fullscreen:
      # Maximize the window if possible. If the window doesn't have a
      # maximize button, then it'll be made fullscreen normally instead.
      maximized: false
```

### Config: Binding modes

Binding modes are used to modify keybindings while GlazeWM is running.

A binding mode can be enabled with `wm-enable-binding-mode --name <NAME>` and disabled with `wm-disable-binding-mode --name <NAME>`.

```yaml
binding_modes:
  # When enabled, the focused window can be resized via arrow keys or HJKL.
  - name: "resize"
    keybindings:
      - commands: ["resize --width -2%"]
        bindings: ["h", "left"]
      - commands: ["resize --width +2%"]
        bindings: ["l", "right"]
      - commands: ["resize --height +2%"]
        bindings: ["k", "up"]
      - commands: ["resize --height -2%"]
        bindings: ["j", "down"]
      # Press enter/escape to return to default keybindings.
      - commands: ["wm-disable-binding-mode --name resize"]
        bindings: ["escape", "enter"]
```

## FAQ

**Q: How do I run GlazeWM on startup?**

Right-click the GlazeWM icon in the system tray and select "Run on system startup".

**Q: How can I create `<insert layout>`?**

You can create custom layouts by changing the tiling direction with `alt+v`. This changes where the next window is placed _in relation to the current window_. If the current window's direction is horizontal, the new window will be placed to the right of it. If it is vertical, it will be placed below it. This also applies when moving windows; the tiling direction of the stationary window will affect where the moved window will be placed.

Community-made scripts like [Dutch-Raptor/GAT-GWM](https://github.com/Dutch-Raptor/GAT-GWM) and [burgr033/GlazeWM-autotiling-python](https://github.com/burgr033/GlazeWM-autotiling-python) can be used to automatically change the tiling direction. Native support for automatic layouts isn't _currently_ supported.

**Q: How do I create a rule for `<insert application>`?**

To match a specific application, you need a command to execute and either the window's process name, title, or class name. For example, if you use Flow-Launcher and want to make the settings window float, you can do the following:

```yaml
window_rules:
  - commands: ["set-floating"]
    match:
      - window_process: { equals: "Flow.Launcher" }
        window_title: { equals: "Settings" }
```

Programs like Winlister or AutoHotkey's Window Spy can be useful for getting info about a window.

**Q: How can I ignore GlazeWM's keybindings when `<insert application>` is focused?**

This isn't currently supported, however, the keybinding `alt+shift+p` in the default config is used to disable all other keybindings until `alt+shift+p` is pressed again.

[discord-badge]: https://img.shields.io/discord/1041662798196908052.svg?logo=discord&colorB=7289DA
[discord-link]: https://discord.gg/ud6z3qjRvM
[downloads-badge]: https://img.shields.io/github/downloads/glzr-io/glazewm/total?logo=github&logoColor=white
[downloads-link]: https://github.com/glzr-io/glazewm/releases
[issues-badge]: https://img.shields.io/badge/good_first_issues-7057ff
[issues-link]: https://github.com/orgs/glzr-io/projects/4/views/1?sliceBy%5Bvalue%5D=good+first+issue
[demo-video]: resources/assets/demo.webp
