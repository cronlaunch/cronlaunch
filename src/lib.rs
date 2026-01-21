// src/lib.rs

//! Library entry-point for the `cronlaunch` crate.
//!
//! This crate primarily exists to share the core LaunchAgent management logic
//! across multiple small CLI binaries (`cronl`, `watchl`, `loginl`).

pub mod core;
