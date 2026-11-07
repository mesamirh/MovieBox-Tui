# Contributing to MovieBox-Tui

Thanks for taking the time to contribute. Bug reports, ideas, docs improvements, and pull requests are all welcome.

If you're planning a large or breaking change, please open an issue first so we can talk it through before you invest significant time.

## Getting set up

You'll need Rust **1.90 or newer** (edition 2024). Install it via [rustup.rs](https://rustup.rs/).

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/<your-username>/MovieBox-Tui.git
cd MovieBox-Tui

# Add the upstream remote to keep in sync
git remote add upstream https://github.com/mesamirh/MovieBox-Tui.git

# IMPORTANT: Enable our pre-commit hooks to ensure your code formatting and lints pass
git config core.hooksPath .githooks

# Build and run
cargo run --release
```

To test playback and download features locally, install `mpv` (see [Media Players](docs/players.md)).

## Project layout

The app is message-driven and organized into focused modules. Full maps live in
[`docs/architecture.md`](docs/architecture.md) and [`docs/modules.md`](docs/modules.md).

Short version:

- `src/tui/app/`: The application object (`App`). `run.rs` holds the thin
  `handle_action` dispatcher that routes every `Action` to a `handle_*` method in its
  module (`run.rs`, `requests.rs`, `search.rs`, `playback.rs`, `download.rs`, `favorites.rs`,
  `navigation.rs`, `tv.rs`, `addons.rs`, `keyboard.rs`, `mouse.rs`, `system.rs`).
- `src/tui/`: UI state, event loop plumbing, slash commands (`commands.rs`), screens, themes, and widgets.
- `src/providers/`: HTTP clients for streaming sources (`moviebox`, `fourkhdhub`, `dramachi`,
  `bdix`), community HTTP addons (`addons/`), and Live TV playlists (`tv/`).
- `src/service.rs`: Unified headless multi-provider client & engine.
- `src/proxy.rs`: Local loopback HTTP proxy for authenticated CloudFront streams.
- `src/download.rs`: Background multi-segment media downloading.
- `src/cache.rs`: Local binary MessagePack disk caching.
The app is message-driven. User input and background tasks produce `Action` values,
handled by the dispatcher in `src/tui/app/run.rs`. When adding behavior, prefer adding a
new `Action` variant over blocking the UI thread.

## Workflow

1. Create a branch off `main`:
   ```bash
   git checkout main
   git pull upstream main
   git checkout -b feat/short-description
   ```
2. Make your change in small, logical commits.
3. Push and open a pull request against `main`.

## Local Checks & Pre-Commit Hook

Formatting and linting are enforced automatically: the pre-commit hook (enabled during
setup with `git config core.hooksPath .githooks`) runs `cargo fmt --check` and
`cargo clippy --all-targets --locked -- -D warnings` on every commit, and the
commit is rejected if either fails. You do not need to run them manually before every
commit, but you should still run the full local/CI parity checks before opening a PR or
cutting a release.

If the hook rejects a commit, run `cargo fmt` to auto-fix the formatting, then stage and
commit again.

For release readiness, static checks are not enough on their own. Before declaring a
release production-ready, work through [`docs/release-checklist.md`](docs/release-checklist.md).

**Guidelines:**

- Follow idiomatic Rust and standard `rustfmt` defaults. Don't hand-format.
- Keep the async, message-passing architecture intact.
- Avoid panics on paths that handle network or user input.
- Don't add new dependencies without a good reason. Mention it in the PR if you do.

## Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org/). Keep the subject concise and in the imperative mood.

Examples:

- `feat: add support for custom mpv arguments`
- `fix: prevent panic when clipboard is unavailable`
- `docs: document /browse categories`
- `refactor: extract stream resolution into helper`

Common types: `feat`, `fix`, `refactor`, `docs`, `style`, `perf`, `chore`.

## Pull requests

- Keep PRs focused on a single concern. Large PRs mixing unrelated changes may be asked to be split.
- In your PR description, explain what changed and why. Link related issues (`Closes #12`) and include screenshots or recordings for anything visible in the UI.
- Never commit `target/`, editor settings, or debug dump files.

## License

By contributing, you agree that your contributions will be dual-licensed under the [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE) licenses, consistent with the rest of the project.
