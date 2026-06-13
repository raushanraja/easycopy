# Build

Build `easycopy` from source.

## Requirements

- Latest stable Rust
- Linux desktop session
- X11 libraries used by `eframe`
- `xdotool` if `auto_paste = true`

## Build Command

```bash
cargo build --release
```

The binary is written to `./target/release/easycopy`.

To install it into your user PATH:

```bash
cargo install --path .
```

## Dependencies

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
