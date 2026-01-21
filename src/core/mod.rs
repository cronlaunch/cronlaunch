// src/core/mod.rs

// This module groups the core building blocks used by all binaries.
// Keeping the surface area in "core" makes it easier to reuse logic across
// cron-style, watch-based, and login-based job creation without duplication.

pub mod constants;
pub mod default_runtime;
pub mod manager;
pub mod plist_io;
pub mod runtime;
pub mod util;
