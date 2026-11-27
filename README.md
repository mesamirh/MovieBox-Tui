<div align="center">

# MovieBox-TUI

**Terminal interface to find, download, and stream movies, TV shows, and live TV using local media players.**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![CI](https://img.shields.io/github/actions/workflow/status/mesamirh/MovieBox-Tui/ci.yml?branch=main&label=CI&logo=github&style=flat)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust&style=flat)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat)](#license)
[![Telegram](https://img.shields.io/badge/Telegram-Channel-2CA5E0?style=flat&logo=telegram&logoColor=white)](https://t.me/getfromme)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## Sponsors

Want to feature your brand or product here? Contact [et6253013@gmail.com](mailto:et6253013@gmail.com).

## Features

- **Streaming:** Movies, TV series, anime, Asian dramas, and community Stremio addons across multiple native providers.
- **Live TV:** M3U playlist import with channel categories, EPG support, and search.
- **Resolution Picker:** Direct stream quality selection (`4K`, `1080p`, `720p`, `480p`, `Auto`) before playback.
- **Hardware Players:** Direct launch in `mpv`, `VLC`, or `IINA` with custom auth header and cookie forwarding.
- **Batch Downloader:** Multi-segment concurrent downloader with HTTP range pause and resume for episodes and full seasons.
- **Subtitle Picker:** Multi-language subtitle tracks extracted and selectable via an interactive picker before playback or download.
- **Terminal UI:** Vim navigation, mouse interaction, slash command palette (`/help`, `/settings`), and 9 built-in themes.
- **Cover Art:** Native Kitty, Sixel, and iTerm2 poster rendering with automatic text fallback.
- **Resume & Library:** Home deck with continue-watching timestamps, watch history, and favorites stored strictly on local disk. Zero telemetry.

## Prerequisites

- **Media Player:** `mpv`, `VLC`, or `IINA` (macOS) / any external video player (Android).
- **Posters:** Terminal with Sixel, Kitty, or iTerm2 support (Ghostty, Kitty, WezTerm, iTerm2, foot, Windows Terminal v1.22+).
- **DASH Downloads:** `yt-dlp` and `ffmpeg` (required only for MovieBox DASH downloads).

## Installation

### macOS & Linux

If you have [Homebrew](https://brew.sh/) on macOS:
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```

> **Note:** If Homebrew prompts for tap verification on initial install, run `brew trust mesamirh/moviebox-tui`.

Direct install via Terminal (macOS & Linux, no package manager needed):
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

### Windows

If you have [Scoop](https://scoop.sh/) (recommended):
```powershell
scoop bucket add moviebox https://github.com/mesamirh/MovieBox-Tui
scoop install moviebox-tui
```

Direct install via PowerShell (no package manager needed):
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

> **SmartScreen prompt:** If Windows displays *"Windows protected your PC"*, click **More info** → **Run anyway**.

### Android (Termux)
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
termux-setup-storage
```

<details>
<summary><b>Cargo & Source Build</b></summary>

```bash
cargo install moviebox-tui --locked
```

From source:
```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build --release --locked
```

</details>

<details>
<summary><b>Verify Release Integrity</b></summary>

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```

</details>

<details>
<summary><b>Uninstall</b></summary>

Re-run install command (`curl ... | bash` or `irm ... | iex`) and select `2) Uninstall`.

Or via package manager:
```bash
brew uninstall moviebox-tui     # Homebrew
scoop uninstall moviebox-tui    # Scoop
cargo uninstall moviebox-tui    # Cargo
```

</details>

## Quick Start

```bash
moviebox-tui
```

- Type to search, press `Enter` to play.
- Press `?` for shortcuts, or type `/settings` for preferences.

## Documentation

Full documentation at [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) or [`docs/`](docs/):

| Guide | Description |
| :--- | :--- |
| [Keyboard & Controls](docs/controls.md) | Keybindings, vim navigation, and shortcuts |
| [Configuration](docs/config.md) | Settings, themes, and environment variables |
| [Content Providers](docs/providers.md) | Native scrapers (MovieBox, 4KHDHub, Dramachi, BDIX) |
| [Stremio Addons](docs/addons-mode.md) | Community addon configuration and streaming |
| [Hardware Players](docs/players.md) | Player detection, launch options, and flags |
| [Live TV & IPTV](docs/tv-mode.md) | M3U playlist import and channel streaming |
| [Batch Downloads](docs/downloads.md) | HTTP range engine, pause, and resume |

## Contributing

Review [CONTRIBUTING.md](CONTRIBUTING.md) before submitting pull requests. Report bugs via [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues).

## Privacy

Zero telemetry, analytics, or user tracking. History, favorites, and config remain local.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).

## Disclaimer

MovieBox-TUI does not host or store media. It plays publicly available streams. Users must comply with local laws.
