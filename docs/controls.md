# Controls & Shortcuts

MovieBox-TUI is designed for fast keyboard navigation with complete mouse support throughout the interface. You can press `?` anywhere inside the application to open the mode-aware interactive help dialog.

## Global Shortcuts

| Key | Action |
| :--- | :--- |
| **`↑` / `↓` / `k` / `j`** | Navigate lists, search results, or move cursor up/down |
| **`←` / `→` / `h` / `l`** | Move text input cursor, step wide grid columns, or switch Details panes (Audio/Seasons/Episodes/Streams) |
| **`Home` / `End` / `g` / `G`** | Jump to start / end of list or search results (auto-fetches next page), or move cursor to beginning / end of input line |
| **`PageUp` / `PageDown`** | Scroll search results, lists, and modal pickers by visible page height (or scroll help overlay) |
| **`Enter`** | Open, play, or confirm the selected item |
| **`Space` / `P`** | Direct resume playback for recorded season/episode on `/history` or Home Continue Watching items |
| **`Esc`** | Focus search input (when results present), dismiss popup dialog, or return to landing |
| **`Tab` / `Shift+Tab`** | Auto-complete suggestion / command; cycle landing deck tabs (Continue Watching / Favorites); switch details panes; toggle dialog buttons |
| **`Backspace`** | Delete character before cursor, or return focus to search bar from results |
| **`Delete`** | Delete character at cursor in text inputs, or remove entry in TV/Addon managers |
| **`Ctrl+U`** | Clear entire input line (Search, TV URL, Addon URL) |
| **`Ctrl+W`** | Delete backward word in text inputs |
| **`c`** | Clear active search query and return to landing screen (Normal mode) |
| **`x` / `X`** | Cancel active download and preserve partial `.part` data |
| **`Ctrl+S`** | Switch to standard **Streaming Mode** |
| **`Ctrl+T`** | Toggle / switch to **TV Mode** |
| **`?`** | Open interactive in-app help menu |
| **`Ctrl+C` / `q`** | Quit application and restore terminal |

## Text Input & Cursor Editing

Text editing across Search, TV Playlist Manager, and Addon Manager uses a unified grapheme-safe input engine:

| Key | Action |
| :--- | :--- |
| **`Left` / `Right`** | Move text cursor one grapheme cluster left or right |
| **`Home` / `End`** | Jump cursor directly to the beginning or end of the input line |
| **`Backspace`** | Delete the grapheme cluster immediately before the cursor |
| **`Delete`** | Delete the grapheme cluster at the cursor position |
| **`Ctrl+W`** | Delete the preceding word (up to space or punctuation delimiter) |
| **`Ctrl+U`** | Clear the entire input buffer |
| **`Tab`** | Auto-complete active search suggestion or slash command |
| **`Enter`** | Submit search query, save TV playlist URL/path, or verify and install Addon manifest |
| **`Esc`** | Cancel input, dismiss input prompt, or clear search buffer |

## Modal Dialogs & Pickers

All popup dialogs (Theme picker, Browse categories, Provider menu, Settings Media Player picker, TV Manager, Addon Manager, Download Confirmation) support standard keyboard controls:

- **`↑` / `↓` / `k` / `j`**: Move selection up / down by one item (vim keys `k`/`j` supported in pickers such as theme selector in Settings).
- **`Home` / `End`**: Jump immediately to the first or last item in the list.
- **`PageUp` / `PageDown`**: Step up or down by 5 items.
- **`Enter`**: Confirm selection, activate entry, or submit dialog.
- **`Esc`**: Dismiss popup dialog without applying changes.
- **Download Confirmation Dialog**:
  - **`Tab` / `Shift+Tab` / `BackTab`**: Toggle active selection between `[ Download ]` and `[ Cancel ]`.
  - **`Left` / `Right`**: Switch between `[ Download ]` and `[ Cancel ]`.
  - **`Enter`**: Confirm the currently focused action.
  - **`Esc`**: Cancel and close the confirmation dialog.
## Mode-Specific Controls

### Streaming Mode
- **`Ctrl+P`**: Cycle content providers (`MovieBox` → `4KHDHub` → `BDIX` → `Addons`). On the Details screen, re-searches and fetches alternate streams for the current movie in-place.
- **`←` / `→` / `h` / `l` / `Tab` / `Shift+Tab`**: Switch Details screen selector panes (Audio Languages, Seasons, Episodes, Streams).
- **`Enter`**: Play selected stream or open selected title.
- **`d`**: Download current episode or full season batch.
- **`r`**: Refresh search results / stream list.
- **`f`**: Favorite / unfavorite the selected title (Home & Details screens).
- **`/settings`**: Open interactive Settings & Preferences Hub.
- **`/browse`**: Open curated browse categories (Trending, Popular, Top Rated, etc.).
- **`/history`**: Open watch history (`Space` or `P` to instantly resume recorded episode/movie).
- **`/favorites`**: Open your starred titles.
- **`/clear`**: Clear active search query and return to landing.

### Live TV Mode
- **`Enter`**: Play selected TV channel immediately with default player.
- **`r`**: Reload all active M3U playlist sources.
- **`/list`**: Show all loaded channels.
### Stremio Addons Provider
- **`/config`**: Open the Addon Manager directly when the active provider is `Addons`.
- **`Enter`**: Select title or play resolved stream.
- **`d`**: Download HTTP stream release.
- **`r`**: Refresh addon catalog search results.
- **`f`**: Favorite / unfavorite the selected title (Home & Details screens).
## Mouse Controls

| Action | Result |
| :--- | :--- |
| **Click provider badge** | Open anchored provider selection menu; click provider to switch directly |
| **Click search bar** | Enter search input mode |
| **Click search result row** | Select item and load preview; click again to open full details |
| **Click landing deck tab header** | Switch between Continue Watching and Favorites tabs |
| **`Click [x] Cancel on download bar`** | Cancel active download |
| **Click Continue Watching row (landing)** | Select an in-progress title; click again to resume playback with auto-play |
| **Click Favorites row (landing)** | Select a starred title; click again to open details |
| **Click "+N more • /history" or "+N more • /favorites"** | Open the full watch history or favorites list |
| **Click audio / season / episode / stream** | Switch audio language, change season, or select episode; click a specific stream row to play; click empty stream pane space to focus without playing |
| **Click footer buttons** | Switch provider / mode, open help (`[?]`), or quit (`[q]`) |
| **Click modal buttons** | Choose a theme, subtitles, player, or confirm actions |
| **Click outside a modal** | Dismiss popup dialog |

## Slash Commands

Type these commands directly into the search bar:

| Command | Applicable Mode | Action |
| :--- | :--- | :--- |
| `/settings` | All | Open interactive Settings & Preferences Hub (aliases `/config`, `/pref`, `/preferences`, `/options`) |
| `/browse` | Streaming / Addon | Browse curated views (Trending, Popular) or Addon catalogs (Top Movies, Top Series) |
| `/history` | Streaming / Addon | View watch history with latest progress |
| `/favorites` | Streaming / Addon | View all starred titles |
| `/clear` | All | Clear search results and return to landing |
| `/help` | All | Open interactive keybinding help menu (alias `/?`) |
| `/list` | TV | View live TV channels |
| `/exit` | All | Exit application and return to shell (aliases `/quit`, `/q`) |

## Help Menu Overlay

- Open with `?`; close with `?`, `Esc`, or `q`.
- `↑`/`↓`, `PageUp`/`PageDown`, and the mouse wheel scroll long content.
- Other keys are ignored while help is open.

## Update Notification & Self-Update Modals

When a new release is detected, a centered modal card displays the version comparison, environment instructions, release highlights, and quick actions:

- `u` / `U`: Download and install update immediately (direct binary replacement platforms).
- `b` / `B`: Copy Homebrew upgrade command (`brew upgrade moviebox-tui`) to clipboard with status toast on Homebrew-managed installations.
- `o` / `O`: Open GitHub release notes in the system browser.
- `Esc`: Dismiss modal and return to previous screen.
- The notification modal formats release notes into clean, indented category sections (`[Added]`, `[Fixed]`) with bold feature titles and bullet items, stripping decorative prefixes and duplicate paragraph text.
- During self-update (`u`), a focused progress modal displays an active spinner with a unified action status (`⠋ Downloading MovieBox-Tui v...` / `⠋ Installing MovieBox-Tui v...`) and a cross-platform warning notice (`⚠ Please wait • do not close terminal` on modern terminals, `[!] Please wait - do not close terminal` on basic terminals) with generous border clearances.
- Modal presentation is deferred while actively typing in the search bar (`InputMode::Editing`) to prevent input hijacking. During update installation, keyboard and mouse inputs are locked while the progress modal is displayed.
## Wide-Terminal Grid

On terminals at least 110 columns wide, search results render in two
columns (three at 160+). `↑`/`↓` move one visual row, `←`/`→` move one
item, and clicks map through column bounds. Narrower terminals keep the
classic single-column list where `←`/`→` jump a full page.
