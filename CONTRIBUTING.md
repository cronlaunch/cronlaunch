# Contributing

Thanks for contributing! This project provides a small set of Rust binaries for managing macOS LaunchAgents in a crontab-like, Unix-y way.

## Requirements

- macOS (this project targets LaunchAgents / `launchctl`)
- Rust toolchain (stable)
  - Install via rustup: https://rustup.rs
- Xcode Command Line Tools (for basic build tooling)
  - `xcode-select --install`

## Project layout

- `src/lib.rs`: shared LaunchAgent logic (plist creation, load/unload, etc.)
- `src/bin/cronl.rs`: crontab-like scheduling (`StartCalendarInterval`)
- `src/bin/watchl.rs`: filesystem path watching (`WatchPaths`)
- `src/bin/loginl.rs`: run at login (`RunAtLoad`)

## Setup

Make sure the correct Rust version is being used.

```sh
rustup toolchain install
```

## Build

From the repo root:

```bash
cargo build
```

Build release binaries:

```bash
cargo build --release
```

Binaries will be located at:

* Debug: `target/debug/{cronl,watchl,loginl}`
* Release: `target/release/{cronl,watchl,loginl}`

## Test

Run tests.

```sh
cargo test
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
cargo llvm-cov
```

## Run locally

You can run via Cargo without installing:

```bash
cargo run --bin cronl -- --help
cargo run --bin watchl -- --help
cargo run --bin loginl -- --help
```

Example (cron job):

```bash
cargo run --bin cronl -- "0 * * * *" -- /usr/bin/true
```

Example (watch a directory):

```bash
cargo run --bin watchl -- "$HOME/Downloads" -- /usr/bin/true
```

Example (run at onlogin):

```bash
cargo run --bin loginl -- "$HOME/bin/my-job" -- /usr/bin/true
```

## Tests

Run all tests:

```bash
cargo test
```

If you add parsing logic (cron syntax, etc.), please include unit tests under `src/` and integration tests under `tests/` where appropriate.

## Formatting

This project follows standard Rust formatting.

Format code:

```bash
cargo fmt
```

Check formatting (CI-friendly):

```bash
cargo fmt --check
```

## Linting

We use Clippy (the Rust linter) for lightweight static analysis.

Run Clippy:

```bash
cargo clippy
```

Treat warnings as errors:

```bash
cargo clippy -- -D warnings
```

## Code style guidelines

* Prefer small, testable functions.
* Return `anyhow::Result<T>` from fallible helpers.
* Avoid `unwrap()` in non-test code (use `?` and good error messages).
* Keep CLI behavior stable (flags and output are user-facing).

## macOS behavior notes

This tool manages LaunchAgents by writing plist files to:

`~/Library/LaunchAgents`

And loading/unloading them using `launchctl`. Behavior can differ slightly across macOS versions, so if you change anything in job loading/unloading, please call it out in the PR description and test on a recent macOS release.

## Submitting changes

1. Create a branch from `main`
2. Make your change
3. Ensure these pass locally:

   * `cargo test`
   * `cargo fmt --check`
   * `cargo clippy -- -D warnings`
4. Open a PR (pull request) with:

   * what changed
   * why it changed
   * how you tested it

## Commit guidance

No strict format required, but helpful commits tend to look like:

* `cron: improve schedule parsing errors`
* `launchd: avoid overwriting existing plist files`
* `watch: document WatchPaths behavior`

## Security

If you believe you've found a security issue, please do not open a public issue. Instead, contact the maintainers privately.

---

Thanks again for contributing.
