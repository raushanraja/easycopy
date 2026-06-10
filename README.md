# clipit-rs

A small Linux clipboard history manager written in Rust.

## What it does

- Runs as a daemon and polls the clipboard.
- Stores text history and PNG snapshots of copied images.
- Opens a keyboard-driven popup with `Ctrl+Alt+V` by default.
- Supports search, arrow navigation, Enter-to-paste, Delete-to-remove, and full history clear.
- Uses a larger, clearer popup UI with bigger rows, bigger search box, stable keyboard navigation, and no constant search-focus reset during scrolling.
- Keeps history on disk under `~/.local/share/clipit/`.
- Keeps config under `~/.config/clipit/config.toml`.

## Install build dependencies

Debian/Ubuntu:

```bash
sudo apt install -y \
  build-essential pkg-config \
  libx11-dev libxcb1-dev libxkbcommon-dev \
  libxrandr-dev libxi-dev libxcursor-dev libxext-dev \
  libxss-dev libxtst-dev libxft-dev libxinerama-dev \
  libwayland-dev libxkbcommon-x11-dev xdotool
```

Arch Linux:

```bash
sudo pacman -S --needed rust base-devel pkgconf \
  libx11 libxcb libxkbcommon libxrandr libxi libxcursor \
  libxext libxss libxtst libxft libxinerama wayland libxkbcommon-x11 xdotool
```

## Build and test

```bash
cargo fmt --check
cargo test --all
cargo build --release
```

Binary:

```bash
./target/release/clipit-rs --help
```

## Usage

```bash
clipit-rs              # start daemon
clipit-rs --popup      # open popup manually
clipit-rs --clear      # delete history and saved images
clipit-rs --version
```

## Config

The first run creates `~/.config/clipit/config.toml`:

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
theme = "dark" # dark, light, or system
```

## Notes

- Auto-paste currently uses `xdotool`, which works best on X11/XWayland.
- On strict Wayland sessions, set `auto_paste = false` or use a compositor-specific paste helper.
- Global hotkeys depend on desktop environment permissions.
