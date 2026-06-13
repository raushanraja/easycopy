# clipit-rs

Linux clipboard history manager. Daemon monitors the clipboard, popup lets you browse, search, and paste.

## Prerequisites

- **Rust** — latest stable (install via [rustup](https://rustup.rs)).
- **X11** — runtime only; falls back to polling on Wayland.
- **xdotool** — for auto-paste on X11.

### System libraries (build time)

<details>
<summary>Debian / Ubuntu</summary>

```bash
sudo apt install build-essential pkg-config \
  libx11-dev libxcb1-dev libxkbcommon-dev \
  libxrandr-dev libxi-dev libxcursor-dev libxext-dev \
  libxss-dev libxtst-dev libxft-dev libxinerama-dev \
  libwayland-dev libxkbcommon-x11-dev xdotool
```
</details>

<details>
<summary>Arch Linux</summary>

```bash
sudo pacman -S --needed base-devel pkgconf \
  libx11 libxcb libxkbcommon libxrandr libxi libxcursor \
  libxext libxss libxtst libxft libxinerama wayland libxkbcommon-x11 xdotool
```
</details>

<details>
<summary>Fedora</summary>

```bash
sudo dnf install gcc pkg-config \
  libX11-devel libxcb-devel libxkbcommon-devel \
  libXrandr-devel libXi-devel libXcursor-devel libXext-devel \
  libXScrnSaver-devel libXtst-devel libXft-devel libXinerama-devel \
  wayland-devel libxkbcommon-x11-devel xdotool
```
</details>

## Build

```bash
git clone https://github.com/your-username/clipit-rs
cd clipit-rs
cargo build --release
```

Binary at `./target/release/clipit-rs`.

## Usage

```bash
clipit-rs              # start daemon
clipit-rs --popup      # open popup
clipit-rs --clear      # delete history + saved images
clipit-rs -V           # version
clipit-rs -h           # help
```

Default hotkey: **Ctrl+Alt+V**.

## Configuration

First run creates `~/.config/easycopy/easycopy.toml`.

<details>
<summary>Full config reference</summary>

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
theme = "dark"            # dark | light | nord | catppuccin | dracula
enable_theming = true
keep_search_on_reopen = true
debug_logging = false
font_preset = "default"   # default | dejavu | liberation | fira | jetbrains | iosevka
font_size = "medium"      # small | medium | large
font_weight = "normal"    # normal | bold

[footer]
enable = true
show_help = true
show_clear = true
show_settings = true
show_theme = true
```
</details>

The popup also has an in-app settings dropdown (palette icon in the footer) where themes, fonts, and font size can be changed and saved instantly.

## i3 integration

<details>
<summary>Auto-start and borderless popup</summary>

Add to your i3 config:

```
exec_always --no-startup-id /path/to/clipit-rs
for_window [class="clipit-rs"] border none
```

The popup already sets `decorations(false)` and `always_on_top`, so with the `border none` rule it appears as a clean floating window.
</details>

<details>
<summary>systemd user service</summary>

```ini
[Unit]
Description=clipit-rs clipboard history daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart=%h/.local/bin/clipit-rs
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
```

Save as `~/.config/systemd/user/clipit-rs.service`, then:

```bash
systemctl --user enable --now clipit-rs
```
</details>

## Notes

- **Auto-paste** uses `xdotool` (X11 only). On strict Wayland, set `auto_paste = false`.
- **X11 event source** — when running on X11, clipboard changes are detected via the XFixes extension (event-driven, no CPU polling). Falls back to timer polling on Wayland.
- **Global hotkeys** depend on desktop environment permissions.
