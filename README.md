<div align="center">

# MovieBox-TUI

**Terminal interface to find, download, and stream movies, TV shows, and live TV using local media players.**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![Telegram](https://telegram-badge.vercel.app/api/telegram-badge?channelId=@getfromme&style=flat&logo=true)](https://t.me/getfromme)
[![Donate](https://img.shields.io/badge/Donate-Crypto-F7931A?style=flat&logo=bitcoin&logoColor=white)](#optional-support)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## Features

- **On Demand Streaming**: Stream movies, series, and anime across multiple providers and community Stremio addons.
- **Live TV and IPTV**: Import custom M3U playlist URLs to search channels, browse categories, and stream live television.
- **Native Video Playback**: Plays directly in your favorite player (`mpv`, `IINA`, `VLC`, or Android video players) with smooth hardware acceleration.
- **Automatic Subtitles**: Automatically searches and loads subtitles in your preferred language into your player.
- **Fast Downloads**: Save single episodes or entire seasons to your computer with pause and resume support.
- **Quality Control**: Pick a preferred resolution once and every stream and download uses it, so you can cap bandwidth on a slow connection.
- **Visual Posters**: Displays cover art and movie posters directly inside your terminal window.
- **Library and History**: Bookmark your favorite titles and pick up watching right where you left off.
- **Custom Themes**: Built in color themes and settings to match your personal terminal look and feel.
- **Cross Platform**: Works identically on macOS, Linux, Windows, and Android.

## Prerequisites

Requires at least one media player for streaming:

- **mpv** (recommended across Linux, macOS, and Windows)
- **IINA** (macOS)
- **VLC** (cross platform)
- **Any Android Video Player** via Termux (VLC, Just Player, MX Player)

*Poster graphics:* Image rendering requires a graphics capable terminal (Ghostty, Kitty, WezTerm, or iTerm2). Standard terminals display clean text layouts automatically.

*Optional for MovieBox downloads:* `yt-dlp` and `ffmpeg` are required only for downloading DASH streams from the MovieBox provider. All other providers download directly with the built in engine.

## Installation

### macOS and Linux

Open Terminal and run:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

Or via Homebrew (macOS):
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```
*Note:* If Homebrew prompts for tap verification, run `brew trust mesamirh/moviebox-tui`.

### Windows

Open PowerShell and run:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

### Android (Termux)

Open Termux and run:
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
termux-setup-storage
```
*Note:* Requires an external video player installed on Android (e.g. VLC or any supported player).
<details>
<summary><b>Cargo and Source Build</b></summary>

From crates.io:
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
<summary><b>Uninstallation</b></summary>

#### Automated Installer (macOS, Linux, Windows, Android)

Simply re-run your original install command (`curl ... | bash` or `irm ... | iex`). When MovieBox-TUI is already installed, the installer automatically detects it and displays an interactive menu:

```text
MovieBox-TUI is already installed.
What would you like to do?
  1) Reinstall / Update to latest version
  2) Uninstall
  3) Cancel
```

Enter `2` to completely remove MovieBox-TUI.

#### Package Managers

```bash
brew uninstall moviebox-tui     # Homebrew (macOS)
cargo uninstall moviebox-tui    # Cargo
```

</details>

## Quick Start

```bash
moviebox-tui
```

- Type any title to search, press `Enter` to play.
- Press `?` inside the TUI for shortcuts, or type `/settings` for preferences.

## Documentation

Comprehensive guides and architectural references are available at [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) or in the [`docs/`](docs/) directory.

## Contributing

Contributions are welcome. Review [CONTRIBUTING.md](CONTRIBUTING.md) before submitting pull requests.

Report bugs or submit feature requests through [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues).

<details>
<summary><b>Optional Support</b></summary>
<div id="optional-support" tabindex="-1"></div>

If you would like to support ongoing development directly:

| Network / Asset | Address |
| :--- | :--- |
| **USDT (TRC20)** | `TL4yW73qmbKZpBWwbEFgjBpwVkPDFTkJgV` |
| **Bitcoin (BTC)** | `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ` |
| **Ethereum / EVM** | `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c` |
| **Solana (SOL)** | `6ctm5WFv73MNywoCKAz3xK72yizSspHa72rFNygooU6` |

</details>

## Privacy

MovieBox-TUI contains zero telemetry, analytics, or user tracking. All search history, bookmarks, and configuration files remain strictly on your local filesystem.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).

## Disclaimer

This project does not host or store any media. It is an independent client for playing publicly available streams. Users are responsible for complying with the laws of their country.
