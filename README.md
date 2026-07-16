# MovieBox-Tui

A lightning fast, zero-config terminal user interface (TUI) for streaming movies and TV series directly from your terminal.

## Installation

```sh
cargo install moviebox-tui
```

*Note: Requires `mpv` installed on your system for video playback.*

## Usage

Launch the app from your terminal:

```sh
moviebox-tui
```

- **Search**: Press `/` to search for movies or shows.
- **Play**: Select a result and press `Enter` to stream instantly.
- **Logs**: Press `Ctrl+L` to view internal network logs.
- **Quit**: Press `q` or `Esc` to exit.

## Features

- Instant streaming with `mpv`
- Full metadata (seasons, episodes, dubs, and subs)
- Built in geo-unblocking (zero VPN required)
- Copy direct stream URLs to clipboard

## Screenshots

<details>
<summary>Click to view screenshots</summary>

<br>

### Home Screen
<img src="assets/screenshots/1-home.jpg" alt="Home Screen" width="800">

### Search Results
<img src="assets/screenshots/2-search.jpg" alt="Search Results" width="800">

### Movie Details
<img src="assets/screenshots/3-movie.jpg" alt="Movie Details" width="800">

### Stream Selection
<img src="assets/screenshots/4-streams.jpg" alt="Stream Selection" width="800">

### TV Series Details
<img src="assets/screenshots/5-series.jpg" alt="TV Series Details" width="800">

</details>

## License

Dual-licensed under MIT or Apache-2.0.
