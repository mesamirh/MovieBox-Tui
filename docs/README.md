# Introduction

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

---

## Documentation Directory Map

### Getting Started

| Guide | Description |
| :--- | :--- |
| [Installation](installation.md) | Platform installation instructions, package managers, and binary verification |
| [Keyboard & Controls](controls.md) | Complete keybindings, vim navigation, text editing, and slash commands |
| [Configuration Guide](config.md) | `config.json` schema, settings hub options, and environment variables |

### Features & Modes

| Guide | Description |
| :--- | :--- |
| [Content Providers](providers.md) | Built-in providers, scrapers, stream extractors, and authentication headers |
| [Hardware Players](players.md) | Media player detection, launch flags, stream headers, and watch tracking |
| [Batch Downloads](downloads.md) | Multi-segment download engine, range resume, and folder layout |
| [Stremio Addons](addons-mode.md) | Addon manifest installation, catalog browsing, and stream resolution |
| [Live TV & IPTV](tv-mode.md) | M3U playlist manager, channel parsing, and live stream playback |

### Architecture & Internals

| Guide | Description |
| :--- | :--- |
| [System Architecture](architecture.md) | Subsystem diagrams, async event loop, and task cancellation |
| [Module Breakdown](modules.md) | Crate structure, module responsibilities, and call boundaries |
| [Caching Strategy](cache.md) | Binary disk caching, TTL policies, and LRU memory management |
| [Logging System](logging.md) | File logging, log rotation, and tracing diagnostics |
| [Cross-Platform Operations](cross-platform.md) | Platform compatibility matrix across macOS, Linux, Windows, and Termux |

### Reference & Maintenance

| Guide | Description |
| :--- | :--- |
| [Testing Suite](testing.md) | Unit tests, integration tests, and verification gates |
| [Debugging Guide](debugging.md) | Troubleshooting common issues, terminal rendering, and player errors |
| [Release Checklist](release-checklist.md) | Pre-release validation, binary packaging, and deployment workflow |
| [Known Issues](known-issues.md) | Tracked limitations, terminal quirks, and workarounds |
| [Contributing Guide](contributing.md) | Contribution guidelines, code standards, and PR process |
| [Changelog](changelog.md) | Complete release history and unreleased changes |
