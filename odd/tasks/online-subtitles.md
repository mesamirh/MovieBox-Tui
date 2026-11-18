# Online subtitles

Add an online subtitle search, keyed to the title being watched, to the Rust core so
the TUI and the iOS client share one implementation. Source is the Stremio
OpenSubtitles v3 addon (no API key). Spanish results rank above English above the rest.

## Specs

**S1 (user, verbatim)** — "ahora quiero tener para seleccionar subtitulos o buscarlos online de acuerdo al contenido que vea."

The user must be able to both select a subtitle track and search subtitles online for
the content currently playing.

**S2 (decision, verbatim)** — "Núcleo Rust (TUI + iOS)"

The online search lives in the Rust core. TUI and iOS consume it through the same
code path; no client re-implements ranking, matching or fetching.

**S3 (decision, verbatim)** — "Addon de Stremio (OpenSubtitles, sin key)"

The source is the Stremio addon at `https://opensubtitles-v3.strem.io`. No API key,
no user credential, no account. Verified response shape:
`{"subtitles":[{"id","url","lang","subtitleFileName","SubEncoding",...}]}` where `url`
is a directly downloadable UTF-8 `.srt`.

**S4 (decision, verbatim)** — "Español > Inglés > resto"

Online results are ordered Spanish first, English second, everything else after,
alphabetically by language among equals. The first entry is the one the picker
preselects.

**S5 (implementation constraint, parent-derived)** — the addon only answers
`/subtitles/{type}/{tt-id}` paths (`manifest.idPrefixes = ["tt"]`), and the core has
no IMDb id today: `MediaDetails` carries `imdb_rating` only
(`src/providers/models.rs:118-133`). The IMDb id is therefore resolved keylessly from
`https://v3.sg.media-imdb.com/suggestion/x/{query}.json`, matched on title + year +
kind (`movie` vs `series`), and cached on disk so repeated plays of one title cost
one lookup. A wrong or low-confidence match must not be used silently.

**S6** — Degradation never blocks playback and never loses provider captions. No
network, no IMDb match, or an empty addon answer yields an empty online list plus a
status message; the provider captions from `get_ext_captions` still play exactly as
they do today.

**S7** — iOS reaches parity with the TUI on provider captions: it must pass the dub
sibling `subject_id`s instead of `&[]` (`src/ffi.rs:239`), which is why it currently
receives a subset of what the TUI shows.

**S8 (user report, 2026-11-18)** — "Parace que la busqueda de subititulos busca usando la palabra [Hindi]. Por lo tanto no me trae data". The online search must clean the content title before querying IMDb: MovieBox subject titles reach `MediaDetails.title` untagged-cleaned (`src/providers/moviebox/adapt.rs:251-255`) and can carry dub markers such as `[Hindi] Title` or `Title - Hindi`, which fail the strict normalized-title match in `pick_best_candidate` and produce an empty list. The cleanup reuses the existing `clean_moviebox_title` (`src/providers/moviebox/title.rs:1-67`), strips a trailing year in parentheses, and must not alter a clean title. The same cleaned title is what the iOS player shows, so the viewer never reads a dub tag as the search query.

## Tasks

| ID | Spec | Route | Commit |
| --- | --- | --- | --- |
| T1 | S3, S5 | writer | IMDb resolver + OpenSubtitles client + unit tests | d753764 |
| T2 | S1, S2, S4, S6 | inline | `MovieBoxService::search_online_subtitles` + ranking + cache | same commit as T1 |
| T3 | S1, S2, S7 | inline | FFI method + dub-sibling ids for iOS | 5b5ea48 |
| T4 | S1, S4, S6 | inline | iOS subtitle menu: online section, ordering, states | same commit as T3 |
| T5 | — | inline | ios/README.md + docs table + CHANGELOG | 5b5ea48 |
| T9 | S8 | writer | normalize the query title + show the cleaned title + tests | `fix(subtitle): clean the title before the imdb lookup` |

### L7 (user, verbatim) — "Parace que la busqueda de subititulos busca usando la palabra [Hindi]. Por lo tanto no me trae data"

- **L8 (evidence, diagnosis)** — nothing in the iOS→Rust chain inserts a language: `PlayerView.swift:451` → `AppModel.swift:307` → `ffi.rs:268-283` → `service.rs:316-317,333`. The raw MovieBox subject title reaches `MediaDetails.title` uncleaned (`moviebox/adapt.rs:251-255`) while the repo already expects tagged titles (`moviebox/title.rs:1-67` strips `[...]` and ` - Hindi`). `[Hindi] Title` normalizes to `"hindi title"`, which cannot equal `"title"`, so the strict match returns `None` and the list is empty. Live probes: `tt1375666` returns 80 entries, `tt0903747:1:1` returns 89, `animal` resolves to `tt13751694` — the pipeline itself works.
- **L9 (evidence, second finding)** — the iOS HUD also shows the raw tagged title, and provider caption rows are named by bare language (`lanName`, `moviebox/adapt.rs:35-58`), so the viewer sees "Hindi" next to a search that never used it.

## Delivery slices

- Slice 1 (d753764): `src/providers/opensubtitles.rs`, `src/providers/mod.rs`,
  `src/cache.rs`, `src/service.rs` — the whole T1+T2 core unit, new client and its
  tests together. 1025 authored lines is above the ~400-line target on its own: this
  commit is the reviewable unit for the core client and must be reviewed as one,
  while the client-facing work stays in slice 2.
- Slice 2 (5b5ea48): `src/ffi.rs`, regenerated `ios/Generated`, the iOS Swift files,
  docs and CHANGELOG — ~294 authored lines.

## Log

- **L1 (user, verbatim)** — "ahora quiero tener para seleccionar subtitulos o buscarlos online de acuerdo al contenido que vea. Adicional no quiero ver contenido hindi."
- **L2 (user decisions, verbatim)** — Alcance: "Núcleo Rust (TUI + iOS)". Fuente: "Addon de Stremio (OpenSubtitles, sin key)". Hindi: "Sólo doblajes/releases hindi". Idioma de subtítulos: "Español > Inglés > resto".
- **L3 (evidence)** — Only MovieBox serves captions today (`src/providers/moviebox/mod.rs:383-393`); the funnel is `MovieBoxService::get_ext_captions` (`src/service.rs:190-299`) with cache keyed by `(subject_id, resource_id)`. No online subtitle source exists anywhere in `src/`. `SubtitleOption` is `{name, url}` (`src/providers/models.rs:232-235`).
- **L4 (evidence)** — Live probes: `opensubtitles-v3.strem.io/subtitles/movie/tt1375666.json` returned 80 entries; `v3.sg.media-imdb.com/suggestion/x/inception.json` returned `tt1375666`, `y: 2010`, `qid: "movie"`. Both keyless.
- **L6 (evidence)** — After slice 2: `cargo test --lib` 459 passed, `cargo clippy --all-targets -- -D warnings` clean, `xcodebuild test` 79 passed on the iPhone 17 Pro simulator. One earlier `--lib` run failed `tui::app::playback::tests::test_explicit_player_incompatible_does_not_launch_alternative`, which passes alone and in two later full runs: an environment-dependent player test, not this change.
- **L5 (decision)** — Online results stay out of the `get_ext_captions` cache, which is keyed per resource: a separate service method keeps provider captions and online results independent, so one cannot poison the other.
