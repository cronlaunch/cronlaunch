// src/core/mod.rs

//! Core implementation for interacting with macOS `launchd` LaunchAgents.
//!
//! The main design goal is to keep the OS interactions (filesystem, launching commands)
//! behind a small runtime abstraction so logic can be tested without calling `launchctl`.

pub mod constants;
pub mod default_runtime;
pub mod manager;
pub mod plist_io;
pub mod runtime;
pub mod util;
