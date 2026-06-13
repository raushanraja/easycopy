# easycopy

Linux clipboard history manager. Daemon monitors the clipboard, popup lets you browse, search, and paste.

<p align="center">
  <img src="assets/minimal.png" alt="easycopy popup showing clipboard history" width="360">
</p>

## Screenshots

<p>
  <img src="assets/app_search.png" alt="App search mode in easycopy" width="330">
  <img src="assets/right_click_image_preview.png" alt="Image preview in easycopy" width="250">
</p>

<p>
  <img src="assets/theme_selection.png" alt="Theme selection menu in easycopy" width="330">
  <img src="assets/footer.png" alt="Footer controls in easycopy" width="330">
</p>

## Usage

```bash
easycopy          # start daemon
easycopy --popup  # open popup
easycopy --clear  # delete history and saved images
easycopy -V       # version
easycopy -h       # help
```

Default hotkey: **Ctrl+Alt+V**.

## Configuration

First run creates `~/.config/easycopy/config.toml`.

<details>
<summary>Default config</summary>

```toml
[general]
max_text_items = 200
max_image_items = 50
hotkey = "Ctrl+Alt+V"
auto_paste = true
poll_interval_ms = 500
popup_width = 640.0
popup_height = 720.0
preview_chars = 220
paste_delay_ms = 120
theme = "dark"
hide_main_header = false
hide_secondary_header = false
hide_counts = false
enable_theming = true
enable_clipping = true
close_on_focus_out = true
keep_search_on_reopen = true
debug_logging = false
font_preset = "default"
font_size = "medium"
font_proportional_path = ""
font_monospace_path = ""
font_weight = "normal"

[footer]
enable = true
show_help = true
show_clear = true
show_settings = true
show_theme = true
```
</details>

Supported values:

- `theme`: `dark`, `light`, `nord`, `catppuccin`, `dracula`, `system`
- `font_preset`: `default`, `dejavu`, `liberation`, `fira`, `jetbrains`, `iosevka`
- `font_size`: `small`, `medium`, `large`
- `font_weight`: `normal`, `bold`

The popup also has an in-app settings dropdown where themes, fonts, and font size can be changed and saved instantly.

## i3 integration

<details>
<summary>Auto-start and borderless popup</summary>

Add to your i3 config:

```
exec_always --no-startup-id /path/to/easycopy
for_window [class="easycopy"] \
    floating enable, \
    border none, \
    move position center
```

The popup already sets `decorations(false)` and `always_on_top`, so with the `border none` rule it appears as a clean floating window.
</details>

<details>
<summary>systemd user service</summary>

```ini
[Unit]
Description=easycopy clipboard history daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart=%h/.local/bin/easycopy
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
```

Save as `~/.config/systemd/user/easycopy.service`, then:

```bash
systemctl --user enable --now easycopy
```
</details>

## Notes

- **Auto-paste** uses `xdotool` (X11 only). On strict Wayland, set `auto_paste = false`.
- **X11 event source** — when running on X11, clipboard changes are detected via the XFixes extension (event-driven, no CPU polling). Falls back to timer polling on Wayland.
- **Global hotkeys** depend on desktop environment permissions.

## Build

See [BUILD.md](BUILD.md) for source build instructions and distro dependencies.
