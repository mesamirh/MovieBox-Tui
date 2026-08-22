# Configuration

## `config.json`

Written atomically under the config directory (`dirs::config_dir()/moviebox-tui/`;
macOS `~/Library/Application Support/moviebox-tui`, Linux `~/.config/moviebox-tui`).

| Field               | Type           | Meaning                                                                                                     |
| ------------------- | -------------- | ----------------------------------------------------------------------------------------------------------- |
| `auto_update`       | bool           | Check for updates on startup (max once/hour).                                                               |
| `last_update_check` | u64            | Epoch seconds of the last update check.                                                                     |
| `active_mode`       | string         | Last active mode (`streaming`, `tv`, `addon`) restored on startup.                                         |
| `active_provider`   | string         | Last provider (`moviebox`, `fourkhdhub`, …).                                                                |
| `active_theme`      | string         | Theme name.                                                                                                 |
| `bdix_enabled`      | bool           | Show BDIX providers (Bangladesh-only).                                                                      |
| `streaming_enabled` | bool           | Enable Streaming Mode navigation in bottom dock (`/enable-streaming`, `/disable-streaming`).                 |
| `tv_enabled`        | bool           | Enable TV Mode navigation in bottom dock (`/enable-tv`, `/disable-tv`).                                     |
| `addons_enabled`    | bool           | Enable Addon Mode navigation in bottom dock (`/enable-addons`, `/disable-addons`).                          |
| `default_player`    | string or null | Preferred player: `mpv`, `iina`, `vlc`, `android`; absent/null until you choose one from the in-app picker. |
| `download_dir`      | string or null | Custom directory for video and subtitle downloads (null uses OS default).                                   |

## Other persisted files

- `addons_config.json` — list of installed HTTP addons in the config directory (see [addons-mode.md](addons-mode.md)).
- `tv_config.json` — list of M3U playlist sources in the config directory (see [tv-mode.md](tv-mode.md)).
- `history.json` — watch history in the system data directory (`dirs::data_dir()/moviebox-tui/`).
- `favorites.json`: starred titles in the system data directory. Independent of `history.json`; clearing watch history or running `/clear-cache` never touches it.
- `playback/` — temporary playback states in the system data directory for resilient progress tracking.
- `scripts/` — bundled player scripts (`moviebox_tracker.lua`) in the system data directory.
- `iptv_cache/` — legacy TV image cache directory that `ClearCache` still removes.

## Environment variables

| Variable                  | Purpose                                                                           |
| ------------------------- | --------------------------------------------------------------------------------- |
| `MOVIEBOX_LOG`            | Log level: `off`, `warn`, `info`, `debug`, `trace`. See [logging.md](logging.md). |
| `MOVIEBOX_PLAYER`         | Preferred player (overrides `default_player`).                                    |
| `MOVIEBOX_MPV_PATH`       | Custom mpv executable.                                                            |
| `MOVIEBOX_VLC_PATH`       | Custom VLC executable.                                                            |
| `MOVIEBOX_IINA_PATH`      | Custom IINA/iina-cli executable.                                                  |
| `MOVIEBOX_FOURKHDHUB_URL` | Override the 4KHDHub base URL.                                                    |
| `MOVIEBOX_THEME`          | Force a theme (e.g. `Dracula`, `Gruvbox`, `Nord`, `Mocha`, `RosePine`, etc.).    |
| `MOVIEBOX_NO_IMAGE`       | Disable poster image queries (set to `1` or `true`).                              |

## CLI

- `moviebox-tui --help` and `moviebox-tui -h` print the help manual and exit.
- `moviebox-tui --version`, `moviebox-tui -v`, and `moviebox-tui -V` print the version and exit.
