# Contributing to MovieBox-Tui

Thanks for taking the time to contribute. Bug reports, ideas, docs improvements, and pull requests are all welcome.

If you're planning a large or breaking change, please open an issue first so we can talk it through before you invest significant time.

## Getting set up

You'll need Rust **1.85 or newer** (edition 2024). Install it via [rustup.rs](https://rustup.rs/).

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/<your-username>/MovieBox-Tui.git
cd MovieBox-Tui

# Add the upstream remote to keep in sync
git remote add upstream https://github.com/mesamirh/MovieBox-Tui.git

# Build and run
cargo run --release
```

To test playback and download features locally, install `mpv` (see the [README](README.md#requirements)).

## Project layout

The app has two layers. See the [Project layout section](README.md#project-layout) in the README for the full tree.

Short version:

- `src/providers/moviebox/` is the HTTP client for the MovieBox API.
- `src/tui/` is the interface, built on Ratatui.
- `src/tui/app.rs` is the central event loop. Every user action and background task funnels through its `match` statement. That's the best place to start reading.

The app is message-driven. User input and background tasks produce `Action` values, handled in `app.rs`. When adding behavior, prefer adding a new `Action` variant over blocking the UI thread.

## Workflow

1. Create a branch off `main`:

   ```bash
   git checkout main
   git pull upstream main
   git checkout -b feat/short-description
   ```

2. Make your change in small, logical commits.
3. Run the checks below.
4. Push and open a pull request against `main`.

## Before opening a PR

Run these locally. All must pass.

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Guidelines:

- Follow idiomatic Rust and standard `rustfmt` defaults. Don't hand-format.
- Keep the async, message-passing architecture intact.
- Avoid panics on paths that handle network or user input.
- Don't add new dependencies without a good reason. Mention it in the PR if you do.

## Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org/). Keep the subject concise and in the imperative mood.

Examples:

```
feat: add support for custom mpv arguments
fix: prevent panic when clipboard is unavailable
docs: document /anime discover command
refactor: extract stream resolution into helper
```

Common types: `feat`, `fix`, `refactor`, `docs`, `style`, `perf`, `chore`.

## Pull requests

Keep PRs focused on a single concern. Large PRs mixing unrelated changes may be asked to be split.

In your PR description, explain what changed and why. Link related issues (`Closes #12`) and include screenshots or recordings for anything visible in the UI.

Never commit `target/`, editor settings, or debug dump files.

## License

By contributing, you agree that your contributions will be dual-licensed under the [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE) licenses, consistent with the rest of the project.
