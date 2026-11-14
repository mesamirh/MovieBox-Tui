# Stremio Addons

MovieBox-TUI supports Stremio HTTP addons. You can install standard addon manifest URLs to browse catalogs and stream media.

## Quick Start

- **Switch to Addons**: Press `Ctrl+P` on the home screen until `[Addons]` is selected.
- **Open Addon Manager**: Type `/config` or click `[Addons]` in the search bar.
- **Browse Catalogs**: Type `/browse` (or `/browse <catalog>`, e.g. `/browse anime` or `/browse top rated`) to browse all catalogs exposed by your enabled addons with automatic pagination.
- **Search**: Type in the search bar to search titles across installed addons.

## Addon Manager

The Addon Manager lets you add, enable, or remove addon manifests:

- **Toggle Addon**: Press `Enter` or `Space` on an addon to enable (`✓`) or disable it.
- **Add Addon**: Select `+ Add manifest URL`, paste the addon manifest URL, and press `Enter`.
- **Remove Addon**: Press `d` or `Delete` to remove an addon.
- **Core Addon (Cinemeta)**: Pre-installed for movie and series metadata. It cannot be deleted.

## Supported Streams

- Streams are parsed and labeled with quality tags (`4K`, `1080p`, `720p`), codec, and file size.
- Video stream headers (`User-Agent`, `Referer`) are automatically forwarded to external players (`mpv`, `VLC`, `IINA`) and download workers.
- Watch progress and episode resumption are saved automatically to `/history`.

## Torrent streams

Stremio addons such as Torrentio return P2P streams (`infoHash` + optional `fileIdx`) instead of HTTP URLs. MovieBox-TUI plays them with its built-in torrent engine, so no external server is needed: open a title and torrent releases appear labelled `<Addon> · Torrent` next to normal HTTP streams.

- The engine starts on the first torrent play and listens only on `127.0.0.1:11490` (automatically falling back up to `+20` if the port is busy; change the start with `MOVIEBOX_BUILTIN_TORRENT_PORT`). Requests for any other hostname are refused.
- **Picks the right file.** The addon's filename hint is matched first; otherwise the requested episode of a season pack (both for playback and `/downloads`), otherwise the largest video (sample clips are ignored).
- **Head + tail pre-buffering.** Before the player opens, the engine pre-warms the first 2 MiB and the final 256 KiB of the container (where MP4 `moov` atoms and MKV `Cues` sit) so playback and seeking start smoothly.
- **Speeds up peer discovery.** Public trackers are added to every magnet next to the addon's own trackers.
- **Automatic cleanup and idle pause.** Switching to another torrent evicts the previous swarm from the session, and closing the player (or cancelling with `Esc`) pauses background peer traffic when no download is active. Downloaded pieces live in the cache directory under `torrents/` and are deleted on exit.
- Only the selected file is downloaded, and torrent downloads use a single sequential stream so piece priority stays at the read head.
- **Accepts incoming peers.** The engine listens on port 4240 (next free port up to +19; change the start with `MOVIEBOX_TORRENT_LISTEN_PORT`) and asks your router for a UPnP mapping, like Stremio does. Allow that port in your firewall for the best speeds. `MOVIEBOX_TORRENT_NO_LISTEN=1` turns this off and `MOVIEBOX_TORRENT_NO_UTP=1` limits it to TCP.
- **Season packs.** Playing another episode of a torrent you already opened works without a restart.
- **Ranked by seeds, then quality.** Torrent releases are grouped by swarm health: 30+ seeders (or no seeder count, such as direct HTTP streams) first, then 5–29, then 1–4, then 0. Inside each group the better quality comes first (2160p, 1080p, 720p, 480p), then more seeders. A healthy 1080p therefore lists above a nearly dead 2160p. CAM, telesync and screener rips always go last. The seeder count is shown in the source label, for example `Torrentio · Torrent · 128 seeds`.
- **"Torrent running" notification.** From the moment you press play on a torrent, a notification stays on screen with live peers, seeders, download speed and the amount downloaded. It does not time out. It disappears when the player starts reading video, or when the start is cancelled or fails. Press `Esc` while it shows "Finding peers" to cancel the start.
- Downloads (`/downloads`) of torrent releases go through the same engine.
- Addons get up to 22 seconds (with one retry) to answer, and an unreadable stream entry is skipped instead of hiding the addon's other results.
- **Esc closes a player that is still loading.** Pressing `Esc` in the terminal while the player runs closes it at once, so a wrong or unsupported file does not block the next link. If the player has been running for more than two minutes, `Esc` asks for a second press within three seconds.
- **Failed addons are named.** When an addon times out or errors, the status line and the "No streams found" message say which one and why. A list that is missing a failed addon is not cached, so the next open asks again. Pasted `/configure` links are turned into the addon's `/manifest.json` automatically, and manifests with odd entries still install.
- `MOVIEBOX_ADDON_USER_AGENT` overrides the User-Agent sent to addons, for hosts that reject unknown clients.

## Nuvio plugins

Nuvio plugin repositories (a `manifest.json` with a `scrapers` list, a bare GitHub repository URL such as `https://github.com/<owner>/<repo>`, or a direct standalone `.js` scraper URL) can be added in the addon manager like any other addon URL.

- Every scraper of the repository becomes one entry of the addon list, **off by default**. Enable only the ones you want; each enabled scraper is run when you open a title.
- Scrapers are JavaScript files, run by a small Node.js script, so **Node.js 18 or newer must be installed** (`node` in your PATH, or set `MOVIEBOX_NODE` to its path).
- Scrapers are looked up by TMDB id. It is taken from Cinemeta; set `MOVIEBOX_TMDB_API_KEY` to your own TMDB API key as a fallback. If neither answers, the IMDb id is passed on.
- Scripts are downloaded to the cache folder (`nuvio/`) and refreshed after 6 hours. Each scraper gets up to 40 seconds.
- Only plain `http(s)` streams are used; the quality, size and headers a scraper reports go through the same ranking as other addon streams.
- Plugin code runs on your computer. Only add repositories you trust.
- A scraper that fails is named in the status line with the reason, for example `Node.js not found`.

## Storage

Installed addons are saved to `addons_config.json` in your configuration directory:
- **Linux / macOS / Termux**: `~/.config/moviebox-tui/addons_config.json`
- **Windows**: `%APPDATA%\MovieBox-Tui\addons_config.json`
