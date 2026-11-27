<div align="center">

# Introduction

**Terminal interface to find, download, and stream movies, TV shows, and live TV using local media players.**

[ English ](../README.md) • [ বাংলা ](../README_BN.md) • [ हिन्दी ](../README_HI.md) • [ Español ](../README_ES.md)

[![CI](https://img.shields.io/github/actions/workflow/status/mesamirh/MovieBox-Tui/ci.yml?branch=main&label=CI&logo=github&style=flat)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust&style=flat)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat)](../README.md#license)
[![Telegram](https://img.shields.io/badge/Telegram-Channel-2CA5E0?style=flat&logo=telegram&logoColor=white)](https://t.me/getfromme)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## Features

- **Streaming:** Movies, TV series, anime, Asian dramas, and community Stremio addons across multiple native providers.
- **Live TV:** M3U playlist import with channel categories, EPG support, and search.
- **Resolution Picker:** Direct stream quality selection (`4K`, `1080p`, `720p`, `480p`, `Auto`) before playback.
- **Hardware Players:** Seamless launch in `mpv`, `VLC`, or `IINA` with custom auth header and cookie forwarding.
- **Batch Downloader:** Multi-segment concurrent downloader with HTTP range pause and resume for episodes and full seasons.
- **Subtitle Picker:** Multi-language subtitle tracks extracted and selectable via an interactive picker before playback or download.
- **Terminal UI:** Vim navigation, mouse interaction, slash command palette (`/help`, `/settings`), and 9 built-in themes.
- **Cover Art:** Native Kitty, Sixel, and iTerm2 poster rendering with automatic text fallback.
- **Resume & Library:** Home deck with continue-watching timestamps, watch history, and favorites stored strictly on local disk. Zero telemetry.

## Prerequisites

- **Media Player:** `mpv`, `VLC`, or `IINA` (macOS) / any external video player (Android).
- **Posters:** Terminal with Sixel, Kitty, or iTerm2 support (Ghostty, Kitty, WezTerm, iTerm2, foot, Windows Terminal v1.22+).
- **DASH Downloads:** `yt-dlp` and `ffmpeg` (required only for MovieBox DASH downloads).
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
