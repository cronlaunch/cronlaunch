// src/core/util.rs

use anyhow::{anyhow, Context, Result};
use libc::geteuid;
use log::LevelFilter;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Initialize logging based on CLI verbosity.
///
/// env_logger defaults to INFO, and higher verbosity is mapped to DEBUG/TRACE.
/// The builder is configured once per process; repeated initialization attempts
/// are ignored to prevent panics in tests and multi-binary usage.
pub fn init_logging(verbosity: u8) {
    // env_logger defaults to INFO; more -v -> DEBUG/TRACE.
    let level = match verbosity {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(level);

    // Ignore "already initialized" errors to keep binaries idempotent and test-friendly.
    let _ = builder.try_init();
}

pub fn uid() -> u32 {
    // Effective UID is what launchctl expects under sudo contexts.
    unsafe { geteuid() as u32 }
}

/// Ensure an executable is discoverable via PATH and return its absolute path.
///
/// A stable absolute path is preferred for launchd plists because the PATH
/// available to launchd jobs can be different from an interactive shell.
pub fn ensure_executable_in_path(cmd: &str) -> Result<PathBuf> {
    let path =
        which::which(cmd).with_context(|| format!("Cannot find executable in PATH: {cmd}"))?;
    Ok(path)
}

/// Ensure a path points to an executable file.
///
/// This prevents creating LaunchAgent plists that point to missing or non-executable
/// targets, which would otherwise fail later at runtime and be harder to diagnose.
pub fn ensure_executable_file(path: &Path) -> Result<()> {
    let md = std::fs::metadata(path)
        .with_context(|| format!("Cannot stat executable: {}", path.display()))?;
    if !md.is_file() {
        return Err(anyhow!("Given path is not a file: {}", path.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // On Unix, require at least one execute bit to be set. This is a pragmatic check
        // because launchd will not run a non-executable file even if it exists.
        if (md.permissions().mode() & 0o111) == 0 {
            return Err(anyhow!("Given file is not executable: {}", path.display()));
        }
    }
    Ok(())
}

/// Execute a command and treat non-zero exit status as an error.
///
/// This wrapper provides:
/// - Debug logging of the invocation for troubleshooting.
/// - A consistent error message surface for callers.
/// - A single place to attach context to process spawning errors.
pub fn run_command(program: &str, args: &[String]) -> Result<()> {
    log::debug!("Executing: {} {:?}", program, args);
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to run: {program}"))?;
    if !status.success() {
        return Err(anyhow!(
            "Command failed: {} {:?} (status={})",
            program,
            args,
            status
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn ensure_executable_file_rejects_nonexistent() {
        // A missing file should fail at metadata lookup to produce a contextual error.
        // This ensures callers get an actionable message rather than a generic "not found".
        let td = TempDir::new().expect("tempdir");
        let p = td.path().join("nope");
        let err = ensure_executable_file(&p).unwrap_err();
        assert!(err.to_string().contains("Cannot stat executable"));
    }

    #[test]
    fn ensure_executable_file_rejects_non_executable() {
        // A file that exists but lacks executable bits should be rejected on Unix.
        // This prevents generating LaunchAgents that will never run.
        let td = TempDir::new().expect("tempdir");
        let p = td.path().join("file");
        fs::write(&p, "hello").expect("write");

        let err = ensure_executable_file(&p).unwrap_err();
        assert!(
            err.to_string().contains("not executable")
                || err.to_string().contains("not executable")
        );
    }

    #[test]
    fn ensure_executable_file_accepts_executable() {
        // Verify the happy path by setting an executable mode on Unix.
        // This guards against regressions in permission checks.
        let td = TempDir::new().expect("tempdir");
        let p = td.path().join("exe");
        fs::write(&p, "#!/bin/sh\necho ok\n").expect("write");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&p).expect("metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&p, perms).expect("chmod");
        }

        ensure_executable_file(&p).expect("should be executable");
    }
}
