// src/core/runtime.rs

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Abstraction over OS interactions needed by the manager layer.
///
/// This trait exists to:
/// - Make the core logic testable by swapping in a fake implementation.
/// - Centralize platform-specific behavior (user id, filesystem paths, process launching).
/// - Keep the manager focused on LaunchAgent plist composition rather than I/O details.
pub trait Runtime: Send + Sync {
    /// Return the effective user id to target for launchctl "gui/<uid>" domains.
    ///
    /// Effective uid is used because launchctl operations are scoped to the login session
    /// associated with that uid, and sudo contexts often require explicitly targeting it.
    fn uid(&self) -> u32;

    /// Return the LaunchAgents directory path (typically ~/Library/LaunchAgents).
    ///
    /// This is intentionally provided by the runtime so tests can redirect it.
    fn launch_agents_dir(&self) -> PathBuf;

    /// Resolve an executable name to an absolute path using PATH lookup.
    ///
    /// This is required to persist stable paths into plists, avoiding reliance on PATH
    /// behavior inside launchd, which can be more restrictive than an interactive shell.
    fn which(&self, cmd: &str) -> Result<PathBuf>;

    /// Validate that the given path exists, is a file, and is executable.
    ///
    /// This ensures launchd jobs fail fast at creation time instead of silently creating
    /// a job that will never run.
    fn ensure_executable_file(&self, path: &Path) -> Result<()>;

    /// Run a system command and surface non-zero exit status as an error.
    ///
    /// This is used for launchctl subcommands and kept behind the runtime to allow
    /// tests to record invocations rather than executing them.
    fn run_command(&self, program: &str, args: &[String]) -> Result<()>;
}
