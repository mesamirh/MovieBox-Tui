# MovieBox-TUI

A fast, lightweight terminal client for streaming and downloading movies, TV shows, anime, and live TV — powered by your local media player.

[moviebox-tui walkthrough.webm](https://github.com/user-attachments/assets/51802c09-abb1-46dd-bd04-d96a9cf836bb)

MovieBox-TUI replaces ad-heavy streaming websites and clunky browser players with a clean, keyboard-driven terminal interface. It scrapes stream links directly from multiple sources and launches playback in your native media player with hardware acceleration, audio track switching, and automatic subtitle synchronization.

## Features

- **Multi-Source Streaming**: Search and stream titles across MovieBox, 4KHDHub, BDIX mirrors, custom IPTV playlists, and community Stremio HTTP addons. Press `Ctrl+P` on the Details screen to switch providers in-place.
- **Hardware-Accelerated Playback**: Direct playback in `mpv`, `IINA` (macOS), or `VLC` with stream authentication headers forwarded automatically.
- **Automatic Subtitles**: Automatically searches, downloads, and syncs subtitles in your preferred language directly into your player.
- **Season Batch Downloads**: Download individual episodes or entire seasons with one keypress (`d`), with HTTP range resume support and clean folder structure (`Movies/` and `Series/`).
- **Interactive Settings Hub**: Configure your default media player, download folder, content modes, and themes inside an in-app visual modal via `/settings` (`Ctrl+S`).
- **Modes**: Switch between Streaming and Live TV (`Ctrl+T`), with community Stremio addons available directly as a streaming provider.
- **Ergonomics & Themes**: Full keyboard navigation (vim-style `j`/`k`, `/`, `Tab`) and mouse support (click, scroll, drag) with 6 built-in themes (Catppuccin, TokyoNight, Nord, Dracula, Gruvbox, Rosé Pine) and terminal theme autodetection.

## Prerequisites

MovieBox-TUI delegates video decoding to an external media player. Install at least one of the following:

| Player | Platform | Quick Install |
| :--- | :--- | :--- |
| **mpv** *(Recommended)* | Linux, macOS, Windows | `brew install mpv` / `sudo apt install mpv` / `winget install mpv` |
| **IINA** | macOS (Native GUI) | `brew install --cask iina` |
| **VLC** | Cross-platform | `brew install --cask vlc` / `sudo apt install vlc` / `winget install VideoLAN.VLC` |
| **Android Player** | Android (Termux) | `pkg install -y termux-tools termux-am` *(launches external player)* |

## Installation

### macOS & Linux

Install via the automated script:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

*macOS users can also install via Homebrew:*
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew trust mesamirh/moviebox-tui
brew install moviebox-tui
```

### Windows

Install via PowerShell:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

### Android (Termux)

Install Termux tools and the intent bridge, download the native precompiled Android ARM64 binary via the installer, and grant storage permissions:
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh -o install.sh && bash install.sh
termux-setup-storage
```

> **Video Playback on Android**: Playback on Android is handled by external video player apps (**VLC**, **MX Player**, **Just Player**, or **MPV Android APK**) launched via Android Intent. Installing CLI `mpv` inside Termux (`pkg install mpv`) provides terminal/audio-only output because Android does not expose a desktop video surface to terminal sessions.

<details>
<summary><b>Manual & Developer Builds (Cargo)</b></summary>

Install directly from crates.io:
```bash
cargo install moviebox-tui --locked
```

Or compile from source:
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

## Quick Start

Launch the application:
```bash
moviebox-tui
```

- **Interactive Help**: Press `?` inside the app anytime to view the mode-aware keyboard shortcuts and mouse guide.
- **Settings**: Type `/settings` or press `Ctrl+S` to configure your default player, download directory, and content modes.
- **Controls Reference**: See [`docs/controls.md`](docs/controls.md) for the complete list of keybindings, mouse actions, and slash commands.

## Documentation

Comprehensive guides are available online at [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) or locally in the [`docs/`](docs/) directory:

- [Controls & Shortcuts](docs/controls.md) — Complete keybindings, navigation, and slash commands
- [Media Players](docs/players.md) — Player options, custom paths, and launch flags
- [Downloads & Organization](docs/downloads.md) — Multi-segment download engine and folder layout
- [Live TV](docs/tv-mode.md) — Adding and managing custom M3U playlists
- [Addon Mode](docs/addons-mode.md) — Installing and managing Stremio HTTP addons
- [Configuration](docs/config.md) — `config.json` reference and `MOVIEBOX_*` environment variables
- [Providers](docs/providers.md) — Supported content sources and resolver protocols
- [Architecture](docs/architecture.md) — Subsystem map, event loop, and caching model
- [Documentation Index](docs/README.md) — Full documentation directory

## Contributing

Contributions are welcome! Please review [CONTRIBUTING.md](CONTRIBUTING.md) before submitting pull requests.

If you encounter a bug or have a feature request, feel free to open an [issue](https://github.com/mesamirh/MovieBox-Tui/issues).

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

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).

## Disclaimer

This project does not host or store any media. It is an independent client for playing publicly available streams. Users are responsible for complying with the laws of their country.
