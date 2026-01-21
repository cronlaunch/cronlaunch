// src/bin/watchl.rs

use clap::Parser;
use std::path::PathBuf;

use cronlaunch::core::manager::LaunchAgentManager;
use cronlaunch::core::util::init_logging;

/// Manage WatchPaths-based launch agents on macOS.
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
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// One or more directories to watch (positional)
    #[arg(
        value_name = "WATCH_PATH",
        num_args = 1..,
        required_unless_present_any = ["show_all", "list", "remove"]
    )]
    watch_path: Vec<PathBuf>,

    /// Handler program and args (positional)
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
        mgr.show_all()?;
        return Ok(());
    }
    if args.list {
        mgr.list_labels(true, false)?;
        return Ok(());
    }
    if let Some(label) = args.remove.as_deref() {
        mgr.remove(label)?;
        return Ok(());
    }

    if args.watch_path.is_empty() || args.handler.is_empty() {
        anyhow::bail!("A directory and executable handler is required");
    }

    mgr.create_watch(&args.watch_path, &args.handler)
}
