// src/bin/loginl.rs

use clap::Parser;
use std::path::PathBuf;

use cronlaunch::core::manager::LaunchAgentManager;
use cronlaunch::core::util::init_logging;

/// Manage login (RunAtLoad) launch agents on macOS.
///
/// Login agents run at user login and are represented in launchd via RunAtLoad=true.
/// This binary provides a dedicated CLI so listing and filtering can stay intuitive.
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
    ///
    /// Extra verbosity is useful for debugging executable resolution and launchctl behavior.
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// A path used only to derive the label (positional)
    ///
    /// This allows stable label generation without forcing the label to match the handler path.
    #[arg(
        value_name = "LABEL_PATH",
        required_unless_present_any = ["show_all", "list", "remove"]
    )]
    label_path: Option<PathBuf>,

    /// Handler program and args (positional, recommended after `--`)
    ///
    /// Using trailing argv parsing avoids conflicts between handler flags and loginl flags.
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
        mgr.show_all(false, true)?;
        return Ok(());
    }
    if args.list {
        // Restrict listing to RunAtLoad jobs so output matches the binary's purpose.
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
