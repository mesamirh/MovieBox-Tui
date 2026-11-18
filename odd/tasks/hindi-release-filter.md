# Hindi release filter

Hide Hindi dubs and Hindi releases from every client, from the Rust core, behind one
persisted config key. Catalog titles stay visible: only the audio/release options in
that language disappear.

## Specs

**S1 (user, verbatim)** — "Adicional no quiero ver contenido hindi."

**S2 (decision, verbatim)** — "Sólo doblajes/releases hindi"

Hide releases whose detected language is Hindi and Hindi entries in the dub/audio
track list. The title itself stays in catalog, search and details: a Hollywood film
with a Hindi dub keeps its original and English versions and stops offering the Hindi
one.

**S3 (decision, verbatim)** — "Núcleo Rust (TUI + iOS)"

The filter runs in the core, so TUI and iOS behave identically with no per-client
duplication.

**S4 (parent-derived, safety)** — the filter never leaves a title unplayable. When
every release for a title is Hindi, they are returned unfiltered instead of an empty
list, and the picker still explains the situation. Hiding the only playable option
would turn a preference into a broken screen.

**S5b (decision, parent)** — the filter is on by default (`hide_hindi = true`), because
the request is "no quiero ver contenido hindi" and not "give me a switch". The config
key and the TUI toggle are how another viewer turns it back on.

**S5** — The preference persists in `src/config.rs` and survives restarts, with a
TUI settings toggle. Detection reuses the existing language data, never new guesses:
`Release.language` (`src/providers/models.rs:243`) and the filename/audio heuristics
already in the providers (`src/providers/fourkhdhub/parser.rs:424-511`,
`src/providers/bdix/common.rs:1-21`, `src/providers/addons/adapter.rs:219-243`).

**S6 (user, verbatim, 2026-11-18)** — "En los listados me sigue apareciendo contenido [Hindi]. No quiero que nada tenga esa palabra y adicional quita todo lo que sea realmente [Hindi]"

Two requirements, both catalog-level, both for every client:

**S6a — the word disappears from what the viewer reads.** Listing titles (home deck, movie/TV/anime grids, search results, favorites, library, details header, and the iOS equivalents) are displayed without the dub marker. The marker is stripped from the display string only; the stored/raw title keeps it so detection still works.

**S6b — genuinely Hindi items leave the listings.** When `hide_hindi` is on, catalog items whose content is Hindi are dropped from every listing, not only their releases and dubs (this supersedes S2's "titles stay visible" for Hindi-original content). Detection happens on the raw title/data before cleaning, because cleaning would erase the only signal.

**S6c — no false positives.** The marker is the bracketed/marked form the provider uses, not the bare word: a real title such as `Hindi Medium` stays. Detection reuses whatever the repository already recognises (the TUI already detects `[hindi]` markers), never a new ad-hoc spelling list.

## Tasks

| ID | Spec | Route | Commit |
| --- | --- | --- | --- |
| T6 | S1-S5 | inline | config key + core filter for releases and dubs + tests | 7274e33 |
| T7 | S5 | inline | TUI settings toggle | same commit as T6 |
| T8 | — | inline | docs + CHANGELOG | 7274e33 |
| T10 | S6a, S6c | writer | strip the dub marker from every listing title (iOS reads it raw today) | `fix(catalog): hide hindi titles and drop the marker from listings` |
| T11 | S6b, S6c | writer | drop Hindi catalog items at the core funnels without breaking pagination | same commit as T10 |

- **L5 (user, verbatim)** — "En los listados me sigue apareciendo contenido [Hindi]. No quiero que nada tenga esa palabra y adicional quita todo lo que sea realmente [Hindi]"
- **L6 (evidence)** — `CatalogItem` has no language field (`src/providers/models.rs:97-104`) and `MobileCatalogItem` neither (`src/ffi.rs:43-52`); MovieBox catalog/search JSON language fields are dropped in `adapt.rs:60-120`. The provider instead tags such content in the subject title itself, and the repository already detects that marker: `src/tui/app/search.rs:529` and `:556-559` test the title for a `[hindi]` marker.
- **L7 (decision)** — the raw title stays the source of truth for detection; only the display projection is cleaned. Filtering before display also means favorite/history entries that already carry a marked title keep matching after the change.

- **L8 (evidence, insertion points)** — TUI listing titles are already cleaned at ingestion (`tui/app/search.rs:529`, `tui/app/requests.rs:311`), so the visible `[Hindi]` leak is iOS: `ffi.rs:371 catalog_item_from` copies the raw `item.title`, and Swift renders it verbatim. The marker detector already in the repo is the bracketed check at `search.rs:556-559` (case-insensitive, `[hindi]/[tamil]/[telugu]/[english]`); `clean_moviebox_title` strips leading/trailing brackets and ` - Hindi` suffixes and keeps `Hindi Medium` (S6c test at `title.rs:281`).
- **L9 (evidence, risk)** — dropping items can desynchronise pagination: TUI ends paging on `raw_count < 15` (`app/requests.rs:305,384`) and iOS on an empty page (`AppModel.swift:145,173,210,236`). TUI also reads 1h/24h caches before the service (`search.rs:466-480`), so a service-only filter leaves stale Hindi rows until expiry.

- **L10 (evidence, after T10/T11)** — `cargo test --lib` 469 passed, `--features mobile` 472 passed, clippy clean. TUI paging now ends on an empty page only while `hide_hindi` is on (a filtered-short page no longer proves the list ended), and on the exact short-page rule while it is off.
- **L11 (new behaviour, user-visible)** — favorited/history rows now store the cleaned title in newly written entries. Identity is `subject_id`-based, so nothing stops matching, but the raw marker is no longer kept in those stored rows.

## Delivery slices

- Slice 4: T10+T11 — catalog-level hiding and the listing-title cleanup.
- Slice 3 (7274e33): the whole feature in one commit — `src/language_filter.rs`,
  the config key, both client boundaries, the TUI toggle and the docs. 375 authored
  lines, inside the ~400-line target.

## Log

- **L1 (user, verbatim)** — "Adicional no quiero ver contenido hindi."
- **L2 (user decisions, verbatim)** — "Sólo doblajes/releases hindi" and "Núcleo Rust (TUI + iOS)".
- **L3 (evidence)** — Catalog items carry no language at all (`CatalogItem` `src/providers/models.rs:97-104`, `MobileCatalogItem` `src/ffi.rs:43-52`): MovieBox sends language fields and `adapt.rs:60-120` drops them, so title-level filtering would need new parsing. Release/dub level already has data: `Release.language`, `dubs: Vec<AudioTrackOption>`, plus provider heuristics that already recognise `hindi`/`hin`.
- **L4 (evidence)** — The only existing filtering is provider enable flags (`src/config.rs:13-19`); no content exclusion mechanism exists.
