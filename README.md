# cronlaunch

Command-line helpers for managing macOS LaunchAgents in a more Unix-like way.
This crate supports macOS only; it requires a logged-in GUI session and `launchctl`.

## Install

Install the latest release from crates.io:

```sh
cargo install cronlaunch --locked
```

This installs `cronl`, `watchl`, and `loginl` to Cargo's bin directory (normally
`~/.cargo/bin`). Re-run the command to upgrade to the latest release.

`cargo install` can download the crate on other systems, but the installed commands
require macOS and will not operate elsewhere.

## Release

Releases are published to crates.io by GitLab CI/CD when a protected `vX.Y.Z` tag
matches the package version in `Cargo.toml`. Configure a masked, hidden, protected
GitLab CI/CD variable named `CARGO_REGISTRY_TOKEN` with a crates.io API token before
creating the first release. Set `CI_RUN_MACOS_SMOKE=true` to run the launchctl smoke
test on a GitLab macOS runner tagged `macos` before publishing.

## Binaries

- `cronl` - create/list/remove scheduled LaunchAgents using a crontab-like string
- `watchl` - create/list/remove WatchPaths-based LaunchAgents
- `loginl` - create/list/remove login (RunAtLoad) LaunchAgents

## Examples

```bash
# Create a cron job
cronl "0 1 * * *" -- /usr/bin/true
cronl "0 1 * * *" -- archive.rb ${HOME}/Downloads/archive

# Show all jobs
cronl --show-all

# Remove by label
cronl --remove com.local.true
cronl --remove com.local.archive

# Watch a directory and run a handler when it changes
watchl ~/Desktop -- ~/.local/usr/bin/desktop.rb

# Watch a directory and run a handler with flags/args
watchl ~/Downloads -- /usr/bin/python3 -m http.server 8000

# Create a login job (run at login)
loginl ~/Desktop -- ~/.local/usr/bin/desktop.rb
```

## Notes

* The LaunchAgents directory is `~/Library/LaunchAgents`.
* Existing plist files are not overwritten.
* For `watchl` and `loginl`, using `--` before the handler command is recommended so handler arguments that start with `-` are not parsed as options for `watchl`/`loginl`.
* Wildcard schedule fields are stored as omitted launchd calendar fields. Numeric fields must be within launchd's valid ranges.
* launchd coalesces missed calendar events while the Mac is asleep and runs the job after wake. Inspect `launchctl print gui/$(id -u)/<label>` and the unified log when troubleshooting.

## Project files

A bird's-eye view of the project files:

```
$ date && tree --gitignore    
Fri Jan 23 16:04:18 CST 2026
.
├── Cargo.lock
├── Cargo.toml
├── CONTRIBUTING.md
├── LICENSE.md
├── README.md
├── src
│   ├── bin
│   │   ├── cronl.rs
│   │   ├── loginl.rs
│   │   └── watchl.rs
│   ├── core
│   │   ├── constants.rs
│   │   ├── default_runtime.rs
│   │   ├── manager.rs
│   │   ├── mod.rs
│   │   ├── plist_io.rs
│   │   ├── runtime.rs
│   │   └── util.rs
│   └── lib.rs
└── tests
    └── cli.rs

5 directories, 17 files
```
