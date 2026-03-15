# Contributing to Meriadoc

## Prerequisites

- Rust stable toolchain — install via [rustup](https://rustup.rs)

## Getting started

```bash
git clone https://github.com/segunmo/meriadoc
cd meriadoc
cargo build
```

## Development commands

Meriadoc uses its own spec file for development tasks. If you have a working binary:

```bash
meriadoc job ci              # lint + fmt check + test (same as CI)
meriadoc task test-one       # run a single test (prompts for name)
meriadoc task build-release  # optimized build
```

Or use cargo directly:

```bash
cargo fmt                    # format code
cargo fmt --check            # check formatting (CI)
cargo clippy -- -D warnings  # lint (CI)
cargo test                   # run all tests (CI)
cargo test <name>            # run a single test
```

## Before submitting a PR

- Run `cargo fmt` — the CI will reject unformatted code
- Run `cargo clippy -- -D warnings` — the CI treats warnings as errors
- Run `cargo test` — all tests must pass
- Keep commits focused; one logical change per commit

## Code style

- Follow the existing module structure — `cli/` for CLI definitions, `app/commands/` for handlers, `core/` for domain logic, `repo/` for filesystem operations
- No business logic in `main.rs` or `cli/`; no CLI parsing in `app/` or `core/`
- Prefer explicit error types over `anyhow` in library code

## Submitting a PR

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Open a pull request with a clear description of what changed and why
4. The CI must be green before merging

For larger changes, open an issue first to discuss the approach.
