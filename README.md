<div align="center">

# MovieBox-Tui

A terminal client for finding and streaming movies, TV shows, and anime from your keyboard.

[![Crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust)](https://crates.io/crates/moviebox-tui)
[![Downloads](https://img.shields.io/crates/d/moviebox-tui.svg)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg?logo=rust)](#requirements)

<br>

<p align="center">
  <img src="assets/screenshots/01-home.png" alt="MovieBox-Tui home screen" width="88%"/>
</p>

</div>

---

## What is this?

**MovieBox-Tui** is a terminal UI I built to search MovieBox's public catalog and stream the results in my favorite video player, without leaving the terminal. It talks to the MovieBox API directly, resolves the video URLs it returns, and hands them off to `mpv`, `IINA`, or `VLC`.

No browsers, no ads, no login walls, no configuration. Type a title, pick a quality, watch.

> **Note:** This project is a client for a third-party service. It does not host, store, or redistribute any media. It only resolves the links the upstream API returns. It is intended strictly for educational and personal use. You are responsible for complying with copyright law in your jurisdiction.

---

## Demo

A short walkthrough of the app in action:

<div align="center">

**[Watch the demo on YouTube](https://youtu.be/0L1Wc3cwMCc)**

</div>

---

## Contents

- [What it can do](#what-it-can-do)
- [Screenshots](#screenshots)
- [Platform support](#platform-support)
- [Requirements](#requirements)
- [Install](#install)
- [Getting started](#getting-started)
- [Keybindings](#keybindings)
- [How it works](#how-it-works)
- [Project layout](#project-layout)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)
- [License](#license)

---

## What it can do

**Search and discovery**
- Type-to-search with live, debounced suggestions.
- Slash commands to browse curated feeds: `/movies`, `/shows`, `/anime`, `/discover`.
- Each result shows a poster, release year, and (when highlighted) IMDb rating and genres.

**Playback**
- Detects `mpv`, `IINA`, and `VLC` at startup. You get a picker for whatever you have installed.
- Full season and episode browsing for TV series and anime, with per-episode stream resolution.
- Multiple resolutions (1080p, 720p, 480p, 360p) are fetched in parallel and listed by quality.
- Attach external subtitle tracks, and switch between available audio dubs before playing.
- If a stream fails or expires, hit <kbd>R</kbd> to re-resolve it.

**Downloading and sharing**
- Built-in multi-connection downloader. Uses up to 16 parallel connections when the source allows range requests, with live speed, ETA, progress bar, and cancel.
- Copy any direct stream URL to your clipboard with a single keystroke.
- Downloads go to your system Downloads folder.

**The interface**
- Real poster art rendered inline in terminals that support graphics (Kitty, WezTerm, iTerm2, Ghostty, foot, etc.).
- In-app notification when a newer version is published to crates.io.
- The details view shows the poster, IMDb rating, year, genres, duration, country, and full description alongside the season, episode, and stream panels.

---

## Screenshots

<details open>
<summary><b>Home and search</b></summary>
<br>

| Home | Search results |
| :---: | :---: |
| <img src="assets/screenshots/01-home.png" alt="Home screen" width="480"> | <img src="assets/screenshots/02-search-results.png" alt="Search results" width="480"> |

</details>

<details>
<summary><b>Details view</b></summary>
<br>

| Movie | TV series |
| :---: | :---: |
| <img src="assets/screenshots/03-movie-details.png" alt="Movie details" width="480"> | <img src="assets/screenshots/04-series-details.png" alt="Series details" width="480"> |

</details>

<details>
<summary><b>Discover feeds</b></summary>
<br>

| Movies | Series | Anime |
| :---: | :---: | :---: |
| <img src="assets/screenshots/05-discover-movies.png" alt="Discover movies" width="320"> | <img src="assets/screenshots/06-discover-series.png" alt="Discover series" width="320"> | <img src="assets/screenshots/07-discover-anime.png" alt="Discover anime" width="320"> |

</details>

<details>
<summary><b>Help overlay</b></summary>
<br>

<p align="center">
  <img src="assets/screenshots/08-help.png" alt="Keybindings help overlay" width="70%">
</p>

</details>

---

## Platform support

| Platform | Status |
| :--- | :--- |
| **macOS** | Fully supported. Developed and tested here. Ghostty, iTerm2, and Kitty all render posters correctly. |
| **Linux** | Experimental. Should work on any terminal with graphics-protocol support. Not extensively tested. |
| **Windows** | Experimental. Expect bugs, especially around inline image rendering. Contributions welcome. |

---

## Requirements

You need three things:

1. **A video player.** `mpv` is the default and the most reliable. `IINA` (macOS) and `VLC` also work. The app detects whichever you have installed.
2. **A terminal at least 85×24.** Anything smaller and the app will show a message asking you to enlarge it.
3. **Rust 1.85 or newer** (edition 2024), only if you want to build from source. See [rustup.rs](https://rustup.rs/). If you install from crates.io, you already have it.

For inline poster art, you'll want a terminal that speaks a graphics protocol: **Kitty, WezTerm, iTerm2, Ghostty**, or **foot**. Other terminals still work fine, you just get placeholder blocks instead of images.

<details>
<summary><b>How to install a video player</b></summary>
<br>

```bash
# macOS
brew install mpv
# or, for a native-feel alternative
brew install --cask iina

# Debian / Ubuntu
sudo apt install mpv

# Arch Linux
sudo pacman -S mpv

# Fedora
sudo dnf install mpv

# Windows (Chocolatey)
choco install mpv
```

MovieBox-Tui auto-detects whichever players you have installed on the first run.

</details>

---

## Install

The recommended way is to install from crates.io:

```bash
cargo install moviebox-tui
```

Make sure `~/.cargo/bin` is on your `PATH`, then run:

```bash
moviebox-tui
```

<details>
<summary><b>Building from source</b></summary>
<br>

If you want a local checkout or a development version:

```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo install --path .
```

Or run it directly without installing:

```bash
cargo run --release
```

</details>

---

## Getting started

Launch the app:

```bash
moviebox-tui
```

You'll land on the home screen. From there:

1. Just start typing. The search bar activates automatically and live suggestions appear as you type.
2. Or use a discover command: type `/movies`, `/shows`, `/anime`, or `/discover` and press <kbd>Enter</kbd> to browse curated feeds.
3. Move through results with <kbd>Up</kbd>/<kbd>Down</kbd>. The selected result loads a poster preview with IMDb rating and genres.
4. Press <kbd>Enter</kbd> to open the details view.
5. For a TV series, pick a season and episode. If multiple language dubs are available, you'll be asked to pick one.
6. Choose a stream quality and hit <kbd>Enter</kbd> to play in `mpv`, or <kbd>o</kbd> to pick a different player.

Press <kbd>?</kbd> at any time to see all keybindings.

---

## Keybindings

### Global

| Key | Action |
| :--- | :--- |
| any letter | Focus the search input and start typing |
| <kbd>?</kbd> | Toggle the help overlay |
| <kbd>Esc</kbd> | Go back, clear search, or close popup |
| <kbd>q</kbd> | Quit |
| <kbd>Ctrl</kbd>+<kbd>C</kbd> | Force quit |

### Navigation

| Key | Action |
| :--- | :--- |
| <kbd>Up</kbd>/<kbd>Down</kbd> | Move selection |
| <kbd>Left</kbd>/<kbd>Right</kbd> | Switch panels or page through results |
| <kbd>Enter</kbd> | Select or confirm |
| <kbd>Esc</kbd> | Go back (details screen) |

### On the details screen

| Key | Action |
| :--- | :--- |
| <kbd>Enter</kbd> | Play the selected stream in `mpv` |
| <kbd>o</kbd> | Choose a different player (`mpv`, `IINA`, or `VLC`) |
| <kbd>R</kbd> | Refresh streams. Useful when a link expires or fails. |
| <kbd>d</kbd> | Download the selected stream |
| <kbd>c</kbd> | Copy the direct stream URL to the clipboard |
| <kbd>x</kbd> | Cancel an in-progress download |

---

## How it works

The app has two layers.

The **provider layer** (`src/providers/moviebox/`) is a small HTTP client for the MovieBox API. It rotates through a pool of API hosts, signs each request with an HMAC-MD5 signature, and retries automatically on transient failures. When you open a title, it fires off requests for every resolution in parallel and deduplicates the results. That is why picking a quality feels instant.

The **TUI layer** (`src/tui/`) is where everything you see happens. It is built on [Ratatui](https://ratatui.rs) and driven by an async event loop. Every keystroke, network response, and background task becomes an `Action` on a single channel, so the interface never blocks. Poster images decode on background tasks and get cached in an LRU so scrolling through search results stays smooth.

Playback is not reinvented. The app just spawns your video player with the resolved URL and any subtitle track you picked. That means all the polish (hotkeys, subtitles, seeking) comes from mpv, IINA, or VLC, exactly as you would expect them to behave.

---

## Project layout

```
src/
├── main.rs                     Binary entry: terminal + tokio setup
├── lib.rs                      Library root
├── providers/
│   └── moviebox/
│       ├── client.rs           HTTP client, host-pool failover, retries
│       ├── crypto.rs           HMAC-MD5 request signing + device spoofing
│       └── mod.rs              High-level API calls
└── tui/
    ├── app.rs                  Event loop and Action handlers
    ├── action.rs               The Action enum (every event in one place)
    ├── event.rs                Crossterm to Action bridge
    ├── state.rs                Application state
    ├── theme.rs                Colors
    └── screens/
        ├── home.rs             Home, search, and result list
        ├── details.rs          Movie / series detail view
        └── help.rs             Keybindings overlay
```

If you're poking around the code, `src/tui/app.rs` is the map. Every user action, network response, and background task funnels through its match statement.

---

## Troubleshooting

<details>
<summary><b>Posters show up as colored blocks instead of images</b></summary>
<br>

Your terminal doesn't support inline graphics. The app falls back to text placeholders. Try Kitty, WezTerm, iTerm2, Ghostty, or foot if you want the images.

</details>

<details>
<summary><b>"mpv player not found in PATH"</b></summary>
<br>

Install `mpv` (see [Requirements](#requirements)), or press <kbd>o</kbd> to pick `IINA` or `VLC` if you already have those. Player detection happens once at startup, so install first, then launch the app.

</details>

<details>
<summary><b>"Terminal too small"</b></summary>
<br>

The app needs at least **85×24** characters. Enlarge the window or shrink the font.

</details>

<details>
<summary><b>Nothing found, stream won't play, or link expired</b></summary>
<br>

Direct stream URLs are short-lived and expire after some time. On the details screen, press <kbd>R</kbd> to re-resolve. If a title has no streams at all, it may be unreleased on MovieBox or temporarily gone from their catalog.

</details>

<details>
<summary><b>Downloads are slow or crawl</b></summary>
<br>

If the source doesn't support HTTP range requests, the downloader falls back to a single connection. Nothing you can do about that except pick a different quality or source. If it does support ranges, you should see `[16x]` in the status line.

</details>

---

## Contributing

Contributions are welcome. If you're planning something bigger than a small fix, please open an issue first so we can talk it through.

```bash
git clone https://github.com/<your-username>/MovieBox-Tui.git
cd MovieBox-Tui
cargo build

# Before opening a PR:
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

Commits follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, etc.). See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide.

---

## Acknowledgements

Built with [Ratatui](https://ratatui.rs), [crossterm](https://github.com/crossterm-rs/crossterm), [ratatui-image](https://github.com/benjajaja/ratatui-image), [tokio](https://tokio.rs), and [reqwest](https://github.com/seanmonstar/reqwest). Playback is powered by [mpv](https://mpv.io), [IINA](https://iina.io), and [VLC](https://www.videolan.org).

---

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.

---

<div align="center">

Made by [**@mesamirh**](https://github.com/mesamirh)

<sub>Not affiliated with MovieBox or its operators.</sub>

</div>
