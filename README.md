Yep - with the Clap changes you made, it’s best to show the `--` separator anywhere the “handler” might include flags (or just as a consistent habit).

Here’s an updated `README.md` with minimal changes (just fixes + clearer examples).

````md
# cronlaunch

Simple command-line helpers for managing macOS LaunchAgents in a more Unix-like way.

## Binaries

- `cronl` - create/list/remove scheduled LaunchAgents using a crontab-like string
- `watchl` - create/list/remove WatchPaths-based LaunchAgents
- `loginl` - create/list/remove login (RunAtLoad) LaunchAgents

## Examples

```bash
# Create a cron job
cronl "0 1 * * * archive.rb ${HOME}/Downloads/archive"

# Show all jobs
cronl --show-all

# Remove by label
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

## Project files

A bird's-eye view of the project files:

```
$ date && tree --gitignore
Wed Jan 21 10:22:53 CST 2026
.
├── CONTRIBUTING.md
├── Cargo.lock
├── Cargo.toml
├── LICENSE.md
├── README.md
└── src
    ├── bin
    │   ├── cronl.rs
    │   ├── loginl.rs
    │   └── watchl.rs
    ├── core
    │   ├── constants.rs
    │   ├── manager.rs
    │   ├── mod.rs
    │   ├── plist_io.rs
    │   └── util.rs
    └── lib.rs

4 directories, 14 files
```

