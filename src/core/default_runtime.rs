// src/core/default_runtime.rs

use crate::core::constants::launch_agents_dir;
use crate::core::runtime::Runtime;
use crate::core::util::{ensure_executable_file, ensure_executable_in_path, run_command, uid};
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Production runtime implementation backed by real OS calls.
///
/// This type is intentionally small and Copy so it can be used as a default value
/// without heap allocation or shared global state.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultRuntime;

impl Runtime for DefaultRuntime {
    fn uid(&self) -> u32 {
        uid()
    }

    fn launch_agents_dir(&self) -> PathBuf {
        launch_agents_dir()
    }

    fn which(&self, cmd: &str) -> Result<PathBuf> {
        ensure_executable_in_path(cmd)
    }

    fn ensure_executable_file(&self, path: &Path) -> Result<()> {
        ensure_executable_file(path)
    }

    fn run_command(&self, program: &str, args: &[String]) -> Result<()> {
        run_command(program, args)
    }
}
