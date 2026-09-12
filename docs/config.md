# Configuration

## `config.json`

Written atomically under the config directory (`dirs::config_dir()/moviebox-tui/`;
macOS `~/Library/Application Support/moviebox-tui`, Linux `~/.config/moviebox-tui`).

| Field               | Type           | Meaning                                                                                                     |
| :--- | :--- | :--- |
| `auto_update`       | bool           | Check for updates on startup (max once/hour).                                                               |
| `last_update_check` | u64            | Epoch seconds of the last update check.                                                                     |
| `active_mode`            | string         | Last active mode (`streaming`, `tv`) restored on startup.                                                  |
| `active_provider`        | string         | Last provider (`moviebox`, `fourkhdhub`, `bdix_circleftp`, `bdix_dhakaflix`, `addons`).                    |
| `active_theme`           | string         | Theme name (`Mocha`, `TokyoNight`, `Nord`, `Dracula`, `Gruvbox`, `RosePine`).                              |
| `moviebox_enabled`       | bool           | Enable MovieBox streaming provider (`/settings` → Content Modes → Streaming Sources).                       |
| `fourkhdhub_enabled`     | bool           | Enable 4KHDHub streaming provider (`/settings` → Content Modes → Streaming Sources).                       |
| `bdix_circleftp_enabled` | bool           | Enable CircleFTP mirror (`/settings` → Content Modes → Streaming Sources).                                  |
| `bdix_dhakaflix_enabled` | bool           | Enable DhakaFlix mirror (`/settings` → Content Modes → Streaming Sources).                                  |
| `bdix_probed`            | bool           | Tracks if initial startup BDIX probe has executed.                                                         |
| `streaming_enabled`      | bool           | Enable Streaming Mode navigation in bottom dock (`/settings` → Content Modes).                               |
| `tv_enabled`             | bool           | Enable TV Mode navigation in bottom dock (`/settings` → Content Modes).                                      |
| `addons_enabled`         | bool           | Enable Stremio Addons provider availability.                                                               |
| `default_player`         | string or null | Preferred player: `mpv`, `iina`, `vlc`, `android`; absent/null until you choose one from the in-app picker. |
| `download_dir`           | string or null | Custom directory for video and subtitle downloads (null uses OS default).                                  |

## Interactive Settings Hub (`/settings`)

All settings in `config.json` can be configured interactively inside the application by typing `/settings` into the search bar.

- **General**: Toggle automatic update checks, choose default media player (`mpv`, `VLC`, `IINA`, `Android`), and edit download folder path.
- **Content Modes**: Toggle Streaming Mode, open the Streaming Sources selector (to enable/disable MovieBox, 4KHDHub, CircleFTP, DhakaFlix), and toggle Live TV.
- **Appearance**: Open the visual theme swatch picker to select among 6 built-in color themes.
- **Maintenance**: Purge disk cache, query GitHub for release updates, open GitHub repository, and trigger a manual local network BDIX re-probe.
## Other persisted files

- `addons_config.json` — list of installed HTTP addons in the config directory (see [addons-mode.md](addons-mode.md)).
- `tv_config.json` — list of M3U playlist sources in the config directory (see [tv-mode.md](tv-mode.md)).
- `history.json` — watch history in the system data directory (`dirs::data_dir()/moviebox-tui/`).
- `favorites.json`: starred titles in the system data directory. Independent of `history.json`; clearing watch history or cache never touches it.
- `playback/` — temporary playback states in the system data directory for resilient progress tracking.
- `scripts/` — bundled player scripts (`moviebox_tracker.lua`) in the system data directory.
- `iptv_cache/` — legacy TV image cache directory that `ClearCache` still removes.

## Environment variables

| Variable                  | Purpose                                                                           |
| :--- | :--- |
| `MOVIEBOX_LOG`            | Log level: `off`, `error`, `warn`, `info`, `debug`, `trace`. See [logging.md](logging.md). |
| `MOVIEBOX_PLAYER`         | Preferred player (overrides `default_player`).                                    |
| `MOVIEBOX_MPV_PATH`       | Custom mpv executable.                                                            |
| `MOVIEBOX_VLC_PATH`       | Custom VLC executable.                                                            |
| `MOVIEBOX_IINA_PATH`      | Custom IINA/iina-cli executable.                                                  |
| `MOVIEBOX_FOURKHDHUB_URL` | Override the 4KHDHub base URL.                                                    |
| `MOVIEBOX_THEME`          | Force a theme (e.g. `Mocha`, `Latte`, `Macchiato`, `Frappe`, `Nord`, `TokyoNight`, `Dracula`, `Gruvbox`, `RosePine`). When unset and no saved theme exists, the app auto-detects: `NO_COLOR` wins, truecolor terminals get full palettes, 256-color terminals get quantized palettes, and the OSC 11 background query picks light/dark variants with WCAG AA contrast. |
| `MOVIEBOX_NO_IMAGE`       | Disable poster image queries (set to `1` or `true`).                              |
| `MOVIEBOX_IMAGE_PROTOCOL` | Override image protocol (`kitty`, `sixel`, `iterm2`, or `none`/`off`).            |
| `MOVIEBOX_CELL_SIZE`      | Override terminal cell size as `WxH` (e.g. `10x20`) for poster scaling.           |

## CLI

- `moviebox-tui --help` and `moviebox-tui -h` print the help manual and exit.
- `moviebox-tui --version`, `moviebox-tui -v`, and `moviebox-tui -V` print the version and exit.
