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

## Tasks

| ID | Spec | Route | Commit |
| --- | --- | --- | --- |
| T6 | S1-S5 | inline | config key + core filter for releases and dubs + tests | `feat(language): hide hindi dubs and releases` |
| T7 | S5 | inline | TUI settings toggle | same commit as T6 |
| T8 | — | inline | docs + CHANGELOG | `docs(language): document the hindi release filter` |

## Log

- **L1 (user, verbatim)** — "Adicional no quiero ver contenido hindi."
- **L2 (user decisions, verbatim)** — "Sólo doblajes/releases hindi" and "Núcleo Rust (TUI + iOS)".
- **L3 (evidence)** — Catalog items carry no language at all (`CatalogItem` `src/providers/models.rs:97-104`, `MobileCatalogItem` `src/ffi.rs:43-52`): MovieBox sends language fields and `adapt.rs:60-120` drops them, so title-level filtering would need new parsing. Release/dub level already has data: `Release.language`, `dubs: Vec<AudioTrackOption>`, plus provider heuristics that already recognise `hindi`/`hin`.
- **L4 (evidence)** — The only existing filtering is provider enable flags (`src/config.rs:13-19`); no content exclusion mechanism exists.
