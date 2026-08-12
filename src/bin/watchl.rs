// src/bin/watchl.rs

use clap::Parser;
use std::path::PathBuf;

use cronlaunch::core::manager::LaunchAgentManager;
use cronlaunch::core::util::init_logging;

/// Manage WatchPaths-based launch agents on macOS.
///
/// This binary exists to provide a focused interface for file-watcher jobs where launchd
/// triggers execution when the watched directories change.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Show watchers
    #[arg(long)]
    show_all: bool,

    /// List watcher labels
    #[arg(short, long)]
    list: bool,

    /// Remove a watcher by label
    #[arg(short, long)]
    remove: Option<String>,

    /// Increase verbosity (-v, -vv)
    ///
    /// Verbosity is passed into env_logger configuration to help diagnose PATH resolution
    /// and launchctl execution behavior.
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// One or more directories to watch (positional)
    ///
    /// This is parsed as PathBuf so filesystem validation can occur before plist creation.
    #[arg(
        value_name = "WATCH_PATH",
        num_args = 1..,
        required_unless_present_any = ["show_all", "list", "remove"]
    )]
    watch_path: Vec<PathBuf>,

    /// Handler program and args (positional)
    ///
    /// The handler is parsed as trailing argv to preserve arguments that start with "-"
    /// and to avoid treating handler flags as watchl flags.
    #[arg(
        value_name = "HANDLER",
        num_args = 1..,
        last = true,
        allow_hyphen_values = true,
        required_unless_present_any = ["show_all", "list", "remove"]
    )]
    handler: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    init_logging(args.verbose);

    let mgr = LaunchAgentManager::default();

    if args.show_all {
        mgr.show_all(true, false)?;
        return Ok(());
    }
    if args.list {
        // Restrict listing to WatchPaths jobs so output matches the binary's purpose.
        mgr.list_labels(true, false)?;
        return Ok(());
    }
    if let Some(label) = args.remove.as_deref() {
        mgr.remove(label)?;
        return Ok(());
    }

    // Defensive validation: clap should enforce these, but explicit checks provide a clearer
    // error message if the CLI schema changes later.
    if args.watch_path.is_empty() || args.handler.is_empty() {
        anyhow::bail!("A directory and executable handler is required");
    }

    mgr.create_watch(&args.watch_path, &args.handler)
}
