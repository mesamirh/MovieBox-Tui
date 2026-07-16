# Contributing to MovieBox

Thank you for your interest in contributing to MovieBox!

## Pull Request Process

1. Ensure any install or build dependencies are removed before the end of the layer when doing a build.
2. Update the README.md with details of changes to the interface, this includes new environment variables, exposed ports, useful file locations and container parameters.
3. Ensure your code passes all linting (`cargo clippy -- -D warnings`) and formatting (`cargo fmt --check`) checks.
4. Add tests for any new features or bug fixes.
5. The PR will be reviewed by a maintainer and merged when approved.

## Development setup

Ensure you have the latest stable Rust compiler.
```sh
rustup default stable
```

### Running Tests

Run the full test suite locally:
```sh
cargo test
```

### Code Style

We follow standard Rust idioms.
All code must be formatted using `rustfmt`.

```sh
cargo fmt
```

Use `clippy` to catch common mistakes.
```sh
cargo clippy --all-targets --all-features -- -D warnings
```
