// src/lib.rs

// Library entry-point for the `cronlaunch` crate.
//
// This crate is organized as a small library with multiple binaries.
// The library exposes the shared "core" module so each binary can remain thin
// and focused on CLI argument parsing.

pub mod core;
