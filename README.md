<div align="center">

# MovieBox-TUI

Watch and download movies, TV shows, anime, and live TV right in your terminal — without opening a browser.

[![Crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust)](https://crates.io/crates/moviebox-tui)
[![Crates.io Downloads](https://img.shields.io/crates/d/moviebox-tui.svg)](https://crates.io/crates/moviebox-tui)
[![CI](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml/badge.svg)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/moviebox-tui.svg)](#license)

</div>

https://github.com/user-attachments/assets/60b5fab9-cf7a-4a59-9bbf-b2357c345091

## Why MovieBox-TUI?

Searching and watching media through web browsers often means dealing with heavy memory usage from multiple open tabs, intrusive popups, tracking scripts, and clunky web players.

**MovieBox-TUI replaces the web browser workflow with a lightweight terminal interface.** It extracts stream links directly and launches them in your preferred local media player (`mpv`, `VLC`, or `IINA`), with no ads or popups inside the application.

### Quick Comparison

| Feature | Streaming in a Web Browser | MovieBox-TUI in Your Terminal |
| :--- | :--- | :--- |
| **Resource Usage** | Heavy browser process overhead | **~5 MB RAM (no web engine required)** |
| **In-App Experience** | Popups, redirects, and tracking scripts | **No ads or popups inside the application** |
| **Video Player** | Browser web player | **Direct playback in mpv, VLC, or IINA** |
| **Account &amp; Login** | Forced account creation &amp; signups | **No account or login required** |
| **Subtitles** | Manual search and file renaming | **Automatic subtitle downloads in your language** |
| **Downloads** | Manual single-file downloads | **1-click full season batch downloads** |
| **File Organization** | Cluttered downloads folder | **Clean, organized Movies and Series folders** |

## Features at a Glance

- **Search Everything:** Find movies, TV shows, anime, and live TV channels across multiple sources (MovieBox, 4KHDHub, BDIX mirrors, community HTTP addons, and custom IPTV playlists).
- **Addon Mode:** Install community HTTP addon manifests for custom catalog metadata (Cinemeta, Anime Kitsu) and direct stream resolution.
- **Smooth Video Playback:** Plays directly in **mpv**, **VLC**, or **IINA** using your computer's hardware for smooth video.
- **Easy Downloads:** Download single episodes or entire seasons with one keypress. Completed episodes are skipped automatically.
- **Favorites:** Star any movie or series with `*` (or `f` on the details screen) and reach it instantly from a dedicated row on the home screen, or view the full list with `/favorites`.
- **Simple Terminal Interface:** Full keyboard and mouse support, beautiful color themes, and movie poster art.

## Installation

A media player (**[mpv](https://mpv.io)**, **[VLC](https://www.videolan.org/vlc/)**, or **[IINA](https://iina.io)**) is recommended for video playback.

### macOS

Using **Homebrew**:
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install mesamirh/moviebox-tui/moviebox-tui
```

Or using the automated installer:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh -o install.sh && bash install.sh
```

*(Or download the prebuilt `MovieBox_macOS_Universal.tar.gz` from [GitHub Releases](https://github.com/mesamirh/MovieBox-Tui/releases/latest))*

---

### Linux

Using the automated installer (installs prebuilt static binary to `~/.local/bin`):
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh -o install.sh && bash install.sh
```

Or manual installation:
```bash
curl -LO https://github.com/mesamirh/MovieBox-Tui/releases/latest/download/MovieBox_Linux_x64.tar.gz
tar -xzf MovieBox_Linux_x64.tar.gz
mkdir -p ~/.local/bin && mv moviebox-tui ~/.local/bin/
```

---

### Windows

Using **PowerShell**:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

Or manual installation:
1. Download `MovieBox_Windows_x64.zip` (or `MovieBox_Windows_arm64.zip`) from [GitHub Releases](https://github.com/mesamirh/MovieBox-Tui/releases/latest).
2. Extract the archive and place `moviebox-tui.exe` in your PATH (e.g. `%LOCALAPPDATA%\Programs\MovieBox-Tui\bin`).

*(Note: If Windows SmartScreen displays an "Unknown Publisher" prompt on first launch, click **More info** → **Run anyway**)*

---

### Android (Termux)

Using the automated installer (installs prebuilt static ARM64 binary into `$PREFIX/bin`):
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh -o install.sh && bash install.sh
termux-setup-storage
```

*(Or compile natively via Cargo: `pkg install rust && cargo install moviebox-tui --locked`)*

---

<details>
<summary><b>From Source / Cargo (Developers)</b></summary>

```bash
cargo install moviebox-tui --locked
```

```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build --release --locked
```

</details>

<details>
<summary><b>Verify Release Integrity & Provenance</b></summary>

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

```bash
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```

</details>

## Usage

Start the app:

```bash
moviebox-tui
```

- **Interactive Help:** Press `?` inside the app anytime to view all keyboard shortcuts and mouse actions.
- **Mouse & Keyboard:** Full mouse navigation and keyboard controls are supported throughout the application.
- **Full Reference:** See [Controls & Shortcuts](docs/controls.md) for the complete list of keybindings and slash commands.

## Documentation

All in-depth guides and technical details are available in the [`docs/`](docs/) directory:

- [Controls & Shortcuts](docs/controls.md) — Complete keyboard shortcuts, mouse actions, and slash commands
- [Architecture & Data Flow](docs/architecture.md) — How the app works under the hood
- [Media Players](docs/players.md) — Supported players, custom paths, and launch options
- [Downloads & Organization](docs/downloads.md) — Folder layout, batch downloading, and subtitles
- [Live TV & Playlists](docs/tv-mode.md) — How to add and manage custom M3U playlists
- [Addon Mode & HTTP Addons](docs/addons-mode.md) — How to install and manage community HTTP addon manifests
- [Configuration Reference](docs/config.md) — Config files and environment variables
- [Providers](docs/providers.md) — Supported content sources and protocols
- [Troubleshooting & Debugging](docs/debugging.md) — Fixing common issues and reporting bugs
- [Testing & QA Architecture](docs/testing.md) — Testing philosophy, unit/integration suites, and QA guidelines
- [Contributing Guide](CONTRIBUTING.md) — How to contribute code and submit PRs

## Roadmap

- [x] **Terminal UI (TUI):** Interactive terminal application with mouse and keyboard navigation, stream playback, and batch downloading.
- [ ] **Command-Line Interface (CLI):** Direct command-line flags and arguments for headless searching, streaming, and scripted downloading.
- [ ] **Desktop GUI Client:** Dedicated graphical desktop application powered by the same backend stream engine.

## Feedback & Support

If you find the project useful, here are a few simple ways to support it:

- **Star the repository** on GitHub to help others discover it.
- **Report bugs or request features** by opening an [issue](https://github.com/mesamirh/MovieBox-Tui/issues).
- **Share the project** with friends, colleagues, or terminal enthusiasts.
- **Contribute improvements** to scrapers, performance, or docs via [pull requests](CONTRIBUTING.md).

<details>
<summary><b>Optional Support</b></summary>

If you would like to support ongoing development directly:

| Network / Asset | Address |
| :--- | :--- |
| **USDT (TRC20)** | `TL4yW73qmbKZpBWwbEFgjBpwVkPDFTkJgV` |
| **Bitcoin (BTC)** | `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ` |
| **Ethereum / EVM** | `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c` |
| **Solana (SOL)** | `6ctm5WFv73MNywoCKAz3xK72yizSspHa72rFNygooU6` |

</details>

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

## Disclaimer

This project does not host or store any media. It is an independent client for playing publicly available streams. Users are responsible for complying with the laws of their country.
