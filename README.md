<div align="center">

# MovieBox-TUI

**Stream movies, shows, anime, and live TV from your terminal.** <br>
Fast and clean. No configuration, no torrents, and no debrid required.

[![Crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust)](https://crates.io/crates/moviebox-tui)
[![Downloads](https://img.shields.io/crates/d/moviebox-tui.svg)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg?logo=rust)](#requirements)

<br>

<img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/01-home-blocky.jpg" alt="MovieBox-TUI Home" width="85%">

**[See what's new in v0.1.7 on YouTube](https://youtu.be/5M2_mjH5r5Y)**

<sub>Found a bug? [Open an issue](https://github.com/mesamirh/MovieBox-Tui/issues) so I can fix it for everyone!</sub>

</div>

## This fork (wakeupbrk) — recommended build

This is **Amar’s improved fork** of MovieBox-TUI (not the stock upstream release).  
Extra features on top of upstream:

- **Download library** — browse and play local downloads from the app  
- **Multi-provider search** — MovieBox + 4KHDHub together, ranked by relevance  
- **Source picker** — `Ctrl+P` → All / MovieBox only / 4KHDHub only  
- Better rate-limit handling and clearer status messages  
- Player picker when multiple players are installed (mpv recommended)  
- Fixed series flow (audio / season / episode / streams navigation)

### Install this fork (macOS / Linux)

**Requirements:** a video player (`mpv`, **IINA**, or VLC) and [Rust](https://rustup.rs) (`rustup`).

```bash
# 1) Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2) Build & install Amar’s fork from GitHub
cargo install --git https://github.com/wakeupbrk/MovieBox-Tui.git --locked

# 3) Run it
moviebox-tui
```

If you already installed the **original** Homebrew/crates version, either:

```bash
# Prefer cargo’s binary (usually already first if cargo is set up)
which moviebox-tui
# should show: ~/.cargo/bin/moviebox-tui

# Or remove the old brew app so only this fork remains:
brew uninstall moviebox-tui 2>/dev/null
```

**Update later:**

```bash
cargo install --git https://github.com/wakeupbrk/MovieBox-Tui.git --locked --force
```

Repo: https://github.com/wakeupbrk/MovieBox-Tui  
Upstream PR: https://github.com/mesamirh/MovieBox-Tui/pull/30

---

## Screenshots

<details>
<summary><b>Movie & Series Details</b></summary><br>
<p align="center">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/07-movie-details.jpg" alt="Movie Details" width="49%">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/08-series-details.jpg" alt="Series Details" width="49%">
</p>
</details>

<details>
<summary><b>Search & Downloads</b></summary><br>
<p align="center">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/06-search-results.jpg" alt="Search Results" width="49%">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/12-download-progress.jpg" alt="Download Progress" width="49%">
</p>
</details>

<details>
<summary><b>Playback & Subtitles</b></summary><br>
<p align="center">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/11-player-picker.jpg" alt="Media Player Selection" width="49%">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/10-playback-subtitles.jpg" alt="Subtitle Language Selection" width="49%">
</p>
</details>

<details>
<summary><b>Live TV Experience</b></summary><br>
<p align="center">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/09-live-tv-list.jpg" alt="Live TV Channels" width="49%">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/05-tv-help.jpg" alt="Live TV Configuration" width="49%">
</p>
</details>

<details>
<summary><b>Home Themes</b></summary><br>
<p align="center">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/03-home-3d.jpg" alt="3D Block Theme" width="49%">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/02-home-ascii.jpg" alt="Minimal ASCII Theme" width="49%">
</p>
</details>

<details>
<summary><b>Help & Configuration</b></summary><br>
<p align="center">
  <img src="https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/assets/screenshots/04-global-help.jpg" alt="Global Help Menu" width="85%">
</p>
</details>


## Features

### Streaming & Playback
- **Instant Search & Catalogs:** Type to search instantly, or browse trending movies, shows, and anime using slash commands (e.g., `/movies`, `/anime`).
- **Seamless Local Playback:** Resolves 4K/1080p streams and opens them instantly in your preferred local video player (`mpv`, `IINA`, or `VLC`).
- **Integrated Subtitles:** Automatically fetches available subtitles and lets you select your preferred language before playback.
- **Live IPTV:** Press `Ctrl+T` to toggle Live TV mode and stream thousands of live television channels globally.

### Advanced Downloading
- **Batch Season Downloader:** Queue up entire television seasons for concurrent downloading with a single keystroke.
- **Resilient Downloads:** Built-in support for download resumes. If a download is interrupted or fails, it picks up right where it left off.
- **Auto-Subtitle Fetching:** Automatically downloads the best-matching `.srt` subtitle files alongside your video files.

### Terminal Experience
- **Native Image Rendering:** Enjoy high-resolution movie posters rendered directly in supported terminals.
- **Dynamic Theming:** Switch between beautiful 3D block layouts and clean ASCII themes to fit your aesthetic.
- **Power-User Slash Commands:** Use terminal-style commands to update the app (`/update`), switch categories, or customize your Live TV playlists (`/config`).
- **Smart Auto-Cleanup:** A silent background worker intelligently manages and deletes old cache files to protect your disk space.


## Installation

**Prerequisites:** You will need a terminal (at least 85×24 characters) and a local video player installed (e.g. `mpv`, `IINA`, or `VLC`).

The easiest way to get started is by using our quick install scripts. These scripts will automatically download the correct version for your computer.

### Homebrew (macOS & Linux)
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```

### Install Script (macOS & Linux)
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

### Windows
```powershell
powershell -c "irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex"
```

### Cargo (For Rust Developers)
```bash
cargo install moviebox-tui
```

<details>
<summary><i>Need to uninstall?</i></summary>

- **Homebrew:** `brew uninstall moviebox-tui && brew untap mesamirh/moviebox-tui`
- **Mac/Linux:** `sudo rm -f /usr/local/bin/moviebox-tui`
- **Windows:** `Remove-Item -Recurse -Force $env:USERPROFILE\AppData\Local\MovieBox-Tui`
- **Cargo:** `cargo uninstall moviebox-tui`
</details>



## Getting Started

Once installed, just open your terminal and type `moviebox-tui` to jump in!

### Keyboard Controls

<table>
  <tr>
    <th align="left">Key</th>
    <th align="left">Action</th>
  </tr>
  <tr>
    <td>Alphanumeric</td>
    <td>Start searching instantly</td>
  </tr>
  <tr>
    <td><kbd>↑</kbd> <kbd>↓</kbd> <kbd>←</kbd> <kbd>→</kbd></td>
    <td>Navigate menus and grids</td>
  </tr>
  <tr>
    <td><kbd>Enter</kbd></td>
    <td>View details, pick episodes, or play video</td>
  </tr>
  <tr>
    <td><kbd>o</kbd></td>
    <td>Switch to a different video player on playback</td>
  </tr>
  <tr>
    <td><kbd>d</kbd></td>
    <td>Download an episode or an entire season</td>
  </tr>
  <tr>
    <td><kbd>Ctrl</kbd>+<kbd>p</kbd></td>
    <td>Switch between different content providers / sources</td>
  </tr>
  <tr>
    <td><kbd>Ctrl</kbd>+<kbd>t</kbd></td>
    <td>Toggle Live TV mode to browse IPTV channels</td>
  </tr>
  <tr>
    <td><kbd>?</kbd></td>
    <td>Open the global help menu</td>
  </tr>
  <tr>
    <td><kbd>q</kbd></td>
    <td>Quit (or use <kbd>Esc</kbd> to go back/clear search)</td>
  </tr>
</table>

### Slash Commands
You can type these special commands straight into the search bar:

<table>
  <tr>
    <th align="left">Command</th>
    <th align="left">Category</th>
    <th align="left">Description</th>
  </tr>
  <tr>
    <td><code>/discover</code> or <code>/home</code></td>
    <td>Streaming</td>
    <td>See what's trending right now</td>
  </tr>
  <tr>
    <td><code>/movies</code>, <code>/shows</code>, <code>/anime</code></td>
    <td>Streaming</td>
    <td>Jump straight to a specific category</td>
  </tr>
  <tr>
    <td><code>/list</code></td>
    <td>Live TV</td>
    <td>Show the list of available live channels</td>
  </tr>
  <tr>
    <td><code>/config</code></td>
    <td>Live TV</td>
    <td>Open the TV configuration menu to add your own m3u playlists</td>
  </tr>
  <tr>
    <td><code>/update</code></td>
    <td>General</td>
    <td>Check to see if there's a new version of the app</td>
  </tr>
  <tr>
    <td><code>/toggle-update</code></td>
    <td>General</td>
    <td>Turn automatic background update checking on or off</td>
  </tr>
</table>


## Contributing

I'd love your help making this even better! If you've got a big feature in mind, it's usually best to open an issue first so we can chat about it.

```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build
```

Just try to follow [Conventional Commits](https://www.conventionalcommits.org/) and make sure `cargo fmt` and `cargo clippy` are happy before you open a PR. You can check out [CONTRIBUTING.md](CONTRIBUTING.md) for the full rundown.


## Credits & Legal

Live TV channel playlists are graciously provided by [iptv-org/iptv](https://github.com/iptv-org/iptv).

> **Disclaimer:** This is a third-party client. It does not host or store any media and only resolves links from upstream APIs. Intended for personal use only.


## Community & Support

The best way to support MovieBox-TUI is simply to use it, share it, and leave a star on GitHub!

If you'd like to buy me a coffee for the late nights spent coding, you can use the addresses below.

- **EVM:** `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c`
- **BTC:** `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ`
- **LTC:** `ltc1qhjkq2n6tsayxj56n3c53uqv23v8vqhvc9g3vxl`

---

<div align="center">

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.<br>
Built by [**@mesamirh**](https://github.com/mesamirh)

<sub>Not affiliated with any third-party content providers or operators.</sub>

</div>
