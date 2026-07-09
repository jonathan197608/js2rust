//! js2rust.toml configuration — thin re-export from js2zig-core.
//!
//! All types and loading logic now live in `js2zig_core::toml_config`.
//! This module re-exports them for backward compatibility so that
//! downstream code using `js2rust_bridge::config::*` continues to work.

pub use js2zig_core::toml_config::{BuildSection, HostFnToml, Js2rustConfig, ProjectSection};
