// src/core/constants.rs

use std::path::PathBuf;

/// Namespace prefix used to derive job labels.
pub const LABEL_NAMESPACE: [&str; 2] = ["com", "local"];

/// Launchd calendar interval keys (mirrors the Ruby constant order).
pub const INTERVALS: [&str; 5] = ["Minute", "Hour", "Day", "Month", "Weekday"];

/// Special schedule string used when a plist indicates a login job.
pub const ON_LOGIN: &str = "@login";

/// Default encoding for plist XML generation.
pub const DEFAULT_ENCODING: &str = "UTF-8";

/// LaunchAgents directory: ~/Library/LaunchAgents
pub fn launch_agents_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("Library").join("LaunchAgents")
}
