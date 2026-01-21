// src/core/constants.rs

use std::path::PathBuf;

/// Namespace prefix used to derive job labels.
///
/// A fixed namespace makes labels predictable and avoids accidental collisions
/// with system-provided jobs.
pub const LABEL_NAMESPACE: [&str; 2] = ["com", "local"];

/// Launchd calendar interval keys (mirrors the Ruby constant order).
///
/// The order is important because the schedule parsing and printing logic
/// assumes these fields correspond to the five cron-style columns.
pub const INTERVALS: [&str; 5] = ["Minute", "Hour", "Day", "Month", "Weekday"];

/// Special schedule string used when a plist indicates a login job.
///
/// This is not a launchd value; it is a display sentinel used by this tool
/// to round-trip "RunAtLoad" jobs through a human-readable format.
pub const ON_LOGIN: &str = "@login";

/// Default encoding for plist XML generation.
///
/// launchd plists are XML and generally assume UTF-8; keeping this explicit
/// documents intent even though the plist crate handles encoding internally.
pub const DEFAULT_ENCODING: &str = "UTF-8";

/// LaunchAgents directory: ~/Library/LaunchAgents
///
/// This is the canonical per-user location for user agents.
/// A fallback of "." is used if HOME cannot be determined to avoid panicking,
/// which is useful for tests and constrained environments.
pub fn launch_agents_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("Library").join("LaunchAgents")
}
