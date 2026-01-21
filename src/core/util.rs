// src/core/util.rs

use anyhow::{anyhow, Context, Result};
use libc::geteuid;
use log::LevelFilter;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn init_logging(verbosity: u8) {
    // env_logger defaults to INFO; more -v -> DEBUG/TRACE.
    let level = match verbosity {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(level);
    let _ = builder.try_init();
}

pub fn uid() -> u32 {
    // Effective UID is what launchctl expects under sudo contexts.
    unsafe { geteuid() as u32 }
}

pub fn ensure_executable_in_path(cmd: &str) -> Result<PathBuf> {
    let path =
        which::which(cmd).with_context(|| format!("Cannot find executable in PATH: {cmd}"))?;
    Ok(path)
}

pub fn ensure_executable_file(path: &Path) -> Result<()> {
    let md = std::fs::metadata(path)
        .with_context(|| format!("Cannot stat executable: {}", path.display()))?;
    if !md.is_file() {
        return Err(anyhow!("Given path is not a file: {}", path.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if (md.permissions().mode() & 0o111) == 0 {
            return Err(anyhow!("Given file is not executable: {}", path.display()));
        }
    }
    Ok(())
}

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
        let td = TempDir::new().expect("tempdir");
        let p = td.path().join("nope");
        let err = ensure_executable_file(&p).unwrap_err();
        assert!(err.to_string().contains("Cannot stat executable"));
    }

    #[test]
    fn ensure_executable_file_rejects_non_executable() {
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
