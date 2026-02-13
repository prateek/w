//! Experimental integration APIs for external tooling.
//!
//! Worktrunk is primarily a CLI, and its Rust library APIs are not stable.
//! This module provides a narrow, explicitly-versioned surface area intended for
//! wrapper CLIs and other integrations.
//!
//! Stability:
//! - **This is experimental** and may change at any time.
//! - Prefer the versioned modules (e.g. [`v1`]) rather than importing internal
//!   modules or CLI output code.

pub mod v1;
