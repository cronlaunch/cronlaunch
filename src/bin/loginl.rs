// src/bin/loginl.rs

use clap::Parser;
use std::path::PathBuf;

use cronlaunch::core::manager::LaunchAgentManager;
use cronlaunch::core::util::init_logging;

/// Manage login (RunAtLoad) launch agents on macOS.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Show login jobs
    #[arg(long)]
    show_all: bool,

    /// List login job labels
    #[arg(short, long)]
    list: bool,

    /// Remove a login job by label
    #[arg(short, long)]
    remove: Option<String>,

    /// Increase verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// A path used only to derive the label (positional)
    #[arg(
        value_name = "LABEL_PATH",
        required_unless_present_any = ["show_all", "list", "remove"]
    )]
    label_path: Option<PathBuf>,

    /// Handler program and args (positional, recommended after `--`)
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
        mgr.list_labels(false, true)?;
        return Ok(());
    }
    if let Some(label) = args.remove.as_deref() {
        mgr.remove(label)?;
        return Ok(());
    }

    let label_path = args
        .label_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("A label path is required"))?;
    if args.handler.is_empty() {
        anyhow::bail!("An executable handler is required");
    }

    mgr.create_login(label_path, &args.handler)
}
