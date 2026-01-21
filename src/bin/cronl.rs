// src/bin/cronl.rs

use clap::Parser;

use cronlaunch::core::manager::LaunchAgentManager;
use cronlaunch::core::util::init_logging;

/// Manage launch agents on macOS with a crontab-like syntax.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Show all jobs
    #[arg(long)]
    show_all: bool,

    /// List job labels
    #[arg(short, long)]
    list: bool,

    /// Remove a job by label
    #[arg(short, long)]
    remove: Option<String>,

    /// Increase verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Crontab-like schedule string, e.g. "0 1 * * *"
    #[arg(
        value_name = "SCHEDULE",
        required_unless_present_any = ["show_all", "list", "remove"]
    )]
    crontab: Option<String>,

    /// Handler program and args (must come after `--`)
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
        mgr.list_labels(false, false)?;
        return Ok(());
    }
    if let Some(label) = args.remove.as_deref() {
        mgr.remove(label)?;
        return Ok(());
    }

    let crontab = args
        .crontab
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("A crontab definition is required"))?;
    mgr.create_cron_parts(crontab, &args.handler)
}
