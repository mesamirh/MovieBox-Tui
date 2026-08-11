<div align="center">

# MovieBox-TUI

Search, browse, play, and download movies, series, anime, and IPTV streams from a keyboard-first terminal interface using external media players.

[![Crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust)](https://crates.io/crates/moviebox-tui)
[![CI](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml/badge.svg)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![Platforms](https://img.shields.io/badge/Platforms-macOS%20%7C%20Linux%20%7C%20Windows%20%7C%20Android-brightgreen)](#requirements)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

<video src="https://github.com/user-attachments/assets/e3dc0c11-524f-4b0e-8902-e0c66d6ca88d" alt="MovieBox-TUI demo" width="85%" autoplay loop muted></video>

</div>

## Documentation

This README is the project landing page (features, install, usage). The full
documentation set — architecture, providers, players, cache, logging, TV mode,
configuration, and debugging — lives in [`docs/`](docs/README.md). Contribution
guidance is in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Features

### Catalogs & Browsing

- Search and browse movies, TV series, and anime from multiple content catalogs
- Filter search results by resolution and subtitle availability

> **Note on BDIX:** BDIX sources are only accessible from supported Bangladeshi ISP networks. Because of this, they are hidden by default. You can enable them manually if your network supports it.

### Playback

- Play streams directly in your local media player (mpv, VLC, or IINA)
- Play protected streams seamlessly without manual configuration

### Downloads

- Download full seasons or episodes with automatic subtitle language selection
- Resume interrupted downloads without losing progress

### IPTV

- Watch live TV by loading local `.m3u` playlists organized by category

### User Interface & App

- View rich graphical posters in supported terminals (Kitty, iTerm2, Sixel) or fallback to text art
- Let the app automatically manage configuration and clean up expired caches

MovieBox-TUI resolves links from upstream services. Availability can change when those services change.

## Requirements

- 64-bit Windows, macOS, Linux, or Android (Termux)
- Terminal size of at least 85×24
- One supported player: mpv, VLC, IINA, or any native Android video player
- Internet connection

## Installation

Prebuilt binaries are available for all supported platforms. All official installers verify the release SHA-256 checksum automatically.

### macOS or Linux

#### Homebrew

```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew trust mesamirh/moviebox-tui
brew install moviebox-tui
```

The formula selects the correct macOS, Linux x86_64, or Linux ARM64 release.

#### Install script

```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

The script detects OS and CPU architecture, then installs to `/usr/local/bin`. Without write access or `sudo`, it uses `~/.local/bin`.

### Windows

Works in PowerShell or Command Prompt (cmd):

```cmd
powershell -Command "irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex"
```

The installer selects x86_64 or ARM64, installs under `%LOCALAPPDATA%\MovieBox-Tui`, and adds that directory to the user PATH. Open a new terminal after first installation.

### Android (Termux)

MovieBox-Tui runs natively in Termux and opens videos directly in your installed Android video apps (VLC, MX Player, etc).

```bash
pkg install rust openssl pkg-config
cargo install moviebox-tui --locked
termux-setup-storage
```
*(Running `termux-setup-storage` ensures downloads are saved to your real Android `Download` folder)*

<details>
<summary><b>Cargo</b></summary>

Requires Rust 1.90 or newer (this only applies to Cargo and source builds; binary users do not need Rust installed):

```bash
cargo install moviebox-tui --locked
```

</details>

<details>
<summary><b>Build from source</b></summary>

```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build --release --locked
```

Binary location: `target/release/moviebox-tui` (`moviebox-tui.exe` on Windows).

</details>

## Supported Players

MovieBox-TUI checks standard application locations, PATH executables, and Linux Flatpak installations.

Detected automatically:

- **macOS:** `/Applications`, `~/Applications`, Homebrew/PATH
- **Linux:** PATH, Flatpak mpv, Flatpak VLC
- **Windows:** PATH, common Program Files locations, Microsoft Store aliases

Portable or custom installations can be selected with environment variables:

| Player | Variable             |
| ------ | -------------------- |
| mpv    | `MOVIEBOX_MPV_PATH`  |
| VLC    | `MOVIEBOX_VLC_PATH`  |
| IINA   | `MOVIEBOX_IINA_PATH` |

macOS/Linux example:

```bash
export MOVIEBOX_MPV_PATH="$HOME/Apps/mpv"
moviebox-tui
```

Windows PowerShell example:

```powershell
$env:MOVIEBOX_VLC_PATH = "D:\Apps\VLC\vlc.exe"
moviebox-tui
```

IINA is macOS-only. mpv provides the broadest source-header compatibility.

## Usage

Run:

```bash
moviebox-tui
```

### Keyboard shortcuts

| Key        | Action                                                |
| ---------- | ----------------------------------------------------- |
| Arrow keys | Navigate lists, grids, seasons, episodes, and dialogs |
| Enter      | Open or confirm selection                             |
| Esc        | Close dialog or go back                               |
| `o`        | Choose another player                                 |
| `d`        | Download selected episode or season                   |
| `r`        | Refresh current content                               |
| `Alt+b`    | Open the Browse menu (Trending / Top Rated / Popular) |
| `s`        | Toggle ascending / descending sort in Browse          |
| `Ctrl+P`   | Switch content provider                               |
| `Ctrl+T`   | Toggle IPTV mode                                      |
| `?`        | Show help                                             |
| `q`        | Quit                                                  |

### Slash commands

| Command              | Action                                    |
| -------------------- | ----------------------------------------- |
| `/browse`            | Open Browse: Trending / Top Rated / Popular |
| `/discover`, `/home` | Open discovery view                       |
| `/movies`            | Browse movies                             |
| `/shows`             | Browse series                             |
| `/anime`             | Browse anime                              |
| `/list`              | Show IPTV channels                        |
| `/config`            | Configure IPTV playlists                  |
| `/update`            | Check for a newer release                 |
| `/toggle-update`     | Enable or disable automatic update checks |
| `/clear-cache`       | Remove cached application data            |
| `/enable-bdix`       | Enable BDIX FTP sources                   |
| `/disable-bdix`      | Disable BDIX FTP sources                  |

`/update` checks availability and shows the release location; it does not replace the running binary. Re-run the installer or Homebrew upgrade command to update.

## Downloads

Downloads are stored under the operating system Downloads directory:

```text
MovieBox-TUI/
├── Movies/
└── Series/<title>/Season <number>/
```

- **Sequential downloads:** Entire seasons are downloaded one by one to limit disk and network pressure.
- **Smart subtitles:** Your subtitle language choice for the first episode is applied to the remaining episodes. Missing subtitles do not discard completed video files.
- **Robust resuming:** Interrupted downloads preserve `.part` and metadata files, and can be resumed without losing progress.

## Configuration & Cache

MovieBox-TUI uses standard OS directories:

| Platform | Configuration                                | Cache                                      |
| -------- | -------------------------------------------- | ------------------------------------------ |
| Linux    | `${XDG_CONFIG_HOME:-~/.config}/moviebox-tui` | `${XDG_CACHE_HOME:-~/.cache}/moviebox-tui` |
| macOS    | `~/Library/Application Support/moviebox-tui` | `~/Library/Caches/moviebox-tui`            |
| Windows  | `%APPDATA%\moviebox-tui`                     | `%LOCALAPPDATA%\moviebox-tui`              |

Catalog providers use separate cache namespaces. Expired or invalid cache entries are discarded automatically; files older than seven days are cleaned at startup.

## Updates

Automatic update checks only notify you about new releases. They do not update the application automatically.

Homebrew:

```bash
brew update
brew upgrade moviebox-tui
```

Script installation: run the install command again.

Windows PowerShell: run the install command again.

Cargo:

```bash
cargo install moviebox-tui --locked --force
```

## Uninstallation

Homebrew:

```bash
brew uninstall moviebox-tui
brew untap mesamirh/moviebox-tui
```

Script installation:

```bash
sudo rm -f /usr/local/bin/moviebox-tui
rm -f "$HOME/.local/bin/moviebox-tui"
```

Windows PowerShell:

```powershell
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\MovieBox-Tui"
```

Cargo:

```bash
cargo uninstall moviebox-tui
```

Configuration and cache directories remain until removed manually.

## Troubleshooting

<details>
<summary><b>No media player found</b></summary>

MovieBox-TUI relies on external players. If it says none are found:

1. Ensure you have installed **mpv**, **VLC**, or **IINA**.
2. Verify it is in your system PATH by running `mpv --version` or `vlc --version` in your terminal.
3. If using a portable or non-standard installation, set the corresponding environment variable before running (e.g., `MOVIEBOX_MPV_PATH=/path/to/mpv`).

</details>

<details>
<summary><b>Images do not render / Only text is shown</b></summary>

MovieBox-TUI supports inline images via Kitty, Sixel, and iTerm2 protocols.

- If images don't show, ensure you are using a compatible terminal emulator (like Kitty, WezTerm, iTerm2, or Windows Terminal Preview).
- If your terminal does not support graphics, the UI gracefully falls back to text-based posters and remains fully usable.
- If you experience crashes when resizing the window (specifically with Sixel), please report it with your OS, terminal name, and version.

</details>

<details>
<summary><b>"moviebox-tui: command not found" (Linux / macOS)</b></summary>

If you installed via the script without `sudo`, the binary was placed in `~/.local/bin`. You need to add this to your PATH. Add this line to your `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Then restart your terminal.

</details>

<details>
<summary><b>Windows PowerShell script fails (Execution Policy)</b></summary>

If you receive an error about running scripts being disabled on your system when installing via PowerShell, run this command as Administrator first:

```powershell
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
```

Then try the installation command again.

</details>

## Development

Formatting and linting are enforced automatically by the pre-commit hook on every
commit (see [CONTRIBUTING.md](CONTRIBUTING.md)); there is no need to run them manually.
Before opening a PR, also run the CI-only checks:

```bash
cargo audit
cargo package --locked
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidance.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

## Legal

MovieBox-TUI does not host media. It is not affiliated with any specific content sources, IPTV providers, player projects, or terminal vendors. Users are responsible for complying with laws and service terms applicable to them.
